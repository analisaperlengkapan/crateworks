//! Comparison against the reference implementation.
//!
//! The fixtures were produced by the official `factur-x` Python package, which
//! is an independent implementation of EN 16931 by the author of the Factur-X
//! specification. For each fixture this test checks two things:
//!
//! * that our parser reads the reference document into the same invoice our
//!   generator produces, and
//! * that the two documents carry the same elements with the same text, as a
//!   set of paths.
//!
//! The comparison is on the set of paths rather than the serialised bytes,
//! because element order within a parent and the exact set of namespace
//! declarations are not part of the meaning. What matters is that every field
//! the reference writes is written by us too, and no field is invented.

use std::collections::BTreeSet;

use factur_x::{Syntax, generate, parse};
use xmlkit::{Element, local_name};

const SAMPLE_CII: &str = include_str!("fixtures/sample_cii.xml");
const SAMPLE_UBL: &str = include_str!("fixtures/sample_ubl.xml");
const RICH_CII: &str = include_str!("fixtures/rich_cii.xml");
const RICH_UBL: &str = include_str!("fixtures/rich_ubl.xml");

/// Every element path with non-blank text, as `parent/child=text`.
fn paths(element: &Element, prefix: &str, out: &mut BTreeSet<String>) {
    let here = format!("{prefix}/{}", local_name(&element.name));
    let text = element.text();
    if !text.trim().is_empty() {
        out.insert(format!("{here}={text}"));
    }
    for node in &element.children {
        if let xmlkit::Node::Element(child) = node {
            paths(child, &here, out);
        }
    }
}

fn differences(ours: &str, reference: &str) -> Vec<String> {
    let ours = xmlkit::parse(ours).expect("our document must be well formed");
    let reference = xmlkit::parse(reference).expect("the fixture must be well formed");
    let mut our_paths = BTreeSet::new();
    let mut reference_paths = BTreeSet::new();
    paths(&ours, "", &mut our_paths);
    paths(&reference, "", &mut reference_paths);

    let mut out = Vec::new();
    for path in our_paths.difference(&reference_paths) {
        out.push(format!("only in ours: {path}"));
    }
    for path in reference_paths.difference(&our_paths) {
        out.push(format!("only in the reference: {path}"));
    }
    out
}

/// The fixtures and the invoice each one represents.
fn cases() -> Vec<(&'static str, &'static str, Syntax, factur_x::Invoice)> {
    vec![
        (
            "sample_cii",
            SAMPLE_CII,
            Syntax::Cii,
            factur_x::test_support::sample_invoice(),
        ),
        (
            "sample_ubl",
            SAMPLE_UBL,
            Syntax::Ubl,
            factur_x::test_support::sample_invoice(),
        ),
        (
            "rich_cii",
            RICH_CII,
            Syntax::Cii,
            factur_x::test_support::rich_invoice(),
        ),
        (
            "rich_ubl",
            RICH_UBL,
            Syntax::Ubl,
            factur_x::test_support::rich_invoice(),
        ),
    ]
}

#[test]
fn the_reference_documents_parse_into_the_expected_invoice() {
    for (name, xml, syntax, expected) in cases() {
        let parsed = parse(xml, syntax).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(parsed, expected, "{name}");
    }
}

#[test]
fn our_documents_carry_the_same_fields_as_the_reference() {
    for (name, xml, syntax, invoice) in cases() {
        let ours = generate(&invoice, syntax);
        let diffs = differences(&ours, xml);
        assert!(diffs.is_empty(), "{name}:\n{}", diffs.join("\n"));
    }
}

#[test]
fn the_reference_syntax_is_detected() {
    for (name, xml, syntax, _) in cases() {
        assert_eq!(factur_x::detect_syntax(xml), Some(syntax), "{name}");
    }
}

#[test]
fn a_document_we_generate_can_be_read_back_by_our_parser() {
    for (name, _, syntax, invoice) in cases() {
        let xml = generate(&invoice, syntax);
        let back = parse(&xml, syntax).unwrap();
        assert_eq!(back, invoice, "{name}");
    }
}
