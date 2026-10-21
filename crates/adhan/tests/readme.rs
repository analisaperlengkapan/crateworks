//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use adhan::{Madhab, Method, Parameters, prayer_times};

    let times = prayer_times(
        2026,
        10,
        21, // the date
        21.4225,
        39.8262, // Makkah
        &Parameters::for_method(Method::MuslimWorldLeague).with_madhab(Madhab::Shafi),
    );

    let (hour, minute, _second) = times.dhuhr.unwrap();
    assert_eq!(format!("{hour:02}:{minute:02}"), "09:06");
}
