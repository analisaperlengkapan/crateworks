//! ISO 20022 payment messages.
//!
//! ISO 20022 is the XML message standard that is replacing the older SWIFT MT
//! formats. It is organised into business areas whose message names carry a
//! three-letter prefix:
//!
//! * `pain` - payment initiation, sent by a customer to their bank.
//! * `pacs` - payment clearing and settlement, sent between banks.
//! * `camt` - cash management, including account statements.
//!
//! # Example
//!
//! ```
//! use iso20022_rs::camt::BankStatement;
//!
//! let xml = r#"<?xml version="1.0"?>
//! <Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.08">
//!   <BkToCstmrStmt>
//!     <Stmt>
//!       <Id>STMT-001</Id>
//!       <Acct><Id><IBAN>DE89370400440532013000</IBAN></Id></Acct>
//!       <Bal><Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp><Amt Ccy="EUR">1000.00</Amt></Bal>
//!       <Ntry><Amt Ccy="EUR">250.00</Amt><CdtDbtInd>CRDT</CdtDbtInd></Ntry>
//!     </Stmt>
//!   </BkToCstmrStmt>
//! </Document>"#;
//!
//! let statement = BankStatement::parse(xml).unwrap();
//! assert_eq!(statement.id, "STMT-001");
//! assert_eq!(statement.iban.as_deref(), Some("DE89370400440532013000"));
//! assert_eq!(statement.opening_balance.as_ref().unwrap().amount, 1000.0);
//! assert_eq!(statement.entries.len(), 1);
//! assert_eq!(statement.closing_balance(), 1250.0);
//! ```
//!
//! # Scope
//!
//! Each message is modelled with the fields an application actually needs,
//! rather than the full generated schema. Unknown elements are ignored, so a
//! document from a newer schema version still parses. Namespace URIs are
//! recorded but not resolved.

#![forbid(unsafe_code)]

pub mod amount;
pub mod camt;
pub mod pain;

pub use amount::{Amount, Currency};

use std::fmt::{self, Display};

use xmlkit::{Element, local_name};

/// A failure while reading a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The document is not well-formed XML.
    Xml(String),
    /// A required element is absent.
    MissingElement(&'static str),
    /// An element is present but its content is not usable.
    InvalidValue {
        /// The element that could not be read.
        element: &'static str,
        /// What was found.
        found: String,
    },
    /// The document root does not belong to a supported message.
    UnsupportedMessage(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Xml(m) => write!(f, "malformed XML: {m}"),
            Self::MissingElement(name) => write!(f, "missing required element {name}"),
            Self::InvalidValue { element, found } => {
                write!(f, "{element} has an unusable value: {found}")
            }
            Self::UnsupportedMessage(m) => write!(f, "unsupported message type {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<xmlkit::Error> for Error {
    fn from(e: xmlkit::Error) -> Self {
        Self::Xml(e.to_string())
    }
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Finds the first descendant whose local name matches, ignoring the namespace
/// prefix.
pub(crate) fn find<'a>(element: &'a Element, name: &str) -> Option<&'a Element> {
    if local_name(&element.name) == name {
        return Some(element);
    }
    for child in &element.children {
        if let xmlkit::Node::Element(e) = child
            && let Some(found) = find(e, name)
        {
            return Some(found);
        }
    }
    None
}

/// Collects every descendant whose local name matches, in document order.
pub(crate) fn find_all<'a>(element: &'a Element, name: &str, out: &mut Vec<&'a Element>) {
    if local_name(&element.name) == name {
        out.push(element);
        // A message never nests the same element type, so do not descend once
        // a match is found.
        return;
    }
    for child in &element.children {
        if let xmlkit::Node::Element(e) = child {
            find_all(e, name, out);
        }
    }
}

/// Reads the text of a required child.
pub(crate) fn required_text(element: &Element, name: &'static str) -> Result<String> {
    find(element, name)
        .map(Element::deep_text)
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or(Error::MissingElement(name))
}

/// Reads the text of an optional child.
pub(crate) fn optional_text(element: &Element, name: &str) -> Option<String> {
    find(element, name)
        .map(Element::deep_text)
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// Parses the `Document` wrapper and returns its single business message child.
///
/// # Errors
///
/// Returns [`Error::Xml`] for malformed input or [`Error::MissingElement`] when
/// the document has no body.
pub(crate) fn document_body(xml: &str) -> Result<Element> {
    let root = xmlkit::parse(xml)?;
    if local_name(&root.name) != "Document" {
        return Err(Error::UnsupportedMessage(root.name));
    }
    root.children
        .iter()
        .find_map(|n| match n {
            xmlkit::Node::Element(e) => Some(e.clone()),
            _ => None,
        })
        .ok_or(Error::MissingElement("Document body"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_ignores_namespace_prefixes() {
        let root = xmlkit::parse(
            r#"<ns:Doc xmlns:ns="urn:x"><ns:Body><ns:Id>7</ns:Id></ns:Body></ns:Doc>"#,
        )
        .unwrap();
        assert!(find(&root, "Body").is_some());
        assert_eq!(required_text(&root, "Id").unwrap(), "7");
    }

    #[test]
    fn find_all_collects_in_order() {
        let root = xmlkit::parse("<r><a>1</a><b><a>2</a></b><a>3</a></r>").unwrap();
        let mut found = Vec::new();
        find_all(&root, "a", &mut found);
        let texts: Vec<String> = found.iter().map(|e| e.deep_text()).collect();
        assert_eq!(texts, ["1", "2", "3"]);
    }

    #[test]
    fn required_text_reports_missing_and_empty() {
        let root = xmlkit::parse("<r><a>  </a></r>").unwrap();
        assert_eq!(required_text(&root, "a"), Err(Error::MissingElement("a")));
        assert_eq!(required_text(&root, "b"), Err(Error::MissingElement("b")));
    }

    #[test]
    fn document_body_requires_a_document_root() {
        assert!(matches!(
            document_body("<NotDoc/>"),
            Err(Error::UnsupportedMessage(_))
        ));
        assert!(matches!(
            document_body("<Document/>"),
            Err(Error::MissingElement(_))
        ));
    }
}
