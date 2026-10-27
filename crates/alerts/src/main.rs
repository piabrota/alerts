//! The alerts service: publishes alerts to Discord and to an Atom + CAP 1.2 feed, and reads
//! the RSS feeds that the rules of `alerts-crud` compose.

use alerts_core::{Alert, Hazard, Severity, Uf};
use alerts_feed::{FeedMeta, Status};
use serde_json::{Value, json};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE: &str = "usage: alerts test-message | rss-fetch | rss-atom <rule>

Publishes to every configured channel:
  DISCORD_WEBHOOK_URL   post to this Discord webhook
  FEED_PATH             write an Atom + CAP 1.2 feed to this file
  FEED_ID               the feed id URI (default: urn:piabrota-alerts)

Reads and composes RSS feeds in the alerts-crud database:
  ALERTS_DB             the database file (default: alerts.db)

  test-message     publish one test alert (CAP status Test)
  rss-fetch        read every enabled feed and store its items
  rss-atom <rule>  print the Atom feed of the rule with this slug";

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
        Some("rss-fetch") => rss_fetch(&open_db()?).await,
        Some("rss-atom") => {
            let Some(slug) = std::env::args().nth(2) else {
                eprintln!("{USAGE}");
                std::process::exit(2);
            };
            let db = open_db()?;
            let rule = alerts_crud::model::find_by::<alerts_crud::models::rules::Rule>(
                &db, "slug", &slug,
            )?
            .ok_or_else(|| format!("no rule {slug}"))?;
            print!("{}", alerts_crud::controllers::rules::atom(&db, rule.id)?);
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

fn open_db() -> Result<alerts_crud::Db, Box<dyn Error>> {
    let path = std::env::var("ALERTS_DB").unwrap_or_else(|_| "alerts.db".into());
    Ok(alerts_crud::open(std::path::Path::new(&path))?)
}

/// Reads each enabled feed once and stores its items. A feed that fails is reported, and the
/// others still run; the command fails when any feed failed.
async fn rss_fetch(db: &alerts_crud::Db) -> Result<(), Box<dyn Error>> {
    use alerts_crud::models::feeds::Feed;
    let client = reqwest::Client::builder()
        .user_agent(alerts_core::USER_AGENT)
        .build()?;
    let mut failed = 0;
    for feed in alerts_crud::model::list::<Feed>(db)?
        .into_iter()
        .filter(|f| f.enabled)
    {
        let read = async {
            let xml = client
                .get(&feed.url)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?;
            let items = alerts_rss::parse(&xml, &feed.slug)?;
            Ok::<_, Box<dyn Error>>(alerts_crud::models::feed_items::record_items(
                db, &feed.slug, &items,
            )?)
        };
        match read.await {
            Ok(n) => println!("{}: {n} items", feed.slug),
            Err(e) => {
                eprintln!("{}: {e}", feed.slug);
                failed += 1;
            }
        }
    }
    if failed > 0 {
        return Err(format!("{failed} feeds failed").into());
    }
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
}
