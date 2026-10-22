//! Canonicalisation against a real request.
//!
//! The fixture `hpb_request.xml` was produced by `node-ebics-client` (MIT), not
//! by this crate, and it carries the digest that client computed over its own
//! header. Reproducing that digest from the message alone is the strongest check
//! available without a bank: it exercises the parse, the canonical form, the
//! namespace injection and SHA-256 together, against an independent
//! implementation's output.
//!
//! The `AuthSignature` in the fixture is over the same canonical bytes, so
//! verifying it with the public key that is also in the fixture would be a
//! second independent check. That key is not in this repository, so the digest
//! is where the test stops.

use ebics::base64;
use ebics::c14n::{canonicalize_in_document, root_namespaces};
use ebics::sha256::Sha256;
use xmlkit::{Element, Node};

const REQUEST: &str = include_str!("fixtures/hpb_request.xml");

fn parse(text: &str) -> Element {
    xmlkit::parse(text).expect("the fixture parses")
}

/// The first child element with the given local name.
fn child<'a>(parent: &'a Element, local: &str) -> &'a Element {
    parent
        .children
        .iter()
        .find_map(|node| match node {
            Node::Element(element) if local_name(&element.name) == local => Some(element),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no child named {local}"))
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

/// The text of the first descendant with the given local name.
fn descendant_text(parent: &Element, local: &str) -> Option<String> {
    for node in &parent.children {
        if let Node::Element(element) = node {
            if local_name(&element.name) == local {
                let text = element
                    .children
                    .iter()
                    .filter_map(|child| match child {
                        Node::Text(text) => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<String>();
                return Some(text);
            }
            if let Some(found) = descendant_text(element, local) {
                return Some(found);
            }
        }
    }
    None
}

#[test]
fn reproduces_the_reference_clients_digest() {
    let root = parse(REQUEST);
    let header = child(&root, "header");
    let expected = descendant_text(&root, "DigestValue").expect("a digest is present");

    let canonical = canonicalize_in_document(&root, header);
    let digest = base64::encode(&Sha256::digest(canonical.as_bytes()));

    assert_eq!(
        digest, expected,
        "the canonical form did not reproduce the reference digest"
    );
}

/// The fixture's root declares `ds`, and the header does not use it, so the
/// injection is what makes the digest above work. If a future xmlkit change
/// dropped the declaration this test would fail before the digest one did, which
/// points at the cause rather than the symptom.
#[test]
fn the_root_declares_the_signature_namespace() {
    let root = parse(REQUEST);
    let namespaces = root_namespaces(&root);
    assert!(
        namespaces
            .iter()
            .any(|(prefix, uri)| prefix == "ds" && uri == "http://www.w3.org/2000/09/xmldsig#"),
        "the root must declare the signature namespace, got {namespaces:?}"
    );
}

/// The canonical form must not contain a self-closing tag, because C14N forbids
/// the short form and a bank's verifier would reject it.
#[test]
fn the_canonical_form_uses_no_short_tags() {
    let root = parse(REQUEST);
    let header = child(&root, "header");
    let canonical = canonicalize_in_document(&root, header);
    assert!(
        !canonical.contains("/>"),
        "found a short tag in {canonical}"
    );
}
