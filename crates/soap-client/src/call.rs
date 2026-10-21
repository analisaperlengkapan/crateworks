//! Turning a WSDL operation into a request that is ready to send.
//!
//! Reading a WSDL gives an endpoint and an operation, and building an envelope
//! gives XML. This module joins the two: it looks the operation up, wraps the
//! caller's payload in the operation's input element, and reports the headers
//! the call needs.

use crate::envelope::{Envelope, Version};
use crate::wsdl::Wsdl;
use crate::{Error, Result};
use crateworks_xml::{escape_attr, escape_text};

/// A SOAP call, ready for an HTTP client to POST.
///
/// The crate does not send this itself. Hand the parts to whichever HTTP
/// client the caller already uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The URL to POST to.
    pub url: String,
    /// The `Content-Type` header. A server that expects one SOAP version will
    /// reject the other, so this follows the version in use.
    pub content_type: &'static str,
    /// The `SOAPAction` value, when the WSDL declared one.
    pub soap_action: Option<String>,
    /// The request envelope XML.
    pub body: String,
}

impl Request {
    /// The `SOAPAction` header value, quoted as SOAP 1.1 requires.
    ///
    /// The value must be wrapped in double quotes, and an operation with no
    /// declared action still needs the header, so that case yields an empty
    /// quoted string.
    #[must_use]
    pub fn soap_action_header(&self) -> Option<String> {
        self.soap_action.as_ref().map(|a| format!("\"{a}\""))
    }
}

impl Wsdl {
    /// Builds a request for an operation, wrapping `payload` in the operation's
    /// input element.
    ///
    /// `payload` is raw XML for the *content* of the input element, not the
    /// element itself. A document-style call puts a tree of child elements
    /// there, so it is inserted as written. Use [`Wsdl::request_text`] when the
    /// content is a single text value that needs escaping.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownOperation`] when the WSDL has no such operation,
    /// [`Error::NoEndpoint`] when it declares no service address, and
    /// [`Error::MissingElement`] when the operation names no input element, so
    /// the payload has nowhere to go.
    pub fn request(&self, operation: &str, payload: &str) -> Result<Request> {
        self.build(operation, payload)
    }

    /// Builds a request whose input element holds a single escaped text value.
    ///
    /// # Errors
    ///
    /// The same errors as [`Wsdl::request`].
    pub fn request_text(&self, operation: &str, value: &str) -> Result<Request> {
        self.build(operation, &escape_text(value))
    }

    fn build(&self, operation: &str, content: &str) -> Result<Request> {
        let op = self
            .operation(operation)
            .ok_or_else(|| Error::UnknownOperation(operation.to_string()))?;
        let url = self.endpoint().ok_or(Error::NoEndpoint)?.to_string();
        let element = op.input.as_deref().ok_or(Error::MissingElement("input"))?;

        // The input element lives in the definitions target namespace, which is
        // what a document-style body is validated against.
        let open = match self.target_namespace.as_deref() {
            Some(ns) if !ns.is_empty() => {
                format!("<{element} xmlns=\"{}\">", escape_attr(ns))
            }
            _ => format!("<{element}>"),
        };
        let body = format!("{open}{content}</{element}>");

        Ok(Request {
            url,
            content_type: Version::Soap11.content_type(),
            soap_action: op.soap_action.clone(),
            body: Envelope::request(Version::Soap11)
                .with_body_xml(&body)
                .to_xml(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WSDL: &str = r#"<definitions targetNamespace="urn:shop"
            xmlns="http://schemas.xmlsoap.org/wsdl/"
            xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/">
      <message name="GetPriceRequest">
        <part name="parameters" element="tns:GetPrice"/>
      </message>
      <portType name="PricePortType">
        <operation name="GetPrice">
          <input message="tns:GetPriceRequest"/>
        </operation>
      </portType>
      <binding name="PriceBinding" type="tns:PricePortType">
        <soap:binding transport="http://schemas.xmlsoap.org/soap/http"/>
        <operation name="GetPrice">
          <soap:operation soapAction="urn:shop/GetPrice"/>
        </operation>
      </binding>
      <service name="PriceService">
        <port name="PricePort" binding="tns:PriceBinding">
          <soap:address location="https://api.example.com/price"/>
        </port>
      </service>
    </definitions>"#;

    #[test]
    fn builds_a_request_from_the_wsdl() {
        let w = Wsdl::parse(WSDL).unwrap();
        let req = w.request("GetPrice", "<Sku>ABC</Sku>").unwrap();
        assert_eq!(req.url, "https://api.example.com/price");
        assert_eq!(req.content_type, "text/xml; charset=utf-8");
        assert!(req.body.contains("soap:Envelope"));
        assert!(req.body.contains(r#"<GetPrice xmlns="urn:shop">"#));
        assert!(req.body.contains("<Sku>ABC</Sku>"));
    }

    #[test]
    fn soap_action_is_quoted_for_the_header() {
        let w = Wsdl::parse(WSDL).unwrap();
        let req = w.request("GetPrice", "").unwrap();
        assert_eq!(
            req.soap_action_header().as_deref(),
            Some("\"urn:shop/GetPrice\"")
        );
    }

    #[test]
    fn a_payload_is_escaped_when_it_is_text() {
        let w = Wsdl::parse(WSDL).unwrap();
        let req = w.request_text("GetPrice", "</GetPrice><Evil/>").unwrap();
        assert!(!req.body.contains("<Evil/>"));
        assert!(req.body.contains("&lt;Evil/&gt;"));
    }

    #[test]
    fn a_raw_payload_is_inserted_as_xml() {
        let w = Wsdl::parse(WSDL).unwrap();
        let req = w.request("GetPrice", "<Sku>ABC</Sku><Qty>2</Qty>").unwrap();
        assert!(req.body.contains("<Sku>ABC</Sku><Qty>2</Qty>"));
        assert!(req.body.ends_with("</soap:Envelope>"));
    }

    #[test]
    fn an_unknown_operation_is_reported() {
        let w = Wsdl::parse(WSDL).unwrap();
        assert_eq!(
            w.request("Nope", ""),
            Err(Error::UnknownOperation("Nope".to_string()))
        );
    }

    #[test]
    fn a_wsdl_without_an_endpoint_is_reported() {
        let xml = r#"<definitions targetNamespace="urn:x"
                xmlns="http://schemas.xmlsoap.org/wsdl/">
            <portType name="P"><operation name="Ping"><input message="m"/></operation></portType>
        </definitions>"#;
        let w = Wsdl::parse(xml).unwrap();
        assert_eq!(w.request("Ping", ""), Err(Error::NoEndpoint));
    }
}
