//! The standard Arabic ligatures.
//!
//! Unicode defines hundreds of Arabic ligatures, but only a handful are in
//! ordinary use: the four lam-alef combinations and the word "Allah". Those are
//! the ones a renderer is expected to produce, and the ones this module
//! substitutes.
//!
//! Substitution happens on the *shaped* text, so the pattern is a sequence of
//! presentation forms rather than of base letters. That is also what makes the
//! context fall out for free: an isolated alef followed by initial-lam,
//! medial-lam and final-heh is the word "Allah", whereas the same letters in
//! another position have different shapes and are correctly left alone.

/// A shaped sequence and the single character it collapses to.
struct Ligature {
    /// The presentation forms to look for.
    pattern: &'static [char],
    /// What they become.
    replacement: char,
}

/// The lam-alef ligatures and the word "Allah".
const LIGATURES: [Ligature; 9] = [
    // Lam followed by an alef. The first pair is the isolated form, which
    // happens when the lam starts a word; the second is the final form, which
    // happens after a joining letter.
    Ligature {
        pattern: &['\u{fedf}', '\u{fe8e}'],
        replacement: '\u{fefb}',
    },
    Ligature {
        pattern: &['\u{fee0}', '\u{fe8e}'],
        replacement: '\u{fefc}',
    },
    // Lam with alef-hamza-above.
    Ligature {
        pattern: &['\u{fedf}', '\u{fe84}'],
        replacement: '\u{fef7}',
    },
    Ligature {
        pattern: &['\u{fee0}', '\u{fe84}'],
        replacement: '\u{fef8}',
    },
    // Lam with alef-hamza-below.
    Ligature {
        pattern: &['\u{fedf}', '\u{fe88}'],
        replacement: '\u{fef9}',
    },
    Ligature {
        pattern: &['\u{fee0}', '\u{fe88}'],
        replacement: '\u{fefa}',
    },
    // Lam with alef-madda-above.
    Ligature {
        pattern: &['\u{fedf}', '\u{fe82}'],
        replacement: '\u{fef5}',
    },
    Ligature {
        pattern: &['\u{fee0}', '\u{fe82}'],
        replacement: '\u{fef6}',
    },
    // The word "Allah": isolated alef, initial lam, medial lam, final heh.
    Ligature {
        pattern: &['\u{fe8d}', '\u{fedf}', '\u{fee0}', '\u{feea}'],
        replacement: '\u{fdf2}',
    },
];

/// Replaces the standard ligatures in shaped text.
///
/// The scan is left to right and non-overlapping, and a replacement is never
/// itself rescanned, so "lam alef alef" collapses only the first pair.
pub(crate) fn apply(shaped: &str) -> String {
    let chars: Vec<char> = shaped.chars().collect();
    let mut out = String::with_capacity(shaped.len());
    let mut i = 0;
    'outer: while i < chars.len() {
        for ligature in &LIGATURES {
            let pattern = ligature.pattern;
            if i + pattern.len() <= chars.len() && chars[i..i + pattern.len()] == *pattern {
                out.push(ligature.replacement);
                i += pattern.len();
                continue 'outer;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lam_alef_collapses_to_one_character() {
        // Shaped "la": lam initial then alef final.
        assert_eq!(apply("\u{fedf}\u{fe8e}"), "\u{fefb}");
    }

    #[test]
    fn lam_alef_after_a_joining_letter_takes_the_final_form() {
        assert_eq!(apply("\u{fee0}\u{fe8e}"), "\u{fefc}");
    }

    #[test]
    fn the_word_allah_collapses() {
        assert_eq!(apply("\u{fe8d}\u{fedf}\u{fee0}\u{feea}"), "\u{fdf2}");
    }

    #[test]
    fn an_alef_in_another_position_does_not_collapse() {
        // The same three letters, but the alef is final rather than isolated,
        // which is not the word "Allah".
        assert_eq!(
            apply("\u{fe8e}\u{fedf}\u{fee0}\u{feea}"),
            "\u{fe8e}\u{fedf}\u{fee0}\u{feea}"
        );
    }

    #[test]
    fn text_without_ligatures_is_unchanged() {
        assert_eq!(apply("hello"), "hello");
        assert_eq!(apply(""), "");
    }

    #[test]
    fn a_ligature_at_the_very_end_is_found() {
        assert_eq!(apply("x\u{fedf}\u{fe8e}"), "x\u{fefb}");
    }
}
