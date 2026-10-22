//! Reading a document that was signed and then had validation data appended.
//!
//! Both fixtures were produced by `pyhanko` from a `reportlab` page. The
//! two-revision file is PAdES B-B; the three-revision file is B-LT, where the
//! last revision adds a DSS whose `/Certs` stream is binary. That binary stream
//! is what used to stop the cross-reference walk, so it is the case this file
//! exists for.
//!
//! Regenerate with `python3 research/tools/gen_pades_corpus.py`.

use pdfa::container::Pdf;
use pdfa::xref;

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// A file with two revisions resolves through `/Prev` to the newest catalog.
#[test]
fn a_two_revision_file_reads_its_xref() {
    let bytes = fixture("signed_bb.pdf");
    let xref = xref::read(&bytes).expect("the cross-reference section is readable");
    // Object 4 is the catalog, redefined in the second revision.
    let location = xref.entries.get(&4).expect("the catalog is in the xref");
    let offset = match location {
        xref::Location::InFile { offset, .. } => *offset,
        other => panic!("the catalog should be in the file, got {other:?}"),
    };
    assert!(
        offset > 1000,
        "the newest catalog wins, so its offset is in the second revision, got {offset}"
    );
}

/// A file with three revisions, the last of which appends a binary stream,
/// still resolves. The binary stream makes everything after it invalid UTF-8,
/// which is what a reader that decodes to the end of the file trips over.
#[test]
fn a_binary_stream_in_a_later_revision_does_not_stop_the_walk() {
    let bytes = fixture("signed_lt.pdf");
    let xref = xref::read(&bytes).expect("the cross-reference section is readable");
    assert!(
        xref.entries.len() >= 10,
        "every revision contributes entries, got {}",
        xref.entries.len()
    );
    // Object 15 is the DSS, defined only in the third revision.
    assert!(
        xref.entries.contains_key(&15),
        "the newest revision defines object 15, which only the third section knows about"
    );
}

/// The newest catalog is the one with `/DSS`, not the one from the earlier
/// revision. Picking the stale catalog silently loses the validation data.
#[test]
fn the_newest_catalog_carries_the_dss() {
    let bytes = fixture("signed_lt.pdf");
    let pdf = Pdf::parse(&bytes).expect("the document parses");
    let catalog = pdf.catalog().expect("there is a catalog");
    assert!(
        catalog.contains("/DSS"),
        "the newest catalog points at the DSS, got {catalog}"
    );
}
