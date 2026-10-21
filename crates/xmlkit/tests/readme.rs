//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use xmlkit::{local_name, parse};

    let doc = parse(r#"<ns:Doc xmlns:ns="urn:x"><ns:Id>7</ns:Id></ns:Doc>"#).unwrap();
    assert_eq!(local_name(&doc.name), "Doc");
    assert_eq!(doc.deep_text(), "7");
}
