//! Normalisation for search, comparison and storage.
//!
//! Arabic text reaches a program with more variation than a reader notices. The
//! same word can be written with any of the alef forms, with or without the
//! short vowels, and with the letter yeh spelled two ways. Normalising collapses
//! those differences so that two spellings of one word compare equal.
//!
//! This is the opposite of what [`crate::reshape`] is for. Shaping prepares text
//! for display and must be applied last; normalisation prepares text for
//! matching and should be applied to what you store and index.

use crate::{TATWEEL, is_harakat};

/// Removes combining marks.
///
/// The Arabic script leaves short vowels out most of the time, so removing them
/// is the single most effective normalisation step.
#[must_use]
pub fn strip_harakat(text: &str) -> String {
    text.chars().filter(|c| !is_harakat(*c)).collect()
}

/// Removes the tatweel, the stretching character used for justification.
#[must_use]
pub fn strip_tatweel(text: &str) -> String {
    text.chars().filter(|c| *c != TATWEEL).collect()
}

/// Normalises Arabic text for comparison.
///
/// Applies, in order: remove combining marks, remove tatweel, unify the alef
/// forms, unify the yeh forms, unify the heh forms, and collapse runs of
/// whitespace. The result is not valid display text: it is a key for looking
/// things up.
///
/// # Example
///
/// ```
/// use arabic::normalize;
///
/// // Two spellings of the same name, differing in the alef and a harakat.
/// assert_eq!(normalize("أَحْمَد"), normalize("احمد"));
///
/// // The input is left alone; the normalised form is a separate string.
/// assert_ne!(normalize("أَحْمَد"), "أَحْمَد");
/// ```
#[must_use]
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_space = false;
    for c in strip_harakat(&strip_tatweel(text)).chars() {
        let mapped = match c {
            // Alef with hamza above, hamza below or madda, all written as a
            // plain alef in ordinary text.
            '\u{0622}' | '\u{0623}' | '\u{0625}' | '\u{0671}' => '\u{0627}',
            // Alef maksura and the two yeh forms.
            '\u{0649}' | '\u{06cc}' => '\u{064a}',
            // Teh marbuta and heh, which are the same sound.
            '\u{0629}' => '\u{0647}',
            // The Arabic letter kaf has two forms that look identical.
            '\u{06a9}' => '\u{0643}',
            other => other,
        };
        if mapped.is_whitespace() {
            if last_was_space || out.is_empty() {
                continue;
            }
            last_was_space = true;
            out.push(' ');
        } else {
            last_was_space = false;
            out.push(mapped);
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_are_removed() {
        // Beh with a fatha and a shadda.
        assert_eq!(strip_harakat("\u{0628}\u{064e}\u{0651}"), "\u{0628}");
    }

    #[test]
    fn tatweel_is_removed() {
        assert_eq!(
            strip_tatweel("\u{0628}\u{0640}\u{0628}"),
            "\u{0628}\u{0628}"
        );
    }

    #[test]
    fn the_alef_forms_become_one_letter() {
        let with_hamza = normalize("\u{0623}\u{0628}");
        let plain = normalize("\u{0627}\u{0628}");
        assert_eq!(with_hamza, plain);
    }

    #[test]
    fn the_yeh_forms_become_one_letter() {
        assert_eq!(normalize("\u{0649}"), normalize("\u{064a}"));
        assert_eq!(normalize("\u{06cc}"), normalize("\u{064a}"));
    }

    #[test]
    fn teh_marbuta_matches_heh() {
        assert_eq!(normalize("\u{0629}"), normalize("\u{0647}"));
    }

    #[test]
    fn marked_and_unmarked_text_normalise_the_same() {
        let marked = normalize("\u{0645}\u{064f}\u{062d}\u{064e}\u{0645}\u{064e}\u{0651}\u{062f}");
        let plain = normalize("\u{0645}\u{062d}\u{0645}\u{062f}");
        assert_eq!(marked, plain);
    }

    #[test]
    fn whitespace_is_collapsed_and_trimmed() {
        assert_eq!(normalize("  a   b  "), "a b");
        assert_eq!(normalize("\t\n"), "");
    }

    #[test]
    fn latin_text_survives_unchanged() {
        assert_eq!(normalize("Hello World"), "Hello World");
    }
}
