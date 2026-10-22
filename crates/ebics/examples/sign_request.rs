//! Signs a request with a fixed test key and prints it.
//!
//! The output is checked by `tests/signature_oracle.rs`, which hands it to
//! Python and asks that implementation to verify the signature. A signature this
//! crate produces is only worth something if another implementation accepts it.

use ebics::header::{OrderAttribute, StaticHeader};
use ebics::message::{Request, RequestKind};
use ebics::rsa::PrivateKey;

const MODULUS: &str = "2aWRwQ9/pII5l29p55PIXEWlJsvSn/d5KOpz/wnUEJ9AZvq2CitcgYhCG0qqn8Eb3G61qqQfNONhHUPdzc8Fndnv0Uqzg+Xtz4kSjHWZQZb069Po6jgLwUbmuuyfdlQpU4GdyrPHtFTd18tP20/5pmic1ui+9GQnyRBhoh55CTU=";
const EXPONENT: &str = "AQAB";
const PRIVATE: &str = "M1NfRvb/s+zd5f9pKypnopLt+nMwjXbPxL1dc3R+5RiZGG/hRA6rtwRKCqOWDyx8BIS7JK2lilRgz4T0ppc+jGbF3FIaqLzrr7b5hQJN97vR1f5PYZxGXQppx2qto2tQbm9Ct6K8UmRQTAI33ndB0yjhTUzXJnxl/lKnuBX0O/U=";

fn main() {
    let key = PrivateKey::from_base64(MODULUS, EXPONENT, PRIVATE).expect("the key is usable");
    let header = StaticHeader {
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
    };
    let xml = Request::new(RequestKind::Signed, header)
        .sign(&key)
        .expect("signs");
    println!("{xml}");
}
