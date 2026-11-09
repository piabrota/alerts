//! `alerts publish` end to end: the real binary polls the mock source, then writes the site into a
//! temp folder. No network: the mock source reads a file.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use alerts_core::{Alert, Hazard, Severity, Uf};
use alerts_rss::time::rfc3339_utc;
use quick_xml::{Reader, events::Event};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const MOCK: &str = r#"[
  {"severity": "Severe", "ufs": ["SC"], "headline": "<b>rio</b> & \"cota\"", "area": "Área simulada em SC."},
  {"severity": "Minor", "ufs": ["PR"], "headline": "outro", "area": "Área simulada no PR."}
]"#;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("alerts-publish-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs the binary in `dir` with a clean environment, so no `.env` and no variable of the
/// caller reaches it.
fn alerts(dir: &Path, args: &[&str]) -> Output {
    let mock = dir.join("mock.json");
    std::fs::write(&mock, MOCK).unwrap();
    Command::new(env!("CARGO_BIN_EXE_alerts"))
        .args(args)
        .current_dir(dir)
        .env_clear()
        .env("ALERTS_DB", dir.join("alerts.db"))
        .env("MOCK_ALERTS_PATH", mock)
        .env("ALERTS_SOURCES", "mock")
        .env("FEED_PATH", dir.join("poll-feed.atom"))
        .output()
        .unwrap()
}

fn ok(o: &Output) {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

/// Reads the whole document and returns the names of its elements, or panics when it is not
/// well-formed.
fn elements(xml: &str) -> Vec<String> {
    let mut r = Reader::from_str(xml);
    let mut out = Vec::new();
    loop {
        match r.read_event() {
            Ok(Event::Start(e)) => out.push(e.name().as_ref().to_owned()),
            Ok(Event::Eof) => return out,
            Ok(_) => {}
            Err(e) => panic!("malformed XML: {e}"),
        }
    }
}

/// The feed without its own `<updated>` time (the first one), which is the time of the run.
fn without_feed_time(feed: &str) -> String {
    let start = feed.find("<updated>").unwrap();
    let end = feed[start..].find("</updated>").unwrap() + start;
    format!("{}{}", &feed[..start], &feed[end..])
}

fn without_generated(json: &str) -> serde_json::Value {
    let mut v: serde_json::Value = serde_json::from_str(json).unwrap();
    v.as_object_mut().unwrap().remove("generated").unwrap();
    v
}

/// Stores one flood alert per `(id, age in days)` with a `sent` time of that age. The mock
/// source cannot make an old alert, so the test writes into the store that `poll` reads.
fn seed(dir: &Path, rows: &[(&str, i64)]) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let now = i64::try_from(now).unwrap();
    let db = alerts_crud::open(&dir.join("alerts.db")).unwrap();
    for (id, days) in rows {
        let a = Alert {
            id: (*id).into(),
            source: "cemaden".into(),
            hazard: Hazard::Flood,
            severity: Severity::Severe,
            ufs: vec![Uf::Sc],
            headline: format!("headline {id}"),
            area: format!("area {id}"),
            sent: rfc3339_utc(now - days * 86_400),
            url: None,
        };
        alerts_crud::models::alerts::record(&db, &a).unwrap();
    }
}

fn entries(xml: &str) -> usize {
    elements(xml).iter().filter(|e| *e == "entry").count()
}

fn json(dir: &Path, name: &str) -> serde_json::Value {
    serde_json::from_str(&read(dir, name)).unwrap()
}

#[test]
fn poll_keeps_all_alerts_in_its_feed_and_publish_applies_the_window() {
    let dir = scratch("window");
    seed(&dir, &[("recent", 0), ("ten", 10), ("thirty", 30)]);
    ok(&alerts(&dir, &["poll"]));
    assert_eq!(entries(&read(&dir, "poll-feed.atom")), 5, "poll keeps all");

    let out = dir.join("site");
    let out_arg = out.to_str().unwrap();
    let windows = [
        (&[][..], 3),
        (&["--days", "14"][..], 4),
        (&["--days", "0"][..], 5),
    ];
    for (days, expected) in windows {
        let mut args = vec!["publish", "--out", out_arg];
        args.extend_from_slice(days);
        ok(&alerts(&dir, &args));
        let (j, h) = (json(&out, "alerts.json"), json(&out, "health.json"));
        assert_eq!(entries(&read(&out, "feed.atom")), expected, "{days:?}");
        assert_eq!(j["alerts"].as_array().unwrap().len(), expected, "{days:?}");
        assert_eq!(h["in_window"], expected);
        assert_eq!(h["alerts"], 5);
        assert_eq!(h["window_days"], j["window_days"]);
    }
    assert!(json(&out, "alerts.json")["window_days"].is_null());
    assert_eq!(
        entries(&read(&dir, "poll-feed.atom")),
        5,
        "publish leaves it"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_page_text_follows_the_window() {
    let dir = scratch("text");
    seed(&dir, &[("thirty", 30)]);
    let out = dir.join("site");
    let out_arg = out.to_str().unwrap();
    ok(&alerts(&dir, &["publish", "--out", out_arg]));
    let page = read(&out, "index.html");
    assert!(page.contains("Sem alertas nos últimos 7 dias. Última verificação:"));
    assert!(!page.contains("headline thirty") && !page.contains("Sem dados"));
    assert_eq!(json(&out, "alerts.json")["window_days"], 7);

    ok(&alerts(&dir, &["publish", "--out", out_arg, "--days", "0"]));
    let page = read(&out, "index.html");
    assert!(page.contains("Todos os alertas guardados, do mais novo ao mais antigo."));
    assert!(page.contains("headline thirty"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_bad_days_value_exits_with_code_2_and_writes_nothing() {
    let dir = scratch("days");
    for bad in ["x", "-1", "1.5", ""] {
        let o = alerts(&dir, &["publish", "--out", "site", "--days", bad]);
        assert_eq!(o.status.code(), Some(2), "{bad:?}");
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(
            err.contains("--days needs a whole number"),
            "{bad:?}: {err}"
        );
    }
    assert!(!dir.join("site").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn publish_writes_the_four_files_of_the_mock_alerts() {
    let dir = scratch("files");
    ok(&alerts(&dir, &["poll"]));
    let out = dir.join("site/nested");
    ok(&alerts(&dir, &["publish", "--out", out.to_str().unwrap()]));

    let mut names: Vec<String> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["alerts.json", "feed.atom", "health.json", "index.html"]
    );

    let feed = read(&out, "feed.atom");
    let n = elements(&feed);
    assert_eq!(n.iter().filter(|e| *e == "entry").count(), 2);
    assert_eq!(n.iter().filter(|e| *e == "alert").count(), 2);
    assert!(feed.contains("<status>Test</status>"));

    let json: serde_json::Value = serde_json::from_str(&read(&out, "alerts.json")).unwrap();
    assert_eq!(json["alerts"].as_array().unwrap().len(), 2);
    assert!(json["generated"].as_str().unwrap().ends_with("+00:00"));

    let health: serde_json::Value = serde_json::from_str(&read(&out, "health.json")).unwrap();
    assert_eq!(health["alerts"], 2);
    assert_eq!(health["generated"], json["generated"]);
    assert!(health["newest"].is_string());

    let page = read(&out, "index.html");
    assert!(page.contains("<html lang=\"pt-BR\">"));
    assert!(page.contains("TESTE: &lt;b&gt;rio&lt;/b&gt; &amp; &quot;cota&quot;"));
    assert!(page.contains("<strong>Alerta de teste</strong>"));
    assert!(!page.contains("<b>rio"));
    assert!(page.contains("<li>mock (alertas simulados, de teste)</li>"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_second_run_changes_only_the_time() {
    let dir = scratch("again");
    ok(&alerts(&dir, &["poll"]));
    let out = dir.join("site");
    let out_arg = out.to_str().unwrap();
    ok(&alerts(&dir, &["publish", "--out", out_arg]));
    let (feed, json) = (read(&out, "feed.atom"), read(&out, "alerts.json"));
    ok(&alerts(&dir, &["publish", "--out", out_arg]));

    assert_eq!(
        without_feed_time(&feed),
        without_feed_time(&read(&out, "feed.atom"))
    );
    assert_eq!(
        without_generated(&json),
        without_generated(&read(&out, "alerts.json"))
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn publish_needs_the_out_folder() {
    let dir = scratch("usage");
    for args in [
        &["publish"][..],
        &["publish", "--out"],
        &["publish", "dir", "x"],
    ] {
        let o = alerts(&dir, args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(String::from_utf8_lossy(&o.stderr).contains("publish --out <dir>"));
    }
    assert!(!dir.join("site").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_empty_store_says_no_data() {
    let dir = scratch("empty");
    let out = dir.join("site");
    ok(&alerts(&dir, &["publish", "--out", out.to_str().unwrap()]));
    assert!(read(&out, "index.html").contains("<strong>Sem dados</strong>"));
    let health: serde_json::Value = serde_json::from_str(&read(&out, "health.json")).unwrap();
    assert_eq!(health["alerts"], 0);
    assert!(health["newest"].is_null());
    assert_eq!(
        elements(&read(&out, "feed.atom"))
            .iter()
            .filter(|e| *e == "entry")
            .count(),
        0
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
