//! The mock source: simulated alerts, to try the harness and the channels with no agency.
//!
//! Every mock alert is a test. Its headline starts with `TESTE:`, its source is `mock`, and the
//! service publishes it with CAP status Test. A mock alert is never published as an alert of an
//! agency (hard rule "Official sources only").
//!
//! The mock does no I/O: the service reads the mock file, and this crate turns its text into
//! `Alert`s.

use alerts_core::{Alert, Hazard, Severity, Uf};
use serde::Deserialize;
use std::fmt;

/// The source id of every mock alert.
pub const ID: &str = "mock";

/// The start of every mock headline, so a reader never takes a mock alert for a real one.
pub const PREFIX: &str = "TESTE:";

/// One simulated alert, as a person writes it in a mock file. The mock fills the id, the source
/// and the time.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MockAlert {
    #[serde(default = "flood")]
    pub hazard: Hazard,
    pub severity: Severity,
    pub ufs: Vec<Uf>,
    pub headline: String,
    pub area: String,
}

fn flood() -> Hazard {
    Hazard::Flood
}

#[derive(Debug)]
pub enum MockError {
    /// The mock file is not a JSON array of mock alerts.
    Json(serde_json::Error),
    /// The mock file has no alert. A source never answers an empty list in silence.
    Empty,
}

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MockError::Json(e) => write!(f, "invalid mock file: {e}"),
            MockError::Empty => f.write_str("the mock file has no alert"),
        }
    }
}

impl std::error::Error for MockError {}

/// The built-in scenarios: one alert per severity in the v0 states, and one alert in SP that the
/// v0 harness drops.
pub fn scenarios(sent: &str) -> Vec<Alert> {
    let mock = |severity, ufs: &[Uf], headline: &str, area: &str| MockAlert {
        hazard: Hazard::Flood,
        severity,
        ufs: ufs.to_vec(),
        headline: headline.into(),
        area: area.into(),
    };
    alerts(
        vec![
            mock(
                Severity::Minor,
                &[Uf::Pr],
                "alerta simulado de inundação de severidade menor.",
                "Área simulada no Paraná. Este alerta não é real.",
            ),
            mock(
                Severity::Moderate,
                &[Uf::Sc],
                "alerta simulado de inundação de severidade moderada.",
                "Área simulada em Santa Catarina. Este alerta não é real.",
            ),
            mock(
                Severity::Severe,
                &[Uf::Rs],
                "alerta simulado de inundação de severidade severa.",
                "Área simulada no Rio Grande do Sul. Este alerta não é real.",
            ),
            mock(
                Severity::Extreme,
                Uf::V0,
                "alerta simulado de inundação de severidade extrema.",
                "Área simulada no Paraná, em Santa Catarina e no Rio Grande do Sul. Este alerta não é real.",
            ),
            mock(
                Severity::Severe,
                &[Uf::Sp],
                "alerta simulado fora da região do v0.",
                "Área simulada em São Paulo. O harness do v0 descarta este alerta.",
            ),
        ],
        sent,
    )
}

/// The alerts of a mock file: a JSON array of [`MockAlert`].
pub fn parse(json: &str, sent: &str) -> Result<Vec<Alert>, MockError> {
    let mocks: Vec<MockAlert> = serde_json::from_str(json).map_err(MockError::Json)?;
    if mocks.is_empty() {
        return Err(MockError::Empty);
    }
    Ok(alerts(mocks, sent))
}

/// Turns mock alerts into `Alert`s sent at `sent`. Each call is a new simulated event: the ids
/// hold the time, so the service delivers the alerts of a new call again.
pub fn alerts(mocks: Vec<MockAlert>, sent: &str) -> Vec<Alert> {
    let stamp: String = sent.chars().filter(char::is_ascii_digit).collect();
    mocks
        .into_iter()
        .enumerate()
        .map(|(n, m)| Alert {
            id: format!("{stamp}-{}", n + 1),
            source: ID.into(),
            hazard: m.hazard,
            severity: m.severity,
            ufs: m.ufs,
            headline: mark(&m.headline),
            area: m.area,
            sent: sent.into(),
            url: None,
        })
        .collect()
}

/// The headline with [`PREFIX`] at the start, added only when it is missing.
fn mark(headline: &str) -> String {
    let headline = headline.trim();
    if headline.starts_with(PREFIX) {
        headline.to_owned()
    } else {
        format!("{PREFIX} {headline}")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const SENT: &str = "2026-10-27T12:00:00+00:00";

    #[test]
    fn every_mock_alert_is_marked_as_a_test() {
        let all = scenarios(SENT);
        assert_eq!(all.len(), 5);
        for a in &all {
            assert!(a.headline.starts_with("TESTE: "), "{}", a.headline);
            assert_eq!(a.source, ID);
            assert_eq!(a.sent, SENT);
        }
    }

    #[test]
    fn ids_are_unique_and_hold_the_time() {
        let all = scenarios(SENT);
        assert_eq!(all[0].id, "202610271200000000-1");
        let mut ids: Vec<_> = all.iter().map(|a| a.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
        assert_ne!(scenarios("2026-10-27T12:00:01+00:00")[0].id, all[0].id);
    }

    #[test]
    fn parses_a_mock_file() {
        let json = r#"[
            {"severity": "Severe", "ufs": ["RS"], "headline": "rio acima da cota.", "area": "Área simulada."},
            {"hazard": "Flood", "severity": "Minor", "ufs": ["PR", "SC"], "headline": "TESTE: já marcado.", "area": "Área simulada."}
        ]"#;
        let all = parse(json, SENT).unwrap();
        assert_eq!(all[0].headline, "TESTE: rio acima da cota.");
        assert_eq!(all[0].hazard, Hazard::Flood);
        assert_eq!(all[1].headline, "TESTE: já marcado.");
        assert_eq!(all[1].ufs, vec![Uf::Pr, Uf::Sc]);
    }

    #[test]
    fn a_bad_mock_file_fails_loudly() {
        assert!(matches!(parse("[]", SENT), Err(MockError::Empty)));
        assert!(matches!(parse("{}", SENT), Err(MockError::Json(_))));
        let unknown =
            r#"[{"severity": "Severe", "ufs": ["RS"], "headline": "x", "area": "y", "id": "1"}]"#;
        assert!(matches!(parse(unknown, SENT), Err(MockError::Json(_))));
        let bad_uf = r#"[{"severity": "Severe", "ufs": ["XX"], "headline": "x", "area": "y"}]"#;
        assert!(matches!(parse(bad_uf, SENT), Err(MockError::Json(_))));
    }

    #[test]
    fn the_example_file_parses() {
        let json = include_str!("../tests/fixtures/alerts.json");
        assert!(!parse(json, SENT).unwrap().is_empty());
    }
}
