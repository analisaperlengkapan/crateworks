//! The scoring functions, matching Python's `fuzzywuzzy.fuzz`.
//!
//! Scores are integers in `0..=100`. The functions come in three families:
//!
//! * [`ratio`] and [`partial_ratio`] work on the raw character sequence;
//! * [`token_sort_ratio`] and [`token_set_ratio`] first normalise the words, so
//!   that word order and extra words matter less;
//! * [`w_ratio`] takes the best of the above, with the partial scores scaled
//!   down so that only a full match can reach 100.
//!
//! Each function has a `_with_options` variant that takes a [`Options`] value.
//! The plain functions use the reference defaults, which is what
//! `fuzzywuzzy` does.

use crate::difflib::SequenceMatcher;
use crate::utils::{full_process, split_whitespace_python, validate_string};

/// How to preprocess the inputs before scoring.
///
/// `fuzzywuzzy` passes these two flags through its whole API. `full_process`
/// defaults to `true`; `force_ascii` defaults to `true` for the ASCII-named
/// functions and is `false` in the `u`-prefixed ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Run [`full_process`] on both inputs first.
    pub full_process: bool,
    /// Drop non-ASCII characters before anything else. Only has an effect when
    /// `full_process` is set.
    pub force_ascii: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            full_process: true,
            force_ascii: true,
        }
    }
}

impl Options {
    /// Options that skip preprocessing entirely.
    pub const RAW: Self = Self {
        full_process: false,
        force_ascii: false,
    };

    /// Options that preprocess but keep Unicode.
    pub const UNICODE: Self = Self {
        full_process: true,
        force_ascii: false,
    };
}

/// `fuzzywuzzy.utils.intr`: round to the nearest integer, halves to even.
///
/// Python's `round` breaks ties towards the even value, and the scores have to
/// agree with it exactly, so this is not `f64::round`.
fn intr(n: f64) -> u8 {
    n.round_ties_even() as u8
}

/// Length in characters, not bytes, as Python's `len` counts.
fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Character-wise slice of `s`, clamped to the string, like Python's `s[a:b]`.
fn char_slice(s: &str, start: usize, end: usize) -> String {
    s.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

/// Port of `fuzzywuzzy.fuzz.ratio`.
///
/// The number of matching characters over the total length, as a percentage:
/// `2 * matches / (len(a) + len(b))`, where the matches come from Python's
/// `difflib.SequenceMatcher`. Two equal strings score 100, and an empty string
/// scores 100 only against another empty string.
pub fn ratio(a: &str, b: &str) -> u8 {
    if a == b {
        return 100;
    }
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    let m = SequenceMatcher::new(a, b, true);
    intr(100.0 * m.ratio())
}

/// Port of `fuzzywuzzy.fuzz.partial_ratio`.
///
/// The best score between the shorter string and any substring of the longer
/// one that is the same length. Use it when one input is a fragment of the
/// other: `"hello world"` against `"hello"` scores 100 here, but 62 with
/// [`ratio`].
pub fn partial_ratio(a: &str, b: &str) -> u8 {
    if a == b {
        return 100;
    }
    if a.is_empty() || b.is_empty() {
        return 0;
    }

    let (shorter, longer) = if char_len(a) <= char_len(b) {
        (a, b)
    } else {
        (b, a)
    };
    let shorter_len = char_len(shorter);

    let blocks = SequenceMatcher::new(shorter, longer, true).get_matching_blocks();
    let mut best = 0.0f64;
    for block in blocks {
        let long_start = block.b.saturating_sub(block.a);
        let long_end = long_start + shorter_len;
        let long_substr = char_slice(longer, long_start, long_end);

        let r = SequenceMatcher::new(shorter, &long_substr, true).ratio();
        if r > 0.995 {
            return 100;
        }
        if r > best {
            best = r;
        }
    }
    intr(100.0 * best)
}

/// `fuzzywuzzy.fuzz._process_and_sort`.
fn process_and_sort(s: &str, options: Options) -> String {
    let ts = if options.full_process {
        full_process(s, options.force_ascii)
    } else {
        s.to_string()
    };
    let mut tokens = split_whitespace_python(&ts);
    tokens.sort_unstable();
    tokens.join(" ")
}

/// `fuzzywuzzy.fuzz._token_sort`.
fn token_sort(a: &str, b: &str, partial: bool, options: Options) -> u8 {
    let sorted_a = process_and_sort(a, options);
    let sorted_b = process_and_sort(b, options);
    if partial {
        partial_ratio(&sorted_a, &sorted_b)
    } else {
        ratio(&sorted_a, &sorted_b)
    }
}

/// Port of `fuzzywuzzy.fuzz.token_sort_ratio`.
///
/// Splits both inputs into words, sorts the words, and scores the results with
/// [`ratio`]. Word order stops mattering: `"fuzzy wuzzy was a bear"` and
/// `"wuzzy fuzzy was a bear"` score 100.
pub fn token_sort_ratio(a: &str, b: &str) -> u8 {
    token_sort(a, b, false, Options::default())
}

/// Port of `fuzzywuzzy.fuzz.token_sort_ratio` with explicit [`Options`].
pub fn token_sort_ratio_with_options(a: &str, b: &str, options: Options) -> u8 {
    token_sort(a, b, false, options)
}

/// Port of `fuzzywuzzy.fuzz.partial_token_sort_ratio`.
///
/// Like [`token_sort_ratio`], but scores with [`partial_ratio`].
pub fn partial_token_sort_ratio(a: &str, b: &str) -> u8 {
    token_sort(a, b, true, Options::default())
}

/// Port of `fuzzywuzzy.fuzz.partial_token_sort_ratio` with explicit [`Options`].
pub fn partial_token_sort_ratio_with_options(a: &str, b: &str, options: Options) -> u8 {
    token_sort(a, b, true, options)
}

/// `fuzzywuzzy.fuzz._token_set`.
fn token_set(a: &str, b: &str, partial: bool, options: Options) -> u8 {
    if !options.full_process && a == b {
        return 100;
    }

    let p1 = if options.full_process {
        full_process(a, options.force_ascii)
    } else {
        a.to_string()
    };
    let p2 = if options.full_process {
        full_process(b, options.force_ascii)
    } else {
        b.to_string()
    };

    if !validate_string(&p1) || !validate_string(&p2) {
        return 0;
    }

    let mut tokens1: Vec<&str> = split_whitespace_python(&p1);
    let mut tokens2: Vec<&str> = split_whitespace_python(&p2);
    tokens1.sort_unstable();
    tokens1.dedup();
    tokens2.sort_unstable();
    tokens2.dedup();

    let intersection: Vec<&str> = tokens1
        .iter()
        .copied()
        .filter(|t| tokens2.binary_search(t).is_ok())
        .collect();
    let diff1to2: Vec<&str> = tokens1
        .iter()
        .copied()
        .filter(|t| tokens2.binary_search(t).is_err())
        .collect();
    let diff2to1: Vec<&str> = tokens2
        .iter()
        .copied()
        .filter(|t| tokens1.binary_search(t).is_err())
        .collect();

    let sorted_sect = intersection.join(" ");
    let sorted_1to2 = diff1to2.join(" ");
    let sorted_2to1 = diff2to1.join(" ");

    // The reference joins with a space and only then trims, so an empty
    // intersection leaves a leading space that is removed here.
    let combined_1to2 = format!("{sorted_sect} {sorted_1to2}").trim().to_string();
    let combined_2to1 = format!("{sorted_sect} {sorted_2to1}").trim().to_string();

    let score = if partial { partial_ratio } else { ratio };
    let mut best = score(&sorted_sect, &combined_1to2);
    best = best.max(score(&sorted_sect, &combined_2to1));
    best.max(score(&combined_1to2, &combined_2to1))
}

/// Port of `fuzzywuzzy.fuzz.token_set_ratio`.
///
/// Compares the words as sets, so extra words in one input cost less than they
/// do with [`token_sort_ratio`]: `"new york mets vs atlanta braves"` and
/// `"atlanta braves vs new york mets"` score 100.
pub fn token_set_ratio(a: &str, b: &str) -> u8 {
    token_set(a, b, false, Options::default())
}

/// Port of `fuzzywuzzy.fuzz.token_set_ratio` with explicit [`Options`].
pub fn token_set_ratio_with_options(a: &str, b: &str, options: Options) -> u8 {
    token_set(a, b, false, options)
}

/// Port of `fuzzywuzzy.fuzz.partial_token_set_ratio`.
///
/// Like [`token_set_ratio`], but scores with [`partial_ratio`].
pub fn partial_token_set_ratio(a: &str, b: &str) -> u8 {
    token_set(a, b, true, Options::default())
}

/// Port of `fuzzywuzzy.fuzz.partial_token_set_ratio` with explicit [`Options`].
pub fn partial_token_set_ratio_with_options(a: &str, b: &str, options: Options) -> u8 {
    token_set(a, b, true, options)
}

/// `fuzzywuzzy.fuzz.QRatio`, the shared body of the quick and weighted ratios.
pub(crate) fn q_ratio_body(a: &str, b: &str, options: Options) -> u8 {
    let p1 = if options.full_process {
        full_process(a, options.force_ascii)
    } else {
        a.to_string()
    };
    let p2 = if options.full_process {
        full_process(b, options.force_ascii)
    } else {
        b.to_string()
    };

    if !validate_string(&p1) || !validate_string(&p2) {
        return 0;
    }
    ratio(&p1, &p2)
}

/// Port of `fuzzywuzzy.fuzz.QRatio`.
///
/// Preprocesses both inputs, then scores with [`ratio`]. Cheaper than
/// [`w_ratio`] and keeps word order significant.
pub fn q_ratio(a: &str, b: &str) -> u8 {
    q_ratio_body(a, b, Options::default())
}

/// Port of `fuzzywuzzy.fuzz.QRatio` with explicit [`Options`].
pub fn q_ratio_with_options(a: &str, b: &str, options: Options) -> u8 {
    q_ratio_body(a, b, options)
}

/// Port of `fuzzywuzzy.fuzz.UQRatio`: [`q_ratio`] without ASCII folding.
pub fn uq_ratio(a: &str, b: &str) -> u8 {
    q_ratio_body(a, b, Options::UNICODE)
}

/// Port of `fuzzywuzzy.fuzz.WRatio`, the default scorer.
///
/// Takes the highest of [`ratio`], [`partial_ratio`] and the token ratios, with
/// the partial and token scores scaled down so that a full match is still
/// needed for 100:
///
/// * a partial match is scaled by 0.9, or 0.6 when one input is more than eight
///   times as long as the other;
/// * the token ratios are additionally scaled by 0.95;
/// * partials are only tried when the inputs differ in length by more than
///   1.5x.
pub fn w_ratio(a: &str, b: &str) -> u8 {
    w_ratio_body(a, b, Options::default())
}

/// Port of `fuzzywuzzy.fuzz.UWRatio`: [`w_ratio`] without ASCII folding.
pub fn uw_ratio(a: &str, b: &str) -> u8 {
    w_ratio_body(a, b, Options::UNICODE)
}

pub(crate) fn w_ratio_body(a: &str, b: &str, options: Options) -> u8 {
    let p1 = if options.full_process {
        full_process(a, options.force_ascii)
    } else {
        a.to_string()
    };
    let p2 = if options.full_process {
        full_process(b, options.force_ascii)
    } else {
        b.to_string()
    };

    if !validate_string(&p1) || !validate_string(&p2) {
        return 0;
    }

    let mut try_partial = true;
    const UNBASE_SCALE: f64 = 0.95;
    let mut partial_scale = 0.90;

    let base = f64::from(ratio(&p1, &p2));
    let len_ratio =
        char_len(&p1).max(char_len(&p2)) as f64 / char_len(&p1).min(char_len(&p2)) as f64;

    if len_ratio < 1.5 {
        try_partial = false;
    }
    if len_ratio > 8.0 {
        partial_scale = 0.6;
    }

    if try_partial {
        let partial = f64::from(partial_ratio(&p1, &p2)) * partial_scale;
        let ptsor = f64::from(partial_token_sort_ratio_with_options(
            &p1,
            &p2,
            Options::RAW,
        )) * UNBASE_SCALE
            * partial_scale;
        let ptser = f64::from(partial_token_set_ratio_with_options(&p1, &p2, Options::RAW))
            * UNBASE_SCALE
            * partial_scale;
        intr(base.max(partial).max(ptsor).max(ptser))
    } else {
        let tsor = f64::from(token_sort_ratio_with_options(&p1, &p2, Options::RAW)) * UNBASE_SCALE;
        let tser = f64::from(token_set_ratio_with_options(&p1, &p2, Options::RAW)) * UNBASE_SCALE;
        intr(base.max(tsor).max(tser))
    }
}
