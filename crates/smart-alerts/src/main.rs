//! The smart-alerts Discord service.

use serde_json::{Value, json};
use smart_alerts_core::{Alert, Hazard, Severity, Uf};
use std::error::Error;

const USAGE: &str = "usage: smart-alerts test-message

Reads the Discord webhook URL from DISCORD_WEBHOOK_URL.

  test-message   post one test alert to the webhook";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    match std::env::args().nth(1).as_deref() {
        Some("test-message") => {
            let url = std::env::var("DISCORD_WEBHOOK_URL")
                .map_err(|_| "DISCORD_WEBHOOK_URL is not set")?;
            post(&url, &test_alert()).await?;
            println!("test message sent");
            Ok(())
        }
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
}

async fn post(webhook: &str, alert: &Alert) -> Result<(), Box<dyn Error>> {
    reqwest::Client::new()
        .post(webhook)
        .json(&payload(alert))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

fn severity_label(s: Severity) -> (&'static str, u32) {
    match s {
        Severity::Minor => ("menor", 0x1682D4),
        Severity::Moderate => ("moderada", 0xF5C518),
        Severity::Severe => ("severa", 0xF28C28),
        Severity::Extreme => ("extrema", 0xD7263D),
    }
}

/// The Discord webhook body for one alert: one embed, pt-BR labels.
fn payload(a: &Alert) -> Value {
    let (label, color) = severity_label(a.severity);
    let ufs = a
        .ufs
        .iter()
        .map(|u| u.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut embed = json!({
        "title": a.headline,
        "description": a.area,
        "color": color,
        "fields": [
            { "name": "Severidade", "value": label, "inline": true },
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

fn test_alert() -> Alert {
    Alert {
        id: "smart-alerts-test".into(),
        source: "smart-alerts".into(),
        hazard: Hazard::Flood,
        severity: Severity::Minor,
        ufs: Uf::V0.to_vec(),
        headline: "TESTE: esta mensagem não é um alerta real.".into(),
        area: "O smart-alerts enviou esta mensagem para testar o webhook do Discord.".into(),
        sent: String::new(),
        url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_has_pt_br_fields() {
        let mut a = test_alert();
        a.severity = Severity::Extreme;
        let p = payload(&a);
        let embed = &p["embeds"][0];
        assert_eq!(embed["fields"][0]["value"], "extrema");
        assert_eq!(embed["fields"][1]["value"], "PR, SC, RS");
        assert_eq!(embed["color"], 0xD7263D);
        assert!(embed.get("url").is_none() && embed.get("timestamp").is_none());
    }
}
