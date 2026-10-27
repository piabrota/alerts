//! The alerts desktop interface. Each page of the menu is a module. A resource of
//! `alerts-crud` becomes a CRUD page drawn from its spec, so a scaffolded resource needs no
//! interface code. A page with its own code (the map, the about page) is in [`custom_pages`].
//! The map is the home page: the app opens on it. The marola strip and brand wrap every page:
//! alerts is the "alertas" section of marola v2.

mod about;
mod alert_view;
mod api;
mod crud;
mod form;
mod map;
mod marola;

use leptos::prelude::*;

#[derive(Debug, Clone, PartialEq)]
enum Page {
    Crud(api::Spec),
    Map,
    About,
}

impl Page {
    fn key(&self) -> String {
        match self {
            Page::Crud(spec) => spec.resource.clone(),
            Page::Map => "map".into(),
            Page::About => "about".into(),
        }
    }

    fn label(&self) -> String {
        match self {
            Page::Crud(spec) => spec.label.clone(),
            Page::Map => "Mapa".into(),
            Page::About => "Sobre".into(),
        }
    }
}

/// The pages with their own code: the map first, so the app opens on it, and the about page
/// last, after the resources.
fn custom_pages() -> (Vec<Page>, Vec<Page>) {
    (vec![Page::Map], vec![Page::About])
}

fn menu(specs: Vec<api::Spec>) -> Vec<Page> {
    let (mut pages, mut last) = custom_pages();
    pages.extend(specs.into_iter().map(Page::Crud));
    pages.append(&mut last);
    pages
}

#[component]
fn App() -> impl IntoView {
    let pages = RwSignal::new(Vec::<Page>::new());
    let current = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    leptos::task::spawn_local(async move {
        // Without the specs the menu keeps its own pages: the map says what failed.
        let specs = api::modules().await.unwrap_or_else(|e| {
            error.set(Some(e.message()));
            Vec::new()
        });
        let ps = menu(specs);
        current.set(ps.first().map(Page::key).unwrap_or_default());
        pages.set(ps);
    });
    let about_open = Signal::derive(move || current.get() == "about");
    let on_about = Callback::new(move |()| current.set("about".into()));

    view! {
        <marola::SiteNav on_about about_open />
        <div class="shell">
            <nav class="menu">
                <div class="brand">
                    <img class="brand-logo" src=marola::LOGO alt="" width="36" height="36" />
                    <span class="brand-name">"marola"<small>"alertas"</small></span>
                </div>
                {move || {
                    pages
                        .get()
                        .into_iter()
                        .map(|p| {
                            let key = p.key();
                            let active = key.clone();
                            view! {
                                <button
                                    class="menu-item"
                                    class:active=move || current.get() == active
                                    on:click=move |_| current.set(key.clone())
                                >
                                    {p.label()}
                                </button>
                            }
                        })
                        .collect_view()
                }}
                <p class="menu-foot">"Só fontes oficiais. Nenhum alerta é criado à mão."</p>
            </nav>
            <main class="page">
                {move || error.get().map(|m| view! { <p class="error">{m}</p> })}
                {move || {
                    let key = current.get();
                    pages
                        .get()
                        .into_iter()
                        .find(|p| p.key() == key)
                        .map(|p| match p {
                            Page::Crud(spec) => view! { <crud::CrudPage spec /> }.into_any(),
                            Page::Map => view! { <map::MapPage /> }.into_any(),
                            Page::About => view! { <about::AboutPage /> }.into_any(),
                        })
                }}
            </main>
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
