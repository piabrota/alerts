//! The calls to the desktop app (Tauri commands) and the types they return. The types mirror
//! `alerts_crud::schema` and `alerts_crud::Error`: the interface reads them as data, so it
//! does not link the backend crate.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// `window.__TAURI__.core.invoke`, present because `app.withGlobalTauri` is true.
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Kind {
    Text,
    LongText,
    Url,
    Bool,
    Int,
    Choice { options: Vec<Choice> },
    Many { options: Vec<Choice> },
    Refs { resource: String },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Field {
    pub name: String,
    pub label: String,
    pub kind: Kind,
    pub help: String,
    pub list: bool,
}

impl Field {
    /// The value of a new record.
    pub fn empty(&self) -> Value {
        match &self.kind {
            Kind::Text | Kind::LongText | Kind::Url => json!(""),
            Kind::Bool => json!(true),
            Kind::Int => json!(0),
            Kind::Choice { options } => json!(options.first().map(|c| c.value.clone())),
            Kind::Many { .. } | Kind::Refs { .. } => json!([]),
        }
    }

    /// The value as the table shows it, with the pt-BR labels of choices.
    pub fn display(&self, v: &Value) -> String {
        let label = |options: &[Choice], s: &str| {
            options
                .iter()
                .find(|c| c.value == s)
                .map_or_else(|| s.to_owned(), |c| c.label.clone())
        };
        match (&self.kind, v) {
            (_, Value::Null) => String::new(),
            (Kind::Bool, Value::Bool(b)) => if *b { "sim" } else { "não" }.into(),
            (Kind::Choice { options }, Value::String(s)) => label(options, s),
            (Kind::Many { options }, Value::Array(a)) => a
                .iter()
                .filter_map(Value::as_str)
                .map(|s| label(options, s))
                .collect::<Vec<_>>()
                .join(", "),
            (_, Value::Array(a)) => a
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", "),
            (_, Value::String(s)) => s.clone(),
            (_, v) => v.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Action {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Spec {
    pub resource: String,
    pub label: String,
    pub singular: String,
    pub about: String,
    pub fields: Vec<Field>,
    pub ops: Vec<String>,
    pub actions: Vec<Action>,
}

impl Spec {
    pub fn allows(&self, op: &str) -> bool {
        self.ops.iter().any(|o| o == op)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

/// `alerts_crud::Error`, or a failure of the call itself.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApiError {
    NotFound {
        resource: String,
        id: i64,
    },
    Invalid {
        errors: Vec<FieldError>,
    },
    UnknownResource {
        resource: String,
    },
    NotAllowed {
        resource: String,
        op: String,
    },
    UnknownAction {
        resource: String,
        action: String,
    },
    Db {
        message: String,
    },
    Json {
        message: String,
    },
    #[serde(skip)]
    Call(String),
}

impl ApiError {
    pub fn message(&self) -> String {
        match self {
            ApiError::NotFound { resource, id } => format!("{resource} {id}: não existe."),
            ApiError::Invalid { .. } => "Corrija os campos marcados.".into(),
            ApiError::UnknownResource { resource } => format!("Recurso desconhecido: {resource}."),
            ApiError::NotAllowed { resource, op } => {
                format!("{resource}: a operação {op} não é permitida.")
            }
            ApiError::UnknownAction { resource, action } => {
                format!("{resource}: ação desconhecida: {action}.")
            }
            ApiError::Db { message } => format!("Banco de dados: {message}"),
            ApiError::Json { message } => format!("JSON: {message}"),
            ApiError::Call(m) => m.clone(),
        }
    }

    pub fn field_errors(&self) -> Vec<FieldError> {
        match self {
            ApiError::Invalid { errors } => errors.clone(),
            _ => Vec::new(),
        }
    }
}

pub type Record = Map<String, Value>;

/// Tauri reads the command arguments from a plain object, so maps become objects, not `Map`s.
fn to_js<T: Serialize>(v: &T) -> Result<JsValue, ApiError> {
    v.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| ApiError::Call(e.to_string()))
}

async fn invoke<T: for<'de> Deserialize<'de>>(cmd: &str, args: Value) -> Result<T, ApiError> {
    match tauri_invoke(cmd, to_js(&args)?).await {
        Ok(v) => serde_wasm_bindgen::from_value(v).map_err(|e| ApiError::Call(e.to_string())),
        Err(e) => Err(serde_wasm_bindgen::from_value::<ApiError>(e.clone()).unwrap_or_else(|_| {
            ApiError::Call(format!(
                "A chamada ao app falhou: {}. Abra a interface pelo app desktop (just desktop-dev).",
                e.as_string()
                    .or_else(|| js_sys::JSON::stringify(&e).ok().and_then(|s| s.as_string()))
                    .unwrap_or_default()
            ))
        })),
    }
}

pub async fn modules() -> Result<Vec<Spec>, ApiError> {
    invoke("modules", json!({})).await
}

pub async fn list(resource: &str) -> Result<Vec<Record>, ApiError> {
    invoke(
        "crud",
        json!({ "request": { "op": "list", "resource": resource } }),
    )
    .await
}

pub async fn create(resource: &str, params: &Record) -> Result<Record, ApiError> {
    invoke(
        "crud",
        json!({ "request": { "op": "create", "resource": resource, "params": params } }),
    )
    .await
}

pub async fn update(resource: &str, id: i64, params: &Record) -> Result<Record, ApiError> {
    invoke(
        "crud",
        json!({ "request": { "op": "update", "resource": resource, "id": id, "params": params } }),
    )
    .await
}

pub async fn delete(resource: &str, id: i64) -> Result<(), ApiError> {
    invoke::<Value>(
        "crud",
        json!({ "request": { "op": "delete", "resource": resource, "id": id } }),
    )
    .await
    .map(|_| ())
}

pub async fn action(resource: &str, id: i64, action: &str) -> Result<String, ApiError> {
    invoke(
        "crud",
        json!({ "request": { "op": "action", "resource": resource, "id": id, "action": action } }),
    )
    .await
}
