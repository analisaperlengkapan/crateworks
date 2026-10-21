//! Comparison against Python's `javaproperties` package.
//!
//! The corpus is two files: a hand-written set that pins each rule, and a
//! four-thousand-case random set built from the format's own punctuation, which
//! is what finds the interactions between the rules that no one thinks to write
//! down.
//!
//! `javaproperties` replicates `java.util.Properties` and is tested against Java
//! itself, so agreeing with it is a much stronger claim than agreeing with our
//! own reading of the format. The corpus covers the rules that are easy to get
//! wrong: escaped separators, continuation, unicode escapes, duplicate keys and
//! the ISO-8859-1 default.

use javaproperties::{Properties, parse};

/// A case from the corpus, encoded as an escaped string.
///
/// The fixture is one case per line, with the input first, a tab, then the
/// pairs, so the file stays readable without a JSON dependency. Non-ASCII is
/// written with `\uXXXX` and decoded when the case is read.
struct Case {
    input: String,
    pairs: Vec<(String, String)>,
}

/// Decodes the escaping Python's `unicode_escape` codec produces.
///
/// The fixture is written that way so that control characters, non-ASCII text
/// and backslashes all survive an editing pass without turning into something
/// else. Only the escapes that codec emits are handled.
fn decode(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('f') => out.push('\u{000c}'),
            Some('\n') => out.push('\n'),
            Some('\r') => out.push('\r'),
            Some('a') => out.push('\u{0007}'),
            Some('b') => out.push('\u{0008}'),
            Some('v') => out.push('\u{000b}'),
            Some('\\') => out.push('\\'),
            Some('\'') => out.push('\''),
            Some('"') => out.push('"'),
            Some('x') => {
                let digits: String = (0..2).filter_map(|_| chars.next()).collect();
                let value = u32::from_str_radix(&digits, 16).unwrap_or(0xfffd);
                out.push(char::from_u32(value).unwrap_or('\u{fffd}'));
            }
            Some('u') => {
                let digits: String = (0..4).filter_map(|_| chars.next()).collect();
                let value = u32::from_str_radix(&digits, 16).unwrap_or(0xfffd);
                out.push(char::from_u32(value).unwrap_or('\u{fffd}'));
            }
            Some('U') => {
                let digits: String = (0..8).filter_map(|_| chars.next()).collect();
                let value = u32::from_str_radix(&digits, 16).unwrap_or(0xfffd);
                out.push(char::from_u32(value).unwrap_or('\u{fffd}'));
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// Cases are stored as `input<TAB>key<TAB>value<TAB>key<TAB>value...` on one
/// line, with `|` standing in for a newline and `!` for a tab inside the input.
fn cases() -> Vec<Case> {
    let mut out = read_cases(include_str!("fixtures/corpus.tsv"));
    out.extend(read_cases(include_str!("fixtures/fuzz.tsv")));
    out
}

/// Reads one fixture file.
fn read_cases(text: &str) -> Vec<Case> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let count: usize = fields.next().unwrap().parse().unwrap();
        let input = decode(fields.next().unwrap());
        let rest: Vec<&str> = fields.collect();
        let mut pairs = Vec::new();
        for index in 0..count {
            pairs.push((decode(rest[index * 2]), decode(rest[index * 2 + 1])));
        }
        out.push(Case { input, pairs });
    }
    out
}

#[test]
fn every_corpus_case_matches_javaproperties() {
    let cases = cases();
    assert!(cases.len() > 4_000, "only {} cases", cases.len());

    let mut failures = Vec::new();
    for case in &cases {
        match parse(case.input.as_bytes()) {
            Ok(got) => {
                if got != case.pairs {
                    failures.push(format!(
                        "input {:?}\n  expected {:?}\n  got      {:?}",
                        case.input, case.pairs, got
                    ));
                }
            }
            Err(error) => failures.push(format!("input {:?} failed: {error}", case.input)),
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} cases disagreed:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn a_key_given_twice_keeps_the_last_value() {
    // Java's own rule, and the reason the pairs are kept in order rather than in
    // a map that would drop the earlier one silently.
    let properties = Properties::parse(b"a=1\na=2\n").unwrap();
    assert_eq!(properties.get("a"), Some("2"));
    assert_eq!(properties.len(), 2, "both entries are still there");
    assert_eq!(
        properties.iter().collect::<Vec<_>>(),
        vec![("a", "1"), ("a", "2")]
    );
}

#[test]
fn a_missing_key_is_none_not_an_empty_string() {
    let properties = Properties::parse(b"a=\n").unwrap();
    assert_eq!(properties.get("a"), Some(""));
    assert_eq!(properties.get("b"), None);
}

#[test]
fn the_default_encoding_is_iso_8859_1() {
    // 0xE9 is é in ISO-8859-1. In UTF-8 it would be the start of a two-byte
    // sequence, which is the bug this rule exists to avoid.
    let properties = Properties::parse(b"name=\xe9\n").unwrap();
    assert_eq!(properties.get("name"), Some("é"));
}

#[test]
fn text_that_was_already_decoded_is_read_as_is() {
    // The same bytes as UTF-8 text now mean something else, which is the whole
    // reason `parse_str` exists separately from `parse`.
    let pairs = javaproperties::parse_str("name=é\n").unwrap();
    assert_eq!(pairs[0], ("name".to_string(), "é".to_string()));
}

#[test]
fn a_comment_and_a_blank_line_are_not_entries() {
    let pairs = parse(b"# a comment\n! another\n\nkey=value\n").unwrap();
    assert_eq!(pairs, vec![("key".to_string(), "value".to_string())]);
}

#[test]
fn a_continuation_joins_the_lines() {
    // The leading whitespace of each continued line is dropped, and nothing is
    // inserted in its place, so the parts run together.
    let pairs = parse(b"key=one\\\n    two\\\n    three\n").unwrap();
    assert_eq!(pairs, vec![("key".to_string(), "onetwothree".to_string())]);
}

#[test]
fn a_doubled_backslash_does_not_continue_the_line() {
    let pairs = parse(b"key=one\\\\\ntwo=three\n").unwrap();
    assert_eq!(
        pairs,
        vec![
            ("key".to_string(), r"one\".to_string()),
            ("two".to_string(), "three".to_string()),
        ]
    );
}

#[test]
fn an_escaped_separator_is_part_of_the_key() {
    let pairs = parse(b"a\\=b=c\n").unwrap();
    assert_eq!(pairs, vec![("a=b".to_string(), "c".to_string())]);
}

#[test]
fn a_malformed_unicode_escape_is_reported() {
    let error = parse(b"key=\\u00g1\n").unwrap_err();
    assert_eq!(error.escape(), Some(r"\u00g1"));
}

#[test]
fn the_separator_can_be_a_colon_or_whitespace() {
    for input in [
        b"key=value\n".as_slice(),
        b"key:value\n",
        b"key value\n",
        b"key\tvalue\n",
        b"key\x0Cvalue\n",
    ] {
        let pairs = parse(input).unwrap();
        assert_eq!(
            pairs,
            vec![("key".to_string(), "value".to_string())],
            "for {:?}",
            String::from_utf8_lossy(input)
        );
    }
}
