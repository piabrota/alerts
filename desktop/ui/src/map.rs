//! The map page, the home of the app: MapLibre GL JS with the OpenFreeMap `positron` tiles (no
//! key, no account), the IBGE UF borders, a legend and the alert list.
//!
//! The layout follows the public disaster maps (INMET Alert-AS, Google Crisis Map): each UF takes
//! the color of the highest severity of the alerts that name it, the legend turns a severity on
//! and off, a click on a UF filters the list, and a click on an alert outlines its UFs.
//! `alerts_core::Alert` has no geometry, so the map draws only the UF borders: the `map` skill
//! forbids a guessed shape. The list gives each area in text, and it is complete without the map.

use crate::alert_view::{self as view, AlertView, BBox, Filter, SEVERITIES, V0};
use crate::api;
use crate::marola::{self, Area, Out};
use alerts_core::Uf;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Serialize as _;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// A muted base map, so the alert colors stand out.
const STYLE: &str = "https://tiles.openfreemap.org/styles/positron";
/// The IBGE UF borders, from `just desktop-vendor` (`vendor/ibge/ufs.geojson`).
const UFS_URL: &str = "ibge/ufs.geojson";
/// South Brazil (PR, SC, RS), as `[lon, lat]`, before the borders load.
const CENTER: (f64, f64) = (-51.2, -27.5);
/// A UF is the most precise area an alert has: never zoom past it.
const MAX_ZOOM: f64 = 6.5;
/// A marola.dev area is a city: it may zoom closer, but an alert still covers its whole UF.
const AREA_ZOOM: f64 = 9.0;
/// The list reads the database again at this interval: `alerts poll` writes it from outside.
const REFRESH: Duration = Duration::from_secs(60);

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = maplibregl, js_name = Map)]
    type MapLibre;

    #[wasm_bindgen(constructor, catch, js_namespace = maplibregl, js_class = "Map")]
    fn new(options: &JsValue) -> Result<MapLibre, JsValue>;

    #[wasm_bindgen(method)]
    fn on(this: &MapLibre, event: &str, listener: &Closure<dyn FnMut(JsValue)>);

    #[wasm_bindgen(method, js_name = on)]
    fn on_layer(this: &MapLibre, event: &str, layer: &str, listener: &Closure<dyn FnMut(JsValue)>);

    #[wasm_bindgen(method)]
    fn remove(this: &MapLibre);

    #[wasm_bindgen(method, js_name = addControl)]
    fn add_control(this: &MapLibre, control: &JsValue, position: &str);

    #[wasm_bindgen(method, js_name = addSource)]
    fn add_source(this: &MapLibre, id: &str, source: &JsValue);

    #[wasm_bindgen(method, js_name = addLayer)]
    fn add_layer(this: &MapLibre, layer: &JsValue, before: Option<String>);

    #[wasm_bindgen(method, js_name = setFeatureState)]
    fn set_feature_state(this: &MapLibre, feature: &JsValue, state: &JsValue);

    #[wasm_bindgen(method, js_name = setFilter)]
    fn set_filter(this: &MapLibre, layer: &str, filter: &JsValue);

    #[wasm_bindgen(method, js_name = fitBounds)]
    fn fit_bounds(this: &MapLibre, bounds: &JsValue, options: &JsValue);

    #[wasm_bindgen(method, js_name = getStyle)]
    fn get_style(this: &MapLibre) -> JsValue;

    #[wasm_bindgen(method, js_name = getCanvas)]
    fn get_canvas(this: &MapLibre) -> JsValue;

    #[wasm_bindgen(js_namespace = maplibregl)]
    type NavigationControl;

    #[wasm_bindgen(constructor, js_namespace = maplibregl, js_class = "NavigationControl")]
    fn new() -> NavigationControl;

    #[wasm_bindgen(js_namespace = maplibregl)]
    type ScaleControl;

    #[wasm_bindgen(constructor, js_namespace = maplibregl, js_class = "ScaleControl")]
    fn new() -> ScaleControl;
}

fn has_maplibre() -> bool {
    js_sys::Reflect::has(&js_sys::global(), &"maplibregl".into()).unwrap_or(false)
}

fn get(v: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(v, &key.into()).unwrap_or(JsValue::UNDEFINED)
}

/// A JSON value as a plain JavaScript object, the form MapLibre reads.
fn js(v: &Value) -> JsValue {
    v.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .unwrap_or(JsValue::NULL)
}

fn options(container: &JsValue) -> JsValue {
    let o = js(&json!({
        "style": STYLE,
        "center": [CENTER.0, CENTER.1],
        "zoom": 4.6,
        "maxZoom": 12,
        "attributionControl": {
            "compact": false,
            "customAttribution": "Limites das UFs: IBGE",
        },
    }));
    let _ = js_sys::Reflect::set(&o, &"container".into(), container);
    o
}

/// The `fetch` of the page: the IBGE file is a static file next to the interface.
async fn fetch_json(url: &str) -> Result<Value, String> {
    let fail = |e: JsValue| e.as_string().unwrap_or_else(|| format!("{e:?}"));
    let fetch: js_sys::Function = get(&js_sys::global(), "fetch")
        .dyn_into()
        .map_err(|_| "no fetch".to_owned())?;
    let resp = JsFuture::from(js_sys::Promise::from(
        fetch.call1(&JsValue::NULL, &url.into()).map_err(fail)?,
    ))
    .await
    .map_err(fail)?;
    if get(&resp, "ok").as_bool() != Some(true) {
        return Err(format!("{url}: HTTP {:?}", get(&resp, "status").as_f64()));
    }
    let json: js_sys::Function = get(&resp, "json")
        .dyn_into()
        .map_err(|_| "no json".to_owned())?;
    let v = JsFuture::from(js_sys::Promise::from(json.call0(&resp).map_err(fail)?))
        .await
        .map_err(fail)?;
    serde_wasm_bindgen::from_value(v).map_err(|e| e.to_string())
}

/// The id of the first label layer of the base style: the UF colors go under the labels.
fn first_symbol_layer(map: &MapLibre) -> Option<String> {
    let layers: js_sys::Array = get(&map.get_style(), "layers").dyn_into().ok()?;
    layers
        .iter()
        .find(|l| get(l, "type").as_string().as_deref() == Some("symbol"))
        .and_then(|l| get(&l, "id").as_string())
}

/// Adds the UF source and its three layers: the severity fill, the borders, the selection.
fn add_uf_layers(map: &MapLibre, data: &Value) {
    map.add_source(
        "ufs",
        &js(&json!({ "type": "geojson", "data": data, "promoteId": "uf" })),
    );
    let mut color = vec![
        json!("match"),
        json!(["coalesce", ["feature-state", "sev"], 0]),
    ];
    for s in SEVERITIES {
        color.push(json!(view::rank(s)));
        color.push(json!(s.css_color()));
    }
    color.push(json!("#000000"));
    let hover = json!(["boolean", ["feature-state", "hover"], false]);
    let v0: Vec<&str> = V0.iter().map(|u| u.as_str()).collect();
    map.add_layer(
        &js(&json!({
            "id": "ufs-fill", "type": "fill", "source": "ufs",
            "paint": {
                "fill-color": color,
                "fill-opacity": ["case",
                    [">", ["coalesce", ["feature-state", "sev"], 0], 0],
                    ["case", hover, 0.5, 0.32],
                    ["case", hover, 0.08, 0]],
            },
        })),
        first_symbol_layer(map),
    );
    map.add_layer(
        &js(&json!({
            "id": "ufs-line", "type": "line", "source": "ufs",
            "paint": {
                "line-color": "#41566b",
                "line-opacity": 0.8,
                "line-width": ["case", ["in", ["get", "uf"], ["literal", v0]], 1.6, 0.6],
            },
        })),
        None,
    );
    map.add_layer(
        &js(&json!({
            "id": "ufs-selected", "type": "line", "source": "ufs",
            "filter": ["in", ["get", "uf"], ["literal", []]],
            "paint": { "line-color": "#14202b", "line-width": 3 },
        })),
        None,
    );
}

/// A click on a UF filters the list by it; the pointer over a UF shows its alerts. MapLibre needs
/// the layer before these listeners, so they come after [`add_uf_layers`].
fn bind_uf_events(
    map: &MapLibre,
    filter: RwSignal<Filter>,
    selected: RwSignal<Option<i64>>,
    hover: RwSignal<Option<Uf>>,
) {
    let on_click = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
        let Some(uf) = event_uf(&e) else { return };
        selected.set(None);
        filter.update(|f| f.uf = if f.uf == Some(uf) { None } else { Some(uf) });
    });
    map.on_layer("click", "ufs-fill", &on_click);
    on_click.forget();

    let on_move = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
        let uf = event_uf(&e);
        if hover.get_untracked() != uf {
            hover.set(uf);
        }
    });
    map.on_layer("mousemove", "ufs-fill", &on_move);
    on_move.forget();

    let on_leave = Closure::<dyn FnMut(JsValue)>::new(move |_| hover.set(None));
    map.on_layer("mouseleave", "ufs-fill", &on_leave);
    on_leave.forget();
}

/// The UF under the pointer of a MapLibre layer event.
fn event_uf(e: &JsValue) -> Option<Uf> {
    let features: js_sys::Array = get(e, "features").dyn_into().ok()?;
    get(&get(&features.get(0), "properties"), "uf")
        .as_string()?
        .parse()
        .ok()
}

fn set_cursor(map: &MapLibre, cursor: &str) {
    let _ = js_sys::Reflect::set(
        &get(&map.get_canvas(), "style"),
        &"cursor".into(),
        &cursor.into(),
    );
}

/// Fits the view to `b`, no closer than `max_zoom`. The left padding keeps the UFs clear of the
/// legend.
fn fit(map: &MapLibre, [w, s, e, n]: BBox, max_zoom: f64) {
    map.fit_bounds(
        &js(&json!([[w, s], [e, n]])),
        &js(&json!({ "padding": { "top": 40, "bottom": 40, "left": 200, "right": 60 }, "maxZoom": max_zoom, "duration": 600 })),
    );
}

fn now_label() -> String {
    String::from(js_sys::Date::new_0().to_locale_time_string("pt-BR"))
}

#[component]
pub fn MapPage() -> impl IntoView {
    let node = NodeRef::<leptos::html::Div>::new();
    let status = RwSignal::new(None::<String>);
    let alerts = RwSignal::new(Vec::<AlertView>::new());
    let unread = RwSignal::new(0usize);
    let updated = RwSignal::new(None::<String>);
    let filter = RwSignal::new(Filter::default());
    let selected = RwSignal::new(None::<i64>);
    let hover = RwSignal::new(None::<Uf>);
    let area = RwSignal::new(None::<&'static Area>);
    let ready = RwSignal::new(false);
    let boxes = StoredValue::new(HashMap::<Uf, BBox>::new());
    let map = StoredValue::new_local(None::<MapLibre>);

    let reload = move || {
        spawn_local(async move {
            match api::list("alerts").await {
                Ok(list) => {
                    let (ok, bad): (Vec<_>, Vec<_>) = list
                        .iter()
                        .map(AlertView::from_record)
                        .partition(Result::is_ok);
                    alerts.set(ok.into_iter().flatten().collect());
                    unread.set(bad.len());
                    updated.set(Some(now_label()));
                }
                Err(e) => status.set(Some(e.message())),
            }
        });
    };
    reload();
    if let Ok(timer) = set_interval_with_handle(reload, REFRESH) {
        on_cleanup(move || timer.clear());
    }

    // The map: created once, when its node exists.
    Effect::new(move |_| {
        let Some(el) = node.get() else { return };
        if map.with_value(Option::is_some) {
            return;
        }
        if !has_maplibre() {
            status.set(Some(
                "O MapLibre não carregou. Rode just desktop-vendor e abra o app de novo. A lista está completa sem o mapa.".into(),
            ));
            return;
        }
        let Ok(m) = MapLibre::new(&options(&el.into())) else {
            status.set(Some("O MapLibre não abriu o mapa.".into()));
            return;
        };
        m.add_control(&NavigationControl::new().into(), "top-right");
        m.add_control(&ScaleControl::new().into(), "bottom-right");

        let on_error = Closure::<dyn FnMut(JsValue)>::new(move |_| {
            status.set(Some(
                "O mapa base não carregou (sem rede?). A lista de alertas está completa sem o mapa.".into(),
            ));
        });
        m.on("error", &on_error);
        on_error.forget();

        let on_load = Closure::<dyn FnMut(JsValue)>::new(move |_| {
            spawn_local(async move {
                let borders = fetch_json(UFS_URL).await.and_then(view::ufs_geojson);
                match borders {
                    Ok((data, b)) => {
                        boxes.set_value(b);
                        map.with_value(|m| {
                            if let Some(m) = m {
                                add_uf_layers(m, &data);
                                bind_uf_events(m, filter, selected, hover);
                            }
                        });
                        ready.set(true);
                    }
                    Err(e) => status.set(Some(format!(
                        "Os limites das UFs não carregaram ({e}). Rode just desktop-vendor."
                    ))),
                }
            });
        });
        m.on("load", &on_load);
        on_load.forget();

        map.set_value(Some(m));
    });
    on_cleanup(move || {
        map.update_value(|m| {
            if let Some(m) = m.take() {
                m.remove();
            }
        })
    });

    // The color of each UF: the highest severity of the alerts that the filter keeps.
    Effect::new(move |_| {
        if !ready.get() {
            return;
        }
        let shown = view::visible(&alerts.get(), &filter.get().without_uf());
        let per_uf = view::by_uf(&shown);
        let hovered = hover.get();
        map.with_value(|m| {
            let Some(m) = m else { return };
            set_cursor(m, if hovered.is_some() { "pointer" } else { "" });
            boxes.with_value(|b| {
                for uf in b.keys() {
                    let sev = per_uf.get(uf).map_or(0, |(s, _)| view::rank(*s));
                    m.set_feature_state(
                        &js(&json!({ "source": "ufs", "id": uf.as_str() })),
                        &js(&json!({ "sev": sev, "hover": hovered == Some(*uf) })),
                    );
                }
            });
        });
    });

    // The outline and the view: the selected alert, else the clicked UF, else the marola.dev
    // area, else the v0 states.
    Effect::new(move |_| {
        if !ready.get() {
            return;
        }
        let picked: Option<Vec<Uf>> = selected
            .get()
            .and_then(|k| alerts.with(|all| all.iter().find(|a| a.key == k).map(|a| a.ufs.clone())))
            .or_else(|| filter.get().uf.map(|u| vec![u]));
        let outline: Vec<&str> = picked.iter().flatten().map(|u| u.as_str()).collect();
        let target = boxes.with_value(|b| match (&picked, area.get()) {
            (Some(ufs), _) => view::union(b, ufs.iter()).map(|b| (b, MAX_ZOOM)),
            (None, Some(a)) => Some((marola::area_bbox(a), AREA_ZOOM)),
            (None, None) => view::union(b, V0.iter()).map(|b| (b, MAX_ZOOM)),
        });
        map.with_value(|m| {
            let Some(m) = m else { return };
            m.set_filter(
                "ufs-selected",
                &js(&json!(["in", ["get", "uf"], ["literal", outline]])),
            );
            if let Some((b, zoom)) = target {
                fit(m, b, zoom);
            }
        });
    });

    let shown = Memo::new(move |_| view::visible(&alerts.get(), &filter.get()));
    let all_sources = Memo::new(move |_| view::sources(&alerts.get()));

    view! {
        <div class="home">
            <header class="home-head">
                <div>
                    <h1>"Mapa de alertas"</h1>
                    <p class="about">
                        "A cor de uma UF é a maior severidade dos alertas que citam a UF. Um clique numa UF filtra a lista. Um clique num alerta marca as UFs dele."
                    </p>
                </div>
                <div class="home-tools">
                    {move || filter.get().uf.map(|uf| view! {
                        <button class="chip on" title="Mostrar todas as UFs"
                            on:click=move |_| filter.update(|f| f.uf = None)>
                            {format!("UF: {uf} ×")}
                        </button>
                    })}
                    <select
                        aria-label="Área"
                        title="As áreas do marola.dev. A área só move o mapa: um alerta cobre a UF inteira."
                        on:change=move |ev| {
                            selected.set(None);
                            filter.update(|f| f.uf = None);
                            area.set(marola::area(&event_target_value(&ev)));
                        }
                    >
                        <option value="">"Sul (PR, SC, RS)"</option>
                        {marola::AREAS.iter().map(|a| view! {
                            <option value=a.id title=a.name selected=move || area.get() == Some(a)>
                                {format!("{} ({})", a.name, a.code)}
                            </option>
                        }).collect_view()}
                    </select>
                    <select
                        aria-label="Fonte"
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            filter.update(|f| f.source = (!v.is_empty()).then_some(v));
                        }
                    >
                        <option value="">"Todas as fontes"</option>
                        {move || all_sources.get().into_iter().map(|s| {
                            let chosen = filter.get().source.as_deref() == Some(s.as_str());
                            view! { <option value=s.clone() selected=chosen>{s.clone()}</option> }
                        }).collect_view()}
                    </select>
                    <label class="switch">
                        <input type="checkbox"
                            prop:checked=move || filter.get().tests
                            on:change=move |ev| filter.update(|f| f.tests = event_target_checked(&ev))
                        />
                        "Testes"
                    </label>
                    <button on:click=move |_| reload() title="Ler o banco de novo">"Atualizar"</button>
                </div>
            </header>
            {move || status.get().map(|m| view! { <p class="warn">{m}</p> })}
            {move || (unread.get() > 0).then(|| view! {
                <p class="warn">{format!(
                    "{} alertas têm um campo que o mapa não lê. A página Alertas mostra todos.",
                    unread.get()
                )}</p>
            })}
            <div class="home-body">
                <div class="map-wrap">
                    <div class="map" node_ref=node></div>
                    {move || hover.get().map(|uf| {
                        let shown = view::visible(&alerts.get(), &filter.get().without_uf());
                        let text = match view::by_uf(&shown).get(&uf) {
                            Some((s, 1)) => format!("1 alerta. Severidade {}.", s.label_pt_br()),
                            Some((s, n)) => format!("{n} alertas. Maior severidade: {}.", s.label_pt_br()),
                            None => "Nenhum alerta.".into(),
                        };
                        view! { <div class="map-hover"><strong>{uf.as_str()}</strong>" "{text}</div> }
                    })}
                    <Legend alerts filter />
                </div>
                <aside class="panel alert-list">
                    <div class="list-head">
                        <h2>{move || format!("Alertas ({})", shown.get().len())}</h2>
                        <span class="meta">{move || updated.get().map(|t| format!("lidos às {t}"))}</span>
                    </div>
                    {move || {
                        let list = shown.get();
                        if list.is_empty() {
                            let text = if alerts.with(Vec::is_empty) {
                                "Nenhum alerta guardado. Os alertas chegam só das fontes oficiais."
                            } else {
                                "Nenhum alerta com este filtro."
                            };
                            return view! { <p class="empty">{text}</p> }.into_any();
                        }
                        list.into_iter().map(|a| view! { <AlertCard a selected /> }).collect_view().into_any()
                    }}
                </aside>
            </div>
        </div>
    }
}

/// The legend: one row per severity, with its color, its label and its count. A click turns
/// the severity on or off. The label is the signal; the color repeats it.
#[component]
fn Legend(alerts: RwSignal<Vec<AlertView>>, filter: RwSignal<Filter>) -> impl IntoView {
    view! {
        <div class="legend" role="group" aria-label="Legenda">
            <p class="legend-title">"Severidade"</p>
            {SEVERITIES.into_iter().map(|s| {
                let off = move || filter.get().hidden.contains(&s);
                view! {
                    <button class="legend-row" class:off=off aria-pressed=move || (!off()).to_string()
                        on:click=move |_| filter.update(|f| f.toggle(s))>
                        <span class="swatch" style=format!("background:{}", s.css_color())></span>
                        <span class="legend-label">{s.label_pt_br()}</span>
                        <span class="legend-count">{move || alerts.with(|a| view::count(a, s))}</span>
                    </button>
                }
            }).collect_view()}
            <div class="legend-row static">
                <span class="swatch none"></span>
                <span class="legend-label">"sem alerta"</span>
            </div>
        </div>
    }
}

#[component]
fn AlertCard(a: AlertView, selected: RwSignal<Option<i64>>) -> impl IntoView {
    let key = a.key;
    let is_selected = move || selected.get() == Some(key);
    let ufs = a
        .ufs
        .iter()
        .map(|u| u.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let sev = format!("{:?}", a.severity);
    let test = a.is_test();
    view! {
        <article class="alert-item" class:selected=is_selected
            style=format!("--sev:{}", a.severity.css_color())
            on:click=move |_| selected.update(|s| *s = if *s == Some(key) { None } else { Some(key) })>
            <div class="alert-tags">
                <span class=format!("badge sev-{sev}")>{a.severity.label_pt_br()}</span>
                {test.then(|| view! { <span class="badge test">"TESTE"</span> })}
            </div>
            <h3>{a.headline.clone()}</h3>
            <p class="meta">{format!("{ufs} · {} · {}", a.source, a.sent)}</p>
            <p>{a.area.clone()}</p>
            {move || is_selected().then(|| view! {
                <p class="meta">{format!("Id na fonte: {}", a.alert_id)}</p>
                {a.url.clone().map(|u| view! {
                    <p class="meta"><Out href=u.clone()>"Página da fonte"</Out></p>
                })}
            })}
        </article>
    }
}
