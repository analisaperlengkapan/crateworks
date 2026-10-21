//! SOAP envelope construction and parsing, for SOAP 1.1 and 1.2.
//!
//! The two versions differ in their namespace URIs and in how a fault is
//! expressed, so [`Version`] selects the right one.

use crate::{Error, Result, find, namespace_of};
use std::fmt::Write as _;
use xmlkit::{Element, escape_attr, escape_text};

/// The SOAP protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Version {
    /// SOAP 1.1, namespace `http://schemas.xmlsoap.org/soap/envelope/`.
    #[default]
    Soap11,
    /// SOAP 1.2, namespace `http://www.w3.org/2003/05/soap-envelope`.
    Soap12,
}

impl Version {
    /// The envelope namespace URI.
    #[must_use]
    pub const fn namespace(self) -> &'static str {
        match self {
            Self::Soap11 => "http://schemas.xmlsoap.org/soap/envelope/",
            Self::Soap12 => "http://www.w3.org/2003/05/soap-envelope",
        }
    }

    /// The `Content-Type` a request should be sent with.
    ///
    /// SOAP 1.1 uses `text/xml` and SOAP 1.2 uses `application/soap+xml`, and a
    /// server that expects one will reject the other.
    #[must_use]
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Soap11 => "text/xml; charset=utf-8",
            Self::Soap12 => "application/soap+xml; charset=utf-8",
        }
    }

    /// Detects the version from an envelope's namespace URI.
    #[must_use]
    pub fn from_namespace(uri: &str) -> Option<Self> {
        match uri {
            "http://schemas.xmlsoap.org/soap/envelope/" => Some(Self::Soap11),
            "http://www.w3.org/2003/05/soap-envelope" => Some(Self::Soap12),
            _ => None,
        }
    }
}

/// A SOAP fault, the protocol's error report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// Machine-readable fault code.
    pub code: Option<String>,
    /// Human-readable description.
    pub string: Option<String>,
    /// The service that raised the fault, in SOAP 1.1.
    pub actor: Option<String>,
    /// Application-specific detail XML.
    pub detail: Option<String>,
}

impl Fault {
    /// Reads a `<Fault>` element in either version's shape.
    #[must_use]
    pub fn from_element(fault: &Element) -> Self {
        // SOAP 1.2 nests code and reason; SOAP 1.1 uses flat elements.
        let code = find(fault, "Value")
            .map(Element::deep_text)
            .or_else(|| find(fault, "faultcode").map(Element::deep_text))
            .map(|t| t.trim().to_string());
        let string = find(fault, "Text")
            .map(Element::deep_text)
            .or_else(|| find(fault, "faultstring").map(Element::deep_text))
            .map(|t| t.trim().to_string());
        let actor = find(fault, "Role")
            .map(Element::deep_text)
            .or_else(|| find(fault, "faultactor").map(Element::deep_text))
            .map(|t| t.trim().to_string());
        let detail = find(fault, "Detail")
            .or_else(|| find(fault, "detail"))
            .map(Element::to_string);
        Self {
            code,
            string,
            actor,
            detail,
        }
    }
}

/// A SOAP envelope: the header, if any, and the body.
#[derive(Debug, Clone, PartialEq)]
pub struct Envelope {
    /// Which SOAP version this envelope uses.
    pub version: Version,
    /// Raw XML of the header contents, if a header is present.
    pub header: Option<String>,
    /// Raw XML of the body contents.
    pub body: String,
    /// The fault, when the body carries one.
    pub fault: Option<Fault>,
}

impl Envelope {
    /// Starts building a request envelope.
    #[must_use]
    pub fn request(version: Version) -> Self {
        Self {
            version,
            header: None,
            body: String::new(),
            fault: None,
        }
    }

    /// Sets the body from a raw XML fragment.
    #[must_use]
    pub fn with_body_xml(mut self, xml: &str) -> Self {
        self.body = xml.trim().to_string();
        self
    }

    /// Sets the header from a raw XML fragment.
    #[must_use]
    pub fn with_header_xml(mut self, xml: &str) -> Self {
        self.header = Some(xml.trim().to_string());
        self
    }

    /// Wraps a named element with a single text value, a common request shape.
    #[must_use]
    pub fn with_body_element(mut self, name: &str, value: &str) -> Self {
        self.body = format!("<{name}>{}</{name}>", escape_text(value));
        self
    }

    /// Serialises the envelope.
    #[must_use]
    pub fn to_xml(&self) -> String {
        let ns = self.version.namespace();
        let mut out = String::with_capacity(256 + self.body.len());
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        let _ = writeln!(out, "<soap:Envelope xmlns:soap=\"{}\">", escape_attr(ns));
        if let Some(header) = &self.header {
            out.push_str("  <soap:Header>");
            out.push_str(header);
            out.push_str("</soap:Header>\n");
        }
        out.push_str("  <soap:Body>");
        out.push_str(&self.body);
        out.push_str("</soap:Body>\n");
        out.push_str("</soap:Envelope>");
        out
    }

    /// Parses an envelope from a document.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] for malformed input, [`Error::NotASoapEnvelope`]
    /// when the root is not an `Envelope` in a SOAP namespace, or
    /// [`Error::MissingBody`] when the body is absent.
    pub fn parse(xml: &str) -> Result<Self> {
        let root = xmlkit::parse(xml)?;
        if xmlkit::local_name(&root.name) != "Envelope" {
            return Err(Error::NotASoapEnvelope);
        }
        let namespace = namespace_of(&root).unwrap_or_default();
        let version = Version::from_namespace(&namespace).ok_or_else(|| Error::WrongNamespace {
            element: "Envelope",
            found: namespace.clone(),
        })?;

        let header = find(&root, "Header").map(Element::to_string);
        let body_element = find(&root, "Body").ok_or(Error::MissingBody)?;
        let fault = find(body_element, "Fault").map(Fault::from_element);
        // The body's children are kept as raw XML so the caller can hand them to
        // whatever XML layer it already uses.
        let body = body_element
            .children
            .iter()
            .filter_map(|n| match n {
                xmlkit::Node::Element(e) => Some(e.to_string()),
                _ => None,
            })
            .collect::<String>();
        Ok(Self {
            version,
            header,
            body,
            fault,
        })
    }

    /// Whether the response carries a fault.
    #[must_use]
    pub const fn is_fault(&self) -> bool {
        self.fault.is_some()
    }

    /// The raw XML of the body.
    #[must_use]
    pub fn body_text(&self) -> &str {
        &self.body
    }

    /// The fault, if any.
    #[must_use]
    pub const fn fault(&self) -> Option<&Fault> {
        self.fault.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE_11: &str = r#"<?xml version="1.0"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <GetPriceResponse xmlns="urn:shop"><Price>9.99</Price></GetPriceResponse>
  </soap:Body>
</soap:Envelope>"#;

    const FAULT_11: &str = r#"<?xml version="1.0"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body>
    <soap:Fault>
      <faultcode>soap:Client</faultcode>
      <faultstring>Invalid SKU</faultstring>
      <faultactor>urn:shop</faultactor>
    </soap:Fault>
  </soap:Body>
</soap:Envelope>"#;

    const FAULT_12: &str = r#"<?xml version="1.0"?>
<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope">
  <env:Body>
    <env:Fault>
      <env:Code><env:Value>env:Sender</env:Value></env:Code>
      <env:Reason><env:Text xml:lang="en">Invalid SKU</env:Text></env:Reason>
    </env:Fault>
  </env:Body>
</env:Envelope>"#;

    #[test]
    fn builds_a_soap11_request() {
        let xml = Envelope::request(Version::Soap11)
            .with_body_xml("<GetPrice xmlns=\"urn:shop\"><Sku>ABC</Sku></GetPrice>")
            .to_xml();
        assert!(xml.contains("http://schemas.xmlsoap.org/soap/envelope/"));
        assert!(xml.contains("<soap:Body><GetPrice"));
        assert!(xml.contains("</soap:Envelope>"));
    }

    #[test]
    fn builds_a_soap12_request_with_the_right_namespace() {
        let xml = Envelope::request(Version::Soap12)
            .with_body_element("Ping", "1")
            .to_xml();
        assert!(xml.contains("http://www.w3.org/2003/05/soap-envelope"));
        assert!(xml.contains("<Ping>1</Ping>"));
    }

    #[test]
    fn header_is_included_when_set() {
        let xml = Envelope::request(Version::Soap11)
            .with_header_xml("<Auth><Token>abc</Token></Auth>")
            .with_body_element("Ping", "1")
            .to_xml();
        assert!(xml.contains("<soap:Header><Auth>"));
    }

    #[test]
    fn body_element_escapes_its_value() {
        let xml = Envelope::request(Version::Soap11)
            .with_body_element("N", "a<b&c")
            .to_xml();
        assert!(xml.contains("<N>a&lt;b&amp;c</N>"));
    }

    #[test]
    fn parses_a_soap11_response() {
        let e = Envelope::parse(RESPONSE_11).unwrap();
        assert_eq!(e.version, Version::Soap11);
        assert!(!e.is_fault());
        assert!(e.body_text().contains("9.99"));
        assert!(e.body_text().contains("GetPriceResponse"));
    }

    #[test]
    fn parses_a_soap11_fault() {
        let e = Envelope::parse(FAULT_11).unwrap();
        assert!(e.is_fault());
        let f = e.fault().unwrap();
        assert_eq!(f.code.as_deref(), Some("soap:Client"));
        assert_eq!(f.string.as_deref(), Some("Invalid SKU"));
        assert_eq!(f.actor.as_deref(), Some("urn:shop"));
    }

    #[test]
    fn parses_a_soap12_fault() {
        let e = Envelope::parse(FAULT_12).unwrap();
        assert_eq!(e.version, Version::Soap12);
        assert!(e.is_fault());
        let f = e.fault().unwrap();
        assert_eq!(f.code.as_deref(), Some("env:Sender"));
        assert_eq!(f.string.as_deref(), Some("Invalid SKU"));
    }

    #[test]
    fn rejects_a_non_envelope_root() {
        assert_eq!(Envelope::parse("<Response/>"), Err(Error::NotASoapEnvelope));
    }

    #[test]
    fn rejects_an_envelope_in_the_wrong_namespace() {
        let xml = r#"<soap:Envelope xmlns:soap="urn:not-soap"><soap:Body/></soap:Envelope>"#;
        assert!(matches!(
            Envelope::parse(xml),
            Err(Error::WrongNamespace { .. })
        ));
    }

    #[test]
    fn rejects_an_envelope_without_a_body() {
        let xml = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"/>"#;
        assert_eq!(Envelope::parse(xml), Err(Error::MissingBody));
    }

    #[test]
    fn content_types_differ_by_version() {
        assert!(Version::Soap11.content_type().starts_with("text/xml"));
        assert!(
            Version::Soap12
                .content_type()
                .starts_with("application/soap+xml")
        );
    }

    #[test]
    fn round_trip_request_through_parse() {
        let built = Envelope::request(Version::Soap11)
            .with_body_xml("<GetPrice xmlns=\"urn:shop\"><Sku>ABC</Sku></GetPrice>")
            .to_xml();
        let parsed = Envelope::parse(&built).unwrap();
        assert_eq!(parsed.version, Version::Soap11);
        assert!(parsed.body_text().contains("<Sku>ABC</Sku>"));
    }
}
