#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

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
