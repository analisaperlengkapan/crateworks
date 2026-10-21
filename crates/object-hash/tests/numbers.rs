//! Number formatting against JavaScript's own output.
//!
//! Rust and JavaScript agree on the digits of a float but disagree about where
//! to put the decimal point. `1e21` is twenty-two characters in Rust and `1e+21`
//! in JavaScript; `1e-7` is `0.0000001` in Rust and `1e-7` in JavaScript. Since
//! the digest has to match the reference, the text has to match too, so the
//! cases at both ends of the range are checked here rather than trusted.
//!
//! The fixture records the value as JavaScript's own `String(value)`, which is
//! also a valid Rust literal, so the same number can be read back and formatted
//! independently.

use object_hash::{Value, canonical, hash};

/// A number case: its name, the text JavaScript gives the value, the canonical
/// form, and the digest.
struct Case {
    name: String,
    value: String,
    canonical: String,
    sha1: String,
}

fn cases() -> Vec<Case> {
    include_str!("fixtures/numbers.tsv")
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            Case {
                name: f[0].to_string(),
                value: f[1].to_string(),
                canonical: f[2].to_string(),
                sha1: f[3].to_string(),
            }
        })
        .collect()
}

#[test]
fn every_number_formats_the_way_javascript_formats_it() {
    let cases = cases();
    assert!(cases.len() >= 18, "only {} cases", cases.len());

    let mut failures = Vec::new();
    for case in &cases {
        // JavaScript's text for a finite number is also valid Rust syntax.
        let Ok(number) = case.value.parse::<f64>() else {
            failures.push(format!("{}: cannot parse {:?}", case.name, case.value));
            continue;
        };
        let value = Value::Number(number);
        let got_canonical = canonical(&value);
        if got_canonical != case.canonical {
            failures.push(format!(
                "{}: canonical\n  expected {:?}\n  got      {:?}",
                case.name, case.canonical, got_canonical
            ));
        }
        let got_digest = hash(&value);
        if got_digest != case.sha1 {
            failures.push(format!(
                "{}: digest\n  expected {}\n  got      {}",
                case.name, case.sha1, got_digest
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn the_awkward_boundaries_are_the_javascript_ones() {
    // The four layout cases from the specification, one each.
    assert_eq!(canonical(&Value::Number(1e21)), "number:1e+21");
    assert_eq!(
        canonical(&Value::Number(1e20)),
        "number:100000000000000000000"
    );
    assert_eq!(canonical(&Value::Number(1e-7)), "number:1e-7");
    assert_eq!(canonical(&Value::Number(1e-6)), "number:0.000001");
    assert_eq!(canonical(&Value::Number(-1e21)), "number:-1e+21");
    assert_eq!(canonical(&Value::Number(123.456)), "number:123.456");
}
