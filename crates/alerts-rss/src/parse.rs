//! RSS 2.0, RSS 1.0 (RDF) and Atom (RFC 4287) documents to items. The parser reads only the
//! fields an [`Item`] holds and ignores every other element and namespace.

use crate::{Item, time};
use quick_xml::escape::resolve_xml_entity;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The document is not well-formed XML.
    Xml(String),
    /// The root element is not `rss`, `RDF` or `feed`.
    NotAFeed(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Xml(e) => write!(f, "malformed XML: {e}"),
            ParseError::NotAFeed(root) => write!(f, "not an RSS or Atom feed (root <{root}>)"),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Default)]
struct Draft {
    id: String,
    title: String,
    link: String,
    summary: String,
    content: String,
    updated: String,
    published: String,
}

impl Draft {
    fn finish(self, feed: &str) -> Option<Item> {
        let title = self.title.trim().to_owned();
        let link = Some(self.link.trim().to_owned()).filter(|l| !l.is_empty());
        let key = [link.as_deref(), Some(self.id.trim()), Some(title.as_str())]
            .into_iter()
            .flatten()
            .find(|k| !k.is_empty())?
            .to_owned();
        let summary = if self.summary.trim().is_empty() {
            self.content
        } else {
            self.summary
        };
        let updated = time::normalize(&self.updated).or_else(|| time::normalize(&self.published));
        Some(Item {
            feed: feed.to_owned(),
            key,
            title,
            link,
            summary: summary.trim().to_owned(),
            updated,
            alert: None,
        })
    }

    fn field(&mut self, name: &str) -> Option<&mut String> {
        Some(match name {
            "title" => &mut self.title,
            "guid" | "id" => &mut self.id,
            "link" => &mut self.link,
            "description" | "summary" => &mut self.summary,
            "content" | "encoded" => &mut self.content,
            "updated" | "pubDate" | "date" | "modified" => &mut self.updated,
            "published" | "issued" => &mut self.published,
            _ => return None,
        })
    }
}

/// The `href` of an Atom `<link>`, when its `rel` is absent or `alternate`.
fn atom_href(e: &BytesStart<'_>) -> Option<String> {
    let attr = |name: &str| {
        e.try_get_attribute(name)
            .ok()
            .flatten()
            .and_then(|a| a.normalized_value(XmlVersion::Implicit1_0).ok())
            .map(|v| v.into_owned())
    };
    let rel = attr("rel");
    matches!(rel.as_deref(), None | Some("alternate"))
        .then(|| attr("href"))
        .flatten()
}

/// Every item of the document, in document order, tagged with `feed`. An entry with no link,
/// no id and no title has no identity, so it is left out.
pub fn parse(xml: &str, feed: &str) -> Result<Vec<Item>, ParseError> {
    let mut r = Reader::from_str(xml);
    let mut items = Vec::new();
    let mut root: Option<String> = None;
    let mut draft: Option<Draft> = None;
    // The element whose text the draft collects, and the depth inside the item.
    let mut field: Option<String> = None;
    let mut depth = 0usize;
    loop {
        let event = r.read_event().map_err(|e| ParseError::Xml(e.to_string()))?;
        match event {
            Event::Start(e) | Event::Empty(e) if root.is_none() => {
                let name = e.local_name().as_ref().to_owned();
                if !matches!(name.as_str(), "rss" | "RDF" | "feed") {
                    return Err(ParseError::NotAFeed(name));
                }
                root = Some(name);
            }
            Event::Start(e) => {
                let name = e.local_name().as_ref().to_owned();
                match &mut draft {
                    None if matches!(name.as_str(), "item" | "entry") => {
                        draft = Some(Draft::default());
                        depth = 0;
                    }
                    Some(d) => {
                        depth += 1;
                        if depth == 1 {
                            if name == "link"
                                && let Some(href) = atom_href(&e)
                            {
                                d.link = href;
                            }
                            field = d.field(&name).is_some().then_some(name);
                        }
                    }
                    None => {}
                }
            }
            Event::Empty(e) => {
                if let Some(d) = &mut draft
                    && depth == 0
                    && e.local_name().as_ref() == "link"
                    && let Some(href) = atom_href(&e)
                {
                    d.link = href;
                }
            }
            Event::End(e) => match &mut draft {
                Some(_) if depth == 0 => {
                    if matches!(e.local_name().as_ref(), "item" | "entry")
                        && let Some(d) = draft.take()
                        && let Some(item) = d.finish(feed)
                    {
                        items.push(item);
                    }
                }
                Some(_) => {
                    depth -= 1;
                    if depth == 0 {
                        field = None;
                    }
                }
                None => {}
            },
            Event::Text(t) => push(&mut draft, &field, &t.xml_content(XmlVersion::Implicit1_0)),
            Event::CData(t) => push(&mut draft, &field, &t.xml_content(XmlVersion::Implicit1_0)),
            Event::GeneralRef(g) => {
                let text = match g.resolve_char_ref() {
                    Ok(Some(c)) => c.to_string(),
                    // An entity that XML does not define stays as the source wrote it.
                    _ => resolve_xml_entity(&g).map_or_else(|| format!("&{};", &*g), str::to_owned),
                };
                push(&mut draft, &field, &text);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    match root {
        Some(_) => Ok(items),
        None => Err(ParseError::NotAFeed(String::new())),
    }
}

fn push(draft: &mut Option<Draft>, field: &Option<String>, text: &str) {
    if let (Some(d), Some(name)) = (draft, field)
        && let Some(f) = d.field(name)
    {
        f.push_str(text);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const RSS: &str = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/">
  <channel><title>Notícias</title><link>https://example.org/</link>
    <item>
      <title>Chuva forte &amp; alagamento</title>
      <link>https://example.org/a</link>
      <guid isPermaLink="false">a-1</guid>
      <description><![CDATA[<p>Rio sobe em <b>Blumenau</b>.</p>]]></description>
      <pubDate>Thu, 22 Oct 2026 15:00:00 GMT</pubDate>
    </item>
    <item><guid>b-2</guid><title>Sem link</title><content:encoded>Corpo</content:encoded></item>
    <item><description>Sem identidade</description></item>
  </channel>
</rss>"#;

    const ATOM: &str = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>t</title>
  <entry><id>urn:x:1</id><title type="text">Primeiro</title>
    <link rel="self" href="https://example.org/self"/>
    <link href="https://example.org/1"/>
    <updated>2026-10-22T12:00:00-03:00</updated><summary>Resumo &#233;</summary>
    <author><name>Não é título</name></author>
  </entry>
</feed>"#;

    #[test]
    fn rss_items() {
        let items = parse(RSS, "g1").unwrap();
        assert_eq!(items.len(), 2);
        let a = &items[0];
        assert_eq!(a.feed, "g1");
        assert_eq!(a.key, "https://example.org/a");
        assert_eq!(a.title, "Chuva forte & alagamento");
        assert_eq!(a.summary, "<p>Rio sobe em <b>Blumenau</b>.</p>");
        assert_eq!(a.updated.as_deref(), Some("2026-10-22T15:00:00+00:00"));
        assert_eq!(items[1].key, "b-2");
        assert_eq!(items[1].summary, "Corpo");
        assert_eq!(items[1].updated, None);
    }

    #[test]
    fn atom_entries() {
        let items = parse(ATOM, "g2").unwrap();
        assert_eq!(items.len(), 1);
        let e = &items[0];
        assert_eq!(e.key, "https://example.org/1");
        assert_eq!(e.title, "Primeiro");
        assert_eq!(e.summary, "Resumo é");
        assert_eq!(e.updated.as_deref(), Some("2026-10-22T15:00:00+00:00"));
    }

    #[test]
    fn not_a_feed() {
        assert!(matches!(
            parse("<html><body/></html>", "x"),
            Err(ParseError::NotAFeed(r)) if r == "html"
        ));
        assert!(parse("", "x").is_err());
    }
}
