//! The map page: MapLibre GL JS with OpenFreeMap tiles (no key, no account), and the alert list.
//!
//! `alerts_core::Alert` has only the area as text, so the map draws no shape: the `map` skill
//! forbids a guessed shape. The list gives each area in text. A geometry field in the core is
//! the next step, and then this page draws it.

use crate::api::{self, Record};
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

const STYLE: &str = "https://tiles.openfreemap.org/styles/liberty";
/// South Brazil (PR, SC, RS), as `[lon, lat]`.
const CENTER: (f64, f64) = (-51.2, -27.5);

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = maplibregl, js_name = Map)]
    type MapLibre;

    #[wasm_bindgen(constructor, catch, js_namespace = maplibregl, js_class = "Map")]
    fn new(options: &JsValue) -> Result<MapLibre, JsValue>;

    #[wasm_bindgen(method)]
    fn on(this: &MapLibre, event: &str, listener: &Closure<dyn FnMut(JsValue)>);

    #[wasm_bindgen(method)]
    fn remove(this: &MapLibre);

    #[wasm_bindgen(method, js_name = addControl)]
    fn add_control(this: &MapLibre, control: &NavigationControl, position: &str);

    #[wasm_bindgen(js_namespace = maplibregl)]
    type NavigationControl;

    #[wasm_bindgen(constructor, js_namespace = maplibregl, js_class = "NavigationControl")]
    fn new() -> NavigationControl;
}

fn has_maplibre() -> bool {
    js_sys::Reflect::has(&js_sys::global(), &"maplibregl".into()).unwrap_or(false)
}

fn set(o: &js_sys::Object, key: &str, v: &JsValue) {
    let _ = js_sys::Reflect::set(o, &key.into(), v);
}

fn options(container: &JsValue) -> JsValue {
    let o = js_sys::Object::new();
    set(&o, "container", container);
    set(&o, "style", &STYLE.into());
    set(
        &o,
        "center",
        &js_sys::Array::of2(&CENTER.0.into(), &CENTER.1.into()),
    );
    set(&o, "zoom", &4.6.into());
    let attribution = js_sys::Object::new();
    set(&attribution, "compact", &false.into());
    set(&o, "attributionControl", &attribution);
    o.into()
}

#[component]
pub fn MapPage() -> impl IntoView {
    let node = NodeRef::<leptos::html::Div>::new();
    let status = RwSignal::new(None::<String>);
    let alerts = RwSignal::new(Vec::<Record>::new());
    let map = StoredValue::new_local(None::<MapLibre>);

    spawn_local(async move {
        match api::list("alerts").await {
            Ok(list) => alerts.set(list),
            Err(e) => status.set(Some(e.message())),
        }
    });

    Effect::new(move |_| {
        let Some(el) = node.get() else { return };
        if map.with_value(Option::is_some) {
            return;
        }
        if !has_maplibre() {
            status.set(Some(
                "O MapLibre não carregou. Rode just desktop-vendor e abra o app de novo.".into(),
            ));
            return;
        }
        match MapLibre::new(&options(&el.into())) {
            Ok(m) => {
                m.add_control(&NavigationControl::new(), "top-right");
                let on_error = Closure::<dyn FnMut(JsValue)>::new(move |_| {
                    status.set(Some(
                        "O mapa base não carregou (sem rede?). A lista de alertas está completa sem o mapa."
                            .into(),
                    ));
                });
                m.on("error", &on_error);
                on_error.forget();
                map.set_value(Some(m));
            }
            Err(_) => status.set(Some("O MapLibre não abriu o mapa.".into())),
        }
    });
    on_cleanup(move || {
        map.update_value(|m| {
            if let Some(m) = m.take() {
                m.remove();
            }
        })
    });

    view! {
        <header class="page-head">
            <div>
                <h1>"Mapa"</h1>
                <p class="about">
                    "O mapa mostra a região dos alertas. Um alerta ainda não tem geometria, então o mapa não desenha forma nenhuma: a lista dá a área em texto."
                </p>
            </div>
        </header>
        {move || status.get().map(|m| view! { <p class="warn">{m}</p> })}
        <div class="map-layout">
            <div class="map" node_ref=node></div>
            <aside class="panel alert-list">
                <h2>"Alertas"</h2>
                {move || {
                    let list = alerts.get();
                    if list.is_empty() {
                        return view! {
                            <p class="empty">"Nenhum alerta ainda. Os alertas chegam só das fontes oficiais."</p>
                        }
                            .into_any();
                    }
                    list.into_iter()
                        .map(|a| {
                            let get = |k: &str| a.get(k).and_then(|v| v.as_str()).unwrap_or("").to_owned();
                            let sev = get("severity");
                            let ufs = a
                                .get("ufs")
                                .and_then(|v| v.as_array())
                                .map(|u| u.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", "))
                                .unwrap_or_default();
                            view! {
                                <article class="alert-item">
                                    <span class=format!("badge sev-{sev}")>{severity_label(&sev)}</span>
                                    <h3>{get("headline")}</h3>
                                    <p class="meta">{format!("{ufs} · {} · {}", get("source"), get("sent"))}</p>
                                    <p>{get("area")}</p>
                                </article>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </aside>
        </div>
    }
}

/// The pt-BR label of a severity, the same text as every channel (`Severity::label_pt_br`).
fn severity_label(s: &str) -> &'static str {
    match s {
        "Minor" => "menor",
        "Moderate" => "moderada",
        "Severe" => "severa",
        "Extreme" => "extrema",
        _ => "",
    }
}
