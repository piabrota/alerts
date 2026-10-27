//! The alerts desktop interface. Each page of the menu is a module. A resource of
//! `alerts-crud` becomes a CRUD page drawn from its spec, so a scaffolded resource needs no
//! interface code. A page with its own code (the map) is in [`custom_pages`].

mod api;
mod crud;
mod form;
mod map;

use leptos::prelude::*;

#[derive(Debug, Clone, PartialEq)]
enum Page {
    Crud(api::Spec),
    Map,
}

impl Page {
    fn key(&self) -> String {
        match self {
            Page::Crud(spec) => spec.resource.clone(),
            Page::Map => "map".into(),
        }
    }

    fn label(&self) -> String {
        match self {
            Page::Crud(spec) => spec.label.clone(),
            Page::Map => "Mapa".into(),
        }
    }
}

/// The pages with their own code. They come right after the first resource (the alerts).
fn custom_pages() -> Vec<Page> {
    vec![Page::Map]
}

fn menu(specs: Vec<api::Spec>) -> Vec<Page> {
    let mut pages: Vec<Page> = specs.into_iter().map(Page::Crud).collect();
    let at = pages.len().min(1);
    for (i, p) in custom_pages().into_iter().enumerate() {
        pages.insert(at + i, p);
    }
    pages
}

#[component]
fn App() -> impl IntoView {
    let pages = RwSignal::new(Vec::<Page>::new());
    let current = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    leptos::task::spawn_local(async move {
        match api::modules().await {
            Ok(specs) => {
                let ps = menu(specs);
                current.set(ps.first().map(Page::key).unwrap_or_default());
                pages.set(ps);
            }
            Err(e) => error.set(Some(e.message())),
        }
    });

    view! {
        <div class="shell">
            <nav class="menu">
                <div class="brand">
                    <span class="brand-mark">"≈"</span>
                    "alerts"
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
