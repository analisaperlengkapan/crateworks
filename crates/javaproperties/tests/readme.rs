//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use javaproperties::parse;

    let pairs = parse(b"name = cipansor\ncity\\:bandung = west java\n").unwrap();
    assert_eq!(pairs[0], ("name".to_string(), "cipansor".to_string()));
    // The escaped colon is part of the key, not a separator.
    assert_eq!(
        pairs[1],
        ("city:bandung".to_string(), "west java".to_string())
    );
}
