//! Checks the conversions against `hijridate`, the reference implementation.

use hijridate::{Gregorian, Hijri};

/// Replays `fixtures/hijridate_vectors.tsv`, whose rows were produced by the
/// `hijridate` Python package (MIT, (c) Mohammed Alshehri), the same package the
/// Umm al-Qura table was generated from.
///
/// The rows cover both directions over the whole tabulated range, the month
/// lengths and the weekday, so a change to the table, to the Julian day
/// arithmetic or to the boundary between the tabulated and the arithmetic
/// calendar fails the build.
#[test]
fn every_conversion_agrees_with_hijridate() {
    let fixture = include_str!("fixtures/hijridate_vectors.tsv");
    let (mut compared, mut disagreements) = (0u32, Vec::new());
    for line in fixture.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 4 {
            continue;
        }
        let (kind, y, m) = (
            f[0],
            f[1].parse::<i32>().unwrap(),
            f[2].parse::<u8>().unwrap(),
        );
        compared += 1;
        match kind {
            "H" => {
                let (d, want) = (f[3].parse::<u8>().unwrap(), (f[4], f[5], f[6]));
                let g = Hijri::new(y, m, d).unwrap().to_gregorian();
                let got = (g.year(), i32::from(g.month()), i32::from(g.day()));
                let want = (
                    want.0.parse::<i32>().unwrap(),
                    want.1.parse::<i32>().unwrap(),
                    want.2.parse::<i32>().unwrap(),
                );
                if got != want {
                    disagreements.push(format!("H {y}-{m}-{d} {got:?} != {want:?}"));
                }
            }
            "G" => {
                let (d, want) = (f[3].parse::<u8>().unwrap(), (f[4], f[5], f[6]));
                let h = Gregorian::new(y, m, d).unwrap().to_hijri();
                let got = (h.year(), i32::from(h.month()), i32::from(h.day()));
                let want = (
                    want.0.parse::<i32>().unwrap(),
                    want.1.parse::<i32>().unwrap(),
                    want.2.parse::<i32>().unwrap(),
                );
                if got != want {
                    disagreements.push(format!("G {y}-{m}-{d} {got:?} != {want:?}"));
                }
            }
            "L" => {
                let want = f[3].parse::<u8>().unwrap();
                let got = Hijri::new(y, m, 1).unwrap().month_length();
                if got != want {
                    disagreements.push(format!("L {y}-{m} {got} != {want}"));
                }
            }
            "V" => {
                let (d, want) = (f[3].parse::<u8>().unwrap(), f[4] == "1");
                if Hijri::new(y, m, d).is_ok() != want {
                    disagreements.push(format!("V {y}-{m}-{d}"));
                }
            }
            "W" => {
                let (d, want) = (f[3].parse::<u8>().unwrap(), f[4].parse::<usize>().unwrap());
                let got = Gregorian::new(y, m, d).unwrap().weekday();
                if got != want {
                    disagreements.push(format!("W {y}-{m}-{d} {got} != {want}"));
                }
            }
            _ => {}
        }
    }
    assert!(compared > 7000, "only {compared} rows were compared");
    assert!(
        disagreements.is_empty(),
        "{} rows disagree with hijridate: {:?}",
        disagreements.len(),
        &disagreements[..disagreements.len().min(20)]
    );
}
