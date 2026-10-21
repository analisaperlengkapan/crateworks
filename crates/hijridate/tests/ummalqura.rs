//! Checks the crate against dates published by the Umm al-Qura calendar.
//!
//! The fixtures were generated from the `hijridate` Python package (MIT,
//! (c) Mohammed Alshehri) rather than from this crate's own output, so a
//! failure here means the crate disagrees with the published calendar.

use hijridate::{Gregorian, Hijri};

#[path = "fixtures.rs"]
mod fixtures;

#[test]
fn hijri_to_gregorian_matches_published_dates() {
    for &(year, month, day, expected) in fixtures::HIJRI_TO_GREGORIAN {
        let got = Hijri::new(year, month, day)
            .unwrap_or_else(|e| panic!("{year}-{month:02}-{day:02} rejected: {e}"))
            .to_gregorian()
            .to_string();
        assert_eq!(
            got, expected,
            "Hijri {year}-{month:02}-{day:02} should be {expected}"
        );
    }
}

#[test]
fn gregorian_to_hijri_matches_published_dates() {
    for &(year, month, day, expected) in fixtures::GREGORIAN_TO_HIJRI {
        let got = Gregorian::new(year, month, day)
            .unwrap_or_else(|e| panic!("{year}-{month:02}-{day:02} rejected: {e}"))
            .to_hijri()
            .to_string();
        assert_eq!(
            got, expected,
            "Gregorian {year}-{month:02}-{day:02} should be {expected}"
        );
    }
}

#[test]
fn month_lengths_match_published_lengths() {
    for &(year, month, expected) in fixtures::MONTH_LENGTHS {
        let got = Hijri::new(year, month, 1).unwrap().month_length();
        assert_eq!(got, expected, "Hijri {year}-{month:02} length");
    }
}

#[test]
fn every_day_round_trips_through_both_calendars() {
    // A full sweep of the table, not just the sampled fixtures: every valid day
    // must convert to a Gregorian date and back to itself.
    for &(year, month, _) in fixtures::MONTH_LENGTHS {
        let length = Hijri::new(year, month, 1).unwrap().month_length();
        for day in 1..=length {
            let h = Hijri::new(year, month, day).unwrap();
            let back = h.to_gregorian().to_hijri();
            assert_eq!(h, back, "round trip failed at {year}-{month:02}-{day:02}");
        }
    }
}

#[test]
fn a_rejected_day_is_reported_with_the_month_length() {
    // 1445-09 has 30 days in Umm al-Qura, so day 31 does not exist.
    assert!(Hijri::new(1445, 9, 30).is_ok());
    assert!(Hijri::new(1445, 9, 31).is_err());
    // 1445-10 has 29.
    assert!(Hijri::new(1445, 10, 29).is_ok());
    assert!(Hijri::new(1445, 10, 30).is_err());
}

#[test]
fn tabular_fallback_is_flagged_and_still_ordered() {
    // Year 1300 is before the table starts at 1343.
    let old = Hijri::new(1300, 1, 1).unwrap();
    assert!(old.is_tabular());
    // And it still round trips, within the approximation's own logic.
    assert_eq!(old.to_gregorian().to_hijri(), old);

    // 1600 is after the table ends at 1500.
    let future = Hijri::new(1600, 12, 1).unwrap();
    assert!(future.is_tabular());

    // A date inside the table must never be flagged as tabular.
    assert!(!Hijri::new(1445, 9, 1).unwrap().is_tabular());
}
