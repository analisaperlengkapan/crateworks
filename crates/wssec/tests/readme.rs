//! Runs the README examples, so the documentation cannot drift from the API.

#[test]
fn readme_building_a_header() {
    use wssec::{Security, Timestamp, UsernameToken};

    let nonce = b"0123456789abcdef".to_vec();
    let created = "2026-10-21T12:00:00Z";

    let token = UsernameToken::digest("NNK", "secret", nonce, created);
    let header = Security::new()
        .with_timestamp(&Timestamp::new(created, "2026-10-21T12:05:00Z"))
        .with_username_token(&token)
        .to_xml();

    assert!(header.contains("wsse:UsernameToken"));
    assert!(header.contains("PasswordDigest"));
    assert!(header.contains("wsu:Timestamp"));
    assert!(!header.contains("secret"));
}

#[test]
fn readme_reading_a_header() {
    use wssec::{ParsedToken, parse_security};

    let xml = r#"<wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
  <wsse:UsernameToken>
    <wsse:Username>scott</wsse:Username>
    <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">g01uA646Xg1x0SE/Xfa3CM/TLMI=</wsse:Password>
    <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">AAECAwQFBgcICQoLDA0ODw==</wsse:Nonce>
    <wsu:Created xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">2026-10-21T12:00:00+00:00</wsu:Created>
  </wsse:UsernameToken>
</wsse:Security>"#;

    let tokens = parse_security(xml).unwrap();
    let ParsedToken::Username(token) = &tokens[0] else {
        unreachable!()
    };
    assert_eq!(token.username, "scott");
    assert!(token.matches_digest("secret"));
    assert!(!token.matches_digest("wrong"));
}
