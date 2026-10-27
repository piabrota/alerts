//! The active alerts of the vault: the status bar line and the `alerts` code block.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A CAP time (`YYYY-MM-DDThh:mm:ss±hh:mm`, or with `Z`) as seconds since 1970-01-01 UTC.
pub fn epoch(t: &str) -> Option<i64> {
    let b = t.as_bytes();
    let num = |r: std::ops::Range<usize>| -> Option<i64> { t.get(r)?.parse().ok() };
    let shape = b.len() >= 19
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':';
    if !shape {
        return None;
    }
    let offset = match t.get(19..)? {
        "Z" | "" => 0,
        o if o.len() == 6 && o.as_bytes()[3] == b':' => {
            let sign = match o.as_bytes()[0] {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            sign * (num(20..22)? * 3600 + num(23..25)? * 60)
        }
        _ => return None,
    };
    let (y, m, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Days since 1970-01-01 from a civil date (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + num(11..13)? * 3600 + num(14..16)? * 60 + num(17..19)? - offset)
}

/// A note of the alerts folder: its path and its frontmatter, as Obsidian read it.
#[derive(Debug, Deserialize)]
pub struct NoteMeta {
    pub path: String,
    #[serde(default)]
    pub frontmatter: Value,
}

/// The last poll of the feed, as the plugin saw it.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PollState {
    pub feed_url: String,
    /// The time of the last successful read, in the local form the plugin shows.
    pub ok_at: String,
    /// The error of the last read, empty when the last read worked.
    pub error: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Active {
    pub path: String,
    pub headline: String,
    pub severity: String,
    pub expires: String,
}

fn str_of<'a>(fm: &'a Value, key: &str) -> &'a str {
    fm.get(key).and_then(Value::as_str).unwrap_or("")
}

fn rank(s: &str) -> u8 {
    match s {
        "Extreme" => 4,
        "Severe" => 3,
        "Moderate" => 2,
        "Minor" => 1,
        _ => 0,
    }
}

/// The alerts that are not cancelled and not expired at `now`, the most severe first.
/// A note without `cap_identifier` is not an alert note.
pub fn active(notes: &[NoteMeta], now: i64) -> Vec<Active> {
    let mut out: Vec<(Active, i64)> = notes
        .iter()
        .filter(|n| !str_of(&n.frontmatter, "cap_identifier").is_empty())
        .filter(|n| str_of(&n.frontmatter, "state") == "active")
        .filter(|n| match str_of(&n.frontmatter, "expires") {
            "" => true,
            e => epoch(e).is_none_or(|t| t > now),
        })
        .map(|n| {
            let fm = &n.frontmatter;
            let path = n.path.strip_suffix(".md").unwrap_or(&n.path).to_owned();
            let headline = match str_of(fm, "headline") {
                "" => path.rsplit('/').next().unwrap_or("").to_owned(),
                h => h.to_owned(),
            };
            let sent = epoch(str_of(fm, "cap_sent")).unwrap_or(0);
            let a = Active {
                path,
                headline,
                severity: str_of(fm, "severity").to_owned(),
                expires: str_of(fm, "expires").to_owned(),
            };
            (a, sent)
        })
        .collect();
    out.sort_by(|(a, sa), (b, sb)| rank(&b.severity).cmp(&rank(&a.severity)).then(sb.cmp(sa)));
    out.into_iter().map(|(a, _)| a).collect()
}

fn label(severity: &str) -> &'static str {
    match severity {
        "Extreme" => "extrema",
        "Severe" => "severa",
        "Moderate" => "moderada",
        "Minor" => "menor",
        _ => "desconhecida",
    }
}

/// The line under every list: where the alerts come from, and when the last read worked or
/// failed. A failed read is never shown as "no alert".
fn poll_line(p: &PollState) -> String {
    match (p.error.is_empty(), p.ok_at.is_empty()) {
        (false, _) => format!(
            "⚠ A última leitura de <{}> falhou: {}. A lista pode estar incompleta.",
            p.feed_url, p.error
        ),
        (true, true) => format!("⚠ O feed <{}> ainda não foi lido.", p.feed_url),
        (true, false) => format!("Última leitura de <{}>: {}.", p.feed_url, p.ok_at),
    }
}

/// The Markdown of the `alerts` code block.
pub fn markdown(list: &[Active], p: &PollState) -> String {
    let mut s = String::new();
    if list.is_empty() {
        if p.error.is_empty() && !p.ok_at.is_empty() {
            s.push_str("Nenhum alerta ativo nas notas.\n");
        }
    } else {
        for a in list {
            // `|` and `]` end a wikilink, so the label drops them.
            let shown: String = a
                .headline
                .chars()
                .map(|c| if "|[]".contains(c) { ' ' } else { c })
                .collect();
            s.push_str(&format!(
                "- **{}** [[{}|{}]]",
                label(&a.severity),
                a.path,
                shown.trim()
            ));
            if !a.expires.is_empty() {
                s.push_str(&format!(" · até {}", a.expires));
            }
            s.push('\n');
        }
    }
    s.push_str(&format!("\n{}\n", poll_line(p)));
    s
}

/// The status bar line.
pub fn status_text(count: usize, p: &PollState) -> String {
    if !p.error.is_empty() {
        return "⚠ Alertas: falha na leitura".into();
    }
    if p.ok_at.is_empty() {
        return "Alertas: aguardando leitura".into();
    }
    match count {
        0 => "Alertas: nenhum ativo".into(),
        1 => "Alertas: 1 ativo".into(),
        n => format!("Alertas: {n} ativos"),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    #[test]
    fn epoch_known_times() {
        assert_eq!(epoch("1970-01-01T00:00:00+00:00"), Some(0));
        assert_eq!(epoch("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(epoch("2000-02-29T00:00:00+00:00"), Some(951_782_400));
        assert_eq!(epoch("2026-10-21T21:00:00-03:00"), Some(1_792_627_200));
        assert_eq!(epoch("2026-10-22"), None);
        assert_eq!(epoch("2026-13-22T00:00:00Z"), None);
        assert_eq!(epoch("2026-10-22T00:00:00 03:00"), None);
    }

    fn note(path: &str, fm: Value) -> NoteMeta {
        NoteMeta {
            path: path.into(),
            frontmatter: fm,
        }
    }

    #[test]
    fn active_drops_cancelled_expired_and_other_notes() {
        let now = epoch("2026-10-22T12:00:00Z").unwrap();
        let notes = [
            note(
                "A/a.md",
                json!({"cap_identifier": "a", "state": "active", "severity": "Moderate",
                "headline": "Chuva [forte] | vento", "expires": "2026-10-23T00:00:00Z"}),
            ),
            note(
                "A/b.md",
                json!({"cap_identifier": "b", "state": "active", "severity": "Extreme"}),
            ),
            note(
                "A/c.md",
                json!({"cap_identifier": "c", "state": "cancelled"}),
            ),
            note(
                "A/d.md",
                json!({"cap_identifier": "d", "state": "active", "expires": "2026-10-22T11:00:00Z"}),
            ),
            note("A/e.md", json!({"title": "minha nota"})),
            note("A/f.md", Value::Null),
        ];
        let list = active(&notes, now);
        assert_eq!(
            list.iter().map(|a| a.path.as_str()).collect::<Vec<_>>(),
            ["A/b", "A/a"]
        );
        assert_eq!(list[0].headline, "b");
        let ok = PollState {
            feed_url: "https://x/feed".into(),
            ok_at: "12:00".into(),
            error: String::new(),
        };
        let md = markdown(&list, &ok);
        assert!(md.starts_with("- **extrema** [[A/b|b]]\n- **moderada** [[A/a|Chuva  forte    vento]] · até 2026-10-23T00:00:00Z\n"));
        assert!(md.ends_with("Última leitura de <https://x/feed>: 12:00.\n"));
        assert_eq!(status_text(list.len(), &ok), "Alertas: 2 ativos");
    }

    #[test]
    fn a_failed_read_is_never_no_alert() {
        let failed = PollState {
            feed_url: "u".into(),
            ok_at: "11:00".into(),
            error: "HTTP 500".into(),
        };
        let md = markdown(&[], &failed);
        assert!(!md.contains("Nenhum alerta"));
        assert!(md.contains("falhou: HTTP 500"));
        assert_eq!(status_text(0, &failed), "⚠ Alertas: falha na leitura");
        assert_eq!(
            status_text(0, &PollState::default()),
            "Alertas: aguardando leitura"
        );
        let ok = PollState {
            error: String::new(),
            ..failed
        };
        assert!(markdown(&[], &ok).starts_with("Nenhum alerta ativo nas notas.\n"));
    }
}
