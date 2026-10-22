//! Runs the README examples, so the documentation cannot drift from the API.
//!
//! The README is also the crate's front page (`#![doc = include_str!(..)]`), so
//! its examples are already doctests. This repeats them as an integration test
//! so a failure names the test, not a line in the README, and so the two code
//! paths that matter (sign, then verify as the bank would) are exercised as a
//! caller outside the crate sees them.

use ebics::header::{OrderAttribute, StaticHeader};
use ebics::message::{Request, RequestKind, verify};
use ebics::response::Response;
use ebics::rsa::PrivateKey;

const MODULUS: &str = "2aWRwQ9/pII5l29p55PIXEWlJsvSn/d5KOpz/wnUEJ9AZvq2CitcgYhCG0qqn8Eb3G61qqQfNONhHUPdzc8Fndnv0Uqzg+Xtz4kSjHWZQZb069Po6jgLwUbmuuyfdlQpU4GdyrPHtFTd18tP20/5pmic1ui+9GQnyRBhoh55CTU=";
const EXPONENT: &str = "AQAB";
const PRIVATE: &str = "M1NfRvb/s+zd5f9pKypnopLt+nMwjXbPxL1dc3R+5RiZGG/hRA6rtwRKCqOWDyx8BIS7JK2lilRgz4T0ppc+jGbF3FIaqLzrr7b5hQJN97vR1f5PYZxGXQppx2qto2tQbm9Ct6K8UmRQTAI33ndB0yjhTUzXJnxl/lKnuBX0O/U=";

#[test]
fn readme_signing_example_produces_a_message_the_bank_accepts() {
    let key = PrivateKey::from_base64(MODULUS, EXPONENT, PRIVATE).unwrap();

    let mut rng = ebics::rng::Rng::from_entropy();
    let nonce = rng
        .bytes(16)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    let header = StaticHeader::signed(
        "HOST",
        "PARTNER",
        "USER",
        "HPB",
        OrderAttribute::WithoutSignatureCompressedEncrypted,
        nonce,
    )
    .unwrap();

    let xml = Request::new(RequestKind::NoPubKeyDigests, header)
        .sign(&key)
        .unwrap();

    assert!(xml.contains("<ds:SignatureValue>"));

    // What the bank does with it: recompute the header digest, then check the
    // signature. A message that only verifies against its own signer is a
    // client that cannot log in.
    verify(&xml, key.public()).unwrap();
}

#[test]
fn readme_response_example_treats_a_refusal_as_an_error() {
    let body = r#"<ebicsKeyManagementResponse xmlns="urn:org:ebics:H004" Version="H004" Revision="1"><header authenticate="true"><static></static><mutable><ReturnCode>000000</ReturnCode><ReportText>[EBICS_OK] OK</ReportText></mutable></header><body><ReturnCode authenticate="true">000000</ReturnCode></body></ebicsKeyManagementResponse>"#;

    let response = Response::parse(body).unwrap().into_result().unwrap();
    assert!(response.is_success());
    assert_eq!(response.technical_code, "000000");
}

#[test]
fn readme_response_example_returns_an_error_on_a_bank_refusal() {
    // The README claims a refusal becomes an `Err`, so `?` is enough. The
    // success case above does not test that claim; this does.
    let body = r#"<ebicsKeyManagementResponse xmlns="urn:org:ebics:H004" Version="H004" Revision="1"><header authenticate="true"><static></static><mutable><ReturnCode>091002</ReturnCode><ReportText>[EBICS_INVALID_USER_OR_USER_STATE]</ReportText></mutable></header><body><ReturnCode authenticate="true">091002</ReturnCode></body></ebicsKeyManagementResponse>"#;
    let result = Response::parse(body).unwrap().into_result();
    assert!(result.is_err(), "a bank refusal must not look like success");
}
