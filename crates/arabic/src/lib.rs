//! Arabic text shaping.
//!
//! Arabic letters change shape depending on their neighbours. An initial `ب`
//! looks different from a final one, and a letter that joins on both sides is
//! drawn in its medial form. Terminals, older renderers and PDF generators do
//! not do this themselves: they expect the text to arrive already shaped, with
//! each letter replaced by the presentation form that Unicode keeps in the
//! Arabic Presentation Forms block.
//!
//! # Example
//!
//! ```
//! use arabic::{Direction, reshape};
//!
//! // "peace be upon you" in Arabic, shaped but still in logical order.
//! let logical = reshape("السلام عليكم", Direction::Logical);
//! assert_eq!(logical, "ﺍﻟﺴﻼﻡ ﻋﻠﻴﻜﻢ");
//!
//! // For a display that draws left to right, the shaped text is reversed.
//! let displayed = reshape("السلام عليكم", Direction::RightToLeft);
//! assert_eq!(displayed, "ﻢﻜﻴﻠﻋ ﻡﺎﻠﺴﻟﺍ");
//! ```
//!
//! # What shaping is for
//!
//! The Unicode standard stores Arabic in *logical* order: the characters as
//! they are typed and as they should be compared and searched. Rendering to a
//! terminal, a PDF or an image usually needs two further steps:
//!
//! 1. Replace each letter with its contextual presentation form, which is what
//!    [`reshape`] does.
//! 2. Reverse the result so it reads right to left, which is what
//!    [`Direction::RightToLeft`] adds.
//!
//! Search, sorting and storage should use the original text. Shaped text is for
//! display only, and comparing two shaped strings is almost always a mistake.
//!
//! # Harakat
//!
//! Short vowels and other combining marks are carried along with the letter they
//! belong to. By default they are preserved and stay attached to the correct
//! letter through the transformation; [`reshape_with`] can drop them instead,
//! which is what a fixed-width display often needs.

#![forbid(unsafe_code)]

/// The README, compiled as a doctest so its example cannot drift from the API.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

mod forms;
mod ligatures;
mod normalize;
mod reshape;

pub use normalize::{normalize, strip_harakat, strip_tatweel};
pub use reshape::{Direction, Options, reshape, reshape_with};

/// The tatweel, a stretching character that joins letters without carrying
/// meaning.
pub const TATWEEL: char = '\u{0640}';

/// The zero-width joiner, which forces a joining form.
pub const ZWJ: char = '\u{200d}';

/// The zero-width non-joiner, which prevents joining.
pub const ZWNJ: char = '\u{200c}';

/// Whether a character is a combining mark that attaches to a preceding letter.
///
/// The ranges are the ones Unicode assigns to Arabic and Syriac marks, which is
/// what a shaper has to keep with its base letter.
#[must_use]
pub fn is_harakat(c: char) -> bool {
    matches!(c,
        '\u{0610}'..='\u{061a}'
        | '\u{064b}'..='\u{065f}'
        | '\u{0670}'
        | '\u{06d6}'..='\u{06dc}'
        | '\u{06df}'..='\u{06e8}'
        | '\u{06ea}'..='\u{06ed}'
        | '\u{08d4}'..='\u{08e1}'
        | '\u{08e3}'..='\u{08ff}')
}

/// Whether a character is an Arabic letter this crate can shape.
#[must_use]
pub fn is_arabic_letter(c: char) -> bool {
    forms::lookup(c).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harakat_ranges_are_recognised() {
        // Fatha, damma, kasra, sukun, shadda and the superscript alef.
        for c in [
            '\u{064e}', '\u{064f}', '\u{0650}', '\u{0652}', '\u{0651}', '\u{0670}',
        ] {
            assert!(is_harakat(c), "{c:?}");
        }
        // Ordinary letters are not marks.
        for c in ['\u{0627}', '\u{0628}', 'a', ' '] {
            assert!(!is_harakat(c), "{c:?}");
        }
    }

    #[test]
    fn arabic_letters_are_recognised() {
        assert!(is_arabic_letter('\u{0628}'));
        assert!(is_arabic_letter('\u{0644}'));
        assert!(!is_arabic_letter('a'));
        assert!(!is_arabic_letter('\u{064e}'));
    }
}
