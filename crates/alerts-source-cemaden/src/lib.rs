//! The CEMADEN source: the municipal alerts of the CEMADEN alert panel.
//!
//! The panel publishes its alerts in an undocumented JSON ([`URL`]). It needs no token. The
//! official PED API needs a token (BYOT, see `alerts-catalog`), but it gives observations, not
//! alerts. An undocumented endpoint can change without notice, so every field this crate does not
//! understand is an error, never a dropped alert.
//!
//! The crate does no I/O: the service reads the panel, and this crate turns its text into
//! `Alert`s.

use alerts_core::{Alert, Hazard, Severity, Uf};
use serde::Deserialize;
use std::fmt;

/// The source id of every CEMADEN alert.
pub const ID: &str = "cemaden";

/// The JSON of the active municipal alerts.
pub const URL: &str = "https://painelalertas.cemaden.gov.br/wsAlertas2";

/// The panel page, for people. Every alert links to it.
pub const PANEL: &str = "https://painelalertas.cemaden.gov.br/";

/// The CEMADEN times have no offset. They are taken as Brasília time (UTC-3) ⚠: nobody checked
/// it with CEMADEN.
const OFFSET: &str = "-03:00";

#[derive(Debug, Deserialize)]
struct Panel {
    alertas: Vec<Alerta>,
    /// The time of the panel, for example `29-10-2026 00:45:01 UTC`. Read only to check that the
    /// answer is the panel.
    #[allow(dead_code)]
    atualizado: String,
}

#[derive(Debug, Deserialize)]
struct Alerta {
    cod_alerta: u64,
    ult_atualizacao: String,
    codibge: u64,
    /// `<kind> - <level>`, for example `Risco Hidrológico - Alto`.
    evento: String,
    nivel: String,
    status: i64,
    uf: String,
    municipio: String,
}

#[derive(Debug)]
pub enum CemadenError {
    /// The answer is not the JSON of the panel.
    Json(serde_json::Error),
    /// A field has a value that this crate does not know. The panel changed: a human must look.
    Unknown {
        alert: u64,
        field: &'static str,
        value: String,
    },
}

impl fmt::Display for CemadenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CemadenError::Json(e) => {
                write!(f, "the CEMADEN panel answer is not the known JSON: {e}")
            }
            CemadenError::Unknown {
                alert,
                field,
                value,
            } => {
                write!(f, "CEMADEN alert {alert}: unknown {field} {value:?}")
            }
        }
    }
}

impl std::error::Error for CemadenError {}

/// The alerts of one answer of [`URL`]. An empty list is valid: no municipality is under alert.
pub fn parse(json: &str) -> Result<Vec<Alert>, CemadenError> {
    let panel: Panel = serde_json::from_str(json).map_err(CemadenError::Json)?;
    panel.alertas.into_iter().map(alert).collect()
}

fn alert(a: Alerta) -> Result<Alert, CemadenError> {
    let unknown = |field, value: &str| CemadenError::Unknown {
        alert: a.cod_alerta,
        field,
        value: value.to_owned(),
    };
    // Every alert seen on 2026-10-28 had status 1. Another value may mean an ended alert ⚠.
    if a.status != 1 {
        return Err(unknown("status", &a.status.to_string()));
    }
    let uf: Uf = a.uf.parse().map_err(|_| unknown("uf", &a.uf))?;
    if Uf::from_ibge(&a.codibge.to_string()) != Some(uf) {
        return Err(unknown("codibge", &a.codibge.to_string()));
    }
    let severity = severity(&a.nivel).ok_or_else(|| unknown("nivel", &a.nivel))?;
    let (kind, level) = a
        .evento
        .split_once(" - ")
        .ok_or_else(|| unknown("evento", &a.evento))?;
    if level != a.nivel {
        return Err(unknown("evento", &a.evento));
    }
    let hazard = match kind {
        "Risco Hidrológico" => Hazard::Flood,
        "Movimentos de Massa" => Hazard::Landslide,
        _ => return Err(unknown("evento", &a.evento)),
    };
    let sent =
        time(&a.ult_atualizacao).ok_or_else(|| unknown("ult_atualizacao", &a.ult_atualizacao))?;
    // The id holds the update time: when CEMADEN changes an alert, the service delivers it again.
    let stamp: String = sent.chars().take(19).filter(char::is_ascii_digit).collect();
    Ok(Alert {
        id: format!("{}-{stamp}", a.cod_alerta),
        source: ID.into(),
        hazard,
        severity,
        ufs: vec![uf],
        headline: format!("{} em {} ({uf}).", a.evento, a.municipio),
        area: format!("{} ({uf}), código IBGE {}.", a.municipio, a.codibge),
        sent,
        url: Some(PANEL.into()),
    })
}

/// The CEMADEN levels as CAP severities. CEMADEN has three levels: moderate, high and very high
/// (`Muito Alto` was not seen in the panel ⚠).
fn severity(nivel: &str) -> Option<Severity> {
    match nivel {
        "Moderado" => Some(Severity::Moderate),
        "Alto" => Some(Severity::Severe),
        "Muito Alto" => Some(Severity::Extreme),
        _ => None,
    }
}

/// `2026-10-25 11:20:44.159` as `2026-10-25T11:20:44-03:00`.
fn time(t: &str) -> Option<String> {
    let b = t.as_bytes();
    let digits = [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18];
    let ok = b.len() >= 19
        && digits.iter().all(|&i| b[i].is_ascii_digit())
        && [(4, b'-'), (7, b'-'), (10, b' '), (13, b':'), (16, b':')]
            .iter()
            .all(|&(i, c)| b[i] == c)
        && (b.len() == 19 || (b[19] == b'.' && b[20..].iter().all(u8::is_ascii_digit)));
    ok.then(|| format!("{}T{}{OFFSET}", &t[..10], &t[11..19]))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    /// The shape and the values of three alerts of `wsAlertas2` on 2026-10-28.
    const PANEL_JSON: &str = r#"{"alertas":[
        {"cod_alerta":3141,"datahoracriacao":"2026-10-25 11:20:44.159","ult_atualizacao":"2026-10-25 11:20:44.159","codibge":4318002,"evento":"Risco Hidrológico - Alto","nivel":"Alto","status":1,"uf":"RS","municipio":"SÃO BORJA","latitude":-28.746805688578,"longitude":-55.796610881728},
        {"cod_alerta":3154,"datahoracriacao":"2026-10-25 17:42:03.819","ult_atualizacao":"2026-10-25 17:42:03.819","codibge":3520301,"evento":"Risco Hidrológico - Moderado","nivel":"Moderado","status":1,"uf":"SP","municipio":"IGUAPE","latitude":-24.549631967648,"longitude":-47.47466614198},
        {"cod_alerta":3163,"datahoracriacao":"2026-10-27 05:12:39.979","ult_atualizacao":"2026-10-27 05:12:39.979","codibge":2905602,"evento":"Movimentos de Massa - Moderado","nivel":"Moderado","status":1,"uf":"BA","municipio":"CAMACAN","latitude":-15.445822667921,"longitude":-39.518998924364}
    ],"atualizado":"29-10-2026 00:45:01 UTC"}"#;

    fn with(field: &str, from: &str, to: &str) -> String {
        let one = PANEL_JSON.replacen(from, to, 1);
        assert_ne!(one, PANEL_JSON, "{field}: {from} not in the sample");
        one
    }

    #[test]
    fn parses_the_panel() {
        let all = parse(PANEL_JSON).unwrap();
        assert_eq!(all.len(), 3);
        let a = &all[0];
        assert_eq!(a.id, "3141-20261025112044");
        assert_eq!(a.source, ID);
        assert_eq!(a.hazard, Hazard::Flood);
        assert_eq!(a.severity, Severity::Severe);
        assert_eq!(a.ufs, vec![Uf::Rs]);
        assert_eq!(a.headline, "Risco Hidrológico - Alto em SÃO BORJA (RS).");
        assert_eq!(a.area, "SÃO BORJA (RS), código IBGE 4318002.");
        assert_eq!(a.sent, "2026-10-25T11:20:44-03:00");
        assert_eq!(all[2].hazard, Hazard::Landslide);
        assert_eq!(all[2].severity, Severity::Moderate);
    }

    #[test]
    fn no_alert_is_valid() {
        assert!(
            parse(r#"{"alertas":[],"atualizado":"x"}"#)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_changed_panel_fails_loudly() {
        assert!(matches!(parse("{}"), Err(CemadenError::Json(_))));
        assert!(matches!(parse("<html>"), Err(CemadenError::Json(_))));
        for (field, from, to) in [
            ("status", r#""status":1"#, r#""status":0"#),
            ("nivel", r#""nivel":"Alto""#, r#""nivel":"Extremo""#),
            ("evento", "Risco Hidrológico - Alto", "Seca - Alto"),
            (
                "evento",
                "Risco Hidrológico - Alto",
                "Risco Hidrológico - Moderado",
            ),
            ("uf", r#""uf":"RS""#, r#""uf":"XX""#),
            ("codibge", "4318002", "4106902"),
            (
                "ult_atualizacao",
                r#""ult_atualizacao":"2026-10-25 11:20:44.159""#,
                r#""ult_atualizacao":"25/10/2026""#,
            ),
        ] {
            match parse(&with(field, from, to)) {
                Err(CemadenError::Unknown {
                    alert: 3141,
                    field: f,
                    ..
                }) => assert_eq!(f, field),
                other => panic!("{field}: {other:?}"),
            }
        }
    }

    #[test]
    fn times() {
        assert_eq!(
            time("2026-10-28 04:15:34.38").unwrap(),
            "2026-10-28T04:15:34-03:00"
        );
        assert_eq!(
            time("2026-10-28 04:15:34").unwrap(),
            "2026-10-28T04:15:34-03:00"
        );
        assert!(time("2026-10-28T04:15:34").is_none());
        assert!(time("2026-10-28 04:15:34.x").is_none());
        assert!(time("").is_none());
    }
}
