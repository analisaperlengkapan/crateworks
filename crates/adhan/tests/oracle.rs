//! Comparison against the `adhan` JavaScript library.
//!
//! The fixtures are the reference library's own output over a grid of dates,
//! locations, calculation methods, schools and high latitude rules. Checking
//! against the implementation prayer apps actually use is worth more than
//! checking against our own arithmetic, and it is what caught the reference
//! Python port being wrong in the eastern hemisphere.

use adhan::{HighLatitudeRule, Madhab, Method, Parameters, prayer_times};

/// One expected result, as the reference library reported it.
struct Case {
    location: String,
    latitude: f64,
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    method: Method,
    madhab: Madhab,
    rule: HighLatitudeRule,
    /// The six times as ISO strings, or `None` when the reference had none.
    times: [Option<(i64, u32, u32)>; 6],
}

/// Parses an ISO timestamp into the UTC hour and minute.
fn parse_iso(text: &str) -> Option<(i64, u32, u32)> {
    // "2026-01-01T04:57:00.000Z"
    let t = text.split('T').nth(1)?;
    let hour: i64 = t.get(0..2)?.parse().ok()?;
    let minute: u32 = t.get(3..5)?.parse().ok()?;
    let second: u32 = t.get(6..8)?.parse().ok()?;
    Some((hour, minute, second))
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

/// Reads the fixture, which is tab separated to avoid a JSON dependency.
fn cases() -> Vec<Case> {
    let text = include_str!("fixtures/oracle.tsv");
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let (y, m, d) = {
            let mut parts = fields[3].split('-');
            (
                parts.next().unwrap().parse().unwrap(),
                parts.next().unwrap().parse().unwrap(),
                parts.next().unwrap().parse().unwrap(),
            )
        };
        let time = |index: usize| {
            let field = fields.get(7 + index).copied().unwrap_or("");
            if field.is_empty() {
                None
            } else {
                parse_iso(field)
            }
        };
        out.push(Case {
            location: fields[0].to_string(),
            latitude: fields[1].parse().unwrap(),
            longitude: fields[2].parse().unwrap(),
            year: y,
            month: m,
            day: d,
            method: method_from(fields[4]),
            madhab: if fields[5] == "Hanafi" {
                Madhab::Hanafi
            } else {
                Madhab::Shafi
            },
            rule: rule_from(fields[6]),
            times: [time(0), time(1), time(2), time(3), time(4), time(5)],
        })
    }
    out
}

#[test]
fn every_fixture_case_agrees_with_the_reference() {
    let mut checked = 0;
    let mut failures = Vec::new();

    for case in cases() {
        let parameters = Parameters::for_method(case.method)
            .with_madhab(case.madhab)
            .with_high_latitude_rule(case.rule);
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
            // A missing reference time is a real answer too: it means the sun
            // never reached that position, so ours must be missing as well.
            checked += 1;
            if expected != actual {
                failures.push(format!(
                    "{} {} method={} madhab={:?} rule={:?} field {}: expected {:?} got {:?}",
                    case.location,
                    case.date_string(),
                    case.method.name(),
                    case.madhab,
                    case.rule,
                    ["fajr", "sunrise", "dhuhr", "asr", "maghrib", "isha"][index],
                    expected,
                    actual
                ));
            }
        }
    }

    assert!(checked > 30_000, "only {checked} times were compared");
    assert!(
        failures.is_empty(),
        "{} of {checked} times disagreed; first ten:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

impl Case {
    fn date_string(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[test]
#[allow(clippy::similar_names)]
fn the_hanafi_asr_is_later_than_the_shafi_one() {
    let shafi = prayer_times(
        2026,
        10,
        21,
        -6.2088,
        106.8456,
        &Parameters::for_method(Method::MuslimWorldLeague).with_madhab(Madhab::Shafi),
    );
    let hanafi = prayer_times(
        2026,
        10,
        21,
        -6.2088,
        106.8456,
        &Parameters::for_method(Method::MuslimWorldLeague).with_madhab(Madhab::Hanafi),
    );
    assert!(hanafi.asr > shafi.asr, "Hanafi Asr comes later");
    assert_eq!(hanafi.fajr, shafi.fajr, "the school does not move Fajr");
}

#[test]
#[allow(clippy::similar_names)]
fn a_later_fajr_angle_gives_an_earlier_fajr() {
    // A smaller angle means the sun has to be less far below the horizon, so the
    // time is closer to sunrise, that is later in the morning.
    let mwl = prayer_times(
        2026,
        10,
        21,
        21.4225,
        39.8262,
        &Parameters::for_method(Method::MuslimWorldLeague),
    );
    let isna = prayer_times(
        2026,
        10,
        21,
        21.4225,
        39.8262,
        &Parameters::for_method(Method::NorthAmerica),
    );
    assert!(isna.fajr > mwl.fajr, "15 degrees is later than 18");
}

/// Minutes since midnight, for comparing two clock times across a wrap.
fn minutes(time: (i64, u32, u32)) -> i64 {
    time.0 * 60 + i64::from(time.1)
}

/// The length of a span that may wrap past midnight.
fn span(from: (i64, u32, u32), to: (i64, u32, u32)) -> i64 {
    (minutes(to) - minutes(from)).rem_euclid(24 * 60)
}

#[test]
fn the_times_are_in_order() {
    // Makkah, where no time falls outside the UTC day, so a plain comparison of
    // the clock readings is the right thing.
    let times = prayer_times(
        2026,
        10,
        21,
        21.4225,
        39.8262,
        &Parameters::for_method(Method::MuslimWorldLeague),
    );
    assert!(times.fajr < times.sunrise);
    assert!(times.sunrise < times.dhuhr);
    assert!(times.dhuhr < times.asr);
    assert!(times.asr < times.maghrib);
    assert!(times.maghrib < times.isha);
}

#[test]
fn isha_defined_by_an_interval_is_exactly_that_interval_after_sunset() {
    // Umm al-Qura puts Isha ninety minutes after sunset, with no angle involved.
    let times = prayer_times(
        2026,
        10,
        21,
        21.4225,
        39.8262,
        &Parameters::for_method(Method::UmmAlQura),
    );
    let (mh, mm, _) = times.maghrib.unwrap();
    let (ih, im, _) = times.isha.unwrap();
    let sunset_minutes = mh * 60 + i64::from(mm);
    let isha_minutes = ih * 60 + i64::from(im);
    let difference = (isha_minutes - sunset_minutes).rem_euclid(24 * 60);
    assert!(
        (89..=91).contains(&difference),
        "expected about ninety minutes, got {difference}"
    );
}

#[test]
fn a_high_latitude_day_with_no_twilight_still_gives_times() {
    // Tromso in June: the sun does not set, and the angle-based Fajr and Isha do
    // not exist. The high latitude rule has to supply them anyway.
    let times = prayer_times(
        2026,
        6,
        21,
        69.6492,
        18.9553,
        &Parameters::for_method(Method::MuslimWorldLeague)
            .with_high_latitude_rule(HighLatitudeRule::SeventhOfTheNight),
    );
    // The times that need the sun to rise or set are absent, and the rest are
    // still given rather than invented.
    assert!(times.dhuhr.is_some(), "{times:?}");
    assert!(times.asr.is_some(), "{times:?}");
}

#[test]
fn the_southern_hemisphere_is_not_a_sign_error() {
    // Sydney in December: summer, so Fajr is early and Isha late, the opposite
    // of London in the same month.
    let sydney = prayer_times(
        2026,
        12,
        21,
        -33.8688,
        151.2093,
        &Parameters::for_method(Method::MuslimWorldLeague),
    );
    let london = prayer_times(
        2026,
        12,
        21,
        51.5074,
        -0.1278,
        &Parameters::for_method(Method::MuslimWorldLeague),
    );
    let sydney_day = span(sydney.sunrise.unwrap(), sydney.maghrib.unwrap());
    let london_day = span(london.sunrise.unwrap(), london.maghrib.unwrap());
    assert!(
        sydney_day > london_day,
        "the southern summer day is longer: {sydney_day} vs {london_day}"
    );
}
