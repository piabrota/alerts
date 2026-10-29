//! The alerts service: publishes alerts to Discord and to an Atom + CAP 1.2 feed, and reads
//! the RSS feeds that the rules of `alerts-crud` compose.

use alerts::sources::{Channels, Fetch, Source, deliver, harness, ufs_of, write_feed};
use alerts::{env, load_dotenv, now_utc, post, test_alert, write_atomic};
use alerts_feed::{FeedMeta, Status};
use std::error::Error;

const USAGE: &str =
    "usage: alerts test-message | poll [<source>...] | sources [<filter>...] | tokens
              | rss-fetch | rss-atom <rule>

Publishes to every configured channel:
  DISCORD_WEBHOOK_URL   post to this Discord webhook
  FEED_PATH             write an Atom + CAP 1.2 feed to this file
  FEED_ID               the feed id URI (default: urn:piabrota-alerts)

Reads the sources and stores what it delivered:
  ALERTS_DB             the database file (default: alerts.db)
  MOCK_ALERTS_PATH      a JSON file of mock alerts, in place of the built-in scenarios
  ALERTS_SOURCES        the sources that `poll` reads when it gets none, for example cemaden
  ALERTS_UFS            the states that `poll` delivers: all, or for example PR,SC,RS
                        (default: PR,SC,RS)
  ALERTS_TOKEN_<ID>     BYOT: your own token for a catalog source that needs one

Reads and composes RSS feeds in the same database.

Every variable can also come from a .env file in the current directory or a parent; a
variable that is already set wins, and an empty one counts as not set.

  test-message     publish one test alert (CAP status Test)
  poll <source>... read each source once, keep the alerts of ALERTS_UFS, and deliver
                   each new alert once (sources: mock, cemaden; default: ALERTS_SOURCES)
  sources [...]    search the curated catalog of official sources, in Brazil and in the
                   world: words, --country <ISO code>, --no-token, --alerts
  tokens           say which BYOT tokens are set (never their values)
  rss-fetch        read every enabled feed and store its items
  rss-atom <rule>  print the Atom feed of the rule with this slug";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    load_dotenv()?;
    match std::env::args().nth(1).as_deref() {
        Some("test-message") => {
            let alert = test_alert(&now_utc());
            let discord = env("DISCORD_WEBHOOK_URL");
            let feed = env("FEED_PATH");
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
        Some("sources") => sources(&std::env::args().skip(2).collect::<Vec<_>>()),
        Some("tokens") => {
            for (entry, _) in alerts_catalog::byot() {
                let state = alerts::tokens::state(entry, env);
                println!("{:<16} {}", entry.id, state.describe());
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

fn open_db() -> Result<alerts_crud::Db, Box<dyn Error>> {
    let path = env("ALERTS_DB").unwrap_or_else(|| "alerts.db".into());
    Ok(alerts_crud::open(std::path::Path::new(&path))?)
}

fn feed_meta() -> FeedMeta {
    FeedMeta {
        id: env("FEED_ID").unwrap_or_else(|| "urn:piabrota-alerts".into()),
        title: "alerts".into(),
    }
}

/// Reads each source once and delivers its new alerts to every configured channel. A source
/// that fails is reported, and the others still run; the command fails when any source failed.
async fn poll(ids: &[String]) -> Result<(), Box<dyn Error>> {
    let subscribed: Vec<String> = env("ALERTS_SOURCES")
        .map(|v| {
            v.split([',', ' '])
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let ids = if ids.is_empty() { &subscribed[..] } else { ids };
    if ids.is_empty() {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
    let sources = ids
        .iter()
        .map(|id| id.parse::<Source>())
        .collect::<Result<Vec<_>, _>>()?;
    let channels = Channels {
        discord: env("DISCORD_WEBHOOK_URL"),
        feed: env("FEED_PATH").map(|p| (p, feed_meta())),
    };
    if channels.discord.is_none() && channels.feed.is_none() {
        return Err("no channel: set DISCORD_WEBHOOK_URL or FEED_PATH".into());
    }
    let db = open_db()?;
    let harness = harness(ufs_of(env("ALERTS_UFS").as_deref())?);
    let ctx = Fetch {
        now: now_utc(),
        mock_file: env("MOCK_ALERTS_PATH"),
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

/// Prints the catalog entries that match the filters: one block per entry, with its token state.
fn sources(args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut q = alerts_catalog::Query::default();
    let mut words = Vec::new();
    let mut args = args.iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--country" => {
                q.country = Some(args.next().ok_or("--country needs an ISO code")?.clone())
            }
            "--no-token" => q.no_token = true,
            "--alerts" => q.alerts_only = true,
            _ if a.starts_with("--") => return Err(format!("unknown option {a}").into()),
            _ => words.push(a.as_str()),
        }
    }
    q.text = words.join(" ");
    let found = alerts_catalog::search(&q);
    for e in &found {
        let countries = if e.countries.is_empty() {
            "world".to_owned()
        } else {
            e.countries.join(" ")
        };
        let kind = match e.kind {
            alerts_catalog::Kind::Alerts => "alerts",
            alerts_catalog::Kind::Observations => "observations",
        };
        let module = e
            .module
            .map_or("catalog only".to_owned(), |m| format!("alerts poll {m}"));
        println!("{} · {} · {}", e.id, e.agency, e.name);
        println!(
            "  {countries} · {kind} · {} · {}",
            e.format,
            e.hazards.join(", ")
        );
        println!("  {}", e.url);
        if let Some(docs) = e.docs {
            println!("  docs: {docs}");
        }
        println!("  {} · {module}", alerts::tokens::state(e, env).describe());
        if let alerts_catalog::Auth::Token { how, .. } = e.auth {
            println!("  get a token: {how}");
        }
        println!("  checked: {}", e.checked);
    }
    println!(
        "{} of {} sources",
        found.len(),
        alerts_catalog::CATALOG.len()
    );
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
