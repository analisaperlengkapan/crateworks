//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use iso6346::{Category, Container, check_digit, validate};

    let container = Container::parse("csqu-305438-3").unwrap();
    assert_eq!(container.owner_code(), "CSQ");
    assert_eq!(container.category(), Category::Freight);
    assert_eq!(container.serial(), 305_438);
    assert_eq!(container.check_digit(), 3);
    assert_eq!(container.to_string(), "CSQU3054383");

    assert!(validate("CSQU3054384").is_err());

    let generated = Container::new("CSQ", Category::Freight, 305_438).unwrap();
    assert_eq!(generated.to_string(), "CSQU3054383");

    assert_eq!(check_digit("CSQ", Category::Freight, 305_438), Some(3));
}
