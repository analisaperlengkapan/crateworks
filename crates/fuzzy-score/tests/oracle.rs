//! Checks every score against values produced by `fuzzywuzzy` 0.18 itself.
//!
//! The fixture is one case per line, tab separated:
//!
//! ```text
//! a   b   ratio  partial  tsort  pts  tset  ptset  wratio  qratio  uwratio  uqratio
//! ```
//!
//! `a` and `b` are escaped with `\t`, `\n`, `\r`, `\uXXXX` and `\UXXXXXXXX`, so
//! a field never contains a literal tab. The remaining ten fields are the
//! scores `fuzzywuzzy` gave, in the order the crate's functions are listed
//! below.
//!
//! The values were produced with `python-Levenshtein` uninstalled. That detail
//! matters: when `Levenshtein` is importable, `fuzzywuzzy.fuzz` swaps in a
//! `Levenshtein`-backed `SequenceMatcher` that returns *different* ratios, so
//! the fixture would then disagree with the crate for reasons that have nothing
//! to do with this crate. The generator asserts the pure-Python
//! `difflib.SequenceMatcher` is in use before writing anything.

use fuzzy_score::fuzz;

/// One line of the fixture.
struct Case {
    a: String,
    b: String,
    scores: [u8; 10],
}

/// Undoes the escaping the generator applied.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                let cp = u32::from_str_radix(&hex, 16).expect("bad \\u escape");
                out.push(char::from_u32(cp).expect("bad codepoint"));
            }
            Some('U') => {
                let hex: String = chars.by_ref().take(8).collect();
                let cp = u32::from_str_radix(&hex, 16).expect("bad \\U escape");
                out.push(char::from_u32(cp).expect("bad codepoint"));
            }
            other => panic!("bad escape: {other:?}"),
        }
    }
    out
}

fn load() -> Vec<Case> {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/oracle.tsv"),
    )
    .expect("oracle fixture is missing");

    let mut cases = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 12, "malformed fixture line: {line:?}");
        let mut scores = [0u8; 10];
        for (i, field) in fields[2..].iter().enumerate() {
            scores[i] = field.parse().expect("score is not a number");
        }
        cases.push(Case {
            a: unescape(fields[0]),
            b: unescape(fields[1]),
            scores,
        });
    }
    assert!(cases.len() > 1000, "fixture looks truncated");
    cases
}

#[test]
fn matches_the_python_oracle() {
    let cases = load();
    let mut failures = 0;
    for case in &cases {
        let (a, b) = (&case.a, &case.b);
        let got = [
            fuzz::ratio(a, b),
            fuzz::partial_ratio(a, b),
            fuzz::token_sort_ratio(a, b),
            fuzz::partial_token_sort_ratio(a, b),
            fuzz::token_set_ratio(a, b),
            fuzz::partial_token_set_ratio(a, b),
            fuzz::w_ratio(a, b),
            fuzz::q_ratio(a, b),
            fuzz::uw_ratio(a, b),
            fuzz::uq_ratio(a, b),
        ];
        if got != case.scores {
            failures += 1;
            if failures <= 10 {
                eprintln!(
                    "mismatch for {:?} vs {:?}\n  expected {:?}\n  got      {:?}",
                    a, b, case.scores, got
                );
            }
        }
    }
    assert_eq!(
        failures,
        0,
        "{failures} of {} cases differ from fuzzywuzzy",
        cases.len()
    );
}
