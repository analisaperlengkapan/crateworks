#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod call;
pub mod envelope;
pub mod wsdl;

pub use call::Request;
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
    /// The WSDL has no operation with that name.
    UnknownOperation(String),
    /// The WSDL declares no service address, so there is nothing to call.
    NoEndpoint,
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
            Self::UnknownOperation(name) => write!(f, "the WSDL has no operation named {name}"),
            Self::NoEndpoint => f.write_str("the WSDL declares no service endpoint"),
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

/// Finds the first descendant whose local name matches.
pub(crate) fn find<'a>(element: &'a xmlkit::Element, name: &str) -> Option<&'a xmlkit::Element> {
    if xmlkit::local_name(&element.name) == name {
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
pub(crate) fn find_all<'a>(
    element: &'a xmlkit::Element,
    name: &str,
    out: &mut Vec<&'a xmlkit::Element>,
) {
    if xmlkit::local_name(&element.name) == name {
        out.push(element);
        return;
    }
    for child in &element.children {
        if let xmlkit::Node::Element(e) = child {
            find_all(e, name, out);
        }
    }
}

/// Finds a direct child by *local* name, ignoring any namespace prefix.
///
/// [`xmlkit::Element::child`] matches the qualified name, so it would
/// need `"soap:binding"` and would miss a document that binds the SOAP
/// namespace to a different prefix. A WSDL is written with whatever prefix the
/// author chose, so matching on the local name is the only portable option.
pub(crate) fn child_local<'a>(
    element: &'a xmlkit::Element,
    name: &str,
) -> Option<&'a xmlkit::Element> {
    element.children.iter().find_map(|n| match n {
        xmlkit::Node::Element(e) if xmlkit::local_name(&e.name) == name => Some(e),
        _ => None,
    })
}

/// The namespace part of a prefixed name, resolved against the `xmlns` bindings
/// declared on the element and its ancestors.
///
/// A full namespace resolver needs the whole ancestor chain, so this looks at
/// the element's own `xmlns:*` attributes, which is where the declaration
/// normally sits on a SOAP message.
pub(crate) fn namespace_of(element: &xmlkit::Element) -> Option<String> {
    let (prefix, _) = element.name.split_once(':')?;
    element.attr(&format!("xmlns:{prefix}")).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xmlkit::parse;

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
