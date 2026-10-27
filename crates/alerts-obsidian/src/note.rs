//! One note per CAP alert: the note path, the filters, and the note text for a new alert, an
//! `Update` and a `Cancel`. The frontmatter values are JSON, which is also valid YAML, so the
//! note reads back without a YAML parser.

use crate::cap::CapAlert;
use alerts_core::{Severity, Uf};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub feed_url: String,
    pub folder: String,
    pub poll_minutes: u32,
    /// UF abbreviations, for example `RS`. Empty with `municipalities` empty: every area.
    pub ufs: Vec<String>,
    /// 7-digit IBGE municipality codes.
    pub municipalities: Vec<String>,
    /// The lowest CAP severity kept: `Unknown` keeps every alert.
    pub min_severity: String,
    /// Keep alerts with CAP status `Test`, to check a feed end to end.
    pub include_tests: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            // The national IDAP feed (Defesa Civil): undocumented, see docs/SOURCES.md.
            feed_url: "https://idapfile.mdr.gov.br/idap/api/rss/cap".into(),
            folder: "Alertas".into(),
            poll_minutes: 5,
            ufs: Uf::V0.iter().map(|u| u.as_str().to_owned()).collect(),
            municipalities: Vec::new(),
            min_severity: "Unknown".into(),
            include_tests: false,
        }
    }
}

/// What the plugin does with one alert.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum Outcome {
    /// Write `text` to the note. `notice` is the line to show the user.
    Write { text: String, notice: String },
    /// Do nothing: a filter dropped the alert, or the note already holds it.
    Skip { reason: String },
    /// Do nothing, and tell the user: the note is not in the form this plugin writes.
    Conflict { reason: String },
}

fn severity(s: &str) -> Option<Severity> {
    match s {
        "Minor" => Some(Severity::Minor),
        "Moderate" => Some(Severity::Moderate),
        "Severe" => Some(Severity::Severe),
        "Extreme" => Some(Severity::Extreme),
        _ => None,
    }
}

/// `Unknown` (or any value CAP does not define) is below `Minor`.
fn rank(s: &str) -> u8 {
    severity(s).map_or(0, |s| s as u8 + 1)
}

fn severity_pt_br(s: &str) -> &'static str {
    severity(s).map_or("desconhecida", Severity::label_pt_br)
}

/// The IBGE codes of the alert area: geocodes named IBGE, or 7-digit numbers.
fn ibge_codes(a: &CapAlert) -> Vec<String> {
    a.geocodes
        .iter()
        .filter(|g| {
            g.name.to_ascii_uppercase().contains("IBGE")
                || (g.value.len() == 7 && g.value.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(|g| g.value.trim().to_owned())
        .collect()
}

/// The UFs of the alert area: geocodes named UF, and the UF of every IBGE code.
fn ufs(a: &CapAlert) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let named = a
        .geocodes
        .iter()
        .filter(|g| g.name.eq_ignore_ascii_case("UF"))
        .filter_map(|g| g.value.parse::<Uf>().ok());
    for uf in named.chain(ibge_codes(a).iter().filter_map(|c| Uf::from_ibge(c))) {
        if !out.iter().any(|u| u == uf.as_str()) {
            out.push(uf.as_str().to_owned());
        }
    }
    out
}

/// The filters of the settings, for an alert that has no note yet.
fn filter(a: &CapAlert, s: &Settings) -> Result<(), String> {
    if rank(&a.severity) < rank(&s.min_severity) {
        return Err(format!(
            "severity {:?} is below {:?}",
            a.severity, s.min_severity
        ));
    }
    if s.ufs.is_empty() && s.municipalities.is_empty() {
        return Ok(());
    }
    let in_uf = ufs(a)
        .iter()
        .any(|u| s.ufs.iter().any(|w| w.trim().eq_ignore_ascii_case(u)));
    let in_city = ibge_codes(a)
        .iter()
        .any(|c| s.municipalities.iter().any(|m| m.trim() == c));
    if in_uf || in_city {
        Ok(())
    } else {
        Err("the area is outside the UFs and municipalities of the settings".into())
    }
}

/// The CAP status filter. `Exercise`, `System` and `Draft` never become notes.
fn status(a: &CapAlert, s: &Settings) -> Result<(), String> {
    match a.status.as_str() {
        "Actual" => Ok(()),
        "Test" if s.include_tests => Ok(()),
        other => Err(format!("status {other:?}")),
    }
}

/// The identifiers whose note this alert changes, in the order to look for them: the alerts
/// that an `Update` or a `Cancel` references, then the alert itself.
pub fn note_keys(a: &CapAlert) -> Vec<String> {
    let mut keys = match a.msg_type.as_str() {
        "Update" | "Cancel" => a.referenced_ids(),
        _ => Vec::new(),
    };
    keys.push(a.identifier.clone());
    keys
}

/// The vault path of the note of `identifier`: one file name per identifier.
pub fn note_path(folder: &str, identifier: &str) -> String {
    let name: String = identifier
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let name = name.trim_start_matches('.');
    match folder.trim_matches('/') {
        "" => format!("{name}.md"),
        f => format!("{f}/{name}.md"),
    }
}

/// The frontmatter of a note as `(key, JSON value)` pairs, and the body after it.
fn split(text: &str) -> Option<(Vec<(String, Value)>, &str)> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest
        .find("\n---\n")
        .or_else(|| rest.strip_suffix("\n---").map(str::len))?;
    let mut fm = Vec::new();
    for line in rest[..end].lines() {
        let (k, v) = line.split_once(": ")?;
        fm.push((k.to_owned(), serde_json::from_str(v).ok()?));
    }
    let body = rest.get(end + 5..).unwrap_or("");
    Some((fm, body))
}

fn join(fm: &[(String, Value)], body: &str) -> String {
    let mut s = String::from("---\n");
    for (k, v) in fm {
        s.push_str(&format!("{k}: {v}\n"));
    }
    s.push_str("---\n");
    s.push_str(body);
    s
}

fn get<'a>(fm: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    fm.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn set(fm: &mut Vec<(String, Value)>, key: &str, value: Value) {
    match fm.iter_mut().find(|(k, _)| k == key) {
        Some(slot) => slot.1 = value,
        None => fm.push((key.to_owned(), value)),
    }
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn title(a: &CapAlert) -> String {
    let t = if a.headline.is_empty() {
        &a.event
    } else {
        &a.headline
    };
    one_line(t)
}

/// The note of an alert. `history` holds the identifiers this note held before.
pub fn render(a: &CapAlert, history: &[String]) -> String {
    let mut fm: Vec<(String, Value)> = Vec::new();
    let mut put = |k: &str, v: Value| {
        let empty =
            v.as_str().is_some_and(str::is_empty) || v.as_array().is_some_and(Vec::is_empty);
        if !empty {
            fm.push((k.to_owned(), v));
        }
    };
    put("cap_identifier", json!(a.identifier));
    put("cap_sender", json!(a.sender));
    put("cap_sent", json!(a.sent));
    put("cap_status", json!(a.status));
    put("cap_msg_type", json!(a.msg_type));
    put("cap_references", json!(a.references));
    put("cap_history", json!(history));
    put("event", json!(a.event));
    put("headline", json!(title(a)));
    put("severity", json!(a.severity));
    put("urgency", json!(a.urgency));
    put("certainty", json!(a.certainty));
    put("onset", json!(a.onset));
    put("expires", json!(a.expires));
    put("area_desc", json!(a.area_desc));
    put("uf", json!(ufs(a)));
    put("ibge", json!(ibge_codes(a)));
    put("sender_name", json!(a.sender_name));
    put("web", json!(a.web));
    put("state", json!("active"));
    let sev = if a.severity.is_empty() {
        "unknown".to_owned()
    } else {
        a.severity.to_ascii_lowercase()
    };
    put("tags", json!(["alerta", format!("alerta/{sev}")]));

    let callout = match severity(&a.severity) {
        Some(Severity::Extreme | Severity::Severe) => "danger",
        Some(Severity::Moderate) => "warning",
        _ => "info",
    };
    let mut body = format!("# {}\n\n", title(a));
    body.push_str(&format!(
        "> [!{callout}] {} · severidade {}\n",
        one_line(&a.event),
        severity_pt_br(&a.severity)
    ));
    if !a.onset.is_empty() {
        body.push_str(&format!("> Início: {}\n", a.onset));
    }
    if !a.expires.is_empty() {
        body.push_str(&format!("> Fim: {}\n", a.expires));
    }
    if !a.area_desc.is_empty() {
        body.push_str(&format!("> Área: {}\n", one_line(&a.area_desc.join("; "))));
    }
    for (heading, text) in [
        ("Descrição", &a.description),
        ("Instruções", &a.instruction),
    ] {
        if !text.is_empty() {
            body.push_str(&format!("\n## {heading}\n\n{text}\n"));
        }
    }
    body.push_str("\n## Fonte\n\n");
    let who = match (a.sender_name.is_empty(), a.sender.is_empty()) {
        (false, false) => format!("{} ({})", a.sender_name, a.sender),
        (false, true) => a.sender_name.clone(),
        _ => a.sender.clone(),
    };
    body.push_str(&format!("{who}, enviado em {}.", a.sent));
    if !a.web.is_empty() {
        body.push_str(&format!(" <{}>", a.web));
    }
    body.push('\n');
    join(&fm, &body)
}

fn cancel(existing: &str, a: &CapAlert) -> Option<String> {
    let (mut fm, body) = split(existing)?;
    set(&mut fm, "state", json!("cancelled"));
    set(&mut fm, "cancelled_by", json!(a.identifier));
    set(&mut fm, "cancelled_at", json!(a.sent));
    if let Some(Value::Array(tags)) = fm.iter_mut().find(|(k, _)| k == "tags").map(|(_, v)| v) {
        tags.push(json!("alerta/cancelado"));
    }
    let mut body = body.to_owned();
    let who = if a.sender_name.is_empty() {
        &a.sender
    } else {
        &a.sender_name
    };
    body.push_str(&format!(
        "\n## Cancelamento\n\n{who} cancelou este alerta em {}.\n",
        a.sent
    ));
    for text in [&a.headline, &a.description, &a.instruction] {
        if !text.is_empty() {
            body.push_str(&format!("\n{text}\n"));
        }
    }
    Some(join(&fm, &body))
}

/// The note text for `a`, given the note that already exists at its path.
pub fn apply(a: &CapAlert, s: &Settings, existing: Option<&str>) -> Outcome {
    if let Err(reason) = status(a, s) {
        return Outcome::Skip { reason };
    }
    let label = severity_pt_br(&a.severity);
    let Some(existing) = existing else {
        if a.msg_type == "Cancel" {
            return Outcome::Skip {
                reason: "a Cancel for an alert without a note".into(),
            };
        }
        return match filter(a, s) {
            Ok(()) => Outcome::Write {
                text: render(a, &[]),
                notice: format!("Alerta ({label}): {}", title(a)),
            },
            Err(reason) => Outcome::Skip { reason },
        };
    };
    let conflict = || Outcome::Conflict {
        reason: "the note exists but its frontmatter is not the one this plugin writes".into(),
    };
    let Some((fm, _)) = split(existing) else {
        return conflict();
    };
    let Some(held) = get(&fm, "cap_identifier").and_then(Value::as_str) else {
        return conflict();
    };
    if a.msg_type == "Cancel" {
        if get(&fm, "state").and_then(Value::as_str) == Some("cancelled") {
            return Outcome::Skip {
                reason: "the alert is already cancelled".into(),
            };
        }
        return match cancel(existing, a) {
            Some(text) => Outcome::Write {
                text,
                notice: format!("Alerta cancelado: {}", title(a)),
            },
            None => conflict(),
        };
    }
    if held == a.identifier {
        return Outcome::Skip {
            reason: "the note already holds this alert".into(),
        };
    }
    let mut history: Vec<String> = get(&fm, "cap_history")
        .and_then(Value::as_array)
        .map(|h| {
            h.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    if history.iter().any(|h| h == &a.identifier) {
        return Outcome::Skip {
            reason: "the note already held this alert".into(),
        };
    }
    history.push(held.to_owned());
    Outcome::Write {
        text: render(a, &history),
        notice: format!("Alerta atualizado ({label}): {}", title(a)),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::cap::Geocode;

    fn alert() -> CapAlert {
        CapAlert {
            identifier: "id-1".into(),
            sender: "dc@example.gov.br".into(),
            sent: "2026-10-22T12:00:00-03:00".into(),
            status: "Actual".into(),
            msg_type: "Alert".into(),
            event: "Inundação".into(),
            severity: "Severe".into(),
            expires: "2026-10-23T12:00:00-03:00".into(),
            headline: "Risco de inundação: \"rio\" alto".into(),
            description: "O rio pode subir.\n\nEvite as margens.".into(),
            area_desc: vec!["Porto Alegre".into()],
            geocodes: vec![Geocode {
                name: "IBGE".into(),
                value: "4314902".into(),
            }],
            ..CapAlert::default()
        }
    }

    fn write(o: Outcome) -> String {
        match o {
            Outcome::Write { text, .. } => text,
            other => panic!("not a write: {other:?}"),
        }
    }

    #[test]
    fn path_is_one_file_per_identifier() {
        assert_eq!(
            note_path("Alertas/", "urn:oid:2.49/a b"),
            "Alertas/urn_oid_2.49_a_b.md"
        );
        assert_eq!(note_path("", "..x"), "x.md");
    }

    #[test]
    fn new_note_has_cap_frontmatter_and_verbatim_text() {
        let t = write(apply(&alert(), &Settings::default(), None));
        let (fm, body) = split(&t).unwrap();
        assert_eq!(get(&fm, "cap_identifier"), Some(&json!("id-1")));
        assert_eq!(get(&fm, "uf"), Some(&json!(["RS"])));
        assert_eq!(get(&fm, "ibge"), Some(&json!(["4314902"])));
        assert_eq!(get(&fm, "state"), Some(&json!("active")));
        assert!(get(&fm, "onset").is_none());
        assert!(body.starts_with("# Risco de inundação: \"rio\" alto\n"));
        assert!(body.contains("> [!danger] Inundação · severidade severa"));
        assert!(body.contains("O rio pode subir.\n\nEvite as margens."));
    }

    #[test]
    fn filters_apply_to_new_alerts() {
        let s = Settings::default();
        let mut a = alert();
        a.geocodes = vec![Geocode {
            name: "UF".into(),
            value: "SP".into(),
        }];
        assert!(matches!(apply(&a, &s, None), Outcome::Skip { .. }));
        let s2 = Settings {
            municipalities: vec!["3550308".into()],
            ..s.clone()
        };
        a.geocodes = vec![Geocode {
            name: "IBGE".into(),
            value: "3550308".into(),
        }];
        assert!(matches!(apply(&a, &s2, None), Outcome::Write { .. }));
        let s3 = Settings {
            min_severity: "Extreme".into(),
            ..s.clone()
        };
        assert!(matches!(apply(&alert(), &s3, None), Outcome::Skip { .. }));
        let s4 = Settings { ufs: vec![], ..s };
        assert!(matches!(apply(&a, &s4, None), Outcome::Write { .. }));
    }

    #[test]
    fn test_status_needs_the_setting() {
        let mut a = alert();
        a.status = "Test".into();
        assert!(matches!(
            apply(&a, &Settings::default(), None),
            Outcome::Skip { .. }
        ));
        let s = Settings {
            include_tests: true,
            ..Settings::default()
        };
        assert!(matches!(apply(&a, &s, None), Outcome::Write { .. }));
        a.status = "Exercise".into();
        assert!(matches!(apply(&a, &s, None), Outcome::Skip { .. }));
    }

    #[test]
    fn update_rewrites_and_keeps_history() {
        let s = Settings::default();
        let first = write(apply(&alert(), &s, None));
        assert!(matches!(
            apply(&alert(), &s, Some(&first)),
            Outcome::Skip { .. }
        ));
        let mut up = alert();
        up.identifier = "id-2".into();
        up.msg_type = "Update".into();
        up.references = vec!["dc@example.gov.br,id-1,2026-10-22T12:00:00-03:00".into()];
        up.severity = "Minor".into();
        assert_eq!(note_keys(&up), ["id-1", "id-2"]);
        // An update keeps its note even when the new severity is below the filter.
        let s2 = Settings {
            min_severity: "Severe".into(),
            ..s.clone()
        };
        let second = write(apply(&up, &s2, Some(&first)));
        let (fm, _) = split(&second).unwrap();
        assert_eq!(get(&fm, "cap_identifier"), Some(&json!("id-2")));
        assert_eq!(get(&fm, "cap_history"), Some(&json!(["id-1"])));
        let mut again = up.clone();
        again.identifier = "id-1".into();
        assert!(matches!(
            apply(&again, &s, Some(&second)),
            Outcome::Skip { .. }
        ));
    }

    #[test]
    fn cancel_marks_the_note_and_keeps_its_text() {
        let s = Settings::default();
        let first = write(apply(&alert(), &s, None));
        let mut c = alert();
        c.identifier = "id-3".into();
        c.msg_type = "Cancel".into();
        c.references = vec!["dc@example.gov.br,id-1,2026-10-22T12:00:00-03:00".into()];
        c.sender_name = "Defesa Civil RS".into();
        c.headline = "O alerta foi encerrado.".into();
        assert!(matches!(apply(&c, &s, None), Outcome::Skip { .. }));
        let t = write(apply(&c, &s, Some(&first)));
        let (fm, body) = split(&t).unwrap();
        assert_eq!(get(&fm, "state"), Some(&json!("cancelled")));
        assert_eq!(get(&fm, "cancelled_by"), Some(&json!("id-3")));
        assert!(body.contains("Evite as margens."));
        assert!(body.contains("## Cancelamento\n\nDefesa Civil RS cancelou este alerta"));
        assert!(body.contains("\nO alerta foi encerrado.\n"));
        assert!(matches!(apply(&c, &s, Some(&t)), Outcome::Skip { .. }));
    }

    #[test]
    fn a_note_of_another_form_is_a_conflict() {
        let s = Settings::default();
        assert!(matches!(
            apply(&alert(), &s, Some("# minha nota\n")),
            Outcome::Conflict { .. }
        ));
        assert!(matches!(
            apply(&alert(), &s, Some("---\ntitle: minha nota\n---\n")),
            Outcome::Conflict { .. }
        ));
    }
}
