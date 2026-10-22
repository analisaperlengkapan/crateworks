//! RSA against an independent implementation.
//!
//! The fixture holds a real 2048-bit key, and every expected value in it was
//! produced by Python's `cryptography` library, not by this crate. The test
//! verifies in both directions: signatures Python made must verify here, and
//! signatures made here must verify in Python. Only the first direction can run
//! in this file; the second is checked by `tests/interop.rs`, which shells out.
//!
//! A digest is checked against the value Python computed for the same key,
//! because the key digest is the one place where a formatting difference is
//! silent: the bank simply does not recognise the key.

use ebics::base64;
use ebics::rng::Rng;
use ebics::rsa::{PrivateKey, PublicKey, encrypt_pkcs1v15};

/// The fixture, parsed once by hand rather than with a JSON dependency, because
/// this crate has none and the shape is fixed.
struct Fixture {
    n: String,
    e: String,
    d: String,
    digest: String,
    pkcs1v15_sig: String,
    pss_sig: String,
    ct: String,
    pt: String,
}

fn fixture() -> Fixture {
    let text = include_str!("fixtures/rsa_2048.json");
    let field = |name: &str| -> String {
        let needle = format!("\"{name}\": \"");
        let start = text.find(&needle).expect("field present") + needle.len();
        let end = start + text[start..].find('"').expect("closing quote");
        text[start..end].to_owned()
    };
    Fixture {
        n: field("n"),
        e: field("e"),
        d: field("d"),
        digest: field("digest"),
        pkcs1v15_sig: field("pkcs1v15_sig"),
        pss_sig: field("pss_sig"),
        ct: field("ct"),
        pt: field("pt"),
    }
}

const MESSAGE: &[u8] = b"EBICS order data digest fixture";

#[test]
fn key_digest_matches_python() {
    let f = fixture();
    let key = PublicKey::from_base64(&f.n, &f.e).expect("key parses");
    assert_eq!(key.digest(), f.digest);
}

#[test]
fn verifies_a_python_pkcs1v15_signature() {
    let f = fixture();
    let key = PublicKey::from_base64(&f.n, &f.e).expect("key parses");
    let signature = base64::decode(&f.pkcs1v15_sig).expect("signature decodes");
    key.verify_pkcs1v15_sha256(MESSAGE, &signature)
        .expect("Python's signature verifies");
}

#[test]
fn verifies_a_python_pss_signature() {
    let f = fixture();
    let key = PublicKey::from_base64(&f.n, &f.e).expect("key parses");
    let signature = base64::decode(&f.pss_sig).expect("signature decodes");
    key.verify_pss_sha256(MESSAGE, &signature)
        .expect("Python's PSS signature verifies");
}

#[test]
fn rejects_a_signature_over_the_wrong_message() {
    let f = fixture();
    let key = PublicKey::from_base64(&f.n, &f.e).expect("key parses");
    let signature = base64::decode(&f.pkcs1v15_sig).expect("signature decodes");
    assert!(
        key.verify_pkcs1v15_sha256(b"a different message", &signature)
            .is_err()
    );
}

#[test]
fn our_own_signatures_verify_here() {
    let f = fixture();
    let key = PrivateKey::from_base64(&f.n, &f.e, &f.d).expect("key parses");
    let mut rng = Rng::from_seed([4; 32]);

    let signature = key.sign_pkcs1v15_sha256(MESSAGE).expect("signs");
    key.public()
        .verify_pkcs1v15_sha256(MESSAGE, &signature)
        .expect("our PKCS#1 v1.5 signature verifies");

    let signature = key.sign_pss_sha256(MESSAGE, &mut rng).expect("signs");
    key.public()
        .verify_pss_sha256(MESSAGE, &signature)
        .expect("our PSS signature verifies");
}

/// A wrapped transaction key must decrypt, under the private exponent, to a
/// PKCS#1 v1.5 block whose payload is the plaintext. That is checked here by
/// recovering the block with the public `Modulus` type rather than by trusting
/// the length.
#[test]
fn wrapped_key_recovers_the_plaintext() {
    use ebics::bignum::{BigUint, Modulus};

    let f = fixture();
    let key = PublicKey::from_base64(&f.n, &f.e).expect("key parses");
    let mut rng = Rng::from_seed([5; 32]);

    let plaintext = b"0123456789abcdef";
    let ciphertext = encrypt_pkcs1v15(&key, plaintext, &mut rng).expect("encrypts");
    assert_ne!(ciphertext, plaintext);

    // Raise the ciphertext to the private exponent, which is what the bank does
    // with its own key, and read the padding back.
    let modulus = Modulus::new(BigUint::from_be_bytes(
        &base64::decode(&f.n).expect("modulus decodes"),
    ))
    .expect("odd modulus");
    let recovered = modulus.pow(
        &BigUint::from_be_bytes(&ciphertext),
        &base64::decode(&f.d).expect("d decodes"),
    );
    let block = recovered.to_be_bytes(256).expect("fits");

    assert_eq!(&block[..2], &[0x00, 0x02], "PKCS#1 v1.5 block header");
    let separator = block[2..].iter().position(|b| *b == 0).expect("separator") + 2;
    assert!(
        block[2..separator].iter().all(|b| *b != 0),
        "padding has no zero"
    );
    assert!(separator >= 10, "at least eight padding bytes");
    assert_eq!(&block[separator + 1..], plaintext, "payload recovered");

    // Python's own ciphertext for the same plaintext is the same length, so the
    // key size agrees with the reference.
    let expected = base64::decode(&f.ct).expect("ciphertext decodes");
    assert_eq!(ciphertext.len(), expected.len());
    assert_eq!(f.pt.as_bytes(), plaintext);
}
