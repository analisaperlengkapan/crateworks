//! The test cases from `fuzzywuzzy` 0.18's own `test_fuzzywuzzy.py`.
//!
//! Each test names the upstream test it comes from. They are transcribed rather
//! than rewritten so that a failure here means a divergence from the reference
//! implementation, not from someone's idea of what the answer should be.

use fuzzy_score::fuzz::{self, Options};
use fuzzy_score::{Scorer, dedupe, extract, extract_one, full_process};

const S1: &str = "new york mets";
const S1A: &str = "new york mets";
const S2: &str = "new YORK mets";
const S3: &str = "the wonderful new york mets";
const S4: &str = "new york mets vs atlanta braves";
const S5: &str = "atlanta braves vs new york mets";
const S7: &str = "new york city mets - atlanta braves";

/// `RatioTest.setUp`, the silly corner cases.
const S8: &str = "{";
const S9: &str = "{a";
const S10: &str = "a{";
const S10A: &str = "{b";

const BASEBALL_STRINGS: [&str; 4] = [
    "new york mets vs chicago cubs",
    "chicago cubs vs chicago white sox",
    "philladelphia phillies vs atlanta braves",
    "braves vs mets",
];

#[test]
fn test_equal() {
    assert_eq!(fuzz::ratio(S1, S1A), 100);
    assert_eq!(fuzz::ratio(S8, S8), 100);
    assert_eq!(fuzz::ratio(S9, S9), 100);
}

#[test]
fn test_case_insensitive() {
    assert_ne!(fuzz::ratio(S1, S2), 100);
    assert_eq!(
        fuzz::ratio(&full_process(S1, true), &full_process(S2, true)),
        100
    );
}

#[test]
fn test_partial_ratio() {
    assert_eq!(fuzz::partial_ratio(S1, S3), 100);
}

#[test]
fn test_token_sort_ratio() {
    assert_eq!(fuzz::token_sort_ratio(S1, S1A), 100);
}

#[test]
fn test_partial_token_sort_ratio() {
    assert_eq!(fuzz::partial_token_sort_ratio(S1, S1A), 100);
    assert_eq!(fuzz::partial_token_sort_ratio(S4, S5), 100);
    assert_eq!(
        fuzz::partial_token_sort_ratio_with_options(S8, S8, Options::RAW),
        100
    );
    assert_eq!(
        fuzz::partial_token_sort_ratio_with_options(S9, S9, Options::default()),
        100
    );
    assert_eq!(
        fuzz::partial_token_sort_ratio_with_options(S9, S9, Options::RAW),
        100
    );
    assert_eq!(
        fuzz::partial_token_sort_ratio_with_options(S10, S10A, Options::RAW),
        50
    );
}

#[test]
fn test_token_set_ratio() {
    assert_eq!(fuzz::token_set_ratio(S4, S5), 100);
    assert_eq!(
        fuzz::token_set_ratio_with_options(S8, S8, Options::RAW),
        100
    );
    assert_eq!(
        fuzz::token_set_ratio_with_options(S9, S9, Options::default()),
        100
    );
    assert_eq!(
        fuzz::token_set_ratio_with_options(S9, S9, Options::RAW),
        100
    );
    assert_eq!(
        fuzz::token_set_ratio_with_options(S10, S10A, Options::RAW),
        50
    );
}

#[test]
fn test_partial_token_set_ratio() {
    assert_eq!(fuzz::partial_token_set_ratio(S4, S7), 100);
}

#[test]
fn test_quick_ratio_equal() {
    assert_eq!(fuzz::q_ratio(S1, S1A), 100);
}

#[test]
fn test_quick_ratio_case_insensitive() {
    assert_eq!(fuzz::q_ratio(S1, S2), 100);
}

#[test]
fn test_quick_ratio_not_equal() {
    assert_ne!(fuzz::q_ratio(S1, S3), 100);
}

#[test]
fn test_w_ratio_equal() {
    assert_eq!(fuzz::w_ratio(S1, S1A), 100);
}

#[test]
fn test_w_ratio_case_insensitive() {
    assert_eq!(fuzz::w_ratio(S1, S2), 100);
}

#[test]
fn test_w_ratio_partial_match() {
    // A partial match is scaled by .9.
    assert_eq!(fuzz::w_ratio(S1, S3), 90);
}

#[test]
fn test_w_ratio_misordered_match() {
    // Misordered full matches are scaled by .95.
    assert_eq!(fuzz::w_ratio(S4, S5), 95);
}

#[test]
fn test_empty_strings_score_100() {
    assert_eq!(fuzz::ratio("", ""), 100);
    assert_eq!(fuzz::partial_ratio("", ""), 100);
}

#[test]
fn test_issue_seven() {
    let s1 = "HSINCHUANG";
    let s2 = "SINJHUAN";
    let s3 = "LSINJHUANG DISTRIC";
    let s4 = "SINJHUANG DISTRICT";

    assert!(fuzz::partial_ratio(s1, s2) > 75);
    assert!(fuzz::partial_ratio(s1, s3) > 75);
    assert!(fuzz::partial_ratio(s1, s4) > 75);
}

#[test]
fn test_ratio_unicode_string() {
    assert_eq!(fuzz::ratio("\u{C1}", "ABCD"), 0);
}

#[test]
fn test_partial_ratio_unicode_string() {
    assert_eq!(fuzz::partial_ratio("\u{C1}", "ABCD"), 0);
}

#[test]
fn test_w_ratio_unicode_string() {
    assert_eq!(fuzz::w_ratio("\u{C1}", "ABCD"), 0);

    // Cyrillic.
    assert_ne!(
        fuzz::w_ratio(
            "\u{43f}\u{441}\u{438}\u{445}\u{43e}\u{43b}\u{43e}\u{433}",
            "\u{43f}\u{441}\u{438}\u{445}\u{43e}\u{442}\u{435}\u{440}\u{430}\u{43f}\u{435}\u{432}\u{442}"
        ),
        0
    );

    // Chinese.
    assert_ne!(
        fuzz::uw_ratio(
            "\u{6211}\u{4e86}\u{89e3}\u{6570}\u{5b66}",
            "\u{6211}\u{5b66}\u{6570}\u{5b66}"
        ),
        0
    );
}

#[test]
fn test_q_ratio_unicode_string() {
    assert_eq!(fuzz::q_ratio("\u{C1}", "ABCD"), 0);

    // Cyrillic.
    assert_ne!(
        fuzz::uq_ratio(
            "\u{43f}\u{441}\u{438}\u{445}\u{43e}\u{43b}\u{43e}\u{433}",
            "\u{43f}\u{441}\u{438}\u{445}\u{43e}\u{442}\u{435}\u{440}\u{430}\u{43f}\u{435}\u{432}\u{442}"
        ),
        0
    );

    // Chinese.
    assert_ne!(
        fuzz::uq_ratio(
            "\u{6211}\u{4e86}\u{89e3}\u{6570}\u{5b66}",
            "\u{6211}\u{5b66}\u{6570}\u{5b66}"
        ),
        0
    );
}

#[test]
fn test_q_ratio_force_ascii() {
    let s1 = "ABCD\u{C1}";
    let s2 = "ABCD";

    assert_eq!(fuzz::q_ratio_with_options(s1, s2, Options::default()), 100);
    assert!(fuzz::q_ratio_with_options(s1, s2, Options::UNICODE) < 100);
}

#[test]
fn test_w_ratio_force_ascii() {
    let s1 = "ABCD\u{C1}";
    let s2 = "ABCD";

    assert_eq!(fuzz::w_ratio(s1, s2), 100);
    assert!(fuzz::uw_ratio(s1, s2) < 100);
}

#[test]
fn test_token_set_force_ascii() {
    let s1 = "ABCD\u{C1} HELP\u{C1}";
    let s2 = "ABCD HELP";

    assert_eq!(fuzz::token_set_ratio(s1, s2), 100);
    assert!(fuzz::token_set_ratio_with_options(s1, s2, Options::UNICODE) < 100);
}

#[test]
fn test_token_sort_force_ascii() {
    let s1 = "ABCD\u{C1} HELP\u{C1}";
    let s2 = "ABCD HELP";

    assert_eq!(fuzz::token_sort_ratio(s1, s2), 100);
    assert!(fuzz::token_sort_ratio_with_options(s1, s2, Options::UNICODE) < 100);
}

// --- process tests ---

#[test]
fn test_get_best_choice_1() {
    let best = extract_one(
        "new york mets at atlanta braves",
        BASEBALL_STRINGS,
        0,
        Scorer::WRatio,
    );
    assert_eq!(best.unwrap().0, "braves vs mets");
}

#[test]
fn test_get_best_choice_2() {
    let best = extract_one(
        "philadelphia phillies at atlanta braves",
        BASEBALL_STRINGS,
        0,
        Scorer::WRatio,
    );
    assert_eq!(best.unwrap().0, BASEBALL_STRINGS[2]);
}

#[test]
fn test_get_best_choice_3() {
    let best = extract_one(
        "atlanta braves at philadelphia phillies",
        BASEBALL_STRINGS,
        0,
        Scorer::WRatio,
    );
    assert_eq!(best.unwrap().0, BASEBALL_STRINGS[2]);
}

#[test]
fn test_get_best_choice_4() {
    let best = extract_one(
        "chicago cubs vs new york mets",
        BASEBALL_STRINGS,
        0,
        Scorer::WRatio,
    );
    assert_eq!(best.unwrap().0, BASEBALL_STRINGS[0]);
}

#[test]
fn test_with_scorer() {
    let choices = [
        "new york mets vs chicago cubs",
        "chicago cubs at new york mets",
        "atlanta braves vs pittsbugh pirates",
        "new york yankees vs boston red sox",
    ];
    let query = "new york mets at chicago cubs";

    // The default scorer prefers the "more complete" match.
    let best = extract_one(query, choices, 0, Scorer::WRatio);
    assert_eq!(best.unwrap().0, choices[1]);

    // With QRatio, which cares about word order, the first choice wins.
    let best = extract_one(query, choices, 0, Scorer::QRatio);
    assert_eq!(best.unwrap().0, choices[0]);
}

#[test]
fn test_with_cutoff() {
    let choices = [
        "new york mets vs chicago cubs",
        "chicago cubs at new york mets",
        "atlanta braves vs pittsbugh pirates",
        "new york yankees vs boston red sox",
    ];
    let query = "los angeles dodgers vs san francisco giants";

    assert!(extract_one(query, choices, 50, Scorer::WRatio).is_none());
}

#[test]
fn test_with_cutoff_2() {
    let choices = [
        "new york mets vs chicago cubs",
        "chicago cubs at new york mets",
        "atlanta braves vs pittsbugh pirates",
        "new york yankees vs boston red sox",
    ];
    let query = "new york mets vs chicago cubs";

    let (best_match, score) = extract_one(query, choices, 100, Scorer::WRatio).unwrap();
    assert_eq!(score, 100);
    assert_eq!(best_match, choices[0]);
}

#[test]
fn test_empty_strings() {
    let choices = [
        "",
        "new york mets vs chicago cubs",
        "new york yankees vs boston red sox",
        "",
        "",
    ];
    let best = extract_one("new york mets at chicago cubs", choices, 0, Scorer::WRatio);
    assert_eq!(best.unwrap().0, choices[1]);
}

#[test]
fn test_list_like_extract() {
    let choices = ["a", "Bb", "CcC"];
    let result = extract("aaa", choices, None, Scorer::WRatio);
    assert!(!result.is_empty());
}

#[test]
fn test_dedupe() {
    let contains_dupes = [
        "Frodo Baggins",
        "Tom Sawyer",
        "Bilbo Baggin",
        "Samuel L. Jackson",
        "F. Baggins",
        "Frody Baggins",
        "Bilbo Baggins",
    ];
    let result = dedupe(contains_dupes, 70);
    assert!(result.len() < contains_dupes.len());

    let no_dupes = ["Tom", "Dick", "Harry"];
    let deduped = dedupe(no_dupes, 70);
    assert_eq!(deduped, vec!["Tom", "Dick", "Harry"]);
}

#[test]
fn test_simple_match() {
    let basic_string = "a, b";
    let match_strings = ["a, b"];

    let result = extract_one(basic_string, match_strings, 0, Scorer::Ratio);
    let part_result = extract_one(basic_string, match_strings, 0, Scorer::PartialRatio);

    assert_eq!(result, Some(("a, b".to_string(), 100)));
    assert_eq!(part_result, Some(("a, b".to_string(), 100)));
}
