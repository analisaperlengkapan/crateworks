//! String preprocessing, the way `fuzzywuzzy.utils` does it.
//!
//! Every scoring function in this crate assumes both inputs have been through
//! [`full_process`] unless it is explicitly told otherwise, so the exact
//! behaviour here decides every score.

use crate::unicode_tables::{PYTHON_LOWERCASE_OVERRIDES, PYTHON_WHITESPACE};
use crate::word_chars::WORD_CHAR_RANGES;

/// Whether `c` is a word character in Python's `re` sense.
///
/// Python's `\W` is the complement of `\w`, which is the Unicode categories
/// `L*` and `N*` plus the underscore. Rust's `char::is_alphanumeric` is close
/// but not equal, so the membership test uses a table generated from CPython.
pub(crate) fn is_word_char(c: char) -> bool {
    if c == '_' {
        return true;
    }
    let cp = c as u32;
    WORD_CHAR_RANGES
        .binary_search_by(|&(lo, hi)| {
            if cp < lo {
                std::cmp::Ordering::Greater
            } else if cp > hi {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// Whether `c` is whitespace to Python's `str.split()` and `str.strip()`.
pub(crate) fn is_python_whitespace(c: char) -> bool {
    let cp = c as u32;
    PYTHON_WHITESPACE.binary_search(&cp).is_ok()
}

/// Port of `fuzzywuzzy.utils.asciidammit`.
///
/// Despite the name this only drops the Latin-1 supplement, `U+0080..=U+00FF`;
/// characters above it are left alone. So `"Á"` becomes `""` but `"中"` is
/// kept, and `"ABCDÁ"` becomes `"ABCD"` while `"ABCD中"` does not.
fn force_ascii(s: &str) -> String {
    s.chars()
        .filter(|&c| !('\u{80}'..='\u{FF}').contains(&c))
        .collect()
}

/// Port of `fuzzywuzzy.utils.full_process`.
///
/// Replaces every non-word character with a single space, lowercases, then
/// trims. The replacement is one-for-one, so runs of punctuation are not
/// collapsed: `"a - b"` becomes `"a   b"`, with three spaces, and the empty
/// token between them is dropped only later, by `split`.
///
/// The underscore is a word character in Python and is kept, which is where a
/// naive "keep alphanumerics" implementation diverges.
pub fn full_process(s: &str, force_ascii_only: bool) -> String {
    let s = if force_ascii_only {
        force_ascii(s)
    } else {
        s.to_string()
    };
    let spaced: String = s
        .chars()
        .map(|c| if is_word_char(c) { c } else { ' ' })
        .collect();
    let mut lowered = String::with_capacity(s.len());
    for c in spaced.chars() {
        match PYTHON_LOWERCASE_OVERRIDES.binary_search_by_key(&(c as u32), |&(cp, _)| cp) {
            Ok(i) => lowered.push_str(PYTHON_LOWERCASE_OVERRIDES[i].1),
            Err(_) => lowered.extend(c.to_lowercase()),
        }
    }
    lowered.trim_matches(is_python_whitespace).to_string()
}

/// Splits on Python's whitespace, dropping empty fields the way `str.split()`
/// with no arguments does.
pub(crate) fn split_whitespace_python(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in s.char_indices() {
        if is_python_whitespace(c) {
            if let Some(st) = start.take() {
                out.push(&s[st..i]);
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(st) = start {
        out.push(&s[st..]);
    }
    out
}

/// `fuzzywuzzy.utils.validate_string`: true when the string is non-empty.
pub(crate) fn validate_string(s: &str) -> bool {
    !s.is_empty()
}
