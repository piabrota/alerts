//! One input per field kind. The form keeps the record as JSON, the same shape the controller
//! reads, so a new kind needs one arm here and nothing else.

use crate::api::{Choice, Field, FieldError, Kind, Record};
use leptos::prelude::*;
use serde_json::{Value, json};

#[component]
pub fn FieldInput(
    field: Field,
    form: RwSignal<Record>,
    /// The slugs a `Refs` field can pick.
    #[prop(into)]
    refs: Signal<Vec<String>>,
    errors: RwSignal<Vec<FieldError>>,
) -> impl IntoView {
    let name = StoredValue::new(field.name.clone());
    let value = move || form.with(|f| f.get(&name.get_value()).cloned().unwrap_or(Value::Null));
    let text = move || match value() {
        Value::String(s) => s,
        Value::Null => String::new(),
        v => v.to_string(),
    };
    let set = move |v: Value| {
        form.update(|f| {
            f.insert(name.get_value(), v);
        })
    };
    let input = match field.kind.clone() {
        Kind::Text | Kind::Url => {
            let kind = if field.kind == Kind::Url { "url" } else { "text" };
            view! {
                <input
                    type=kind
                    prop:value=text
                    on:input=move |ev| set(json!(event_target_value(&ev)))
                />
            }
            .into_any()
        }
        Kind::LongText => view! {
            <textarea
                rows="3"
                prop:value=text
                on:input=move |ev| set(json!(event_target_value(&ev)))
            ></textarea>
        }
        .into_any(),
        Kind::Bool => view! {
            <label class="switch">
                <input
                    type="checkbox"
                    prop:checked=move || value().as_bool().unwrap_or(false)
                    on:change=move |ev| set(json!(event_target_checked(&ev)))
                />
                <span>{move || if value().as_bool().unwrap_or(false) { "sim" } else { "não" }}</span>
            </label>
        }
        .into_any(),
        Kind::Int => view! {
            <input
                type="number"
                prop:value=text
                on:input=move |ev| {
                    set(json!(event_target_value(&ev).parse::<i64>().unwrap_or(0)))
                }
            />
        }
        .into_any(),
        Kind::Choice { options } => view! {
            <select on:change=move |ev| set(json!(event_target_value(&ev)))>
                {options
                    .into_iter()
                    .map(|c| {
                        let v = c.value.clone();
                        view! {
                            <option value=c.value prop:selected=move || text() == v>
                                {c.label}
                            </option>
                        }
                    })
                    .collect_view()}
            </select>
        }
        .into_any(),
        Kind::Many { options } => checks(options, value, set).into_any(),
        Kind::Refs { resource } => {
            let empty = format!("Cadastre primeiro um registro em {resource}.");
            view! {
            {move || {
                let options: Vec<Choice> = refs
                    .get()
                    .into_iter()
                    .map(|s| Choice { value: s.clone(), label: s })
                    .collect();
                if options.is_empty() {
                    view! {
                        <p class="hint">{empty.clone()}</p>
                    }
                    .into_any()
                } else {
                    checks(options, value, set).into_any()
                }
            }}
        }
        .into_any()
        }
    };
    let help = field.help.clone();
    view! {
        <div class="field" class:invalid=move || has_error(errors, &name.get_value())>
            <label class="field-label">{field.label.clone()}</label>
            {input}
            {(!help.is_empty()).then(|| view! { <small class="hint">{help}</small> })}
            {move || {
                errors
                    .get()
                    .into_iter()
                    .filter(|e| e.field == name.get_value())
                    .map(|e| view! { <p class="field-error">{e.message}</p> })
                    .collect_view()
            }}
        </div>
    }
}

fn has_error(errors: RwSignal<Vec<FieldError>>, name: &str) -> bool {
    errors.with(|es| es.iter().any(|e| e.field == name))
}

/// A group of checkboxes over a list value.
fn checks(
    options: Vec<Choice>,
    value: impl Fn() -> Value + Copy + Send + Sync + 'static,
    set: impl Fn(Value) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let has = move |v: &str| {
        value()
            .as_array()
            .is_some_and(|a| a.iter().any(|x| x.as_str() == Some(v)))
    };
    view! {
        <div class="chips">
            {options
                .into_iter()
                .map(|c| {
                    let v = StoredValue::new(c.value.clone());
                    view! {
                        <label class="chip" class:on=move || has(&v.get_value())>
                            <input
                                type="checkbox"
                                prop:checked=move || has(&v.get_value())
                                on:change=move |ev| {
                                    let on = event_target_checked(&ev);
                                    let mut list: Vec<Value> = value()
                                        .as_array()
                                        .cloned()
                                        .unwrap_or_default()
                                        .into_iter()
                                        .filter(|x| x.as_str() != Some(v.get_value().as_str()))
                                        .collect();
                                    if on {
                                        list.push(json!(v.get_value()));
                                    }
                                    set(Value::Array(list));
                                }
                            />
                            {c.label}
                        </label>
                    }
                })
                .collect_view()}
        </div>
    }
}
