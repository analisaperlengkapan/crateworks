//! Escape sequences in keys and values.
//!
//! A backslash introduces an escape when it is not itself escaped. Two escapes
//! are not the obvious ones: a `\u` not followed by four hex digits is an error
//! rather than a literal `u`, and a backslash before an ordinary character is
//! dropped rather than kept.

use std::fmt;

/// What went wrong while reading a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A `\u` escape that was not followed by four hexadecimal digits.
    BadEscape {
        /// The escape as it appeared, starting with the backslash.
        escape: String,
    },
    /// The input could not be read from the stream.
    Io,
}

impl Error {
    /// The offending escape sequence, if this was a bad escape.
    #[must_use]
    pub fn escape(&self) -> Option<&str> {
        match self {
            Self::BadEscape { escape } => Some(escape),
            Self::Io => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadEscape { escape } => {
                write!(f, "invalid \\u escape sequence: {escape}")
            }
            Self::Io => f.write_str("could not read the properties input"),
        }
    }
}

impl std::error::Error for Error {}

/// Decodes the escape sequences in a field.
///
/// `\uXXXX` becomes the code point, `\t`, `\n`, `\f` and `\r` become the control
/// characters, `\\` becomes a single backslash, and any other backslash is
/// dropped.
///
/// The result is built as code points rather than characters, because Java
/// writes anything above the basic multilingual plane as a surrogate pair and a
/// lone surrogate is not a `char`. The pair is rejoined at the end.
pub(crate) fn unescape(field: &str) -> Result<String, Error> {
    let mut code_points: Vec<u32> = Vec::with_capacity(field.len());
    let mut chars = field.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '\\' {
            code_points.push(c as u32);
            continue;
        }
        let Some(&next) = chars.peek() else {
            // A lone trailing backslash is dropped, as Java does.
            break;
        };
        chars.next();
        match next {
            't' => code_points.push('\t' as u32),
            'n' => code_points.push('\n' as u32),
            'f' => code_points.push(0x0c),
            'r' => code_points.push('\r' as u32),
            'u' => code_points.push(read_unicode(&mut chars)?),
            other => code_points.push(other as u32),
        }
    }

    Ok(combine_surrogates(&code_points))
}

/// Reads the four hex digits of a `\uXXXX` escape.
///
/// The value is returned as it stands, so a surrogate stays a surrogate and can
/// be paired with the one after it.
fn read_unicode(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<u32, Error> {
    let mut digits = String::with_capacity(4);
    for _ in 0..4 {
        match chars.next() {
            Some(c) => digits.push(c),
            None => break,
        }
    }
    if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::BadEscape {
            escape: format!("\\u{digits}"),
        });
    }
    Ok(u32::from_str_radix(&digits, 16).expect("four hex digits"))
}

/// Turns code points into text, rejoining surrogate pairs.
///
/// A lone surrogate has no character to become, and Rust text cannot hold one,
/// so it is replaced. That is the one place this differs from Java, which keeps
/// the raw unit; the case cannot arise in text that was ever valid Unicode.
fn combine_surrogates(code_points: &[u32]) -> String {
    const REPLACEMENT: char = '\u{fffd}';
    let mut out = String::with_capacity(code_points.len());
    let mut index = 0;

    while index < code_points.len() {
        let value = code_points[index];
        if (0xd800..=0xdbff).contains(&value) && index + 1 < code_points.len() {
            let low = code_points[index + 1];
            if (0xdc00..=0xdfff).contains(&low) {
                let combined = 0x10000 + ((value - 0xd800) << 10) + (low - 0xdc00);
                if let Some(c) = char::from_u32(combined) {
                    out.push(c);
                    index += 2;
                    continue;
                }
            }
        }
        out.push(char::from_u32(value).unwrap_or(REPLACEMENT));
        index += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_simple_escapes_decode() {
        assert_eq!(unescape(r"a\tb\nc\rd\fe").unwrap(), "a\tb\nc\rd\u{000c}e");
    }

    #[test]
    fn two_backslashes_become_one() {
        assert_eq!(unescape(r"a\\b").unwrap(), r"a\b");
    }

    #[test]
    fn a_backslash_before_an_ordinary_character_is_dropped() {
        // Not kept, and not an error: `\q` is `q`.
        assert_eq!(unescape(r"\q").unwrap(), "q");
        assert_eq!(unescape(r"a\ b").unwrap(), "a b");
        assert_eq!(unescape(r"a\=b").unwrap(), "a=b");
    }

    #[test]
    fn unicode_escapes_decode() {
        assert_eq!(unescape(r"\u0041").unwrap(), "A");
        assert_eq!(unescape(r"\u00e9").unwrap(), "é");
        assert_eq!(unescape(r"\u4e2d\u6587").unwrap(), "中文");
        // Hex case does not matter.
        assert_eq!(unescape(r"\u00E9").unwrap(), "é");
    }

    #[test]
    fn a_unicode_escape_that_is_too_short_is_an_error() {
        let error = unescape(r"\u004").unwrap_err();
        assert_eq!(error.escape(), Some(r"\u004"));
        assert_eq!(error.to_string(), r"invalid \u escape sequence: \u004");
    }

    #[test]
    fn a_unicode_escape_with_a_non_hex_digit_is_an_error() {
        assert!(unescape(r"\u00g1").is_err());
        assert!(unescape(r"\u").is_err());
    }

    #[test]
    fn a_surrogate_pair_becomes_one_character() {
        // U+1F600, written the way Java writes it.
        assert_eq!(unescape(r"\ud83d\ude00").unwrap(), "😀");
    }

    #[test]
    fn a_lone_surrogate_becomes_the_replacement_character() {
        // Java would keep the raw code unit; Rust text cannot hold a lone
        // surrogate, so it is replaced. A lone surrogate is not valid Unicode and
        // cannot appear in text that was ever well formed.
        let text = unescape(r"\ud83d").unwrap();
        assert_eq!(text, "\u{fffd}");
    }

    #[test]
    fn a_trailing_backslash_is_dropped() {
        assert_eq!(unescape(r"abc\").unwrap(), "abc");
    }
}
