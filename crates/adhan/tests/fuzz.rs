//! A corpus of randomly chosen locations, dates and parameters, computed by the
//! reference and by this crate.
//!
//! The curated fixtures walk a grid of ten cities on eight known dates. That is
//! good at answering "does Makkah in October agree" and bad at the spaces
//! between: latitudes near the poles, longitudes near the date line, small-date
//! years, and the two settings no curated case touched at all - the manual
//! `adjustments` and `Rounding::None`.
//!
//! Six thousand cases are drawn from a deterministic generator, so the corpus
//! can be rebuilt exactly, over latitudes weighted to the polar circles and
//! everywhere else, all thirteen methods, both schools, all three high latitude
//! rules, all three rounding modes, and adjustments from five minutes before to
//! five minutes after.
//!
//! It found one real gap: `Rounding::None` keeps the seconds the reference's own
//! arithmetic produces, and a result of whole minutes could not express them.
//! The times now carry seconds.
//!
//! The reference reports an absolute instant, and a time drawn near a pole may
//! land on the day before or after. Only the clock reading is compared, which is
//! what this crate returns.

use adhan::{HighLatitudeRule, Madhab, Method, Parameters, Rounding, prayer_times};

const CORPUS: &str = include_str!("fixtures/fuzz.tsv");

/// One expected result: the parameters, and six times as `hh:mm:ss` or `None`.
struct Case {
    latitude: f64,
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    method: Method,
    madhab: Madhab,
    rule: HighLatitudeRule,
    rounding: Rounding,
    adjustments: [f64; 6],
    times: [Option<(i64, u32, u32)>; 6],
}

/// The clock reading of an ISO instant, ignoring the date.
fn clock(text: &str) -> Option<(i64, u32, u32)> {
    let time = text.split('T').nth(1)?;
    Some((
        time.get(0..2)?.parse().ok()?,
        time.get(3..5)?.parse().ok()?,
        time.get(6..8)?.parse().ok()?,
    ))
}

fn method_from(name: &str) -> Method {
    match name {
        "MuslimWorldLeague" => Method::MuslimWorldLeague,
        "Egyptian" => Method::Egyptian,
        "Karachi" => Method::Karachi,
        "UmmAlQura" => Method::UmmAlQura,
        "Dubai" => Method::Dubai,
        "MoonsightingCommittee" => Method::MoonsightingCommittee,
        "NorthAmerica" => Method::NorthAmerica,
        "Kuwait" => Method::Kuwait,
        "Qatar" => Method::Qatar,
        "Singapore" => Method::Singapore,
        "Tehran" => Method::Tehran,
        "Turkey" => Method::Turkey,
        "Other" => Method::Other,
        other => panic!("unknown method {other}"),
    }
}

fn rule_from(name: &str) -> HighLatitudeRule {
    match name {
        "MiddleOfTheNight" => HighLatitudeRule::MiddleOfTheNight,
        "SeventhOfTheNight" => HighLatitudeRule::SeventhOfTheNight,
        "TwilightAngle" => HighLatitudeRule::TwilightAngle,
        other => panic!("unknown rule {other}"),
    }
}

fn rounding_from(name: &str) -> Rounding {
    match name {
        "nearest" => Rounding::Nearest,
        "up" => Rounding::Up,
        "none" => Rounding::None,
        other => panic!("unknown rounding {other}"),
    }
}

fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    for line in CORPUS.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let date: Vec<&str> = f[2].split('-').collect();
        let time = |column: usize| {
            let field = f[column];
            if field == "null" { None } else { clock(field) }
        };
        out.push(Case {
            latitude: f[0].parse().unwrap(),
            longitude: f[1].parse().unwrap(),
            year: date[0].parse().unwrap(),
            month: date[1].parse().unwrap(),
            day: date[2].parse().unwrap(),
            method: method_from(f[3]),
            madhab: if f[4] == "Hanafi" {
                Madhab::Hanafi
            } else {
                Madhab::Shafi
            },
            rule: rule_from(f[5]),
            rounding: rounding_from(f[6]),
            adjustments: [
                f[7].parse().unwrap(),
                f[8].parse().unwrap(),
                f[9].parse().unwrap(),
                f[10].parse().unwrap(),
                f[11].parse().unwrap(),
                f[12].parse().unwrap(),
            ],
            times: [time(13), time(14), time(15), time(16), time(17), time(18)],
        })
    }
    out
}

#[test]
fn every_random_case_agrees_with_the_reference() {
    let cases = cases();
    assert!(cases.len() >= 6000, "only {} cases", cases.len());

    let mut failures = Vec::new();
    for case in &cases {
        let mut parameters = Parameters::for_method(case.method)
            .with_madhab(case.madhab)
            .with_high_latitude_rule(case.rule);
        parameters.rounding = case.rounding;
        parameters.fajr_minutes = case.adjustments[0];
        parameters.sunrise_minutes = case.adjustments[1];
        parameters.dhuhr_minutes = case.adjustments[2];
        parameters.asr_minutes = case.adjustments[3];
        parameters.maghrib_minutes = case.adjustments[4];
        parameters.isha_minutes = case.adjustments[5];

        let got = prayer_times(
            case.year,
            case.month,
            case.day,
            case.latitude,
            case.longitude,
            &parameters,
        );
        let ours = [
            got.fajr,
            got.sunrise,
            got.dhuhr,
            got.asr,
            got.maghrib,
            got.isha,
        ];

        for (index, (expected, actual)) in case.times.iter().zip(ours.iter()).enumerate() {
            if expected == actual {
                continue;
            }
            failures.push(format!(
                "lat={} lon={} date={}-{}-{} method={} madhab={:?} rule={:?} rounding={:?} adj={:?} field {index}: expected {expected:?} got {actual:?}",
                case.latitude,
                case.longitude,
                case.year,
                case.month,
                case.day,
                case.method.name(),
                case.madhab,
                case.rule,
                case.rounding,
                case.adjustments,
            ));
        }
    }

    assert!(
        failures.len() < 10,
        "{} failures, first few:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        failures.is_empty(),
        "{} of {} cases disagreed:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn keeping_the_seconds_is_not_the_same_as_rounding_them_away() {
    // The correction the corpus forced: with `Rounding::None` the reference
    // reports the second its arithmetic produced, so a whole-minute result
    // cannot reproduce it.
    let base = Parameters::for_method(Method::MuslimWorldLeague);
    let mut unrounded = base;
    unrounded.rounding = Rounding::None;
    let times = prayer_times(2026, 10, 21, 21.4225, 39.8262, &unrounded);
    let (_, _, second) = times.fajr.unwrap();
    assert!(
        second != 0,
        "the reference keeps the second here: {times:?}"
    );

    let mut rounded = base;
    rounded.rounding = Rounding::Nearest;
    let rounded_times = prayer_times(2026, 10, 21, 21.4225, 39.8262, &rounded);
    assert_eq!(rounded_times.fajr.unwrap().2, 0);
}
