//! The alerts service: publishes alerts to Discord and to an Atom + CAP 1.2 feed.

use alerts_core::{Alert, Hazard, Severity, Uf};
use alerts_feed::{FeedMeta, Status};
use serde_json::{Value, json};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE: &str = "usage: alerts test-message

Publishes to every configured channel:
  DISCORD_WEBHOOK_URL   post to this Discord webhook
  FEED_PATH             write an Atom + CAP 1.2 feed to this file
  FEED_ID               the feed id URI (default: urn:piabrota-alerts)

  test-message   publish one test alert (CAP status Test)";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    match std::env::args().nth(1).as_deref() {
        Some("test-message") => {
            let alert = test_alert(&now_utc());
            let discord = std::env::var("DISCORD_WEBHOOK_URL").ok();
            let feed = std::env::var("FEED_PATH").ok();
            if discord.is_none() && feed.is_none() {
                return Err("no channel: set DISCORD_WEBHOOK_URL or FEED_PATH".into());
            }
            if let Some(url) = discord {
                post(&url, &alert).await?;
                println!("discord: test message sent");
            }
            if let Some(path) = feed {
                let meta = FeedMeta {
                    id: std::env::var("FEED_ID").unwrap_or_else(|_| "urn:piabrota-alerts".into()),
                    title: "alerts".into(),
                };
                let xml = alerts_feed::atom(&meta, &[alert], Status::Test, &now_utc())?;
                write_atomic(&path, &xml)?;
                println!("feed: test entry written to {path}");
            }
            Ok(())
        }
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
}

async fn post(webhook: &str, alert: &Alert) -> Result<(), Box<dyn Error>> {
    reqwest::Client::builder()
        .user_agent(alerts_core::USER_AGENT)
        .build()?
        .post(webhook)
        .json(&payload(alert))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

/// Readers never see a half-written feed: write a sibling file, then rename it.
fn write_atomic(path: &str, contents: &str) -> std::io::Result<()> {
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

fn severity_color(s: Severity) -> u32 {
    match s {
        Severity::Minor => 0x1682D4,
        Severity::Moderate => 0xF5C518,
        Severity::Severe => 0xF28C28,
        Severity::Extreme => 0xD7263D,
    }
}

/// The Discord webhook body for one alert: one embed, pt-BR labels.
fn payload(a: &Alert) -> Value {
    let ufs = a
        .ufs
        .iter()
        .map(|u| u.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut embed = json!({
        "title": a.headline,
        "description": a.area,
        "color": severity_color(a.severity),
        "fields": [
            { "name": "Severidade", "value": a.severity.label_pt_br(), "inline": true },
            { "name": "UF", "value": ufs, "inline": true },
            { "name": "Fonte", "value": a.source, "inline": true },
        ],
    });
    // Discord rejects null or empty url and timestamp, so add them only when present.
    if let Some(url) = &a.url {
        embed["url"] = json!(url);
    }
    if !a.sent.is_empty() {
        embed["timestamp"] = json!(a.sent);
    }
    json!({ "embeds": [embed] })
}

fn test_alert(sent: &str) -> Alert {
    Alert {
        id: "alerts-test".into(),
        source: "alerts".into(),
        hazard: Hazard::Flood,
        severity: Severity::Minor,
        ufs: Uf::V0.to_vec(),
        headline: "TESTE: esta mensagem não é um alerta real.".into(),
        area: "O alerts enviou esta mensagem para testar os canais de publicação.".into(),
        sent: sent.into(),
        url: None,
    }
}

/// The current time as `YYYY-MM-DDThh:mm:ss+00:00`, the form CAP 1.2 and Atom accept.
fn now_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    rfc3339_utc(secs)
}

fn rfc3339_utc(secs: u64) -> String {
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let (h, mi, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}+00:00")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_has_pt_br_fields() {
        let mut a = test_alert("");
        a.severity = Severity::Extreme;
        let p = payload(&a);
        let embed = &p["embeds"][0];
        assert_eq!(embed["fields"][0]["value"], "extrema");
        assert_eq!(embed["fields"][1]["value"], "PR, SC, RS");
        assert_eq!(embed["color"], 0xD7263D);
        assert!(embed.get("url").is_none() && embed.get("timestamp").is_none());
    }

    #[test]
    fn rfc3339_known_dates() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00+00:00");
        assert_eq!(rfc3339_utc(951_782_400), "2000-02-29T00:00:00+00:00");
        assert_eq!(rfc3339_utc(1_792_627_200), "2026-10-22T00:00:00+00:00");
    }
}
