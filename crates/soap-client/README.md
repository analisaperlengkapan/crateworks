# soap-client

SOAP 1.1 and 1.2 envelopes, plus WSDL reading.

```rust
use soap_client::{Envelope, Version, Wsdl};

let request = Envelope::request(Version::Soap11)
    .with_body_xml("<Ping xmlns=\"urn:x\">1</Ping>")
    .to_xml();
assert!(request.contains("soap:Envelope"));

let response = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body><Pong xmlns="urn:x">1</Pong></soap:Body>
</soap:Envelope>"#;
assert!(Envelope::parse(response).unwrap().body_text().contains("Pong"));

// Go from a WSDL straight to a request an HTTP client can POST.
let wsdl = Wsdl::parse(r#"<definitions targetNamespace="urn:x"
        xmlns="http://schemas.xmlsoap.org/wsdl/">
    <message name="PingRequest"><part element="tns:Ping"/></message>
    <portType name="P"><operation name="Ping"><input message="tns:PingRequest"/></operation></portType>
    <service name="S"><port name="p"><soap:address
        xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
        location="https://example.com/x"/></port></service>
</definitions>"#).unwrap();

let call = wsdl.request("Ping", "<Value>1</Value>").unwrap();
assert_eq!(call.url, "https://example.com/x");
assert_eq!(call.content_type, "text/xml; charset=utf-8");
```

Build requests, parse responses, read faults in either version's shape, turn a
WSDL operation into a ready-to-send request, and extract a service's endpoint and
operations. The HTTP call and WS-Security are left to the caller.

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under Apache-2.0 or MIT.
