//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use soap_client::{Envelope, Version};

    let request = Envelope::request(Version::Soap11)
        .with_body_xml("<Ping xmlns=\"urn:x\">1</Ping>")
        .to_xml();
    assert!(request.contains("soap:Envelope"));

    let response = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Body><Pong xmlns="urn:x">1</Pong></soap:Body>
</soap:Envelope>"#;
    assert!(
        Envelope::parse(response)
            .unwrap()
            .body_text()
            .contains("Pong")
    );
}
