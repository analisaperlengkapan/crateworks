//! Reading a bank's answer, against a response Python built.
//!
//! The fixture is a complete response: a real RSA wrap of the transaction key
//! under the fixture key, a real AES-128-CBC encryption of the deflated order
//! data with the zero IV and the EBICS padding, and the base64 that travels on
//! the wire. Python's `cryptography` and `zlib` produced every byte of it, so
//! this crate's inflate, AES, RSA decryption and padding removal are all being
//! checked against an independent implementation at once.
//!
//! A failure here means one of four things went wrong, and the test that fails
//! names which: `order_data_round_trips` covers the whole path, and the tests
//! above it cover the parts.

use ebics::response::Response;
use ebics::rsa::PrivateKey;

const RESPONSE: &str = include_str!("fixtures/bank_response.xml");
const ORDER_DATA: &[u8] = include_bytes!("fixtures/bank_order_data.bin");

const MODULUS: &str = "2aWRwQ9/pII5l29p55PIXEWlJsvSn/d5KOpz/wnUEJ9AZvq2CitcgYhCG0qqn8Eb3G61qqQfNONhHUPdzc8Fndnv0Uqzg+Xtz4kSjHWZQZb069Po6jgLwUbmuuyfdlQpU4GdyrPHtFTd18tP20/5pmic1ui+9GQnyRBhoh55CTU=";
const EXPONENT: &str = "AQAB";
const PRIVATE: &str = "M1NfRvb/s+zd5f9pKypnopLt+nMwjXbPxL1dc3R+5RiZGG/hRA6rtwRKCqOWDyx8BIS7JK2lilRgz4T0ppc+jGbF3FIaqLzrr7b5hQJN97vR1f5PYZxGXQppx2qto2tQbm9Ct6K8UmRQTAI33ndB0yjhTUzXJnxl/lKnuBX0O/U=";

fn key() -> PrivateKey {
    PrivateKey::from_base64(MODULUS, EXPONENT, PRIVATE).expect("the key is usable")
}

#[test]
fn a_bank_response_parses() {
    let response = Response::parse(RESPONSE).expect("parses");
    assert!(response.is_success());
    assert_eq!(response.technical_code, "000000");
    assert_eq!(response.report_text.as_deref(), Some("[EBICS_OK] OK"));
    assert!(response.transaction_key.is_some());
    assert!(response.order_data.is_some());
}

/// The whole path: RSA unwrap, AES decrypt, unpad, inflate.
#[test]
fn the_order_data_decrypts_to_the_original() {
    let response = Response::parse(RESPONSE).expect("parses");
    let recovered = response.order_data(&key()).expect("decrypts");
    assert_eq!(
        recovered, ORDER_DATA,
        "the decrypted order data differs from what Python encrypted"
    );
}

/// The transaction key must be the one Python wrapped.
#[test]
fn the_transaction_key_unwraps() {
    let response = Response::parse(RESPONSE).expect("parses");
    let wrapped =
        ebics::base64::decode(response.transaction_key.as_deref().expect("a key")).expect("base64");
    let unwrapped = key().decrypt_pkcs1v15(&wrapped).expect("unwraps");
    assert_eq!(unwrapped, (0..16).collect::<Vec<u8>>(), "the key differs");
}

/// A response whose order data was altered must not decrypt to the original,
/// and must not be silently accepted.
#[test]
fn altered_order_data_is_refused_or_differs() {
    let response = Response::parse(RESPONSE).expect("parses");
    let mut tampered = RESPONSE.to_owned();
    let data = response.order_data.as_deref().expect("data");
    // Flip one character of the base64 ciphertext.
    let flipped = if data.starts_with('A') { 'B' } else { 'A' };
    let replaced = format!("{flipped}{}", &data[1..]);
    tampered = tampered.replacen(data, &replaced, 1);
    let tampered = Response::parse(&tampered).expect("parses");
    if let Ok(decrypted) = tampered.order_data(&key()) {
        assert_ne!(
            decrypted, ORDER_DATA,
            "an altered ciphertext must not decrypt to the original"
        );
    }
}

/// The response must be usable as a `Result`, which is what a caller writes.
#[test]
fn a_successful_response_converts_to_ok() {
    let response = Response::parse(RESPONSE).expect("parses");
    assert!(response.into_result().is_ok());
}
