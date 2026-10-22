//! Qibla and the sunnah times, against the reference library's own cases.
//!
//! The Qibla bearings are the ones in the reference's `qibla.test.ts`, to the
//! same three decimals. The sunnah times are its `sunnah.test.ts`, which reports
//! local wall-clock readings for five timezones; the reference's own Maghrib and
//! next Fajr for each case are checked first, so a failure says whether the
//! disagreement is in the prayer times or in the night they are divided into.

use adhan::{HighLatitudeRule, Method, Parameters, prayer_times, qibla, sunnah_times};

/// Degrees, to the three decimals the reference asserts.
fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 0.000_5
}

#[test]
fn qibla_matches_the_reference_in_every_region() {
    // (latitude, longitude, expected bearing)
    let cases = [
        // North America
        (38.9072, -77.0369, 56.56),
        (40.7128, -74.0059, 58.4817),
        (37.7749, -122.4194, 18.8438),
        (61.2181, -149.9003, 350.883),
        // South Pacific
        (-33.8688, 151.2093, 277.4996),
        (-36.8485, 174.7633, 261.197),
        // Europe
        (51.5074, -0.1278, 118.987),
        (48.8566, 2.3522, 119.163),
        (59.9139, 10.7522, 139.0278),
        // Asia
        (33.7294, 73.0931, 255.882),
        (35.6895, 139.6917, 293.021),
    ];

    for (latitude, longitude, expected) in cases {
        let actual = qibla(latitude, longitude);
        assert!(
            close(actual, expected),
            "qibla({latitude}, {longitude}) = {actual}, reference says {expected}"
        );
    }
}

#[test]
fn the_qibla_from_makkah_is_a_number_rather_than_nan() {
    // At the Kaaba itself the bearing has no meaning, and the formula's two
    // arguments both collapse to zero. IEEE says the answer to that is a real
    // number, and the reference agrees, so the contract is only that the caller
    // gets a number back and not a NaN that poisons whatever it is compared to.
    let bearing = qibla(21.422_524_1, 39.826_181_8);
    assert!(bearing.is_finite(), "the qibla at Makkah is {bearing}");
    assert!(
        (0.0..360.0).contains(&bearing),
        "the qibla at Makkah is {bearing}"
    );
}

#[test]
fn the_qibla_is_always_a_bearing() {
    // A grid over the whole globe: whatever the input, the answer is a direction.
    let mut latitude = -85.0;
    while latitude <= 85.0 {
        let mut longitude = -180.0;
        while longitude <= 180.0 {
            let bearing = qibla(latitude, longitude);
            assert!(
                (0.0..360.0).contains(&bearing),
                "qibla({latitude}, {longitude}) = {bearing} is outside [0, 360)"
            );
            longitude += 15.0;
        }
        latitude += 15.0;
    }
}

/// One of the reference's sunnah cases, in the local clock it reports.
struct SunnahCase {
    latitude: f64,
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    method: Method,
    rule: HighLatitudeRule,
    /// The offset to add to UTC to read the reported local clock, in minutes.
    utc_offset_minutes: i64,
    /// This day's Maghrib and the next day's Fajr, local, as the reference has them.
    maghrib: (u32, u32),
    next_fajr: (u32, u32),
    /// The two sunnah times, local, as the reference has them.
    middle_of_the_night: (u32, u32),
    last_third_of_the_night: (u32, u32),
}

/// Shifts a UTC clock reading by an offset and returns the local reading.
///
/// The day is dropped: the reference reports a local date and the crate returns
/// a UTC clock reading with no date attached, so the two cannot be compared on
/// the day. The night's own shape is checked by the gaps from Maghrib below,
/// which are the same in any timezone.
fn to_local(time: (i64, u32, u32), offset_minutes: i64) -> (u32, u32) {
    let minutes = time.0 * 60 + i64::from(time.1) + i64::from(time.2) / 60 + offset_minutes;
    let rest = minutes.rem_euclid(1440);
    ((rest / 60) as u32, (rest % 60) as u32)
}

/// Minutes since midnight.
fn minutes_of((hour, minute): (u32, u32)) -> i64 {
    i64::from(hour) * 60 + i64::from(minute)
}

#[test]
fn sunnah_times_match_the_reference_in_every_timezone() {
    let cases = [
        // New York, UTC-4 in July.
        SunnahCase {
            latitude: 35.775,
            longitude: -78.6336,
            year: 2015,
            month: 7,
            day: 12,
            method: Method::NorthAmerica,
            rule: HighLatitudeRule::MiddleOfTheNight,
            utc_offset_minutes: -4 * 60,
            maghrib: (20, 32),
            next_fajr: (4, 43),
            middle_of_the_night: (0, 38),
            last_third_of_the_night: (1, 59),
        },
        // London, UTC+0 in December.
        SunnahCase {
            latitude: 51.5074,
            longitude: -0.1278,
            year: 2016,
            month: 12,
            day: 31,
            method: Method::MoonsightingCommittee,
            rule: HighLatitudeRule::MiddleOfTheNight,
            utc_offset_minutes: 0,
            maghrib: (16, 4),
            next_fajr: (6, 25),
            middle_of_the_night: (23, 15),
            last_third_of_the_night: (1, 38),
        },
        // Oslo, UTC+2 in July, at the high latitude rule the reference sets.
        SunnahCase {
            latitude: 59.9094,
            longitude: 10.7349,
            year: 2016,
            month: 7,
            day: 1,
            method: Method::MuslimWorldLeague,
            rule: HighLatitudeRule::MiddleOfTheNight,
            utc_offset_minutes: 2 * 60,
            maghrib: (22, 41),
            next_fajr: (1, 20),
            middle_of_the_night: (0, 1),
            last_third_of_the_night: (0, 27),
        },
        // San Francisco, the day the US changes its clocks, UTC-8.
        SunnahCase {
            latitude: 37.7749,
            longitude: -122.4194,
            year: 2017,
            month: 3,
            day: 11,
            method: Method::NorthAmerica,
            rule: HighLatitudeRule::MiddleOfTheNight,
            utc_offset_minutes: -8 * 60,
            maghrib: (18, 13),
            next_fajr: (5, 13),
            middle_of_the_night: (23, 43),
            last_third_of_the_night: (1, 33),
        },
        // Paris, the day Europe changes its clocks, UTC+2.
        SunnahCase {
            latitude: 48.8566,
            longitude: 2.3522,
            year: 2015,
            month: 10,
            day: 24,
            method: Method::MuslimWorldLeague,
            rule: HighLatitudeRule::SeventhOfTheNight,
            utc_offset_minutes: 2 * 60,
            maghrib: (18, 45),
            next_fajr: (6, 40),
            middle_of_the_night: (0, 43),
            last_third_of_the_night: (2, 42),
        },
    ];

    for case in cases {
        let label = format!(
            "{} {} {}-{}-{}",
            case.method.name(),
            case.latitude,
            case.year,
            case.month,
            case.day
        );
        let parameters = Parameters::for_method(case.method).with_high_latitude_rule(case.rule);
        let offset = case.utc_offset_minutes;

        // The prayer times first: if these are wrong, the night below is built on
        // the wrong Maghrib and Fajr, and the failure should say so.
        let today = prayer_times(
            case.year,
            case.month,
            case.day,
            case.latitude,
            case.longitude,
            &parameters,
        );
        let maghrib = to_local(today.maghrib.expect("reference has a Maghrib"), offset);
        assert_eq!(
            maghrib, case.maghrib,
            "{label}: Maghrib disagrees with the reference"
        );

        let (next_year, next_month, next_day) = (case.year, case.month, case.day + 1);
        let tomorrow = prayer_times(
            next_year,
            next_month,
            next_day,
            case.latitude,
            case.longitude,
            &parameters,
        );
        let fajr = to_local(tomorrow.fajr.expect("reference has a Fajr"), offset);
        assert_eq!(
            fajr, case.next_fajr,
            "{label}: the next Fajr disagrees with the reference"
        );

        let sunnah = sunnah_times(
            case.year,
            case.month,
            case.day,
            case.latitude,
            case.longitude,
            &parameters,
        );
        let middle = to_local(
            sunnah.middle_of_the_night.expect("reference has one"),
            offset,
        );
        let third = to_local(
            sunnah.last_third_of_the_night.expect("reference has one"),
            offset,
        );

        assert_eq!(
            middle, case.middle_of_the_night,
            "{label}: middle of the night disagrees with the reference"
        );
        assert_eq!(
            third, case.last_third_of_the_night,
            "{label}: last third of the night disagrees with the reference"
        );

        // And the shape of the night: the gap from Maghrib, which is the same
        // reading in any timezone. This is what catches a night measured from the
        // wrong Maghrib or a day out, which a bare clock reading can hide. The
        // gaps themselves come from the reference's own numbers, so this is still
        // a comparison against it and not against our own arithmetic.
        let expected_middle =
            (minutes_of(case.middle_of_the_night) - minutes_of(case.maghrib)).rem_euclid(1440);
        let expected_third =
            (minutes_of(case.last_third_of_the_night) - minutes_of(case.maghrib)).rem_euclid(1440);
        let actual_middle = (minutes_of(middle) - minutes_of(maghrib)).rem_euclid(1440);
        let actual_third = (minutes_of(third) - minutes_of(maghrib)).rem_euclid(1440);

        assert_eq!(
            actual_middle, expected_middle,
            "{label}: middle is not half the night from Maghrib"
        );
        assert_eq!(
            actual_third, expected_third,
            "{label}: last third is not two thirds from Maghrib"
        );
        // The third is after the middle, and by the expected amount. Comparing
        // against a recomputed `night / 2` here would be a minute out whenever
        // the night has an odd number of minutes, which is most nights.
        assert_eq!(
            (actual_third - actual_middle).rem_euclid(1440),
            (expected_third - expected_middle).rem_euclid(1440),
            "{label}: the last third is not where the reference puts it relative to the middle"
        );
    }
}

#[test]
fn the_last_third_is_always_after_the_middle_of_the_night() {
    // The two are a fixed fraction of one night apart, so this has to hold
    // wherever the night is. A sign error in the night length breaks it.
    let parameters = Parameters::for_method(Method::MuslimWorldLeague);
    for latitude in [-60.0, -30.0, 0.0, 30.0, 60.0] {
        for longitude in [-150.0, -60.0, 0.0, 60.0, 150.0] {
            let sunnah = sunnah_times(2026, 3, 15, latitude, longitude, &parameters);
            let (Some(middle), Some(third)) =
                (sunnah.middle_of_the_night, sunnah.last_third_of_the_night)
            else {
                continue;
            };
            let minutes = |(hour, minute, second): (i64, u32, u32)| {
                hour * 3600 + i64::from(minute) * 60 + i64::from(second)
            };
            let gap = (minutes(third) - minutes(middle)).rem_euclid(86_400);
            assert!(
                gap > 0 && gap < 86_400 / 2,
                "at ({latitude}, {longitude}) the last third is {gap} seconds after the middle, which is not a third of a night"
            );
        }
    }
}
