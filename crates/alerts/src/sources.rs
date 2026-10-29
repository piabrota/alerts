//! The sources of the service and the poll that joins them to the channels.
//!
//! A source crate turns the data of one agency into `Alert`s. [`Source`] is the router: a pure
//! match on an enum, with no trait hierarchy. A new source adds one variant and one arm.

use crate::{post, write_atomic};
use alerts_core::{Alert, Harness, Uf, only_ufs};
use alerts_crud::models::alerts::{AlertRecord, find, record};
use alerts_feed::{Entry, FeedMeta, Status};
use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// One source of alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Simulated test alerts (`alerts-source-mock`). Never an agency.
    Mock,
    /// The municipal alerts of the CEMADEN panel (`alerts-source-cemaden`).
    Cemaden,
}

impl Source {
    pub const ALL: &[Source] = &[Source::Mock, Source::Cemaden];

    pub fn id(self) -> &'static str {
        match self {
            Source::Mock => alerts_source_mock::ID,
            Source::Cemaden => alerts_source_cemaden::ID,
        }
    }

    /// The CAP status of the alerts of this source. A mock alert is always a test.
    pub fn status(self) -> Status {
        match self {
            Source::Mock => Status::Test,
            Source::Cemaden => Status::Actual,
        }
    }

    /// Reads the alerts that the source states now, in the order of the source.
    pub async fn fetch(self, ctx: &Fetch) -> Result<Vec<Alert>, Box<dyn Error>> {
        match self {
            Source::Mock => match &ctx.mock_file {
                Some(path) => Ok(alerts_source_mock::parse(
                    &std::fs::read_to_string(path)?,
                    &ctx.now,
                )?),
                None => Ok(alerts_source_mock::scenarios(&ctx.now)),
            },
            Source::Cemaden => Ok(alerts_source_cemaden::parse(
                &get(alerts_source_cemaden::URL).await?,
            )?),
        }
    }
}

/// The body of a `GET`, sent with the honest User-Agent.
async fn get(url: &str) -> Result<String, Box<dyn Error>> {
    Ok(reqwest::Client::builder()
        .user_agent(alerts_core::USER_AGENT)
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?)
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for Source {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Source::ALL
            .iter()
            .copied()
            .find(|src| src.id() == s)
            .ok_or_else(|| {
                let ids: Vec<_> = Source::ALL.iter().map(|s| s.id()).collect();
                format!("unknown source {s:?}; the sources are: {}", ids.join(", "))
            })
    }
}

/// The CAP status of a stored alert. A source that this build does not know is published as a
/// test: a test alert must never be published as `Actual`.
pub fn status_of(source: &str) -> Status {
    source.parse().map_or(Status::Test, Source::status)
}

/// What a source may need to read its alerts.
#[derive(Debug, Clone)]
pub struct Fetch {
    /// The time of this poll, in the form of `Alert::sent`.
    pub now: String,
    /// A mock file (`MOCK_ALERTS_PATH`) in place of the built-in mock scenarios.
    pub mock_file: Option<String>,
}

/// The harness of v0: floods in PR, SC and RS.
pub fn v0_harness() -> Harness {
    harness(Uf::V0.to_vec())
}

/// The harness that keeps the alerts of these states.
pub fn harness(ufs: Vec<Uf>) -> Harness {
    Harness::new().then(only_ufs(ufs))
}

/// The states of `ALERTS_UFS`: `all` (every UF), or UFs separated by commas or spaces, for
/// example `PR,SC,RS`. Not set: the states of v0.
pub fn ufs_of(var: Option<&str>) -> Result<Vec<Uf>, alerts_core::UnknownUf> {
    let Some(v) = var else {
        return Ok(Uf::V0.to_vec());
    };
    if v.trim().eq_ignore_ascii_case("all") {
        return Ok(Uf::ALL.to_vec());
    }
    let ufs = v
        .split([',', ' '])
        .filter(|s| !s.is_empty())
        .map(str::parse)
        .collect::<Result<Vec<Uf>, _>>()?;
    if ufs.is_empty() {
        return Ok(Uf::V0.to_vec());
    }
    Ok(ufs)
}

/// The channels of one poll. Each one is optional, but a poll needs at least one.
#[derive(Debug, Clone, Default)]
pub struct Channels {
    pub discord: Option<String>,
    pub feed: Option<(String, FeedMeta)>,
}

/// What one source gave in one poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Report {
    pub fetched: usize,
    pub passed: usize,
    pub delivered: usize,
}

/// Delivers the new alerts of one source: each alert that passes the harness and that the
/// store does not have yet goes to Discord, then into the store. An alert is stored only after
/// Discord stored it, so a failed delivery is tried again at the next poll.
pub async fn deliver(
    db: &alerts_crud::Db,
    harness: &Harness,
    channels: &Channels,
    source: Source,
    alerts: Vec<Alert>,
) -> Result<Report, Box<dyn Error>> {
    let mut report = Report {
        fetched: alerts.len(),
        ..Report::default()
    };
    for alert in alerts.into_iter().filter_map(|a| harness.run(a)) {
        report.passed += 1;
        if alert.source != source.id() {
            return Err(format!(
                "{source}: alert {} names the source {:?}",
                alert.id, alert.source
            )
            .into());
        }
        if find(db, &alert.source, &alert.id)?.is_some() {
            continue;
        }
        if let Some(url) = &channels.discord {
            post(url, &alert).await?;
        }
        record(db, &alert)?;
        report.delivered += 1;
    }
    Ok(report)
}

/// Writes the feed of every stored alert, newest first. The feed and Discord read the same
/// store, so every alert that reached Discord is in the feed, with the same text.
pub fn write_feed(
    db: &alerts_crud::Db,
    path: &str,
    meta: &FeedMeta,
    now: &str,
) -> Result<usize, Box<dyn Error>> {
    let mut alerts: Vec<Alert> = alerts_crud::model::list::<AlertRecord>(db)?
        .into_iter()
        .map(Alert::from)
        .collect();
    alerts.reverse();
    let entries: Vec<Entry<'_>> = alerts
        .iter()
        .map(|a| Entry {
            id: format!("{}:{}", meta.id, alerts_feed::identifier(a)),
            title: &a.headline,
            link: a.url.as_deref(),
            updated: &a.sent,
            summary: &a.area,
            alert: Some((a, status_of(&a.source))),
        })
        .collect();
    write_atomic(path, &alerts_feed::atom_entries(meta, &entries, now)?)?;
    Ok(entries.len())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const NOW: &str = "2026-10-27T12:00:00+00:00";

    fn mock() -> Fetch {
        Fetch {
            now: NOW.into(),
            mock_file: None,
        }
    }

    #[test]
    fn source_ids_round_trip() {
        for &s in Source::ALL {
            assert_eq!(s.id().parse::<Source>(), Ok(s));
        }
        assert!("idap".parse::<Source>().is_err());
    }

    #[test]
    fn every_agency_source_is_in_the_catalog() {
        for &s in Source::ALL.iter().filter(|&&s| s != Source::Mock) {
            let entry = alerts_catalog::get(s.id());
            assert_eq!(entry.and_then(|e| e.module), Some(s.id()), "{s}");
        }
        for e in alerts_catalog::CATALOG {
            if let Some(m) = e.module {
                assert!(m.parse::<Source>().is_ok(), "{}: no source {m}", e.id);
            }
        }
    }

    #[test]
    fn alerts_ufs() {
        assert_eq!(ufs_of(None).unwrap(), Uf::V0);
        assert_eq!(ufs_of(Some(" ")).unwrap(), Uf::V0);
        assert_eq!(ufs_of(Some("all")).unwrap(), Uf::ALL);
        assert_eq!(ufs_of(Some("ALL")).unwrap().len(), 27);
        assert_eq!(ufs_of(Some("ba, sp")).unwrap(), vec![Uf::Ba, Uf::Sp]);
        assert!(ufs_of(Some("PR,XX")).is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn every_uf_keeps_the_alerts_outside_v0() {
        let db = alerts_crud::open_in_memory().unwrap();
        let alerts = Source::Mock.fetch(&mock()).await.unwrap();
        let r = deliver(
            &db,
            &harness(Uf::ALL.to_vec()),
            &Channels::default(),
            Source::Mock,
            alerts,
        )
        .await
        .unwrap();
        assert_eq!(r.passed, 5);
    }

    #[test]
    fn agency_sources_are_actual() {
        assert_eq!(status_of("cemaden"), Status::Actual);
    }

    #[test]
    fn mock_and_unknown_sources_are_tests() {
        assert_eq!(status_of("mock"), Status::Test);
        assert_eq!(status_of("no-such-source"), Status::Test);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn delivers_each_alert_once() {
        let db = alerts_crud::open_in_memory().unwrap();
        let h = v0_harness();
        let none = Channels::default();
        let alerts = Source::Mock.fetch(&mock()).await.unwrap();

        let first = deliver(&db, &h, &none, Source::Mock, alerts.clone())
            .await
            .unwrap();
        // The SP scenario is outside v0: the harness drops it.
        assert_eq!(
            first,
            Report {
                fetched: 5,
                passed: 4,
                delivered: 4
            }
        );
        let again = deliver(&db, &h, &none, Source::Mock, alerts).await.unwrap();
        assert_eq!(again.delivered, 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_source_cannot_speak_for_another() {
        let db = alerts_crud::open_in_memory().unwrap();
        let mut alerts = Source::Mock.fetch(&mock()).await.unwrap();
        alerts[0].source = "defesa-civil-idap".into();
        let r = deliver(
            &db,
            &v0_harness(),
            &Channels::default(),
            Source::Mock,
            alerts,
        )
        .await;
        assert!(r.is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn the_feed_publishes_mock_alerts_as_tests() {
        let db = alerts_crud::open_in_memory().unwrap();
        let alerts = Source::Mock.fetch(&mock()).await.unwrap();
        deliver(
            &db,
            &v0_harness(),
            &Channels::default(),
            Source::Mock,
            alerts,
        )
        .await
        .unwrap();
        let dir = std::env::temp_dir().join(format!("alerts-feed-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("feed.xml");
        let path = path.to_str().unwrap();
        let meta = FeedMeta {
            id: "urn:piabrota-alerts".into(),
            title: "alerts".into(),
        };
        assert_eq!(write_feed(&db, path, &meta, NOW).unwrap(), 4);
        let xml = std::fs::read_to_string(path).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(xml.matches("<status>Test</status>").count(), 4);
        assert!(!xml.contains("<status>Actual</status>"));
        assert!(xml.contains("TESTE: alerta simulado de inundação de severidade extrema."));
    }
}
