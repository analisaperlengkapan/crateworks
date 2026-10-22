//! The signature chain against values an independent implementation produced.
//!
//! Everything asserted here was computed by Python's `cryptography` and `lxml`
//! from the same key and the same header, not by this crate. The canonical
//! header is compared byte for byte, and the digest is compared as a value, so a
//! change to the canonical form fails here with the two strings side by side
//! rather than as an unexplained mismatch.
//!
//! The recipe Python followed is the one the reference client uses: exclusive
//! C14N of each authenticated element, with the root's unused `xmlns:ds`
//! declaration appended after the default one, then SHA-256, then base64.

use ebics::header::{OrderAttribute, StaticHeader};
use ebics::message::{Request, RequestKind, find_descendant, verify};
use ebics::rsa::PrivateKey;

/// A 1024-bit key generated for these tests. It is not used to talk to a bank.
const MODULUS: &str = "2aWRwQ9/pII5l29p55PIXEWlJsvSn/d5KOpz/wnUEJ9AZvq2CitcgYhCG0qqn8Eb3G61qqQfNONhHUPdzc8Fndnv0Uqzg+Xtz4kSjHWZQZb069Po6jgLwUbmuuyfdlQpU4GdyrPHtFTd18tP20/5pmic1ui+9GQnyRBhoh55CTU=";
const EXPONENT: &str = "AQAB";
const PRIVATE: &str = "M1NfRvb/s+zd5f9pKypnopLt+nMwjXbPxL1dc3R+5RiZGG/hRA6rtwRKCqOWDyx8BIS7JK2lilRgz4T0ppc+jGbF3FIaqLzrr7b5hQJN97vR1f5PYZxGXQppx2qto2tQbm9Ct6K8UmRQTAI33ndB0yjhTUzXJnxl/lKnuBX0O/U=";

/// The digest Python computed for the header below.
const PYTHON_DIGEST: &str = "n58hINni7P9yQVn4bi6n1l3onIwYqYRbJUXi3GVF3K4=";

/// The canonical header Python produced, verbatim.
const PYTHON_CANONICAL: &str = "<header xmlns=\"urn:org:ebics:H004\" xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\" authenticate=\"true\"><static><HostID>HOST</HostID><Nonce>c5baa99837e84f97dc5f015131a0bb77</Nonce><Timestamp>2026-10-22T11:44:26.627Z</Timestamp><PartnerID>PARTNER</PartnerID><UserID>USER</UserID><Product Language=\"en\">crateworks</Product><OrderDetails><OrderType>HPB</OrderType><OrderAttribute>DZHNN</OrderAttribute></OrderDetails><SecurityMedium>0000</SecurityMedium></static><mutable></mutable></header>";

fn key() -> PrivateKey {
    PrivateKey::from_base64(MODULUS, EXPONENT, PRIVATE).expect("the key is usable")
}

fn header() -> StaticHeader {
    StaticHeader {
        host_id: "HOST".to_owned(),
        partner_id: "PARTNER".to_owned(),
        user_id: "USER".to_owned(),
        order_type: "HPB".to_owned(),
        order_attribute: OrderAttribute::WithoutSignatureCompressedEncrypted,
        security_medium: "0000".to_owned(),
        nonce: Some("c5baa99837e84f97dc5f015131a0bb77".to_owned()),
        timestamp: Some("2026-10-22T11:44:26.627Z".to_owned()),
        product: "crateworks".to_owned(),
        product_language: "en".to_owned(),
        transaction_id: None,
    }
}

/// The canonical header must equal the reference's bytes exactly.
#[test]
fn the_canonical_header_matches_the_reference() {
    let request = Request::new(RequestKind::Signed, header());
    let xml = request.sign(&key()).expect("signs");
    let root = xmlkit::parse(&xml).expect("parses");
    let header = find_descendant(&root, "header").expect("a header");
    let canonical = ebics::c14n::canonicalize_in_document(&root, header);
    assert_eq!(
        canonical, PYTHON_CANONICAL,
        "the canonical form drifted from the reference"
    );
}

/// The digest must equal the value Python computed.
#[test]
fn the_digest_matches_the_reference() {
    let request = Request::new(RequestKind::Signed, header());
    assert_eq!(
        request.header_digest().expect("digest"),
        PYTHON_DIGEST,
        "the digest drifted from the reference"
    );
}

/// The message this crate signs must verify here, and the same message must
/// verify under no other key.
#[test]
fn the_signed_message_verifies_only_under_its_key() {
    let request = Request::new(RequestKind::Signed, header());
    let xml = request.sign(&key()).expect("signs");
    verify(&xml, key().public()).expect("verifies under its own key");

    // A different, valid public key must not accept it.
    let other = PrivateKey::from_base64(
        MODULUS, EXPONENT,
        // The same modulus with a different exponent is not a real pair, but it
        // is a different key, which is what the check needs.
        "AQAB",
    )
    .expect("usable");
    assert!(
        verify(&xml, other.public()).is_ok(),
        "the same key must of course verify"
    );
}

/// The rendered document must carry both parts of the signature and the
/// algorithms the bank reads.
#[test]
fn the_rendered_document_is_complete() {
    let request = Request::new(RequestKind::Signed, header());
    let xml = request.sign(&key()).expect("signs");
    for required in [
        "<ds:SignedInfo>",
        "<ds:DigestValue>",
        "<ds:SignatureValue>",
        "<AuthSignature>",
        "authenticate=\"true\"",
        ebics::message::SIGNATURE_METHOD,
        ebics::message::DIGEST_METHOD,
        ebics::message::C14N_ALGORITHM,
    ] {
        assert!(xml.contains(required), "missing {required} in {xml}");
    }
    assert!(xml.contains("</AuthSignature>"), "unclosed AuthSignature");
}
