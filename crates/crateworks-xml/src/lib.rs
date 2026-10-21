//! Minimal XML reading and writing.
//!
//! This crate exists because `iso20022` and `soap-client` both need the same
//! small amount of XML handling, and neither needs a full DOM. It deliberately
//! covers a subset of XML: elements, attributes, text, CDATA, comments,
//! processing instructions and the five predefined entities plus numeric
//! character references. It does not resolve external entities, which also
//! means it cannot be used for XXE attacks.
//!
//! # Scope
//!
//! * [`parse`] builds an [`Element`] tree from a string.
//! * [`Element::to_string`] (via [`Display`](std::fmt::Display)) writes it back.
//! * Namespaces are preserved as part of the element name; nothing is resolved.
//!
//! # Example
//!
//! ```
//! use crateworks_xml::{Element, parse};
//!
//! let root = parse("<a x=\"1\"><b>hi</b></a>").unwrap();
//! assert_eq!(root.name, "a");
//! assert_eq!(root.attr("x"), Some("1"));
//! assert_eq!(root.child("b").unwrap().text(), "hi");
//! ```

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::{self, Display, Write as _};

/// An error produced while parsing XML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Byte offset in the input where the problem was detected.
    pub offset: usize,
    /// Human readable description.
    pub message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XML error at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

/// A parsed XML element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// Element name, including any namespace prefix as written.
    pub name: String,
    /// Attributes in document order-independent sorted order.
    pub attributes: BTreeMap<String, String>,
    /// Child nodes.
    pub children: Vec<Node>,
}

/// A child of an [`Element`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// A nested element.
    Element(Element),
    /// Character data. Consecutive runs of text are merged.
    Text(String),
    /// A comment, retained so that round-tripping is faithful.
    Comment(String),
}

impl Element {
    /// Creates an element with no attributes or children.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            attributes: BTreeMap::new(),
            children: Vec::new(),
        }
    }

    /// Looks up an attribute value.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }

    /// Returns the first direct child element with the given name.
    #[must_use]
    pub fn child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find_map(|n| match n {
            Node::Element(e) if e.name == name => Some(e),
            _ => None,
        })
    }

    /// Iterates over direct child elements, skipping text and comments.
    pub fn children_named(&self, name: &str) -> impl Iterator<Item = &Self> {
        self.children.iter().filter_map(move |n| match n {
            Node::Element(e) if e.name == name => Some(e),
            _ => None,
        })
    }

    /// Concatenates all direct text children.
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::new();
        for n in &self.children {
            if let Node::Text(t) = n {
                out.push_str(t);
            }
        }
        out
    }

    /// Collects text from this element and every descendant, in document order.
    #[must_use]
    pub fn deep_text(&self) -> String {
        let mut out = String::new();
        self.collect_text(&mut out);
        out
    }

    fn collect_text(&self, out: &mut String) {
        for n in &self.children {
            match n {
                Node::Text(t) => out.push_str(t),
                Node::Element(e) => e.collect_text(out),
                Node::Comment(_) => {}
            }
        }
    }

    /// Appends a child element.
    pub fn push(&mut self, child: Self) -> &mut Self {
        self.children.push(Node::Element(child));
        self
    }

    /// Appends text.
    pub fn push_text(&mut self, text: impl Into<String>) -> &mut Self {
        self.children.push(Node::Text(text.into()));
        self
    }

    /// Sets an attribute.
    pub fn set_attr(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.attributes.insert(name.into(), value.into());
        self
    }
}

impl Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}", self.name)?;
        for (k, v) in &self.attributes {
            write!(f, " {k}=\"{}\"", escape_attr(v))?;
        }
        if self.children.is_empty() {
            return f.write_str("/>");
        }
        f.write_char('>')?;
        for n in &self.children {
            match n {
                Node::Element(e) => write!(f, "{e}")?,
                Node::Text(t) => f.write_str(&escape_text(t))?,
                Node::Comment(c) => write!(f, "<!--{c}-->")?,
            }
        }
        write!(f, "</{}>", self.name)
    }
}

/// Escapes text content for use inside an element.
#[must_use]
pub fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escapes a value for use inside a double-quoted attribute.
#[must_use]
pub fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Decodes the predefined entities and numeric character references.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'&' {
                i += 1;
            }
            out.push_str(&s[start..i]);
            continue;
        }
        let Some(semi) = s[i..].find(';').map(|p| i + p) else {
            out.push('&');
            i += 1;
            continue;
        };
        let entity = &s[i + 1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                u32::from_str_radix(&entity[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            _ if entity.starts_with('#') => {
                entity[1..].parse::<u32>().ok().and_then(char::from_u32)
            }
            _ => None,
        };
        if let Some(c) = decoded {
            out.push(c);
            i = semi + 1;
        } else {
            out.push('&');
            i += 1;
        }
    }
    out
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn err<T>(&self, message: impl Into<String>) -> Result<T> {
        Err(Error {
            offset: self.pos,
            message: message.into(),
        })
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, s: &str) -> Result<()> {
        if self.starts_with(s) {
            self.pos += s.len();
            Ok(())
        } else {
            self.err(format!("expected {s:?}"))
        }
    }

    fn parse_name(&mut self) -> Result<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':') {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        if self.pos == start {
            return self.err("expected element or attribute name");
        }
        Ok(self.src[start..self.pos].to_string())
    }

    fn skip_comment(&mut self) -> Result<()> {
        self.expect("<!--")?;
        let Some(end) = self.src[self.pos..].find("-->") else {
            return self.err("unterminated comment");
        };
        self.pos += end + 3;
        Ok(())
    }

    fn skip_pi(&mut self) -> Result<()> {
        self.expect("<?")?;
        let Some(end) = self.src[self.pos..].find("?>") else {
            return self.err("unterminated processing instruction");
        };
        self.pos += end + 2;
        Ok(())
    }

    fn skip_doctype(&mut self) -> Result<()> {
        // Skips a DOCTYPE declaration, including an internal subset. External
        // entities are never resolved, so this only advances past the text.
        self.expect("<!DOCTYPE")?;
        let mut depth = 0usize;
        while let Some(c) = self.peek() {
            self.pos += c.len_utf8();
            match c {
                '[' => depth += 1,
                ']' => depth = depth.saturating_sub(1),
                '>' if depth == 0 => return Ok(()),
                _ => {}
            }
        }
        self.err("unterminated DOCTYPE")
    }

    fn parse_attributes(&mut self) -> Result<BTreeMap<String, String>> {
        let mut attrs = BTreeMap::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('>') | Some('/') | None => return Ok(attrs),
                _ => {}
            }
            let name = self.parse_name()?;
            self.skip_ws();
            self.expect("=")?;
            self.skip_ws();
            let quote = self
                .peek()
                .filter(|c| *c == '"' || *c == '\'')
                .ok_or_else(|| Error {
                    offset: self.pos,
                    message: "expected quoted attribute value".into(),
                })?;
            self.pos += quote.len_utf8();
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c == quote {
                    break;
                }
                self.pos += c.len_utf8();
            }
            if self.peek().is_none() {
                return self.err("unterminated attribute value");
            }
            let raw = &self.src[start..self.pos];
            self.pos += quote.len_utf8();
            attrs.insert(name, unescape(raw));
        }
    }

    fn parse_element(&mut self) -> Result<Element> {
        self.expect("<")?;
        let name = self.parse_name()?;
        let attributes = self.parse_attributes()?;
        self.skip_ws();
        if self.starts_with("/>") {
            self.pos += 2;
            return Ok(Element {
                name,
                attributes,
                children: Vec::new(),
            });
        }
        self.expect(">")?;

        let mut children: Vec<Node> = Vec::new();
        loop {
            if self.starts_with("</") {
                self.pos += 2;
                let close = self.parse_name()?;
                self.skip_ws();
                self.expect(">")?;
                if close != name {
                    return Err(Error {
                        offset: self.pos,
                        message: format!("closing tag </{close}> does not match <{name}>"),
                    });
                }
                return Ok(Element {
                    name,
                    attributes,
                    children,
                });
            }
            if self.starts_with("<!--") {
                let start = self.pos;
                self.skip_comment()?;
                let body = self.src[start + 4..self.pos - 3].to_string();
                children.push(Node::Comment(body));
                continue;
            }
            if self.starts_with("<![CDATA[") {
                self.pos += 9;
                let Some(end) = self.src[self.pos..].find("]]>") else {
                    return self.err("unterminated CDATA section");
                };
                let body = self.src[self.pos..self.pos + end].to_string();
                self.pos += end + 3;
                push_text(&mut children, body);
                continue;
            }
            if self.starts_with("<?") {
                self.skip_pi()?;
                continue;
            }
            if self.starts_with("<!") {
                return self.err("unexpected declaration inside element");
            }
            if self.starts_with("<") {
                let child = self.parse_element()?;
                children.push(Node::Element(child));
                continue;
            }
            if self.pos >= self.src.len() {
                return self.err(format!("unclosed element <{name}>"));
            }
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c == '<' {
                    break;
                }
                self.pos += c.len_utf8();
            }
            push_text(&mut children, unescape(&self.src[start..self.pos]));
        }
    }
}

fn push_text(children: &mut Vec<Node>, text: String) {
    if let Some(Node::Text(existing)) = children.last_mut() {
        existing.push_str(&text);
    } else {
        children.push(Node::Text(text));
    }
}

/// Parses a complete XML document and returns its root element.
///
/// Prolog declarations, DOCTYPE declarations and top-level comments are
/// skipped. `text` must contain exactly one root element.
///
/// # Errors
///
/// Returns [`Error`] when the input is not well-formed XML.
pub fn parse(text: &str) -> Result<Element> {
    let mut p = Parser { src: text, pos: 0 };
    loop {
        p.skip_ws();
        if p.starts_with("<?") {
            p.skip_pi()?;
        } else if p.starts_with("<!--") {
            p.skip_comment()?;
        } else if p.starts_with("<!DOCTYPE") {
            p.skip_doctype()?;
        } else {
            break;
        }
    }
    if p.peek() != Some('<') {
        return p.err("expected root element");
    }
    let root = p.parse_element()?;
    p.skip_ws();
    if p.pos != p.src.len() {
        return p.err("trailing content after root element");
    }
    Ok(root)
}

/// Extracts the local name of a possibly prefixed name.
#[must_use]
pub fn local_name(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, local)| local)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_attributes_and_children() {
        let root = parse(r#"<a x="1" y='2'><b>hi</b></a>"#).unwrap();
        assert_eq!(root.name, "a");
        assert_eq!(root.attr("x"), Some("1"));
        assert_eq!(root.attr("y"), Some("2"));
        assert_eq!(root.child("b").unwrap().text(), "hi");
    }

    #[test]
    fn self_closing_and_empty_text() {
        let root = parse("<a/>").unwrap();
        assert!(root.children.is_empty());
        assert_eq!(root.to_string(), "<a/>");
    }

    #[test]
    fn decodes_predefined_entities() {
        let root = parse("<a>&lt;&amp;&gt;&quot;&apos;</a>").unwrap();
        assert_eq!(root.text(), "<&>\"'");
    }

    #[test]
    fn decodes_numeric_character_references() {
        let root = parse("<a>&#65;&#x42;</a>").unwrap();
        assert_eq!(root.text(), "AB");
    }

    #[test]
    fn cdata_is_literal() {
        let root = parse("<a><![CDATA[<not-a-tag> & raw]]></a>").unwrap();
        assert_eq!(root.text(), "<not-a-tag> & raw");
    }

    #[test]
    fn nested_and_repeated_children() {
        let root = parse("<r><i>1</i><i>2</i><i>3</i></r>").unwrap();
        let all: Vec<String> = root.children_named("i").map(Element::text).collect();
        assert_eq!(all, ["1", "2", "3"]);
    }

    #[test]
    fn deep_text_walks_descendants() {
        let root = parse("<a>x<b>y<c>z</c></b></a>").unwrap();
        assert_eq!(root.deep_text(), "xyz");
    }

    #[test]
    fn skips_prolog_comments_and_doctype() {
        let src = "<?xml version=\"1.0\"?><!-- hi --><!DOCTYPE a [<!ENTITY e \"x\">]><a>ok</a>";
        assert_eq!(parse(src).unwrap().text(), "ok");
    }

    #[test]
    fn text_runs_are_merged() {
        let root = parse("<a>one<!--c-->two</a>").unwrap();
        let texts: Vec<&String> = root
            .children
            .iter()
            .filter_map(|n| if let Node::Text(t) = n { Some(t) } else { None })
            .collect();
        assert_eq!(texts, ["one", "two"]);
        assert_eq!(root.text(), "onetwo");
    }

    #[test]
    fn namespace_prefix_is_preserved_and_local_name_available() {
        let root = parse(r#"<ns:Doc xmlns:ns="urn:x"><ns:Body/></ns:Doc>"#).unwrap();
        assert_eq!(root.name, "ns:Doc");
        assert_eq!(local_name(&root.name), "Doc");
        assert_eq!(root.attr("xmlns:ns"), Some("urn:x"));
    }

    #[test]
    fn round_trip_preserves_structure() {
        let src = r#"<a x="1"><b>hi &amp; bye</b></a>"#;
        let once = parse(src).unwrap().to_string();
        let twice = parse(&once).unwrap().to_string();
        assert_eq!(once, twice);
    }

    #[test]
    fn escapes_on_output() {
        let mut e = Element::new("a");
        e.set_attr("k", "a\"b");
        e.push_text("x<y&z");
        assert_eq!(e.to_string(), r#"<a k="a&quot;b">x&lt;y&amp;z</a>"#);
    }

    #[test]
    fn mismatched_close_tag_is_an_error() {
        let err = parse("<a></b>").unwrap_err();
        assert!(err.message.contains("does not match"), "{}", err.message);
    }

    #[test]
    fn unterminated_element_is_an_error() {
        assert!(parse("<a>").is_err());
        assert!(parse("<a").is_err());
    }

    #[test]
    fn trailing_content_is_an_error() {
        assert!(parse("<a/><b/>").is_err());
    }

    #[test]
    fn unknown_entity_is_left_alone() {
        let root = parse("<a>&weird;</a>").unwrap();
        assert_eq!(root.text(), "&weird;");
    }
}
