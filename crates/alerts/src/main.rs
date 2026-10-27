//! The alerts service: publishes alerts to Discord and to an Atom + CAP 1.2 feed, and reads
//! the RSS feeds that the rules of `alerts-crud` compose.

use alerts::sources::{Channels, Fetch, Source, deliver, v0_harness, write_feed};
use alerts::{now_utc, post, test_alert, write_atomic};
use alerts_feed::{FeedMeta, Status};
use std::error::Error;

const USAGE: &str = "usage: alerts test-message | poll <source>... | rss-fetch | rss-atom <rule>

Publishes to every configured channel:
  DISCORD_WEBHOOK_URL   post to this Discord webhook
  FEED_PATH             write an Atom + CAP 1.2 feed to this file
  FEED_ID               the feed id URI (default: urn:piabrota-alerts)

Reads the sources and stores what it delivered:
  ALERTS_DB             the database file (default: alerts.db)
  MOCK_ALERTS_PATH      a JSON file of mock alerts, in place of the built-in scenarios

Reads and composes RSS feeds in the same database.

  test-message     publish one test alert (CAP status Test)
  poll <source>... read each source once, run the v0 harness, and deliver each new
                   alert once (sources: mock)
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
                let id = post(&url, &alert).await?;
                println!("discord: test message sent (message {id})");
            }
            if let Some(path) = feed {
                let xml = alerts_feed::atom(&feed_meta(), &[alert], Status::Test, &now_utc())?;
                write_atomic(&path, &xml)?;
                println!("feed: test entry written to {path}");
            }
            Ok(())
        }
        Some("poll") => poll(&std::env::args().skip(2).collect::<Vec<_>>()).await,
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

fn open_db() -> Result<alerts_crud::Db, Box<dyn Error>> {
    let path = std::env::var("ALERTS_DB").unwrap_or_else(|_| "alerts.db".into());
    Ok(alerts_crud::open(std::path::Path::new(&path))?)
}

fn feed_meta() -> FeedMeta {
    FeedMeta {
        id: std::env::var("FEED_ID").unwrap_or_else(|_| "urn:piabrota-alerts".into()),
        title: "alerts".into(),
    }
}

/// Reads each source once and delivers its new alerts to every configured channel. A source
/// that fails is reported, and the others still run; the command fails when any source failed.
async fn poll(ids: &[String]) -> Result<(), Box<dyn Error>> {
    if ids.is_empty() {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
    let sources = ids
        .iter()
        .map(|id| id.parse::<Source>())
        .collect::<Result<Vec<_>, _>>()?;
    let channels = Channels {
        discord: std::env::var("DISCORD_WEBHOOK_URL").ok(),
        feed: std::env::var("FEED_PATH").ok().map(|p| (p, feed_meta())),
    };
    if channels.discord.is_none() && channels.feed.is_none() {
        return Err("no channel: set DISCORD_WEBHOOK_URL or FEED_PATH".into());
    }
    let db = open_db()?;
    let harness = v0_harness();
    let ctx = Fetch {
        now: now_utc(),
        mock_file: std::env::var("MOCK_ALERTS_PATH").ok(),
    };
    let mut failed = 0;
    for source in sources {
        let polled = async {
            let alerts = source.fetch(&ctx).await?;
            deliver(&db, &harness, &channels, source, alerts).await
        };
        match polled.await {
            Ok(r) => println!(
                "{source}: {} read, {} passed the harness, {} delivered",
                r.fetched, r.passed, r.delivered
            ),
            Err(e) => {
                eprintln!("{source}: {e}");
                failed += 1;
            }
        }
    }
    if let Some((path, meta)) = &channels.feed {
        let n = write_feed(&db, path, meta, &ctx.now)?;
        println!("feed: {n} entries written to {path}");
    }
    if failed > 0 {
        return Err(format!("{failed} sources failed").into());
    }
    Ok(())
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
