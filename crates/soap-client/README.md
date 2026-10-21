# soap-client

SOAP 1.1 and 1.2 envelopes, plus WSDL reading.

```rust
use soap_client::{Envelope, Version};

let request = Envelope::request(Version::Soap11)
    .with_body_xml("<Ping xmlns=\"urn:x\">1</Ping>")
    .to_xml();
assert!(request.contains("soap:Envelope"));

let response = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body><Pong xmlns="urn:x">1</Pong></soap:Body>
</soap:Envelope>"#;
assert!(Envelope::parse(response).unwrap().body_text().contains("Pong"));
```

Build requests, parse responses, read faults in either version's shape, and
extract a service's endpoint and operations from its WSDL. The HTTP call and
WS-Security are left to the caller.

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under Apache-2.0 or MIT.
