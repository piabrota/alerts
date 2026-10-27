//! WeatherDisaster, the Obsidian plugin of alerts: it reads an Atom or RSS feed of CAP 1.2 alerts
//! and keeps one note per alert. This crate holds the plugin logic and builds to WebAssembly.
//! `plugin/main.js` is only the bridge to the Obsidian API: HTTP, files, notices, settings.
//!
//! The functions below are that bridge's interface. They take and return JSON strings, and an
//! `Err` becomes a JavaScript exception.

pub mod active;
pub mod cap;
pub mod note;
pub mod xml;

use serde::Serialize;
use wasm_bindgen::prelude::*;

fn from_json<T: serde::de::DeserializeOwned>(what: &str, json: &str) -> Result<T, JsError> {
    serde_json::from_str(json).map_err(|e| JsError::new(&format!("{what}: {e}")))
}

fn to_json<T: Serialize>(value: &T) -> Result<String, JsError> {
    serde_json::to_string(value).map_err(|e| JsError::new(&e.to_string()))
}

/// The User-Agent of every request the plugin sends.
#[wasm_bindgen]
pub fn user_agent() -> String {
    alerts_core::USER_AGENT.to_owned()
}

#[wasm_bindgen]
pub fn default_settings() -> Result<String, JsError> {
    to_json(&note::Settings::default())
}

/// A feed as JSON `{alerts, links, unreadable}`.
#[wasm_bindgen]
pub fn parse_feed(xml: &str) -> Result<String, JsError> {
    to_json(&cap::parse_feed(xml).map_err(|e| JsError::new(&e))?)
}

/// One CAP document as a JSON alert.
#[wasm_bindgen]
pub fn parse_cap(xml: &str) -> Result<String, JsError> {
    to_json(&cap::parse_cap(xml).map_err(|e| JsError::new(&e))?)
}

/// The identifiers whose note the alert changes, as a JSON array.
#[wasm_bindgen]
pub fn note_keys(alert: &str) -> Result<String, JsError> {
    to_json(&note::note_keys(&from_json("alert", alert)?))
}

#[wasm_bindgen]
pub fn note_path(folder: &str, identifier: &str) -> String {
    note::note_path(folder, identifier)
}

/// What to do with one alert, as JSON: `{action: "write", text, notice}`, `{action: "skip",
/// reason}` or `{action: "conflict", reason}`.
#[wasm_bindgen]
pub fn apply(alert: &str, settings: &str, existing: Option<String>) -> Result<String, JsError> {
    let a: cap::CapAlert = from_json("alert", alert)?;
    let s: note::Settings = from_json("settings", settings)?;
    to_json(&note::apply(&a, &s, existing.as_deref()))
}

#[derive(Debug, Serialize)]
struct View {
    count: usize,
    markdown: String,
    status: String,
}

/// The active alerts of `notes` (`[{path, frontmatter}]`) at `now` (seconds since 1970), as JSON
/// `{count, markdown, status}`: the status bar line and the `alerts` code block.
#[wasm_bindgen]
pub fn active_view(notes: &str, poll: &str, now: f64) -> Result<String, JsError> {
    let notes: Vec<active::NoteMeta> = from_json("notes", notes)?;
    let poll: active::PollState = from_json("poll", poll)?;
    let list = active::active(&notes, now as i64);
    to_json(&View {
        count: list.len(),
        markdown: active::markdown(&list, &poll),
        status: active::status_text(list.len(), &poll),
    })
}
