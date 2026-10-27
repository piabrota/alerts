//! The CRUD page of one resource, drawn from its spec: a table, a form and the record actions.
//! The spec decides what shows: a read-only resource (the alerts) has no form.

use crate::api::{self, ApiError, FieldError, Kind, Record, Spec};
use crate::form::FieldInput;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;

/// Which form is open: a new record, or the record with this id.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Editing {
    New,
    Id(i64),
}

fn record_id(r: &Record) -> i64 {
    r.get("id").and_then(|v| v.as_i64()).unwrap_or(0)
}

#[component]
pub fn CrudPage(spec: Spec) -> impl IntoView {
    let spec = StoredValue::new(spec);
    let rows = RwSignal::new(Vec::<Record>::new());
    let loaded = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let editing = RwSignal::new(None::<Editing>);
    let form = RwSignal::new(Record::new());
    let field_errors = RwSignal::new(Vec::<FieldError>::new());
    let refs = RwSignal::new(HashMap::<String, Vec<String>>::new());
    let output = RwSignal::new(None::<(String, String)>);
    let confirm_delete = RwSignal::new(None::<i64>);

    let fail = move |e: ApiError| {
        field_errors.set(e.field_errors());
        error.set(Some(e.message()));
    };
    let reload = move || {
        spawn_local(async move {
            match api::list(&spec.get_value().resource).await {
                Ok(list) => {
                    rows.set(list);
                    error.set(None);
                }
                Err(e) => fail(e),
            }
            loaded.set(true);
        })
    };
    reload();

    let open_form = move |which: Editing, record: Option<Record>| {
        let s = spec.get_value();
        let values = s
            .fields
            .iter()
            .map(|f| {
                let v = record
                    .as_ref()
                    .and_then(|r| r.get(&f.name).cloned())
                    .unwrap_or_else(|| f.empty());
                (f.name.clone(), v)
            })
            .collect();
        form.set(values);
        field_errors.set(Vec::new());
        error.set(None);
        output.set(None);
        editing.set(Some(which));
        for f in s.fields {
            if let Kind::Refs { resource } = f.kind {
                spawn_local(async move {
                    if let Ok(list) = api::list(&resource).await {
                        let slugs = list
                            .iter()
                            .filter_map(|r| r.get("slug").and_then(|v| v.as_str()))
                            .map(String::from)
                            .collect();
                        refs.update(|m| {
                            m.insert(resource.clone(), slugs);
                        });
                    }
                });
            }
        }
    };

    let save = move |_| {
        let Some(which) = editing.get() else { return };
        let params = form.get();
        spawn_local(async move {
            let resource = spec.get_value().resource;
            let r = match which {
                Editing::New => api::create(&resource, &params).await,
                Editing::Id(id) => api::update(&resource, id, &params).await,
            };
            match r {
                Ok(_) => {
                    editing.set(None);
                    reload();
                }
                Err(e) => fail(e),
            }
        });
    };

    let remove = move |id: i64| {
        spawn_local(async move {
            confirm_delete.set(None);
            match api::delete(&spec.get_value().resource, id).await {
                Ok(()) => reload(),
                Err(e) => fail(e),
            }
        });
    };

    let run_action = move |id: i64, action: String, label: String| {
        spawn_local(async move {
            match api::action(&spec.get_value().resource, id, &action).await {
                Ok(text) => output.set(Some((label, text))),
                Err(e) => fail(e),
            }
        });
    };

    let details = move |r: Record| {
        let s = spec.get_value();
        let text = s
            .fields
            .iter()
            .map(|f| {
                let v = r.get(&f.name).cloned().unwrap_or_default();
                format!("{}: {}", f.label, f.display(&v))
            })
            .collect::<Vec<_>>()
            .join("\n");
        output.set(Some((s.singular, text)));
    };

    let s = spec.get_value();
    let columns: Vec<_> = s.fields.iter().filter(|f| f.list).cloned().collect();
    let columns = StoredValue::new(columns);
    let can_create = s.allows("create");
    let can_update = s.allows("update");
    let can_delete = s.allows("delete");

    view! {
        <header class="page-head">
            <div>
                <h1>{s.label.clone()}</h1>
                <p class="about">{s.about.clone()}</p>
            </div>
            {can_create
                .then(|| {
                    view! {
                        <button class="primary" on:click=move |_| open_form(Editing::New, None)>
                            {format!("Novo: {}", s.singular)}
                        </button>
                    }
                })}
        </header>

        {move || error.get().map(|m| view! { <p class="error">{m}</p> })}

        {move || {
            editing
                .get()
                .map(|which| {
                    let s = spec.get_value();
                    let title = match which {
                        Editing::New => format!("Novo: {}", s.singular),
                        Editing::Id(id) => format!("Editar: {} {id}", s.singular),
                    };
                    view! {
                        <section class="panel form">
                            <h2>{title}</h2>
                            <div class="fields">
                                {s
                                    .fields
                                    .into_iter()
                                    .map(|f| {
                                        let target = match &f.kind {
                                            Kind::Refs { resource } => resource.clone(),
                                            _ => String::new(),
                                        };
                                        let options = Signal::derive(move || {
                                            refs.with(|m| m.get(&target).cloned().unwrap_or_default())
                                        });
                                        view! {
                                            <FieldInput field=f form refs=options errors=field_errors />
                                        }
                                    })
                                    .collect_view()}
                            </div>
                            <div class="buttons">
                                <button class="primary" on:click=save>
                                    "Salvar"
                                </button>
                                <button on:click=move |_| editing.set(None)>"Cancelar"</button>
                            </div>
                        </section>
                    }
                })
        }}

        {move || {
            output
                .get()
                .map(|(title, text)| {
                    view! {
                        <section class="panel output">
                            <div class="output-head">
                                <h2>{title}</h2>
                                <button on:click=move |_| output.set(None)>"Fechar"</button>
                            </div>
                            <pre>{text}</pre>
                        </section>
                    }
                })
        }}

        <section class="panel">
            <table>
                <thead>
                    <tr>
                        {columns
                            .get_value()
                            .into_iter()
                            .map(|f| view! { <th>{f.label}</th> })
                            .collect_view()}
                        <th class="actions-col"></th>
                    </tr>
                </thead>
                <tbody>
                    {move || {
                        rows.get()
                            .into_iter()
                            .map(|r| {
                                let id = record_id(&r);
                                let cells = columns
                                    .get_value()
                                    .into_iter()
                                    .map(|f| {
                                        let v = r.get(&f.name).cloned().unwrap_or_default();
                                        let shown = f.display(&v);
                                        let sev = (f.name == "severity" || f.name == "min_severity")
                                            .then(|| format!("badge sev-{}", v.as_str().unwrap_or("")));
                                        match sev {
                                            Some(class) => view! { <td><span class=class>{shown}</span></td> }.into_any(),
                                            None => view! { <td>{shown}</td> }.into_any(),
                                        }
                                    })
                                    .collect_view();
                                let edit_rec = r.clone();
                                let show_rec = r.clone();
                                let actions = spec
                                    .get_value()
                                    .actions
                                    .into_iter()
                                    .map(|a| {
                                        let label = a.label.clone();
                                        view! {
                                            <button on:click=move |_| run_action(id, a.id.clone(), label.clone())>
                                                {a.label.clone()}
                                            </button>
                                        }
                                    })
                                    .collect_view();
                                view! {
                                    <tr>
                                        {cells}
                                        <td class="actions-col">
                                            {actions}
                                            {if can_update {
                                                view! {
                                                    <button on:click=move |_| open_form(Editing::Id(id), Some(edit_rec.clone()))>
                                                        "Editar"
                                                    </button>
                                                }
                                                    .into_any()
                                            } else {
                                                view! {
                                                    <button on:click=move |_| details(show_rec.clone())>"Ver"</button>
                                                }
                                                    .into_any()
                                            }}
                                            {can_delete
                                                .then(|| {
                                                    view! {
                                                        {move || {
                                                            if confirm_delete.get() == Some(id) {
                                                                view! {
                                                                    <button class="danger" on:click=move |_| remove(id)>
                                                                        "Confirmar"
                                                                    </button>
                                                                    <button on:click=move |_| confirm_delete.set(None)>
                                                                        "Não"
                                                                    </button>
                                                                }
                                                                    .into_any()
                                                            } else {
                                                                view! {
                                                                    <button on:click=move |_| confirm_delete.set(Some(id))>
                                                                        "Apagar"
                                                                    </button>
                                                                }
                                                                    .into_any()
                                                            }
                                                        }}
                                                    }
                                                })}
                                        </td>
                                    </tr>
                                }
                            })
                            .collect_view()
                    }}
                </tbody>
            </table>
            {move || {
                (loaded.get() && rows.with(Vec::is_empty))
                    .then(|| {
                        let s = spec.get_value();
                        let msg = if s.allows("create") {
                            format!("Nenhum registro ainda. Use o botão Novo: {}.", s.singular)
                        } else {
                            "Nenhum registro ainda. Os alertas chegam só das fontes oficiais.".into()
                        };
                        view! { <p class="empty">{msg}</p> }
                    })
            }}
        </section>
    }
}
