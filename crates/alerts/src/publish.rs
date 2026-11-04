//! The static site of the service: `alerts publish` writes `feed.atom`, `alerts.json`,
//! `index.html` and `health.json` into one folder, and a web server serves the folder as plain
//! files. The renderers are pure functions of the alerts and of the time, so tests need no
//! database and no network.
//!
//! The page shows only what an [`Alert`] holds. An alert has no start time, no end time, no
//! urgency and no certainty, so the page shows none of them. Without an end time the page cannot
//! tell which alerts are still in force, so it never calls one "active". It shows a window
//! instead: the alerts that the source sent in the last `--days` days (see [`Selection`]).

use crate::sources::{Source, status_of, stored, write_feed_of};
use crate::write_atomic;
use alerts_core::{Alert, Severity};
use alerts_feed::{FeedMeta, Status};
use alerts_rss::time::{epoch_secs, normalize, rfc3339_utc};
use serde_json::json;
use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Write;

/// Brasília time is UTC-3 all year: Brazil has had no daylight saving time since 2019.
const BRASILIA_SECS: i64 = 3 * 3600;

/// The rules of the page. The severity colors come from `alerts-core`, so every channel uses
/// one palette.
const CSS: &str = "body{font:16px/1.5 system-ui,sans-serif;max-width:46rem;margin:0 auto;\
padding:1rem;color:#1a1a1a;background:#fff}\
a{color:#0b5cad}h1{margin:.2em 0}h2{margin-top:1.6em}h3{margin:1.2em 0 .2em}\
article{border-left:.4rem solid #888;padding:.2rem .8rem;margin:.8rem 0;background:#f4f4f4}\
article h4{margin:.2em 0}article p,article ul{margin:.2em 0}article ul{padding-left:1.2rem}\
@media(prefers-color-scheme:dark){body{color:#eee;background:#111}a{color:#8ab8f0}\
article{background:#1c1c1c}}";

/// The days of the window when `--days` is not given.
pub const DEFAULT_DAYS: u32 = 7;

/// The command line of `alerts publish`.
#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub out: String,
    /// The window in days, or `None` for no window (`--days 0`).
    pub days: Option<u32>,
}

/// Reads `--out <dir>` and `--days <N>` in any order. The error text is for the user.
pub fn parse_args(args: &[String]) -> Result<Args, String> {
    let (mut out, mut days) = (None, Some(DEFAULT_DAYS));
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        match (flag.as_str(), it.next()) {
            ("--out", Some(v)) => out = Some(v.clone()),
            ("--days", Some(v)) => days = parse_days(v)?,
            ("--out" | "--days", None) => return Err(format!("{flag} needs a value")),
            _ => return Err(format!("unknown argument: {flag}")),
        }
    }
    let out = out.ok_or("--out <dir> is required")?;
    Ok(Args { out, days })
}

/// A whole number of days. `0` means no window.
fn parse_days(v: &str) -> Result<Option<u32>, String> {
    match v.parse::<u32>() {
        Ok(0) => Ok(None),
        Ok(n) => Ok(Some(n)),
        Err(_) => Err(format!(
            "--days needs a whole number of days (0 for no window), not {v:?}"
        )),
    }
}

/// The stored alerts that the page, `alerts.json` and `feed.atom` show.
#[derive(Debug)]
pub struct Selection<'a> {
    /// The window in days, or `None` for no window.
    pub days: Option<u32>,
    /// The number of stored alerts.
    pub total: usize,
    /// The newest send time of all stored alerts, in UTC, or `None`.
    pub newest: Option<String>,
    /// The alerts in the window, in the order of the store (oldest first).
    pub alerts: Vec<&'a Alert>,
}

impl<'a> Selection<'a> {
    /// An alert is in the window when its `sent` time is not older than `now - days`. An alert
    /// whose `sent` time does not parse is in the window: the page does not drop it silently.
    pub fn new(all: &'a [Alert], now: &str, days: Option<u32>) -> Self {
        let cutoff = days.and_then(|d| Some(epoch_secs(now)? - i64::from(d) * 86_400));
        let alerts = all
            .iter()
            .filter(|a| match (cutoff, epoch_secs(&a.sent)) {
                (Some(cutoff), Some(sent)) => sent >= cutoff,
                _ => true,
            })
            .collect();
        let newest = all.iter().filter_map(|a| normalize(&a.sent)).max();
        Self {
            days,
            total: all.len(),
            newest,
            alerts,
        }
    }
}

/// Writes the four files into `dir` and returns the number of alerts in the window and the
/// number of stored alerts. `dir` is created when it is missing. Each file goes in with
/// `write_atomic`, so a reader never sees half a file.
pub fn publish(
    db: &alerts_crud::Db,
    dir: &str,
    now: &str,
    meta: &FeedMeta,
    sources: &[Source],
    days: Option<u32>,
) -> Result<(usize, usize), Box<dyn Error>> {
    std::fs::create_dir_all(dir)?;
    let path = |name: &str| format!("{}/{name}", dir.trim_end_matches('/'));
    let all = stored(db)?;
    let sel = Selection::new(&all, now, days);
    write_feed_of(sel.alerts.iter().copied(), &path("feed.atom"), meta, now)?;
    write_atomic(&path("alerts.json"), &render_json(&sel, now)?)?;
    write_atomic(&path("health.json"), &render_health(&sel, now))?;
    write_atomic(&path("index.html"), &render_page(&sel, now, sources))?;
    Ok((sel.alerts.len(), sel.total))
}

/// `alerts.json`: the time of the file, the window and the alerts of the window, newest first.
/// The fields of an alert are the fields of [`Alert`]; `docs/PUBLISH.md` lists them.
pub fn render_json(sel: &Selection<'_>, now: &str) -> Result<String, serde_json::Error> {
    let list = serde_json::to_value(newest_first(&sel.alerts))?;
    let mut text = serde_json::to_string_pretty(
        &json!({ "generated": now, "window_days": sel.days, "alerts": list }),
    )?;
    text.push('\n');
    Ok(text)
}

/// `health.json`: lets a watchdog see that the timer stopped (`generated` is old). `alerts` is
/// the number of stored alerts and `in_window` the number that the other files show. `newest` is
/// the send time of the newest stored alert, in UTC, or `null`.
pub fn render_health(sel: &Selection<'_>, now: &str) -> String {
    format!(
        "{}\n",
        json!({
            "generated": now,
            "window_days": sel.days,
            "alerts": sel.total,
            "in_window": sel.alerts.len(),
            "newest": sel.newest,
        })
    )
}

/// `index.html`: the page of the alerts in the window, grouped by state, newest first.
pub fn render_page(sel: &Selection<'_>, now: &str, sources: &[Source]) -> String {
    let mut x = head();
    x.push_str("<body>\n<main>\n<h1>alerts</h1>\n");
    x.push_str("<p>Os alertas são copiados de órgãos oficiais, como foram publicados.</p>\n");
    let _ = writeln!(
        x,
        "<p>Última atualização: {}. <a href=\"feed.atom\">feed.atom</a> · \
         <a href=\"alerts.json\">alerts.json</a></p>",
        when(now)
    );
    x.push_str(&alert_section(sel, now));
    x.push_str(&sources_section(sources));
    x.push_str("</main>\n</body>\n</html>\n");
    x
}

fn head() -> String {
    let mut x = String::from(
        "<!doctype html>\n<html lang=\"pt-BR\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; \
         style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'\">\n\
         <title>alerts</title>\n<style>",
    );
    x.push_str(CSS);
    for s in [
        Severity::Minor,
        Severity::Moderate,
        Severity::Severe,
        Severity::Extreme,
    ] {
        let _ = write!(
            x,
            ".sev-{}{{border-color:{}}}",
            s.label_pt_br(),
            s.css_color()
        );
    }
    x.push_str("</style>\n</head>\n");
    x
}

/// Replaces the five characters that can end a text or an attribute value in HTML.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// The URL when the page may link it: only `http://` and `https://`, never `javascript:` or
/// any other scheme.
fn safe_href(url: &str) -> Option<&str> {
    ["https://", "http://"]
        .iter()
        .any(|p| {
            url.get(..p.len())
                .is_some_and(|h| h.eq_ignore_ascii_case(p))
        })
        .then_some(url)
}

/// A link with the text `text`, or only the text when the URL is not safe to link.
fn link(url: &str, text: &str) -> String {
    match safe_href(url) {
        Some(href) => format!(
            "<a href=\"{}\" rel=\"noopener noreferrer\">{}</a>",
            escape(href),
            escape(text)
        ),
        None => escape(text),
    }
}

/// `YYYY-MM-DD hh:mm` of a Unix time.
fn minute(secs: i64) -> String {
    let t = rfc3339_utc(secs);
    t.get(..16).unwrap_or(&t).replace('T', " ")
}

/// A time in UTC and in Brasília time, for the header of the page.
fn when(now: &str) -> String {
    match epoch_secs(now) {
        Some(s) => format!(
            "{} UTC ({} em Brasília, UTC-3)",
            minute(s),
            minute(s - BRASILIA_SECS)
        ),
        None => escape(now),
    }
}

/// The send time of an alert in UTC, or the text as the source gave it when it is not a time.
fn sent_text(sent: &str) -> String {
    match epoch_secs(sent) {
        Some(s) => format!("{} UTC", minute(s)),
        None => escape(sent),
    }
}

/// The alerts newest first. An alert with a time that does not parse goes last. Equal times keep
/// the later alert of the input first, the order of the feed.
fn newest_first<'a>(alerts: &[&'a Alert]) -> Vec<&'a Alert> {
    let mut v: Vec<&Alert> = alerts.iter().rev().copied().collect();
    v.sort_by_cached_key(|a| Reverse(normalize(&a.sent).unwrap_or_default()));
    v
}

const END_UNKNOWN: &str = "A fonte não informa até quando um alerta vale.";

/// The sentence above the list: it says which alerts the list holds.
fn intro(days: Option<u32>) -> String {
    match days {
        Some(n) => format!(
            "Alertas enviados nos últimos {n} dias, do mais novo ao mais antigo. {END_UNKNOWN}"
        ),
        None => format!("Todos os alertas guardados, do mais novo ao mais antigo. {END_UNKNOWN}"),
    }
}

/// The section for a window without alerts: the store has none (`Sem dados`), or it has some
/// that are older than the window.
fn empty_section(sel: &Selection<'_>, now: &str) -> String {
    let what = match sel.days {
        Some(n) if sel.total > 0 => format!("Sem alertas nos últimos {n} dias."),
        _ => "<strong>Sem dados</strong>: nenhum alerta guardado.".to_owned(),
    };
    format!(
        "<section>\n<h2>Alertas</h2>\n<p>{what} Última verificação: {}.</p>\n</section>\n",
        when(now)
    )
}

fn alert_section(sel: &Selection<'_>, now: &str) -> String {
    if sel.alerts.is_empty() {
        return empty_section(sel, now);
    }
    let mut by_uf: BTreeMap<&str, Vec<&Alert>> = BTreeMap::new();
    let mut no_uf = Vec::new();
    for a in newest_first(&sel.alerts) {
        if a.ufs.is_empty() {
            no_uf.push(a);
        }
        for uf in &a.ufs {
            by_uf.entry(uf.as_str()).or_default().push(a);
        }
    }
    let mut x = format!("<section>\n<h2>Alertas</h2>\n<p>{}</p>\n", intro(sel.days));
    for (uf, list) in &by_uf {
        x.push_str(&group(uf, list));
    }
    if !no_uf.is_empty() {
        x.push_str(&group("Sem UF informada", &no_uf));
    }
    x.push_str("</section>\n");
    x
}

fn group(title: &str, alerts: &[&Alert]) -> String {
    let mut x = format!("<section>\n<h3>{}</h3>\n", escape(title));
    for a in alerts {
        x.push_str(&article(a));
    }
    x.push_str("</section>\n");
    x
}

fn article(a: &Alert) -> String {
    let mut x = format!(
        "<article class=\"sev-{}\">\n<h4>{}</h4>\n<p>{}</p>\n",
        a.severity.label_pt_br(),
        escape(&a.headline),
        escape(&a.area)
    );
    if status_of(&a.source) == Status::Test {
        x.push_str("<p><strong>Alerta de teste</strong></p>\n");
    }
    x.push_str("<ul>\n");
    item(&mut x, "Evento", a.hazard.label_pt_br());
    item(&mut x, "Severidade", a.severity.label_pt_br());
    x.push_str(&details(a));
    x.push_str("</ul>\n</article>\n");
    x
}

/// The items of an alert that exist only when the alert holds them.
fn details(a: &Alert) -> String {
    let mut x = String::new();
    if !a.ufs.is_empty() {
        let ufs: Vec<&str> = a.ufs.iter().map(|u| u.as_str()).collect();
        item(&mut x, "UF", &ufs.join(", "));
    }
    if !a.sent.is_empty() {
        item(&mut x, "Enviado", &sent_text(&a.sent));
    }
    let source = alerts_catalog::get(&a.source).map_or(a.source.as_str(), |e| e.agency);
    item(&mut x, "Fonte", &escape(source));
    if let Some(url) = &a.url {
        item(&mut x, "Link", &link(url, url));
    }
    x
}

/// One list item. `html` is already escaped.
fn item(x: &mut String, label: &str, html: &str) {
    let _ = writeln!(x, "<li>{label}: {html}</li>");
}

fn sources_section(sources: &[Source]) -> String {
    let mut x = String::from("<section>\n<h2>Fontes</h2>\n");
    if sources.is_empty() {
        x.push_str("<p>Nenhuma fonte configurada neste servidor.</p>\n");
    } else {
        x.push_str("<ul>\n");
        for &s in sources {
            let _ = writeln!(x, "<li>{}</li>", source_item(s));
        }
        x.push_str("</ul>\n");
    }
    x.push_str("</section>\n");
    x
}

/// The name of an enabled source, with a link to the page of the agency when the catalog has
/// one.
fn source_item(s: Source) -> String {
    if s == Source::Mock {
        return format!("{} (alertas simulados, de teste)", escape(s.id()));
    }
    let Some(e) = alerts_catalog::get(s.id()) else {
        return escape(s.id());
    };
    let name = format!("{}: {}", e.agency, e.name);
    e.docs
        .map_or_else(|| escape(&name), |docs| link(docs, &name))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use alerts_core::{Hazard, Uf};

    const NOW: &str = "2026-10-27T12:00:00+00:00";

    fn alert(id: &str, ufs: &[Uf], sent: &str) -> Alert {
        Alert {
            id: id.into(),
            source: "cemaden".into(),
            hazard: Hazard::Flood,
            severity: Severity::Severe,
            ufs: ufs.to_vec(),
            headline: format!("headline {id}"),
            area: format!("area {id}"),
            sent: sent.into(),
            url: None,
        }
    }

    fn sel(alerts: &[Alert], days: Option<u32>) -> Selection<'_> {
        Selection::new(alerts, NOW, days)
    }

    fn page_in(alerts: &[Alert], days: Option<u32>) -> String {
        render_page(&sel(alerts, days), NOW, &[Source::Cemaden])
    }

    fn page(alerts: &[Alert]) -> String {
        page_in(alerts, None)
    }

    #[test]
    fn source_text_is_escaped() {
        let mut a = alert("a", &[Uf::Sc], "2026-10-27T10:00:00-03:00");
        a.headline = "<script>alert(\"x\")</script> & 'y'".into();
        a.area = "<b>area</b>".into();
        let p = page(&[a]);
        assert!(!p.contains("<script"), "{p}");
        assert!(!p.contains("<b>"));
        assert!(p.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; &#39;y&#39;"));
    }

    #[test]
    fn only_http_links_get_an_href() {
        let mut a = alert("a", &[Uf::Sc], "2026-10-27T10:00:00-03:00");
        for bad in [
            "javascript:alert(1)",
            "data:text/html,x",
            " https://x.org",
            "//x.org",
        ] {
            a.url = Some(bad.into());
            let p = page(std::slice::from_ref(&a));
            assert_eq!(p.matches("href=").count(), 2, "{bad}: only feed and json");
            assert!(p.contains(&escape(bad)), "{bad}: the text stays");
        }
        a.url = Some("HTTPS://example.org/a?b=1&c=\"2\"".into());
        let p = page(&[a]);
        assert!(p.contains("href=\"HTTPS://example.org/a?b=1&amp;c=&quot;2&quot;\""));
    }

    #[test]
    fn an_alert_without_an_end_time_is_shown() {
        let p = page(&[alert("a", &[Uf::Pr], "2026-10-27T10:00:00-03:00")]);
        assert!(p.contains("headline a") && p.contains("area a"));
        assert!(p.contains("Severidade: severa") && p.contains("Evento: Inundação"));
        assert!(!p.contains("Sem dados"));
    }

    #[test]
    fn no_alert_says_no_data_with_the_time_of_the_check() {
        let p = page(&[]);
        assert!(p.contains("Sem dados"));
        assert!(
            p.contains("Última verificação: 2026-10-27 12:00 UTC (2026-10-27 09:00 em Brasília")
        );
    }

    #[test]
    fn the_header_gives_utc_and_brasilia_time_and_links() {
        let p = page(&[]);
        assert!(p.contains(
            "Última atualização: 2026-10-27 12:00 UTC (2026-10-27 09:00 em Brasília, UTC-3)"
        ));
        assert!(p.contains("href=\"feed.atom\"") && p.contains("href=\"alerts.json\""));
        let late = render_page(
            &Selection::new(&[], "2026-10-27T01:30:00+00:00", None),
            "2026-10-27T01:30:00+00:00",
            &[],
        );
        assert!(late.contains("2026-10-26 22:30 em Brasília"));
    }

    #[test]
    fn groups_by_uf_and_lists_newest_first() {
        let p = page(&[
            alert("old", &[Uf::Sc], "2026-10-27T08:00:00+00:00"),
            alert("new", &[Uf::Sc], "2026-10-27T11:00:00+00:00"),
            alert("both", &[Uf::Sc, Uf::Pr], "2026-10-27T09:00:00+00:00"),
            alert("none", &[], "2026-10-27T10:00:00+00:00"),
        ]);
        let at = |s: &str| p.find(s).unwrap_or_else(|| panic!("{s} missing"));
        let (pr, sc, none) = (
            at("<h3>PR</h3>"),
            at("<h3>SC</h3>"),
            at("<h3>Sem UF informada</h3>"),
        );
        assert!(pr < sc && sc < none);
        let sc_part = &p[sc..none];
        assert!(sc_part.find("headline new") < sc_part.find("headline both"));
        assert!(sc_part.find("headline both") < sc_part.find("headline old"));
        assert_eq!(
            p.matches("headline both").count(),
            2,
            "an alert of two states is in both"
        );
        assert_eq!(p[none..].matches("headline none").count(), 1);
    }

    #[test]
    fn a_mock_alert_is_marked_as_a_test() {
        let mut a = alert("a", &[Uf::Pr], "2026-10-27T10:00:00+00:00");
        assert!(!page(std::slice::from_ref(&a)).contains("Alerta de teste"));
        a.source = "mock".into();
        assert!(page(&[a]).contains("<strong>Alerta de teste</strong>"));
    }

    #[test]
    fn the_page_is_self_contained() {
        let p = page(&[alert("a", &[Uf::Pr], NOW)]);
        assert!(p.starts_with("<!doctype html>\n<html lang=\"pt-BR\">"));
        assert!(p.contains(
            "content=\"default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; \
             form-action 'none'\""
        ));
        for bad in [
            "<script", "<link", "<img", "src=", "@import", "url(", "<iframe",
        ] {
            assert!(!p.contains(bad), "{bad}");
        }
        let css = p
            .split("<style>")
            .nth(1)
            .and_then(|s| s.split("</style>").next());
        assert!(css.unwrap().len() < 2048);
    }

    #[test]
    fn the_sources_list_links_only_what_the_catalog_has() {
        let p = render_page(&sel(&[], None), NOW, &[Source::Mock, Source::Cemaden]);
        assert!(p.contains("<li>mock (alertas simulados, de teste)</li>"));
        assert!(p.contains("CEMADEN (Centro Nacional de Monitoramento e Alertas de Desastres Naturais): Painel de alertas"));
        assert_eq!(
            p.matches("href=").count(),
            2,
            "cemaden has no agency page in the catalog"
        );
        assert!(render_page(&sel(&[], None), NOW, &[]).contains("Nenhuma fonte configurada"));
    }

    #[test]
    fn json_has_the_stable_fields_newest_first() {
        let mut a = alert("a", &[Uf::Pr, Uf::Sc], "2026-10-27T08:00:00+00:00");
        a.url = Some("https://example.org/a".into());
        let b = alert("b", &[], "2026-10-27T09:00:00+00:00");
        let v: serde_json::Value =
            serde_json::from_str(&render_json(&sel(&[a, b], None), NOW).unwrap()).unwrap();
        assert_eq!(v["generated"], NOW);
        let list = v["alerts"].as_array().unwrap();
        assert_eq!(list[0]["id"], "b");
        let mut keys: Vec<&str> = list[1]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "area", "hazard", "headline", "id", "sent", "severity", "source", "ufs", "url"
            ]
        );
        assert_eq!(list[1]["ufs"], serde_json::json!(["PR", "SC"]));
        assert_eq!(list[1]["severity"], "Severe");
        assert_eq!(list[1]["hazard"], "Flood");
        assert!(list[0]["url"].is_null());
    }

    #[test]
    fn json_and_page_are_the_same_for_the_same_input() {
        let alerts = [alert("a", &[Uf::Pr], "2026-10-27T08:00:00+00:00")];
        assert_eq!(
            render_json(&sel(&alerts, None), NOW).unwrap(),
            render_json(&sel(&alerts, None), NOW).unwrap()
        );
        assert_eq!(page(&alerts), page(&alerts));
        let later = render_json(&sel(&alerts, None), "2026-10-27T12:10:00+00:00").unwrap();
        assert_eq!(
            later.replace("12:10:00", "12:00:00"),
            render_json(&sel(&alerts, None), NOW).unwrap()
        );
    }

    #[test]
    fn health_counts_and_names_the_newest_alert() {
        let h = |alerts: &[Alert]| -> serde_json::Value {
            serde_json::from_str(&render_health(&sel(alerts, None), NOW)).unwrap()
        };
        let empty = h(&[]);
        assert_eq!(
            (empty["alerts"].as_u64(), empty["generated"].as_str()),
            (Some(0), Some(NOW))
        );
        assert!(empty["newest"].is_null());
        let two = h(&[
            alert("a", &[], "2026-10-27T08:00:00+00:00"),
            alert("b", &[], "2026-10-27T09:00:00-03:00"),
        ]);
        assert_eq!(two["alerts"], 2);
        assert_eq!(two["newest"], "2026-10-27T12:00:00+00:00");
    }

    /// `NOW` is 2026-10-27T12:00:00Z, so a 7-day window starts at 2026-10-20T12:00:00Z.
    const EDGE: &str = "2026-10-20T12:00:00+00:00";

    fn ids(sel: &Selection<'_>) -> Vec<String> {
        sel.alerts.iter().map(|a| a.id.clone()).collect()
    }

    #[test]
    fn the_window_keeps_the_edge_and_drops_one_second_older() {
        let all = [
            alert("older", &[Uf::Sc], "2026-10-20T11:59:59+00:00"),
            alert("edge", &[Uf::Sc], EDGE),
            alert("edge-brt", &[Uf::Sc], "2026-10-20T09:00:00-03:00"),
            alert("new", &[Uf::Sc], "2026-10-27T11:00:00+00:00"),
        ];
        let s = sel(&all, Some(7));
        assert_eq!(ids(&s), ["edge", "edge-brt", "new"]);
        assert_eq!(s.total, 4);
        let p = page_in(&all, Some(7));
        assert!(p.contains("headline edge") && !p.contains("headline older"));
    }

    #[test]
    fn no_window_keeps_every_alert() {
        let all = [
            alert("old", &[Uf::Sc], "2001-01-01T00:00:00+00:00"),
            alert("new", &[Uf::Sc], NOW),
        ];
        assert_eq!(ids(&sel(&all, None)), ["old", "new"]);
    }

    #[test]
    fn an_alert_with_an_unparsable_time_is_kept_and_last() {
        let all = [
            alert("bad", &[Uf::Sc], "ontem à tarde"),
            alert("empty", &[Uf::Sc], ""),
            alert("old", &[Uf::Sc], "2026-01-01T00:00:00+00:00"),
            alert("new", &[Uf::Sc], "2026-10-27T11:00:00+00:00"),
        ];
        let s = sel(&all, Some(7));
        assert_eq!(ids(&s), ["bad", "empty", "new"]);
        let v: serde_json::Value = serde_json::from_str(&render_json(&s, NOW).unwrap()).unwrap();
        let order: Vec<&str> = v["alerts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["id"].as_str().unwrap())
            .collect();
        assert_eq!(order[0], "new");
        assert_eq!(order[1..], ["empty", "bad"]);
    }

    #[test]
    fn the_page_says_which_alerts_it_lists() {
        let all = [alert("a", &[Uf::Pr], "2026-10-27T10:00:00+00:00")];
        let end = "A fonte não informa até quando um alerta vale.";
        let windowed = page_in(&all, Some(3));
        assert!(windowed.contains(&format!(
            "Alertas enviados nos últimos 3 dias, do mais novo ao mais antigo. {end}"
        )));
        let all_of_them = page_in(&all, None);
        assert!(all_of_them.contains(&format!(
            "Todos os alertas guardados, do mais novo ao mais antigo. {end}"
        )));
        for p in [windowed, all_of_them] {
            assert!(!p.contains("ativo") && !p.contains("ativa") && !p.contains("em vigor"));
        }
    }

    #[test]
    fn an_empty_window_is_not_no_data() {
        let old = [alert("old", &[Uf::Sc], "2026-01-01T00:00:00+00:00")];
        let p = page_in(&old, Some(7));
        assert!(
            p.contains("Sem alertas nos últimos 7 dias. Última verificação: 2026-10-27 12:00 UTC")
        );
        assert!(!p.contains("Sem dados") && !p.contains("headline old"));
        let none = page_in(&[], Some(7));
        assert!(none.contains("<strong>Sem dados</strong>"));
        assert!(!none.contains("Sem alertas nos últimos"));
    }

    #[test]
    fn json_and_health_name_the_window() {
        let all = [
            alert("old", &[], "2026-01-01T00:00:00+00:00"),
            alert("new", &[], "2026-10-27T11:00:00+00:00"),
        ];
        let json = |days| -> serde_json::Value {
            serde_json::from_str(&render_json(&sel(&all, days), NOW).unwrap()).unwrap()
        };
        assert_eq!(json(Some(7))["window_days"], 7);
        assert_eq!(json(Some(7))["alerts"].as_array().unwrap().len(), 1);
        assert!(json(None)["window_days"].is_null());
        assert_eq!(json(None)["alerts"].as_array().unwrap().len(), 2);

        let health: serde_json::Value =
            serde_json::from_str(&render_health(&sel(&all, Some(7)), NOW)).unwrap();
        assert_eq!(
            (health["window_days"].as_u64(), health["alerts"].as_u64()),
            (Some(7), Some(2))
        );
        assert_eq!(health["in_window"], 1);
        let none: serde_json::Value =
            serde_json::from_str(&render_health(&sel(&all, None), NOW)).unwrap();
        assert!(none["window_days"].is_null());
        assert_eq!(none["in_window"], 2);
    }

    #[test]
    fn health_names_the_newest_alert_even_outside_the_window() {
        let all = [alert("old", &[], "2026-01-01T00:00:00+00:00")];
        let h: serde_json::Value =
            serde_json::from_str(&render_health(&sel(&all, Some(7)), NOW)).unwrap();
        assert_eq!(h["in_window"], 0);
        assert_eq!(h["newest"], "2026-01-01T00:00:00+00:00");
    }

    #[test]
    fn days_on_the_command_line() {
        let args = |a: &[&str]| parse_args(&a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
        let out = |days| {
            Ok(Args {
                out: "d".into(),
                days,
            })
        };
        assert_eq!(args(&["--out", "d"]), out(Some(7)));
        assert_eq!(args(&["--days", "30", "--out", "d"]), out(Some(30)));
        assert_eq!(args(&["--out", "d", "--days", "0"]), out(None));
        for bad in ["x", "-1", "1.5", "", "99999999999"] {
            let e = args(&["--out", "d", "--days", bad]).unwrap_err();
            assert!(e.contains("--days needs a whole number"), "{bad}: {e}");
        }
        assert!(args(&["--out", "d", "--days"]).is_err());
        assert!(args(&["--days", "3"]).is_err());
        assert!(args(&["dir", "x"]).is_err());
    }
}
