//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use stdnum_rs::{Iban, Luhn};

    assert!(Iban::parse("GB82 WEST 1234 5698 7654 32").is_ok());
    assert!(Luhn::parse("4539148803436467").is_ok());
}
