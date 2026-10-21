//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use eu_vat::{Country, Vat};

    let vat = Vat::parse("NL 0044.95445.B01").unwrap();
    assert_eq!(vat.country(), Country::Netherlands);
    assert_eq!(vat.to_string(), "NL004495445B01");

    assert!(Vat::parse("NL004495446B01").is_err());
}
