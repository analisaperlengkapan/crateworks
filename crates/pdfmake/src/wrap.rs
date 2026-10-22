//! Wrapping text into lines that fit a column.
//!
//! Text is broken at whitespace. A single word longer than the column is broken
//! between characters rather than allowed to overflow, because an overflowing
//! word in a table cell is worse than a hyphen-less split.

use crate::fonts::StandardFont;

/// One line of text after wrapping, with the width it occupies.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    /// The text of the line, without its trailing space.
    pub text: String,
    /// The line's width at the size it was measured at.
    pub width: f32,
}

/// Break `text` into lines no wider than `max_width`.
///
/// Existing newlines are honoured, and every explicit line is wrapped again if
/// it is too long. `max_width` is in points, and so is the result.
pub(crate) fn wrap(text: &str, font: StandardFont, size: f32, max_width: f32) -> Vec<Line> {
    let mut lines = Vec::new();

    for paragraph in text.split('\n') {
        // A newline with nothing between it and the next one is a blank line,
        // which has to survive.
        if paragraph.is_empty() {
            lines.push(Line {
                text: String::new(),
                width: 0.0,
            });
            continue;
        }
        wrap_one(paragraph, font, size, max_width, &mut lines);
    }

    // `split` gives one empty piece for an empty string, which is the right
    // answer: an empty paragraph is one empty line.
    if lines.is_empty() {
        lines.push(Line {
            text: String::new(),
            width: 0.0,
        });
    }
    lines
}

/// Wrap a single line of text, appending to `out`.
fn wrap_one(text: &str, font: StandardFont, size: f32, max_width: f32, out: &mut Vec<Line>) {
    // A column too narrow for a space still has to produce something, so never
    // treat the width as zero or negative.
    let max_width = max_width.max(1.0);
    let mut current = String::new();
    let mut current_width = 0.0f32;

    for word in text.split(' ') {
        let word_width = font.width_of(word, size);
        let space_width = if current.is_empty() {
            0.0
        } else {
            font.width_of(" ", size)
        };

        if !current.is_empty() && current_width + space_width + word_width > max_width {
            out.push(Line {
                text: std::mem::take(&mut current),
                width: current_width,
            });
            current_width = 0.0;
        }

        if !current.is_empty() {
            current.push(' ');
            current_width += space_width;
        }

        if word_width > max_width && current.is_empty() {
            // The word cannot fit on a line of its own, so break it by
            // character. Characters are pushed one at a time until the next one
            // would overflow.
            let mut piece = String::new();
            let mut piece_width = 0.0f32;
            for c in word.chars() {
                let char_width = font.width_of(&c.to_string(), size);
                if piece_width + char_width > max_width && !piece.is_empty() {
                    out.push(Line {
                        text: std::mem::take(&mut piece),
                        width: piece_width,
                    });
                    piece_width = 0.0;
                }
                piece.push(c);
                piece_width += char_width;
            }
            current = piece;
            current_width = piece_width;
        } else {
            current.push_str(word);
            current_width += word_width;
        }
    }

    if !current.is_empty() || out.is_empty() {
        out.push(Line {
            text: current,
            width: current_width,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(lines: &[Line]) -> Vec<&str> {
        lines.iter().map(|l| l.text.as_str()).collect()
    }

    #[test]
    fn short_text_is_one_line() {
        let lines = wrap("hello", StandardFont::Helvetica, 10.0, 1000.0);
        assert_eq!(texts(&lines), vec!["hello"]);
    }

    #[test]
    fn text_wraps_at_spaces() {
        // At 10pt Helvetica, "alpha beta" is 25.56 + 2.78 + 22.24 = 50.58pt.
        let lines = wrap("alpha beta", StandardFont::Helvetica, 10.0, 30.0);
        assert_eq!(texts(&lines), vec!["alpha", "beta"]);
    }

    #[test]
    fn the_wrapped_width_never_exceeds_the_column() {
        let text = "the quick brown fox jumps over the lazy dog";
        for width in [40.0, 60.0, 90.0, 120.0, 200.0] {
            for line in wrap(text, StandardFont::TimesRoman, 11.0, width) {
                assert!(
                    line.width <= width + 0.001,
                    "line {:?} is {} wide, column is {width}",
                    line.text,
                    line.width
                );
            }
        }
    }

    #[test]
    fn explicit_newlines_are_kept() {
        let lines = wrap("one\ntwo", StandardFont::Helvetica, 10.0, 1000.0);
        assert_eq!(texts(&lines), vec!["one", "two"]);
    }

    #[test]
    fn blank_lines_survive() {
        let lines = wrap("one\n\ntwo", StandardFont::Helvetica, 10.0, 1000.0);
        assert_eq!(texts(&lines), vec!["one", "", "two"]);
    }

    #[test]
    fn an_empty_string_is_one_empty_line() {
        let lines = wrap("", StandardFont::Helvetica, 10.0, 100.0);
        assert_eq!(texts(&lines), vec![""]);
    }

    #[test]
    fn a_word_longer_than_the_column_is_broken() {
        // Courier is 600/1000 per character, so 10pt Courier fits 5 characters
        // in 30 points.
        let lines = wrap("abcdefghij", StandardFont::Courier, 10.0, 30.0);
        assert_eq!(texts(&lines), vec!["abcde", "fghij"]);
        for line in &lines {
            assert!(line.width <= 30.001, "{:?} is too wide", line.text);
        }
    }

    #[test]
    fn breaking_a_long_word_loses_no_characters() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let lines = wrap(text, StandardFont::Courier, 10.0, 25.0);
        let rejoined: String = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(rejoined, text);
    }

    #[test]
    fn a_narrow_column_still_produces_lines() {
        let lines = wrap("abc", StandardFont::Courier, 10.0, 0.0);
        assert!(!lines.is_empty());
        let rejoined: String = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(rejoined, "abc");
    }

    #[test]
    fn multibyte_text_is_wrapped_by_character_not_byte() {
        // Each of these is one character and one glyph; a byte-wise split would
        // cut them in half.
        let lines = wrap("éééé", StandardFont::Courier, 10.0, 18.0);
        let rejoined: String = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(rejoined, "éééé");
        assert!(lines.iter().all(|l| l.text.chars().count() <= 3));
    }
}
