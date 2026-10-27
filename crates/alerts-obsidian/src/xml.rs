//! A small element tree over quick-xml: enough to walk an Atom, RSS or CAP document.

use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesStart, Event};

#[derive(Debug, Default)]
pub struct El {
    /// The local name, without the namespace prefix.
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub kids: Vec<Node>,
}

#[derive(Debug)]
pub enum Node {
    El(El),
    Text(String),
}

impl El {
    pub fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a El> + 'a {
        self.kids.iter().filter_map(move |k| match k {
            Node::El(e) if e.name == name => Some(e),
            _ => None,
        })
    }

    pub fn child<'a>(&'a self, name: &'a str) -> Option<&'a El> {
        self.children(name).next()
    }

    /// The first element named `name` below this one, depth first.
    pub fn descendant(&self, name: &str) -> Option<&El> {
        self.kids.iter().find_map(|k| match k {
            Node::El(e) if e.name == name => Some(e),
            Node::El(e) => e.descendant(name),
            Node::Text(_) => None,
        })
    }

    /// The text of the direct children, trimmed at both ends.
    pub fn text(&self) -> String {
        let mut s = String::new();
        for k in &self.kids {
            if let Node::Text(t) = k {
                s.push_str(t);
            }
        }
        s.trim().to_owned()
    }

    /// The trimmed text of the first child named `name`, or an empty string.
    pub fn child_text(&self, name: &str) -> String {
        self.child(name).map(El::text).unwrap_or_default()
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

fn start(e: &BytesStart<'_>) -> Result<El, String> {
    let mut attrs = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|e| format!("XML attribute: {e}"))?;
        let v = a
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|e| format!("XML attribute: {e}"))?;
        attrs.push((a.key.local_name().as_ref().to_owned(), v.into_owned()));
    }
    Ok(El {
        name: e.local_name().as_ref().to_owned(),
        attrs,
        kids: Vec::new(),
    })
}

/// The root element of `xml`. Malformed XML is an error, never an empty tree.
pub fn parse(xml: &str) -> Result<El, String> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<El> = Vec::new();
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("XML at byte {}: {e}", reader.error_position()))?;
        let text = match event {
            Event::Start(e) => {
                stack.push(start(&e)?);
                continue;
            }
            Event::Empty(e) => {
                let el = start(&e)?;
                match stack.last_mut() {
                    Some(parent) => parent.kids.push(Node::El(el)),
                    None => return Ok(el),
                }
                continue;
            }
            Event::End(_) => {
                let el = stack.pop().ok_or("XML: an end tag without a start tag")?;
                match stack.last_mut() {
                    Some(parent) => parent.kids.push(Node::El(el)),
                    None => return Ok(el),
                }
                continue;
            }
            Event::Text(t) => t.xml10_content().into_owned(),
            Event::CData(t) => t.xml10_content().into_owned(),
            Event::GeneralRef(r) => match r.resolve_char_ref() {
                Ok(Some(c)) => c.to_string(),
                _ => match resolve_predefined_entity(&r) {
                    Some(s) => s.to_owned(),
                    // An entity that XML does not define: keep it as written.
                    None => format!("&{};", &*r),
                },
            },
            Event::Eof => return Err("XML: the document ended before its root element".into()),
            _ => continue,
        };
        if let Some(parent) = stack.last_mut() {
            parent.kids.push(Node::Text(text));
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn tree_text_and_entities() {
        let el = parse("<?xml version=\"1.0\"?><a:r xmlns:a=\"x\"><b k=\"v&amp;w\">x &amp; y &#233;</b><c/><![CDATA[<z>]]></a:r>").unwrap();
        assert_eq!(el.name, "r");
        assert_eq!(el.child_text("b"), "x & y é");
        assert_eq!(el.child("b").unwrap().attr("k"), Some("v&w"));
        assert!(el.child("c").is_some());
        assert_eq!(el.text(), "<z>");
    }

    #[test]
    fn malformed_is_an_error() {
        assert!(parse("<a><b></a>").is_err());
        assert!(parse("").is_err());
        assert!(parse("<a>").is_err());
    }
}
