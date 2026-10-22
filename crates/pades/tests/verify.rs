//! Runs the README example and the public API against the committed corpus.
//!
//! The fixtures are signed PDFs, and the labels beside them are what pyHanko
//! reports for each one. The tests assert against those labels rather than
//! against what this crate happens to produce, so a change in the reader that
//! makes it agree with itself but not with the oracle fails here.

use pades::{Coverage, Integrity, Kind, Level, verify};

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|_| panic!("the fixture {name} is present"))
}

#[test]
fn readme_example() {
    let document = fixture("signed_lt.pdf");
    let report = verify(&document).unwrap();
    let signature = &report.signatures[0];
    assert_eq!(signature.integrity, Integrity::Intact);
    assert_eq!(signature.coverage, Coverage::EntireRevision);
    assert_eq!(report.level, Some(Level::B_LT));
}

#[test]
fn the_corpus_matches_the_pyhanko_labels() {
    // B-B: signed once, nothing appended. pyHanko reports ENTIRE_FILE.
    let report = verify(&fixture("signed_bb.pdf")).unwrap();
    assert_eq!(report.signatures[0].coverage, Coverage::EntireFile);
    assert_eq!(report.signatures[0].integrity, Integrity::Intact);
    assert_eq!(report.level, Some(Level::B_B));

    // B-LT: the DSS was appended after signing, so pyHanko reports
    // ENTIRE_REVISION and the signature is still intact.
    let report = verify(&fixture("signed_lt.pdf")).unwrap();
    assert_eq!(report.signatures[0].coverage, Coverage::EntireRevision);
    assert_eq!(report.signatures[0].integrity, Integrity::Intact);
    assert_eq!(report.level, Some(Level::B_LT));

    // Appended: one byte past the signed range, which does not break anything.
    let report = verify(&fixture("appended.pdf")).unwrap();
    assert_eq!(report.signatures[0].coverage, Coverage::EntireRevision);
    assert_eq!(report.signatures[0].integrity, Integrity::Intact);

    // Tampered: a byte inside the signed range, which pyHanko reports as
    // `intact: false`.
    let report = verify(&fixture("tampered.pdf")).unwrap();
    assert_eq!(report.signatures[0].integrity, Integrity::Broken);
}

#[test]
fn every_signature_names_a_signer() {
    for name in [
        "signed_bb.pdf",
        "signed_lt.pdf",
        "appended.pdf",
        "tampered.pdf",
    ] {
        let report = verify(&fixture(name)).unwrap();
        let signer = report.signatures[0]
            .signer
            .as_ref()
            .unwrap_or_else(|| panic!("{name} names a signer"));
        assert_eq!(signer.subject, "CN=PAdES Corpus");
        assert_eq!(signer.key_algorithm, "1.2.840.113549.1.1.1");
    }
}

#[test]
fn the_sub_filter_is_reported_as_written() {
    let report = verify(&fixture("signed_bb.pdf")).unwrap();
    assert_eq!(report.signatures[0].sub_filter, "ETSI.CAdES.detached");
    assert_eq!(report.signatures[0].kind, Kind::CadesDetached);
}

#[test]
fn a_pdf_without_a_signature_is_rejected() {
    assert!(verify(&fixture("blank.pdf")).is_err());
}

#[test]
fn a_file_that_is_not_a_pdf_is_rejected() {
    assert!(verify(b"this is not a PDF at all").is_err());
}
