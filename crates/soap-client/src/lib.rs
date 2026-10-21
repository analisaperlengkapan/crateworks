//! SOAP envelopes and WSDL reading.
//!
//! SOAP is the XML web service protocol that predates REST, and it is still the
//! only interface many banks, government systems and enterprise products offer.
//! This crate covers the parts an integrator needs most: building a request,
//! reading a response, and finding out from a WSDL document which operations
//! exist and what they expect.
//!
//! # Example
//!
//! ```
//! use soap_client::{Envelope, Version};
//!
//! let request = Envelope::request(Version::Soap11)
//!     .with_body_xml("<GetPrice xmlns=\"urn:shop\"><Sku>ABC</Sku></GetPrice>")
//!     .to_xml();
//!
//! assert!(request.contains("soap:Envelope"));
//! assert!(request.contains("GetPrice"));
//!
//! let response = r#"<?xml version="1.0"?>
//! <soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
//!   <soap:Body><GetPriceResponse xmlns="urn:shop"><Price>9.99</Price></GetPriceResponse></soap:Body>
//! </soap:Envelope>"#;
//!
//! let envelope = Envelope::parse(response).unwrap();
//! assert!(!envelope.is_fault());
//! assert!(envelope.body_text().contains("9.99"));
//! ```
//!
//! # Scope
//!
//! This crate builds and reads messages and describes a service from its WSDL.
//! It does not perform the HTTP call, does not implement WS-Security, and does
//! not generate typed client code. Those are deliberate omissions: the HTTP
//! layer belongs to the caller's chosen client, and WS-Security needs a
//! certificate story that should not be guessed at.

#![forbid(unsafe_code)]

pub mod envelope;
pub mod wsdl;

pub use envelope::{Envelope, Fault, Version};
pub use wsdl::{Operation, PortType, Service, Wsdl};

use std::fmt::{self, Display};

/// A failure while reading a SOAP message or a WSDL document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The document is not well-formed XML.
    Xml(String),
    /// The document is not a SOAP envelope.
    NotASoapEnvelope,
    /// The envelope has no `Body`.
    MissingBody,
    /// A required element is absent.
    MissingElement(&'static str),
    /// A namespace is absent or does not match what SOAP requires.
    WrongNamespace {
        /// Element being checked.
        element: &'static str,
        /// Namespace that was found.
        found: String,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Xml(m) => write!(f, "malformed XML: {m}"),
            Self::NotASoapEnvelope => f.write_str("document is not a SOAP envelope"),
            Self::MissingBody => f.write_str("envelope has no Body element"),
            Self::MissingElement(name) => write!(f, "missing required element {name}"),
            Self::WrongNamespace { element, found } => {
                write!(
                    f,
                    "{element} is in namespace {found:?}, which is not the SOAP namespace"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<crateworks_xml::Error> for Error {
    fn from(e: crateworks_xml::Error) -> Self {
        Self::Xml(e.to_string())
    }
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Finds the first descendant whose local name matches.
pub(crate) fn find<'a>(
    element: &'a crateworks_xml::Element,
    name: &str,
) -> Option<&'a crateworks_xml::Element> {
    if crateworks_xml::local_name(&element.name) == name {
        return Some(element);
    }
    for child in &element.children {
        if let crateworks_xml::Node::Element(e) = child
            && let Some(found) = find(e, name)
        {
            return Some(found);
        }
    }
    None
}

/// Collects every descendant whose local name matches, in document order.
pub(crate) fn find_all<'a>(
    element: &'a crateworks_xml::Element,
    name: &str,
    out: &mut Vec<&'a crateworks_xml::Element>,
) {
    if crateworks_xml::local_name(&element.name) == name {
        out.push(element);
        return;
    }
    for child in &element.children {
        if let crateworks_xml::Node::Element(e) = child {
            find_all(e, name, out);
        }
    }
}

/// Finds a direct child by *local* name, ignoring any namespace prefix.
///
/// [`crateworks_xml::Element::child`] matches the qualified name, so it would
/// need `"soap:binding"` and would miss a document that binds the SOAP
/// namespace to a different prefix. A WSDL is written with whatever prefix the
/// author chose, so matching on the local name is the only portable option.
pub(crate) fn child_local<'a>(
    element: &'a crateworks_xml::Element,
    name: &str,
) -> Option<&'a crateworks_xml::Element> {
    element.children.iter().find_map(|n| match n {
        crateworks_xml::Node::Element(e) if crateworks_xml::local_name(&e.name) == name => Some(e),
        _ => None,
    })
}

/// The namespace part of a prefixed name, resolved against the `xmlns` bindings
/// declared on the element and its ancestors.
///
/// A full namespace resolver needs the whole ancestor chain, so this looks at
/// the element's own `xmlns:*` attributes, which is where the declaration
/// normally sits on a SOAP message.
pub(crate) fn namespace_of(element: &crateworks_xml::Element) -> Option<String> {
    let (prefix, _) = element.name.split_once(':')?;
    element.attr(&format!("xmlns:{prefix}")).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crateworks_xml::parse;

    #[test]
    fn find_and_find_all_ignore_prefixes() {
        let root = parse("<ns:A xmlns:ns=\"urn:x\"><ns:B>1</ns:B><ns:B>2</ns:B></ns:A>").unwrap();
        assert_eq!(find(&root, "B").unwrap().text(), "1");
        let mut all = Vec::new();
        find_all(&root, "B", &mut all);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn namespace_of_reads_the_prefix_binding() {
        let root =
            parse("<soap:Envelope xmlns:soap=\"http://schemas.xmlsoap.org/soap/envelope/\"/>")
                .unwrap();
        assert_eq!(
            namespace_of(&root).as_deref(),
            Some("http://schemas.xmlsoap.org/soap/envelope/")
        );
    }

    #[test]
    fn namespace_of_is_none_without_a_prefix() {
        let root = parse("<Envelope/>").unwrap();
        assert!(namespace_of(&root).is_none());
    }
}
