//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use hijridate::{Gregorian, Hijri};

    let h = Hijri::new(1445, 9, 1).unwrap();
    assert_eq!(h.to_gregorian().to_string(), "2024-03-11");
    assert_eq!(h.month_name(), "Ramadan");

    let g = Gregorian::new(2024, 3, 11).unwrap();
    assert_eq!(g.to_hijri().to_string(), "1445-09-01");
}
