//! Runs the README examples, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use factur_x::{Syntax, generate, parse, validate};

    let invoice = factur_x::test_support::sample_invoice();
    assert!(validate(&invoice).is_valid());

    let xml = generate(&invoice, Syntax::Cii);
    assert!(xml.starts_with("<?xml"));

    let back = parse(&xml, Syntax::Cii).unwrap();
    assert_eq!(back, invoice);
}

#[test]
fn readme_validation_example() {
    use factur_x::{Amount, validate};

    let mut invoice = factur_x::test_support::sample_invoice();
    invoice.tax_breakdown[0].tax_amount = Amount::parse("180.00").unwrap();

    let report = validate(&invoice);
    assert!(!report.is_valid());
    assert!(report.rules().contains(&"BR-CO-17"));
}
