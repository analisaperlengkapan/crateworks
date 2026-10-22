//! Checks the times, the qibla and the sunnah times against adhan-js.
//!
//! The rows in `tests/fixtures/adhan_js_*.tsv` were produced by the `adhan`
//! JavaScript package (MIT, (c) Batoul Apps), which is the reference this crate
//! follows. The Python port is not used as the oracle: its minute rounding uses
//! Python's banker's rounding, so a time that lands on exactly thirty seconds
//! comes out a minute earlier than the reference. See the README.

use adhan::{HighLatitudeRule, Madhab, Method, Parameters, prayer_times, qibla, sunnah_times};

const NAMES: [&str; 6] = ["fajr", "sunrise", "dhuhr", "asr", "maghrib", "isha"];

fn method(name: &str) -> Option<Method> {
    Some(match name {
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
        _ => return None,
    })
}

fn format(time: Option<(i64, u32, u32)>) -> String {
    match time {
        None => "None".to_string(),
        Some((h, m, s)) => format!("{h:02}:{m:02}:{s:02}"),
    }
}

#[test]
fn every_time_agrees_with_adhan_js() {
    let fixture = include_str!("fixtures/adhan_js_times.tsv");
    let (mut compared, mut disagreements) = (0u32, Vec::new());
    for line in fixture.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 13 {
            continue;
        }
        let (lat, lon): (f64, f64) = (f[0].parse().unwrap(), f[1].parse().unwrap());
        let (y, m, d): (i32, u32, u32) = (
            f[2].parse().unwrap(),
            f[3].parse().unwrap(),
            f[4].parse().unwrap(),
        );
        let Some(meth) = method(f[5]) else { continue };
        let mut p = Parameters::for_method(meth);
        if f[6] == "Hanafi" {
            p = p.with_madhab(Madhab::Hanafi);
        }
        let t = prayer_times(y, m, d, lat, lon, &p);
        let ours = [t.fajr, t.sunrise, t.dhuhr, t.asr, t.maghrib, t.isha];
        for (i, name) in NAMES.iter().enumerate() {
            compared += 1;
            let got = format(ours[i]);
            if got != f[7 + i] {
                disagreements.push(format!(
                    "{name} {lat},{lon} {y}-{m}-{d} {} {}: {got} != {}",
                    f[5],
                    f[6],
                    f[7 + i]
                ));
            }
        }
    }
    assert!(compared > 10_000, "only {compared} times were compared");
    assert!(
        disagreements.is_empty(),
        "{} times disagree with adhan-js: {:?}",
        disagreements.len(),
        &disagreements[..disagreements.len().min(20)]
    );
}

#[test]
fn every_high_latitude_time_agrees_with_adhan_js() {
    let fixture = include_str!("fixtures/adhan_js_high_latitude.tsv");
    let (mut compared, mut disagreements) = (0u32, Vec::new());
    for line in fixture.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 14 {
            continue;
        }
        let (lat, lon): (f64, f64) = (f[0].parse().unwrap(), f[1].parse().unwrap());
        let (y, m, d): (i32, u32, u32) = (
            f[2].parse().unwrap(),
            f[3].parse().unwrap(),
            f[4].parse().unwrap(),
        );
        let rule = match f[5] {
            "MiddleOfTheNight" => HighLatitudeRule::MiddleOfTheNight,
            "SeventhOfTheNight" => HighLatitudeRule::SeventhOfTheNight,
            "TwilightAngle" => HighLatitudeRule::TwilightAngle,
            _ => continue,
        };
        let p = Parameters::for_method(Method::MuslimWorldLeague).with_high_latitude_rule(rule);
        let t = prayer_times(y, m, d, lat, lon, &p);
        let s = sunnah_times(y, m, d, lat, lon, &p);
        let ours = [
            t.fajr,
            t.sunrise,
            t.dhuhr,
            t.asr,
            t.maghrib,
            t.isha,
            s.middle_of_the_night,
            s.last_third_of_the_night,
        ];
        for (i, want) in f[6..].iter().enumerate() {
            compared += 1;
            let got = format(ours[i]);
            if got != *want {
                disagreements.push(format!(
                    "field{i} {lat},{lon} {y}-{m}-{d} {}: {got} != {want}",
                    f[5]
                ));
            }
        }
    }
    assert!(compared > 1_000, "only {compared} values were compared");
    assert!(
        disagreements.is_empty(),
        "{} values disagree with adhan-js: {:?}",
        disagreements.len(),
        &disagreements[..disagreements.len().min(20)]
    );
}

#[test]
fn every_qibla_agrees_with_adhan_js() {
    let fixture = include_str!("fixtures/adhan_js_qibla.tsv");
    let mut compared = 0u32;
    for line in fixture.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 3 {
            continue;
        }
        let (lat, lon): (f64, f64) = (f[0].parse().unwrap(), f[1].parse().unwrap());
        let want: f64 = f[2].parse().unwrap();
        let got = qibla(lat, lon);
        assert!(
            (got - want).abs() < 1e-5,
            "qibla at {lat},{lon}: {got:.6} != {want:.6}"
        );
        compared += 1;
    }
    assert!(compared > 10, "only {compared} bearings were compared");
}
