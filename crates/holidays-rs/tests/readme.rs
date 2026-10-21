//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use holidays_rs::{Calendar, Date};

    let cal = Calendar::indonesia();
    let d = Date::new(2026, 8, 17).unwrap();
    assert!(cal.is_holiday(d));
    assert_eq!(cal.next_working_day(d), Date::new(2026, 8, 18).unwrap());
}
