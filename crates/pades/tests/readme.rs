//! Runs the README example, so the documentation cannot drift from the API.

use pades::{Coverage, Integrity, Level, verify};

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|_| panic!("the fixture {name} is present"))
}

#[test]
fn readme_example() {
    // A PDF signed with pyHanko, read back to see what the signature covers.
    let document = fixture("signed_lt.pdf");
    let report = verify(&document).unwrap();
    let signature = &report.signatures[0];
    assert_eq!(signature.integrity, Integrity::Intact);
    // The validation data was appended after signing, so the range stops at the
    // end of the signature's own revision rather than at the end of the file.
    assert_eq!(signature.coverage, Coverage::EntireRevision);
    assert_eq!(report.level, Some(Level::B_LT));
}
