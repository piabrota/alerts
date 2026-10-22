//! The Atom + CAP 1.2 channel: an Atom feed (RFC 4287) whose entries each hold one
//! CAP 1.2 alert (OASIS), the same shape as the national IDAP feed.

use quick_xml::escape::escape;
use smart_alerts_core::{Alert, Severity};
use std::fmt::{self, Write};

pub const CAP_NS: &str = "urn:oasis:names:tc:emergency:cap:1.2";
const SENDER: &str = "smart-alerts";

/// CAP `<status>`. A test alert must never be published as `Actual`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Actual,
    Test,
}

#[derive(Debug, Clone)]
pub struct FeedMeta {
    /// A permanent URI for the feed, for example `urn:smart-alerts`.
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedError {
    /// `sent` is not `YYYY-MM-DDThh:mm:ss±hh:mm`, the only form CAP 1.2 accepts.
    InvalidTime(String),
}

impl fmt::Display for FeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeedError::InvalidTime(t) => write!(f, "invalid CAP time: {t:?}"),
        }
    }
}

impl std::error::Error for FeedError {}

fn check_time(t: &str) -> Result<&str, FeedError> {
    let b = t.as_bytes();
    let digits = [
        0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18, 20, 21, 23, 24,
    ];
    let seps = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (22, b':'),
    ];
    let ok = b.len() == 25
        && digits.iter().all(|&i| b[i].is_ascii_digit())
        && seps.iter().all(|&(i, c)| b[i] == c)
        && matches!(b[19], b'+' | b'-');
    if ok {
        Ok(t)
    } else {
        Err(FeedError::InvalidTime(t.to_owned()))
    }
}

/// CAP forbids spaces, commas and restricted characters in `<identifier>`.
fn identifier(a: &Alert) -> String {
    format!("{SENDER}-{}-{}", a.source, a.id)
        .chars()
        .map(|c| {
            if c.is_whitespace() || ",<>&\"'".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect()
}

fn severity(s: Severity) -> &'static str {
    match s {
        Severity::Minor => "Minor",
        Severity::Moderate => "Moderate",
        Severity::Severe => "Severe",
        Severity::Extreme => "Extreme",
    }
}

/// One CAP 1.2 `<alert>` element, without an XML declaration.
pub fn cap(a: &Alert, status: Status) -> Result<String, FeedError> {
    let sent = check_time(&a.sent)?;
    let status = match status {
        Status::Actual => "Actual",
        Status::Test => "Test",
    };
    // Writing to a String cannot fail, so the write! results are ignored.
    let mut x = String::new();
    let _ = write!(
        x,
        "<alert xmlns=\"{CAP_NS}\"><identifier>{}</identifier><sender>{SENDER}</sender>\
         <sent>{sent}</sent><status>{status}</status><msgType>Alert</msgType>\
         <source>{}</source><scope>Public</scope><info><language>pt-BR</language>\
         <category>Met</category><event>{}</event><urgency>Unknown</urgency>\
         <severity>{}</severity><certainty>Unknown</certainty><headline>{}</headline>",
        escape(identifier(a)),
        escape(a.source.as_str()),
        a.hazard.label_pt_br(),
        severity(a.severity),
        escape(a.headline.as_str()),
    );
    if let Some(url) = &a.url {
        let _ = write!(x, "<web>{}</web>", escape(url.as_str()));
    }
    let _ = write!(x, "<area><areaDesc>{}</areaDesc>", escape(a.area.as_str()));
    for uf in &a.ufs {
        let _ = write!(
            x,
            "<geocode><valueName>UF</valueName><value>{uf}</value></geocode>"
        );
    }
    x.push_str("</area></info></alert>");
    Ok(x)
}

/// The whole Atom feed. `updated` is the feed time, in the same form as `Alert::sent`.
pub fn atom(
    meta: &FeedMeta,
    alerts: &[Alert],
    status: Status,
    updated: &str,
) -> Result<String, FeedError> {
    let mut x = String::new();
    let _ = write!(
        x,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <feed xmlns=\"http://www.w3.org/2005/Atom\"><id>{}</id><title>{}</title>\
         <updated>{}</updated><author><name>{SENDER}</name></author>",
        escape(meta.id.as_str()),
        escape(meta.title.as_str()),
        check_time(updated)?,
    );
    for a in alerts {
        let _ = write!(
            x,
            "<entry><id>{}:{}</id><title>{}</title><updated>{}</updated>\
             <summary>{}</summary><content type=\"application/cap+xml\">{}</content></entry>",
            escape(meta.id.as_str()),
            escape(identifier(a)),
            escape(a.headline.as_str()),
            check_time(&a.sent)?,
            escape(a.area.as_str()),
            cap(a, status)?,
        );
    }
    x.push_str("</feed>\n");
    Ok(x)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use quick_xml::{Reader, events::Event};
    use smart_alerts_core::{Hazard, Uf};

    fn alert() -> Alert {
        Alert {
            id: "a 1,<x>".into(),
            source: "inmet".into(),
            hazard: Hazard::Flood,
            severity: Severity::Severe,
            ufs: vec![Uf::Rs, Uf::Sc],
            headline: "Risco de inundação em Porto Alegre & região.".into(),
            area: "Porto Alegre (RS)".into(),
            sent: "2026-10-22T12:00:00-03:00".into(),
            url: Some("https://example.org/a?b=1&c=2".into()),
        }
    }

    /// Every element name in document order; panics on malformed XML.
    fn names(xml: &str) -> Vec<String> {
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

    #[test]
    fn cap_has_required_elements_in_schema_order() {
        let x = cap(&alert(), Status::Actual).unwrap();
        let n = names(&x);
        let order = [
            "alert",
            "identifier",
            "sender",
            "sent",
            "status",
            "msgType",
            "source",
            "scope",
            "info",
            "language",
            "category",
            "event",
            "urgency",
            "severity",
            "certainty",
            "headline",
            "web",
            "area",
            "areaDesc",
            "geocode",
        ];
        let pos: Vec<usize> = order
            .iter()
            .map(|o| n.iter().position(|e| e == o).unwrap())
            .collect();
        assert!(pos.windows(2).all(|w| w[0] < w[1]), "{n:?}");
        assert!(x.contains("<identifier>smart-alerts-inmet-a_1__x_</identifier>"));
        assert!(x.contains("&amp; região"));
        assert_eq!(x.matches("<geocode>").count(), 2);
    }

    #[test]
    fn test_status_is_kept() {
        let x = cap(&alert(), Status::Test).unwrap();
        assert!(x.contains("<status>Test</status>"));
    }

    #[test]
    fn invalid_time_is_rejected() {
        let mut a = alert();
        a.sent = "2026-10-22 12:00".into();
        assert!(matches!(
            cap(&a, Status::Actual),
            Err(FeedError::InvalidTime(_))
        ));
        a.sent = String::new();
        assert!(cap(&a, Status::Actual).is_err());
    }

    #[test]
    fn atom_is_well_formed_with_inline_cap() {
        let meta = FeedMeta {
            id: "urn:smart-alerts".into(),
            title: "smart-alerts".into(),
        };
        let updated = "2026-10-22T15:00:00+00:00";
        let x = atom(&meta, &[alert(), alert()], Status::Actual, updated).unwrap();
        let n = names(&x);
        assert_eq!(n.iter().filter(|e| *e == "entry").count(), 2);
        assert_eq!(n.iter().filter(|e| *e == "alert").count(), 2);
        assert!(x.contains("<content type=\"application/cap+xml\"><alert xmlns="));
    }
}
