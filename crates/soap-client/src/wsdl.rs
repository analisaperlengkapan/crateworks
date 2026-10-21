//! WSDL reading.
//!
//! A WSDL document describes a SOAP service: where it lives, which operations
//! it exposes, and the message types those operations use. This module extracts
//! the part an integrator needs in order to call the service: the endpoint URL,
//! and per operation the input and output element names.

use crate::{Error, Result, child_local, find, find_all};
use std::collections::BTreeMap;
use xmlkit::{Element, local_name};

/// One operation exposed by a service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// Operation name.
    pub name: String,
    /// Name of the element the request body should contain.
    ///
    /// This is the element named by the input message's `part`, which is what a
    /// document-style call must wrap. When a message names only a type, as
    /// RPC-style services do, the message name is reported instead.
    pub input: Option<String>,
    /// Name of the element the response body will contain.
    pub output: Option<String>,
    /// The `soapAction` header value, taken from the binding when the port type
    /// does not carry one inline.
    pub soap_action: Option<String>,
}

/// A port type, which groups operations. In WSDL 2 these are called interfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortType {
    /// Port type name.
    pub name: String,
    /// The operations it declares.
    pub operations: Vec<Operation>,
}

/// A service endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    /// Service name.
    pub name: String,
    /// The endpoint address, which is the URL to POST to.
    pub address: String,
    /// The transport binding URI, usually HTTP.
    pub transport: Option<String>,
}

/// A parsed WSDL document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Wsdl {
    /// The target namespace the service is defined in.
    pub target_namespace: Option<String>,
    /// Port types and their operations.
    pub port_types: Vec<PortType>,
    /// Service endpoints.
    pub services: Vec<Service>,
}

impl Wsdl {
    /// Parses a WSDL document.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] for malformed input or
    /// [`Error::MissingElement`] when the root is not a `definitions`
    /// element.
    pub fn parse(xml: &str) -> Result<Self> {
        let root = xmlkit::parse(xml)?;
        if local_name(&root.name) != "definitions" {
            return Err(Error::MissingElement("definitions"));
        }
        let target_namespace = root.attr("targetNamespace").map(str::to_string);

        // A message's `part` names the element that appears in the body, so the
        // messages are indexed first and the operations resolved against them.
        let mut messages: BTreeMap<String, String> = BTreeMap::new();
        let mut message_elements = Vec::new();
        find_all(&root, "message", &mut message_elements);
        for message in message_elements {
            let Some(name) = message.attr("name") else {
                continue;
            };
            if let Some(part) = message
                .children
                .iter()
                .filter_map(as_element)
                .find(|e| local_name(&e.name) == "part")
                && let Some(element) = part.attr("element")
            {
                messages.insert(name.to_string(), strip_prefix(element));
            }
        }

        // The `soapAction` usually lives on the binding operation, so the
        // bindings are indexed by operation name before the port types are read.
        let mut actions: BTreeMap<String, String> = BTreeMap::new();
        let mut binding_elements = Vec::new();
        find_all(&root, "binding", &mut binding_elements);
        for binding in binding_elements {
            for op in binding.children.iter().filter_map(as_element) {
                if local_name(&op.name) != "operation" {
                    continue;
                }
                if let Some(action) =
                    child_local(op, "operation").and_then(|e| e.attr("soapAction"))
                    && let Some(name) = op.attr("name")
                {
                    actions.insert(name.to_string(), action.to_string());
                }
            }
        }

        let mut port_type_elements = Vec::new();
        find_all(&root, "portType", &mut port_type_elements);
        // WSDL 2 names the same construct "interface".
        find_all(&root, "interface", &mut port_type_elements);
        let mut port_types = Vec::with_capacity(port_type_elements.len());
        for pt in port_type_elements {
            let name = pt.attr("name").unwrap_or_default().to_string();
            let mut operations = Vec::new();
            for op in pt.children.iter().filter_map(as_element) {
                if local_name(&op.name) != "operation" {
                    continue;
                }
                let op_name = op.attr("name").unwrap_or_default().to_string();
                // The `message` attribute names a message; the element inside
                // the body is that message's part element.
                let resolve = |reference: &str| -> String {
                    let local = strip_prefix(reference);
                    messages.get(&local).cloned().unwrap_or(local)
                };
                let input = op
                    .attr("input")
                    .or_else(|| find(op, "input").and_then(|e| e.attr("message")))
                    .map(resolve);
                let output = op
                    .attr("output")
                    .or_else(|| find(op, "output").and_then(|e| e.attr("message")))
                    .map(resolve);
                let soap_action = child_local(op, "operation")
                    .and_then(|e| e.attr("soapAction"))
                    .map(str::to_string)
                    .or_else(|| actions.get(&op_name).cloned());
                operations.push(Operation {
                    name: op_name,
                    input,
                    output,
                    soap_action,
                });
            }
            port_types.push(PortType { name, operations });
        }

        let mut services = Vec::new();
        let mut service_elements = Vec::new();
        find_all(&root, "service", &mut service_elements);
        for svc in service_elements {
            let name = svc.attr("name").unwrap_or_default().to_string();
            let mut port_elements = Vec::new();
            find_all(svc, "port", &mut port_elements);
            find_all(svc, "endpoint", &mut port_elements);
            for port in port_elements {
                let Some(address) =
                    child_local(port, "address").or_else(|| child_local(port, "endpoint"))
                else {
                    continue;
                };
                let location = address
                    .attr("location")
                    .or_else(|| address.attr("address"))
                    .unwrap_or_default()
                    .to_string();
                if location.is_empty() {
                    continue;
                }
                // The port names its binding in an attribute; the transport
                // lives on the matching <binding> element at the top level.
                let transport = port
                    .attr("binding")
                    .map(strip_prefix)
                    .and_then(|name| {
                        let mut bindings = Vec::new();
                        find_all(&root, "binding", &mut bindings);
                        bindings
                            .into_iter()
                            .find(|b| b.attr("name") == Some(name.as_str()))
                    })
                    .and_then(|b| child_local(b, "binding"))
                    .and_then(|soap_binding| soap_binding.attr("transport"))
                    .map(str::to_string);
                services.push(Service {
                    name: name.clone(),
                    address: location,
                    transport,
                });
            }
        }

        Ok(Self {
            target_namespace,
            port_types,
            services,
        })
    }

    /// The first endpoint address, which is what a caller usually wants.
    #[must_use]
    pub fn endpoint(&self) -> Option<&str> {
        self.services.first().map(|s| s.address.as_str())
    }

    /// Looks up an operation by name across every port type.
    #[must_use]
    pub fn operation(&self, name: &str) -> Option<&Operation> {
        self.port_types
            .iter()
            .flat_map(|p| p.operations.iter())
            .find(|o| o.name == name)
    }

    /// Every operation across every port type.
    #[must_use]
    pub fn operations(&self) -> Vec<&Operation> {
        self.port_types
            .iter()
            .flat_map(|p| p.operations.iter())
            .collect()
    }
}

fn as_element(node: &xmlkit::Node) -> Option<&Element> {
    match node {
        xmlkit::Node::Element(e) => Some(e),
        _ => None,
    }
}

/// Strips the `tns:` style prefix from a message reference, leaving the local
/// name.
fn strip_prefix(reference: &str) -> String {
    reference
        .rsplit_once(':')
        .map_or(reference, |(_, local)| local)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WSDL: &str = r#"<?xml version="1.0"?>
<definitions name="PriceService"
             targetNamespace="urn:shop"
             xmlns="http://schemas.xmlsoap.org/wsdl/"
             xmlns:tns="urn:shop"
             xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/">
  <message name="GetPriceRequest">
    <part name="parameters" element="tns:GetPrice"/>
  </message>
  <message name="GetPriceResponse">
    <part name="parameters" element="tns:GetPriceResponse"/>
  </message>
  <portType name="PricePortType">
    <operation name="GetPrice">
      <input message="tns:GetPriceRequest"/>
      <output message="tns:GetPriceResponse"/>
    </operation>
    <operation name="ListPrices">
      <input message="tns:ListPricesRequest"/>
      <output message="tns:ListPricesResponse"/>
    </operation>
  </portType>
  <binding name="PriceBinding" type="tns:PricePortType">
    <soap:binding style="document" transport="http://schemas.xmlsoap.org/soap/http"/>
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
    fn parses_target_namespace_and_endpoint() {
        let w = Wsdl::parse(WSDL).unwrap();
        assert_eq!(w.target_namespace.as_deref(), Some("urn:shop"));
        assert_eq!(w.endpoint(), Some("https://api.example.com/price"));
        assert_eq!(w.services[0].name, "PriceService");
    }

    #[test]
    fn parses_port_types_and_operations() {
        let w = Wsdl::parse(WSDL).unwrap();
        assert_eq!(w.port_types.len(), 1);
        let pt = &w.port_types[0];
        assert_eq!(pt.name, "PricePortType");
        assert_eq!(pt.operations.len(), 2);
        assert_eq!(pt.operations[0].name, "GetPrice");
        // The message is resolved to the element that goes in the body.
        assert_eq!(pt.operations[0].input.as_deref(), Some("GetPrice"));
        assert_eq!(pt.operations[0].output.as_deref(), Some("GetPriceResponse"));
    }

    #[test]
    fn finds_an_operation_by_name() {
        let w = Wsdl::parse(WSDL).unwrap();
        let op = w.operation("ListPrices").unwrap();
        // This WSDL never defines the ListPrices messages, so the message name
        // itself is the fallback answer.
        assert_eq!(op.input.as_deref(), Some("ListPricesRequest"));
        assert!(w.operation("DoesNotExist").is_none());
        assert_eq!(w.operations().len(), 2);
    }

    #[test]
    fn soap_action_comes_from_the_binding() {
        let w = Wsdl::parse(WSDL).unwrap();
        assert_eq!(
            w.operation("GetPrice").unwrap().soap_action.as_deref(),
            Some("urn:shop/GetPrice")
        );
        // An operation with no binding entry stays empty rather than guessed at.
        assert_eq!(w.operation("ListPrices").unwrap().soap_action, None);
    }

    #[test]
    fn falls_back_to_the_message_name_when_no_element_is_named() {
        // An RPC-style message names a type rather than an element, so the
        // message name is the best answer available.
        let xml = r#"<definitions xmlns="http://schemas.xmlsoap.org/wsdl/"
                xmlns:xsd="http://www.w3.org/2001/XMLSchema">
            <message name="AddRequest">
              <part name="a" type="xsd:int"/>
            </message>
            <portType name="Calc">
              <operation name="Add"><input message="AddRequest"/></operation>
            </portType>
        </definitions>"#;
        let w = Wsdl::parse(xml).unwrap();
        assert_eq!(
            w.operation("Add").unwrap().input.as_deref(),
            Some("AddRequest")
        );
    }

    #[test]
    fn transport_is_reported() {
        let w = Wsdl::parse(WSDL).unwrap();
        assert_eq!(
            w.services[0].transport.as_deref(),
            Some("http://schemas.xmlsoap.org/soap/http")
        );
    }

    #[test]
    fn rejects_a_document_that_is_not_wsdl() {
        assert_eq!(
            Wsdl::parse("<html/>"),
            Err(Error::MissingElement("definitions"))
        );
    }

    #[test]
    fn a_wsdl_without_a_service_still_parses_operations() {
        let xml = r#"<definitions targetNamespace="urn:x" xmlns="http://schemas.xmlsoap.org/wsdl/">
            <portType name="P"><operation name="Ping"/></portType>
        </definitions>"#;
        let w = Wsdl::parse(xml).unwrap();
        assert_eq!(w.operations().len(), 1);
        assert!(w.endpoint().is_none());
        assert_eq!(w.operations()[0].name, "Ping");
    }

    #[test]
    fn soap_action_is_read_when_declared_on_the_operation() {
        // A document that carries soapAction inline is read; when the action
        // lives only on a binding operation, the field stays empty rather than
        // being guessed at.
        let xml = r#"<definitions xmlns="http://schemas.xmlsoap.org/wsdl/">
            <portType name="P">
              <operation name="Ping">
                <soap:operation soapAction="urn:ping" xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"/>
              </operation>
            </portType>
        </definitions>"#;
        let w = Wsdl::parse(xml).unwrap();
        assert_eq!(
            w.operation("Ping").unwrap().soap_action.as_deref(),
            Some("urn:ping")
        );
    }
}
