//! The channels of the alerts service: the Discord webhook body and its delivery, the test
//! alert, and the feed file. `main.rs` and the delivery tests in `tests/` use them.

use alerts_core::{Alert, Hazard, Severity, Uf};
use serde_json::{Value, json};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

/// A Discord webhook URL, the only kind of URL that `post` accepts.
pub fn is_discord_webhook(url: &str) -> bool {
    [
        "https://discord.com/api/webhooks/",
        "https://discordapp.com/api/webhooks/",
    ]
    .iter()
    .any(|p| url.starts_with(p))
}

/// Posts one alert to a Discord webhook and returns the id of the message that Discord stored.
/// An error never contains the webhook URL: the URL holds the webhook token.
pub async fn post(webhook: &str, alert: &Alert) -> Result<String, Box<dyn Error>> {
    if !is_discord_webhook(webhook) {
        return Err("the webhook URL is not a Discord webhook URL".into());
    }
    // `wait=true` makes Discord answer with the stored message, so a `2xx` proves delivery.
    let sep = if webhook.contains('?') { '&' } else { '?' };
    let message: Value = reqwest::Client::builder()
        .user_agent(alerts_core::USER_AGENT)
        .build()?
        .post(format!("{webhook}{sep}wait=true"))
        .json(&payload(alert))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.without_url())?
        .json()
        .await
        .map_err(|e| e.without_url())?;
    match message["id"].as_str() {
        Some(id) if !id.is_empty() => Ok(id.to_owned()),
        _ => Err("Discord answered without a message id".into()),
    }
}

/// Readers never see a half-written feed: write a sibling file, then rename it.
pub fn write_atomic(path: &str, contents: &str) -> std::io::Result<()> {
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
pub fn payload(a: &Alert) -> Value {
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

pub fn test_alert(sent: &str) -> Alert {
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
pub fn now_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    rfc3339_utc(secs)
}

fn rfc3339_utc(secs: u64) -> String {
    alerts_rss::time::rfc3339_utc(i64::try_from(secs).unwrap_or(i64::MAX))
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

    #[test]
    fn only_discord_webhooks() {
        assert!(is_discord_webhook("https://discord.com/api/webhooks/1/abc"));
        assert!(is_discord_webhook(
            "https://discordapp.com/api/webhooks/1/abc"
        ));
        assert!(!is_discord_webhook("http://discord.com/api/webhooks/1/abc"));
        assert!(!is_discord_webhook(
            "https://discord.com.example.org/api/webhooks/1"
        ));
        assert!(!is_discord_webhook(""));
    }
}
