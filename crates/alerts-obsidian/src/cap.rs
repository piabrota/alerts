//! Reads CAP 1.2 alerts (OASIS) from an Atom feed, an RSS feed, or one CAP document.

use crate::xml::{self, El};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Geocode {
    pub name: String,
    pub value: String,
}

/// One CAP alert with the fields a note needs. A missing CAP field is an empty string. The text
/// fields are the issuer's text, unchanged except for the whitespace at both ends.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CapAlert {
    pub identifier: String,
    pub sender: String,
    pub sent: String,
    pub status: String,
    pub msg_type: String,
    /// CAP `<references>`: `sender,identifier,sent` triples.
    pub references: Vec<String>,
    pub language: String,
    pub event: String,
    pub urgency: String,
    pub severity: String,
    pub certainty: String,
    pub onset: String,
    pub expires: String,
    pub sender_name: String,
    pub headline: String,
    pub description: String,
    pub instruction: String,
    pub web: String,
    pub area_desc: Vec<String>,
    pub geocodes: Vec<Geocode>,
}

impl CapAlert {
    /// The identifiers that `<references>` names, oldest first as the issuer wrote them.
    pub fn referenced_ids(&self) -> Vec<String> {
        self.references
            .iter()
            .filter_map(|r| r.split(',').nth(1))
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty())
            .collect()
    }
}

/// What a feed holds: the alerts written inside it, the links to CAP documents that the reader
/// must fetch, and the entries that hold neither.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feed {
    pub alerts: Vec<CapAlert>,
    pub links: Vec<String>,
    pub unreadable: Vec<String>,
}

pub fn alert_from(el: &El) -> Result<CapAlert, String> {
    let identifier = el.child_text("identifier");
    if identifier.is_empty() {
        return Err("CAP: an <alert> without <identifier>".into());
    }
    let infos: Vec<&El> = el.children("info").collect();
    // One alert can carry one <info> per language: take the Portuguese one.
    let info = infos
        .iter()
        .find(|i| {
            i.child_text("language")
                .to_ascii_lowercase()
                .starts_with("pt")
        })
        .or(infos.first());
    let mut a = CapAlert {
        identifier,
        sender: el.child_text("sender"),
        sent: el.child_text("sent"),
        status: el.child_text("status"),
        msg_type: el.child_text("msgType"),
        references: el
            .child_text("references")
            .split_whitespace()
            .map(str::to_owned)
            .collect(),
        ..CapAlert::default()
    };
    if let Some(i) = info {
        a.language = i.child_text("language");
        a.event = i.child_text("event");
        a.urgency = i.child_text("urgency");
        a.severity = i.child_text("severity");
        a.certainty = i.child_text("certainty");
        a.onset = i.child_text("onset");
        a.expires = i.child_text("expires");
        a.sender_name = i.child_text("senderName");
        a.headline = i.child_text("headline");
        a.description = i.child_text("description");
        a.instruction = i.child_text("instruction");
        a.web = i.child_text("web");
        for area in i.children("area") {
            let desc = area.child_text("areaDesc");
            if !desc.is_empty() {
                a.area_desc.push(desc);
            }
            for g in area.children("geocode") {
                a.geocodes.push(Geocode {
                    name: g.child_text("valueName"),
                    value: g.child_text("value"),
                });
            }
        }
    }
    Ok(a)
}

/// One CAP document, fetched from a feed link.
pub fn parse_cap(xml: &str) -> Result<CapAlert, String> {
    let root = xml::parse(xml)?;
    match root.name.as_str() {
        "alert" => alert_from(&root),
        _ => match root.descendant("alert") {
            Some(a) => alert_from(a),
            None => Err(format!("CAP: no <alert> in a <{}> document", root.name)),
        },
    }
}

/// The CAP link of an Atom entry or an RSS item: a link typed as CAP, else the first link.
fn link(entry: &El) -> Option<String> {
    let atom: Vec<&El> = entry
        .children("link")
        .filter(|l| l.attr("href").is_some())
        .collect();
    let typed = atom
        .iter()
        .find(|l| l.attr("type").is_some_and(|t| t.contains("cap")))
        .or(atom.first());
    if let Some(href) = typed.and_then(|l| l.attr("href")) {
        return Some(href.to_owned());
    }
    let rss = entry.child_text("link");
    (!rss.is_empty()).then_some(rss)
}

fn entry_name(entry: &El) -> String {
    let id = entry.child_text("id");
    if !id.is_empty() {
        return id;
    }
    let guid = entry.child_text("guid");
    if !guid.is_empty() {
        return guid;
    }
    entry.child_text("title")
}

/// An Atom feed (RFC 4287), an RSS 2.0 feed, or one CAP document. A document of another kind is
/// an error: a reader must never take it for an empty feed.
pub fn parse_feed(xml: &str) -> Result<Feed, String> {
    let root = xml::parse(xml)?;
    let entries: Vec<&El> = match root.name.as_str() {
        "alert" => {
            return Ok(Feed {
                alerts: vec![alert_from(&root)?],
                ..Feed::default()
            });
        }
        "feed" => root.children("entry").collect(),
        "rss" => root
            .children("channel")
            .flat_map(|c| c.children("item"))
            .collect(),
        other => {
            return Err(format!(
                "not an Atom feed, an RSS feed or a CAP alert: <{other}>"
            ));
        }
    };
    let mut feed = Feed::default();
    for entry in entries {
        if let Some(a) = entry.descendant("alert") {
            feed.alerts.push(alert_from(a)?);
            continue;
        }
        // Some feeds escape the CAP document as the text of the entry.
        let escaped = ["content", "description", "summary"]
            .iter()
            .map(|n| entry.child_text(n))
            .find(|t| t.contains("<alert"));
        if let Some(text) = escaped {
            feed.alerts.push(parse_cap(&text)?);
        } else if let Some(href) = link(entry) {
            feed.links.push(href);
        } else {
            feed.unreadable.push(entry_name(entry));
        }
    }
    Ok(feed)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    pub const CAP: &str = r#"<alert xmlns="urn:oasis:names:tc:emergency:cap:1.2">
<identifier>urn:oid:2.49.0.1.76.0.2026.1</identifier><sender>defesacivil@example.gov.br</sender>
<sent>2026-10-22T12:00:00-03:00</sent><status>Actual</status><msgType>Alert</msgType><scope>Public</scope>
<info><language>en-US</language><event>Flood</event><headline>English</headline></info>
<info><language>pt-BR</language><category>Met</category><event>Inundação</event>
<urgency>Immediate</urgency><severity>Severe</severity><certainty>Likely</certainty>
<onset>2026-10-22T12:00:00-03:00</onset><expires>2026-10-23T12:00:00-03:00</expires>
<senderName>Defesa Civil RS</senderName><headline>Risco de inundação &amp; alagamento</headline>
<description>  O rio Guaíba pode subir.
Evite as margens.  </description><instruction>Siga a Defesa Civil.</instruction>
<web>https://example.gov.br/a?b=1&amp;c=2</web>
<area><areaDesc>Porto Alegre</areaDesc><geocode><valueName>IBGE</valueName><value>4314902</value></geocode></area>
<area><areaDesc>Canoas</areaDesc><geocode><valueName>UF</valueName><value>RS</value></geocode></area>
</info></alert>"#;

    #[test]
    fn cap_fields_are_verbatim() {
        let a = parse_cap(CAP).unwrap();
        assert_eq!(a.identifier, "urn:oid:2.49.0.1.76.0.2026.1");
        assert_eq!(a.language, "pt-BR");
        assert_eq!(a.event, "Inundação");
        assert_eq!(a.headline, "Risco de inundação & alagamento");
        assert_eq!(a.description, "O rio Guaíba pode subir.\nEvite as margens.");
        assert_eq!(a.web, "https://example.gov.br/a?b=1&c=2");
        assert_eq!(a.area_desc, ["Porto Alegre", "Canoas"]);
        assert_eq!(a.geocodes.len(), 2);
        assert!(a.referenced_ids().is_empty());
    }

    #[test]
    fn references_name_identifiers() {
        let a = CapAlert {
            references: vec!["s,id-1,2026-10-22T12:00:00-03:00".into(), "s,id-2,x".into()],
            ..CapAlert::default()
        };
        assert_eq!(a.referenced_ids(), ["id-1", "id-2"]);
    }

    #[test]
    fn atom_inline_escaped_and_linked() {
        let escaped = quick_xml::escape::escape(CAP);
        let xml = format!(
            "<feed xmlns=\"http://www.w3.org/2005/Atom\"><id>f</id>\
             <entry><id>e1</id><content type=\"application/cap+xml\">{CAP}</content></entry>\
             <entry><id>e2</id><content type=\"text\">{escaped}</content></entry>\
             <entry><id>e3</id><link rel=\"alternate\" href=\"https://x/page\"/>\
             <link type=\"application/cap+xml\" href=\"https://x/cap/3\"/></entry>\
             <entry><id>e4</id><title>nada</title></entry></feed>"
        );
        let f = parse_feed(&xml).unwrap();
        assert_eq!(f.alerts.len(), 2);
        assert_eq!(f.alerts[1].headline, "Risco de inundação & alagamento");
        assert_eq!(f.links, ["https://x/cap/3"]);
        assert_eq!(f.unreadable, ["e4"]);
    }

    #[test]
    fn rss_items_link_to_cap() {
        let xml = "<rss version=\"2.0\"><channel><title>t</title>\
                   <item><title>a</title><link>https://x/avisos/rss/1</link></item></channel></rss>";
        assert_eq!(parse_feed(xml).unwrap().links, ["https://x/avisos/rss/1"]);
    }

    #[test]
    fn other_documents_are_errors() {
        assert!(parse_feed("<html><body/></html>").is_err());
        assert!(parse_feed("not xml").is_err());
        assert!(parse_cap("<alert><sender>x</sender></alert>").is_err());
    }
}
