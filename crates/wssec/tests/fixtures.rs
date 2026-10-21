//! Comparison against the reference implementations.
//!
//! The digest fixtures were produced by Python's `hashlib` across every length
//! that straddles a 64-byte block boundary, which is where a hand-written
//! implementation usually breaks. The WS-Security fixtures were produced by
//! `zeep`, the most widely used Python SOAP client, so the structure of the
//! header is checked against something that talks to real services.

use std::collections::BTreeSet;

use wssec::{Security, Timestamp, UsernameToken};

/// A digest case from the fixture file.
struct DigestCase {
    data: Vec<u8>,
    sha1: String,
    md5: String,
}

/// Reads the fixture file.
///
/// The file is small JSON written by the generator script, and the workspace has
/// no JSON crate, so this reads just the fields it needs.
fn digest_cases() -> Vec<DigestCase> {
    let text = include_str!("fixtures/digests.json");
    let mut cases = Vec::new();
    for entry in text.split(" {").skip(1) {
        let field = |name: &str| -> Option<String> {
            let start = entry.find(&format!("\"{name}\""))? + name.len() + 5;
            let rest = &entry[start..];
            let end = rest.find('"')?;
            Some(rest[..end].to_string())
        };
        let len: usize = entry
            .split("\"len\":")
            .nth(1)
            .and_then(|rest| rest.trim_start().split(',').next())
            .and_then(|n| n.trim().parse().ok())
            .expect("every case has a length");
        let data = if entry.contains("\"repeat_a\"") {
            vec![b'a'; len]
        } else if let Some(text) = field("text") {
            text.into_bytes()
        } else {
            let hex = field("data_hex").expect("a case has data");
            (0..hex.len() / 2)
                .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
                .collect()
        };
        cases.push(DigestCase {
            data,
            sha1: field("sha1").unwrap(),
            md5: field("md5").unwrap(),
        });
    }
    cases
}

#[test]
fn every_digest_fixture_matches_hashlib() {
    let cases = digest_cases();
    assert!(cases.len() >= 20, "expected the full fixture set");
    for case in &cases {
        assert_eq!(
            wssec::digest::hex(&wssec::digest::sha1(&case.data)),
            case.sha1,
            "sha1 of {} bytes",
            case.data.len()
        );
        assert_eq!(
            wssec::digest::hex(&wssec::digest::md5(&case.data)),
            case.md5,
            "md5 of {} bytes",
            case.data.len()
        );
    }
}

#[test]
fn base64_fixtures_match_python() {
    // Python: base64.b64encode(bytes(range(n))) for each length.
    let expected = [
        (0usize, ""),
        (1, "AA=="),
        (2, "AAE="),
        (3, "AAEC"),
        (16, "AAECAwQFBgcICQoLDA0ODw=="),
        (17, "AAECAwQFBgcICQoLDA0ODxA="),
    ];
    for (length, encoded) in expected {
        let data: Vec<u8> = (0..length as u8).collect();
        assert_eq!(wssec::base64::encode(&data), encoded, "{length} bytes");
        assert_eq!(
            wssec::base64::decode(encoded).unwrap(),
            data,
            "{length} bytes"
        );
    }
}

/// Every element path, relative to the `Security` element.
///
/// The comparison is anchored at `Security` so that the envelope nesting the
/// reference happens to have does not enter into it, and prefixes are dropped
/// for the same reason. Text values are deliberately excluded: the nonce and the
/// password digest differ between the two conventions by design, and the fields
/// whose values must match are asserted one by one.
fn paths(xml: &str) -> BTreeSet<String> {
    fn walk(element: &xmlkit::Element, prefix: &str, out: &mut BTreeSet<String>) {
        let here = format!("{prefix}/{}", xmlkit::local_name(&element.name));
        out.insert(here.clone());
        for node in &element.children {
            if let xmlkit::Node::Element(child) = node {
                walk(child, &here, out);
            }
        }
    }
    fn find(element: &xmlkit::Element) -> Option<&xmlkit::Element> {
        if xmlkit::local_name(&element.name) == "Security" {
            return Some(element);
        }
        element.children.iter().find_map(|node| match node {
            xmlkit::Node::Element(child) => find(child),
            _ => None,
        })
    }
    let root = xmlkit::parse(xml).expect("fixture is well formed");
    let security = find(&root).expect("a Security element");
    let mut out = BTreeSet::new();
    walk(security, "", &mut out);
    out
}

/// The structural paths of the reference documents, from `zeep`.
const ZEEP_TEXT_11: &str = include_str!("fixtures/text_11.xml");
const ZEEP_DIGEST_11: &str = include_str!("fixtures/digest_11.xml");

#[test]
fn the_reference_documents_parse_back_into_the_tokens_we_would_build() {
    let tokens = wssec::parse_security(ZEEP_TEXT_11).unwrap();
    let wssec::ParsedToken::Username(token) = &tokens[0] else {
        panic!("expected a user name token");
    };
    assert_eq!(token.username, "scott");
    assert!(!token.is_digest);
    // zeep base64 encodes the password; this crate does the same.
    assert_eq!(token.plaintext_password().unwrap(), "tiger");
}

#[test]
fn the_reference_digest_token_verifies_with_our_pipeline() {
    // zeep takes the digest over the base64 text of the nonce rather than the
    // raw bytes, so the fixture verifies against our zeep-compatible helper.
    let tokens = wssec::parse_security(ZEEP_DIGEST_11).unwrap();
    let wssec::ParsedToken::Username(token) = &tokens[0] else {
        panic!("expected a user name token");
    };
    assert_eq!(token.username, "NNK");
    assert!(token.is_digest);

    // The nonce decodes to the ASCII bytes of the base64 text, which is exactly
    // the behaviour being pinned down.
    let nonce_text = wssec::base64::encode(token.nonce.as_deref().unwrap());
    assert_eq!(nonce_text, "QUFFQ0F3UUZCZ2NJQ1FvTERBME9Edz09");

    let created = token.created.clone().unwrap();
    let digest = wssec::digest_with_text_nonce("AAECAwQFBgcICQoLDA0ODw==", &created, b"secret");
    assert_eq!(Some(&digest), token.password.as_ref());
}

#[test]
fn our_header_carries_the_same_structure_as_the_reference() {
    // Same elements in the same places. Values are checked separately because
    // the nonce and digest encodings differ between the two conventions.
    let nonce: Vec<u8> = (0..16u8).collect();
    let ours = Security::new()
        .with_timestamp(&Timestamp::new(
            "2026-10-21T12:00:00+00:00",
            "2026-10-21T12:05:00+00:00",
        ))
        .with_username_token(&UsernameToken::digest(
            "NNK",
            "secret",
            nonce,
            "2026-10-21T12:00:00+00:00",
        ))
        .to_xml();

    let ours_paths = paths(&ours);
    let reference_paths = paths(ZEEP_DIGEST_11);
    for path in &reference_paths {
        assert!(
            ours_paths.contains(path),
            "the reference has {path} and we do not\nours: {ours_paths:?}"
        );
    }

    // The values that must agree between the two documents.
    let token = username_token(&ours);
    let reference = username_token(ZEEP_DIGEST_11);
    assert_eq!(token.username, reference.username);
    assert_eq!(token.created, reference.created);
    assert_eq!(token.is_digest, reference.is_digest);
}

/// The user name token in a document, whichever position it holds.
fn username_token(xml: &str) -> wssec::ParsedUsernameToken {
    wssec::parse_security(xml)
        .unwrap()
        .into_iter()
        .find_map(|token| match token {
            wssec::ParsedToken::Username(token) => Some(token),
            wssec::ParsedToken::Timestamp(_) => None,
        })
        .expect("a user name token")
}
