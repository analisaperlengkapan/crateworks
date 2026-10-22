//! Comparison of the navigation helpers against the `adhan` JavaScript library.
//!
//! For each case the reference was asked which prayer had begun and which comes
//! next at a given instant. The corpus is drawn from a deterministic generator
//! (`scripts/fuzz/adhan/gen_nav.js`), and a case whose six times did not all land
//! on the same UTC date was dropped rather than written with a guess: this crate
//! returns a clock reading with no day attached, so a time that rolls over
//! midnight is not the same comparison the reference makes.

use adhan::{HighLatitudeRule, Madhab, Method, Parameters, Prayer, Rounding, prayer_times};

/// One expected result, as the reference library reported it.
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
    /// The clock reading the question was asked at, in UTC.
    now: (i64, u32, u32),
    /// The reference's answer to `currentPrayer`.
    current: Option<Prayer>,
    /// The reference's answer to `nextPrayer`.
    next: Option<Prayer>,
}

fn parse_clock(text: &str) -> (i64, u32, u32) {
    let mut parts = text.split(':');
    (
        parts.next().unwrap().parse().unwrap(),
        parts.next().unwrap().parse().unwrap(),
        parts.next().unwrap().parse().unwrap(),
    )
}

fn prayer_from(name: &str) -> Option<Prayer> {
    match name {
        "Fajr" => Some(Prayer::Fajr),
        "Sunrise" => Some(Prayer::Sunrise),
        "Dhuhr" => Some(Prayer::Dhuhr),
        "Asr" => Some(Prayer::Asr),
        "Maghrib" => Some(Prayer::Maghrib),
        "Isha" => Some(Prayer::Isha),
        "None" => None,
        other => panic!("unknown prayer {other}"),
    }
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
    let text = include_str!("fixtures/navigation.tsv");
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        out.push(Case {
            latitude: f[1].parse().unwrap(),
            longitude: f[2].parse().unwrap(),
            year: f[3].parse().unwrap(),
            month: f[4].parse().unwrap(),
            day: f[5].parse().unwrap(),
            method: method_from(f[6]),
            madhab: if f[7] == "Hanafi" {
                Madhab::Hanafi
            } else {
                Madhab::Shafi
            },
            rule: rule_from(f[8]),
            rounding: rounding_from(f[9]),
            now: parse_clock(f[10]),
            current: prayer_from(f[11]),
            next: prayer_from(f[12]),
        });
    }
    out
}

#[test]
fn navigation_agrees_with_the_reference() {
    let mut checked = 0;
    let mut failures = Vec::new();

    for case in cases() {
        let mut parameters = Parameters::for_method(case.method)
            .with_madhab(case.madhab)
            .with_high_latitude_rule(case.rule);
        parameters.rounding = case.rounding;
        let times = prayer_times(
            case.year,
            case.month,
            case.day,
            case.latitude,
            case.longitude,
            &parameters,
        );

        let current = times.current_prayer(case.now);
        let next = times.next_prayer(case.now);
        if current != case.current || next != case.next {
            failures.push(format!(
                "{} at {} {}-{:02}-{:02} now={:?}: current {current:?} want {:?}, next {next:?} want {:?}",
                case.latitude, case.longitude, case.year, case.month, case.day, case.now, case.current, case.next
            ));
        }
        checked += 1;
    }

    assert!(checked > 3000, "expected a large corpus, got {checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} cases disagreed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
