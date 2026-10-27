//! The marola brand around the app: the site strip, the GitHub pills, the areas of marola.dev and
//! the link opener. alerts is the "alertas" section of marola v2: marola.dev lists it as "em
//! breve". The links, the areas and the logo come from marola, read on 2026-10-27:
//! `marola-dev/marola-site` `site/static/index.html` and `site/areas.json`, and
//! `marola-dev/marola` `docs/assets/logo-v2/marola-logo-v2.svg` at `19fdf0df` (CC BY 4.0).

use crate::alert_view::BBox;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// The logo, copied into `assets/` and served by trunk.
pub const LOGO: &str = "assets/marola-logo-v2.svg";

/// The sections of the marola.dev strip. `None` is this app's own page.
pub const SITE: &[(&str, Option<&str>)] = &[
    ("docs", Some("https://docs.marola.dev/")),
    ("mapa do mar", Some("https://marola.dev/")),
    ("alertas", None),
    ("notícias", Some("https://marola.dev/news.html")),
    ("contato", Some("https://marola.dev/contact.html")),
    (
        "pesquise",
        Some("https://github.com/marola-dev/marola-research/blob/main/README.md"),
    ),
    (
        "contribua",
        Some("https://github.com/marola-dev/marola-research/blob/main/chamada/chamada.pdf"),
    ),
    ("membros", Some("https://marola.dev/members.html")),
    ("apoie", Some("https://marola.dev/support.html")),
];

/// A repository and what it holds, as the marola-dev profile describes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Repo {
    pub owner: &'static str,
    pub name: &'static str,
    pub about: &'static str,
}

impl Repo {
    pub fn url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner, self.name)
    }
}

/// The repo of this app.
pub const ALERTS: Repo = Repo {
    owner: "piabrota",
    name: "alerts",
    about: "Este app: alertas oficiais de inundação, no Discord, no feed e no mapa.",
};

/// The marola-dev repos, in the order of the org profile.
pub const REPOS: &[Repo] = &[
    Repo {
        owner: "marola-dev",
        name: "marola",
        about: "O guarda-chuva: planos (MIPs), regras, o site de docs e cada repositório como submódulo.",
    },
    Repo {
        owner: "marola-dev",
        name: "marola-app",
        about: "O produto: a nota de cada praia e o veto de segurança.",
    },
    Repo {
        owner: "marola-dev",
        name: "marola-site",
        about: "O mapa em marola.dev.",
    },
    Repo {
        owner: "marola-dev",
        name: "marola-oods",
        about: "Um arquivo aberto de praias e qualidade da água.",
    },
    Repo {
        owner: "marola-dev",
        name: "marola-corpus",
        about: "O conhecimento sobre o mar, com fontes.",
    },
    Repo {
        owner: "marola-dev",
        name: "marola-ml",
        about: "Compilação de prompts, o fine-tune e o portão de benchmark.",
    },
    Repo {
        owner: "marola-dev",
        name: "marola-research",
        about: "A pesquisa do marola.",
    },
];

/// An area of marola.dev (`site/areas.json`): a center and a radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub id: &'static str,
    /// The two-letter code of the marola.dev area picker (`AREA_CODES` in its `app.js`).
    pub code: &'static str,
    pub name: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub radius_km: f64,
}

/// The names drop the UF that `areas.json` adds to "Salvador, BA": the picker shows the code.
pub const AREAS: &[Area] = &[
    Area {
        id: "floripa",
        code: "FL",
        name: "Florianópolis",
        lat: -27.60,
        lon: -48.48,
        radius_km: 30.0,
    },
    Area {
        id: "rio",
        code: "RJ",
        name: "Rio de Janeiro",
        lat: -22.9878,
        lon: -43.1913,
        radius_km: 20.0,
    },
    Area {
        id: "salvador",
        code: "BA",
        name: "Salvador",
        lat: -12.9777,
        lon: -38.5016,
        radius_km: 25.0,
    },
];

pub fn area(id: &str) -> Option<&'static Area> {
    AREAS.iter().find(|a| a.id == id)
}

/// The square around an area: its radius on each side. One degree of latitude is 111.32 km.
pub fn area_bbox(a: &Area) -> BBox {
    let dlat = a.radius_km / 111.32;
    let dlon = a.radius_km / (111.32 * a.lat.to_radians().cos());
    [a.lon - dlon, a.lat - dlat, a.lon + dlon, a.lat + dlat]
}

/// Opens `url` in the system browser: the app window never leaves the app. Tauri does it with
/// the opener plugin; a plain browser (the screenshots) opens a tab.
pub fn open(url: &str) {
    let get = |root: JsValue, keys: &[&str]| {
        keys.iter().try_fold(root, |v, k| {
            js_sys::Reflect::get(&v, &JsValue::from_str(k))
                .ok()
                .filter(|v| !v.is_undefined() && !v.is_null())
        })
    };
    let global: JsValue = js_sys::global().into();
    let call = |path: &[&str], args: &[JsValue]| {
        let f = get(global.clone(), path)?
            .dyn_into::<js_sys::Function>()
            .ok()?;
        let this = JsValue::NULL;
        match args {
            [a] => f.call1(&this, a).ok(),
            [a, b] => f.call2(&this, a, b).ok(),
            _ => None,
        }
    };
    if call(&["__TAURI__", "opener", "openUrl"], &[url.into()]).is_none() {
        let _ = call(&["open"], &[url.into(), "_blank".into()]);
    }
}

/// A link out of the app.
#[component]
pub fn Out(
    href: String,
    #[prop(optional, into)] class: String,
    children: Children,
) -> impl IntoView {
    let target = href.clone();
    view! {
        <a href=href class=class on:click=move |ev| {
            ev.prevent_default();
            open(&target);
        }>{children()}</a>
    }
}

const GITHUB_MARK: &str = "M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.06-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z";

/// The GitHub pill of marola.dev: the mark, `owner/name` and a star.
#[component]
pub fn Pill(repo: Repo) -> impl IntoView {
    view! {
        <Out href=repo.url() class="gh">
            <svg class="gh-mark" viewBox="0 0 16 16" aria-hidden="true">
                <path fill="currentColor" d=GITHUB_MARK></path>
            </svg>
            <span class="gh-name"><span class="gh-owner">{format!("{}/", repo.owner)}</span>{repo.name}</span>
            <span class="gh-star">
                <svg viewBox="0 0 16 16" aria-hidden="true">
                    <path d="M8 1.6l1.9 3.9 4.3.6-3.1 3 .7 4.3L8 11.4l-3.8 2 .7-4.3-3.1-3 4.3-.6z"></path>
                </svg>
                "star"
            </span>
        </Out>
    }
}

/// The strip of marola.dev over the app: its sections, then the two GitHub pills.
#[component]
pub fn SiteNav(on_about: Callback<()>, about_open: Signal<bool>) -> impl IntoView {
    view! {
        <nav class="sitenav" aria-label="marola">
            {SITE.iter().map(|&(label, href)| match href {
                Some(h) => view! { <Out href=h.to_owned()>{label}</Out> }.into_any(),
                None => view! { <span class="here" aria-current="page">{label}</span> }.into_any(),
            }).collect_view()}
            <button class="sitenav-about" class:on=move || about_open.get()
                on:click=move |_| on_about.run(())>"sobre"</button>
            <span class="sitenav-gap"></span>
            <Pill repo=REPOS[0] />
            <Pill repo=ALERTS />
        </nav>
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn the_areas_of_marola_dev() {
        assert_eq!(
            AREAS.iter().map(|a| a.code).collect::<Vec<_>>(),
            ["FL", "RJ", "BA"]
        );
        let fl = area("floripa").unwrap();
        let [w, s, e, n] = area_bbox(fl);
        assert!((n - s - 2.0 * 30.0 / 111.32).abs() < 1e-9);
        // A degree of longitude is shorter away from the equator: the box is wider in degrees.
        assert!(e - w > n - s);
        assert!(w < fl.lon && fl.lon < e && s < fl.lat && fl.lat < n);
        assert!(area("nowhere").is_none());
    }

    #[test]
    fn every_repo_has_a_github_url() {
        assert_eq!(ALERTS.url(), "https://github.com/piabrota/alerts");
        assert!(
            REPOS
                .iter()
                .all(|r| r.owner == "marola-dev" && r.url().ends_with(r.name))
        );
    }
}
