//! Checks the crate against the `container-validator` npm package.
//!
//! The fixture is one line per case, tab separated:
//!
//! ```text
//! owner   category   serial   check_digit   code
//! CSQ     U          305438   3             CSQU3054383
//! ```
//!
//! `check_digit` is what the reference computed, and `code` is the full code
//! that includes it. Every row is rebuilt from the three parts and compared
//! against both, so a divergence in either the arithmetic or the parsing shows
//! up as a named case rather than a wrong answer.

use iso6346::{Category, Container, check_digit, validate};

/// One line of the fixture.
struct Case {
    owner: String,
    category: Category,
    serial: u32,
    check_digit: u32,
    code: String,
}

fn category_from(letter: &str) -> Category {
    match letter {
        "U" => Category::Freight,
        "J" => Category::Detachable,
        "Z" => Category::Chassis,
        other => panic!("fixture has an unknown category {other:?}"),
    }
}

fn cases() -> Vec<Case> {
    let text = include_str!("fixtures/oracle.tsv");
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 5, "malformed fixture line: {line:?}");
            Case {
                owner: fields[0].to_string(),
                category: category_from(fields[1]),
                serial: fields[2].parse().expect("serial is a number"),
                check_digit: fields[3].parse().expect("check digit is a number"),
                code: fields[4].to_string(),
            }
        })
        .collect()
}

#[test]
fn check_digit_agrees_with_the_reference() {
    let cases = cases();
    assert!(cases.len() > 3000, "the fixture looks truncated");

    for case in &cases {
        let ours = check_digit(&case.owner, case.category, case.serial);
        assert_eq!(
            ours,
            Some(case.check_digit),
            "check digit for {}{}{:06}",
            case.owner,
            case.category.letter(),
            case.serial
        );
    }
}

#[test]
fn every_reference_code_parses_back_to_its_parts() {
    for case in &cases() {
        let container = Container::parse(&case.code)
            .unwrap_or_else(|e| panic!("{} from the reference failed to parse: {e}", case.code));

        assert_eq!(container.owner_code(), case.owner);
        assert_eq!(container.category(), case.category);
        assert_eq!(container.serial(), case.serial);
        assert_eq!(container.check_digit(), case.check_digit);
        assert_eq!(container.as_str(), case.code);
        assert!(validate(&case.code).is_ok());
    }
}

#[test]
fn a_generated_code_matches_the_reference() {
    // Generating from the parts and parsing the reference's own code have to
    // agree, or one of the two directions is wrong.
    for case in &cases() {
        let generated = Container::new(&case.owner, case.category, case.serial)
            .expect("the fixture parts are valid");
        assert_eq!(generated.as_str(), case.code);
    }
}

#[test]
fn a_changed_digit_is_rejected() {
    // The check digit exists to catch exactly this. Stepping the serial changes
    // the digit the code should carry, so the reference's code must not parse.
    for case in &cases() {
        if case.serial == 0 {
            continue;
        }
        let changed = format!(
            "{}{}{:06}{}",
            case.owner,
            case.category.letter(),
            case.serial - 1,
            case.check_digit
        );
        assert!(
            validate(&changed).is_err(),
            "{changed} should not pass with the check digit of another code"
        );
    }
}
