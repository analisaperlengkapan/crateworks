//! Line handling and the key/value split.
//!
//! The separator rule is the fiddly part. A key ends at the first unescaped `=`,
//! `:`, or whitespace, and "unescaped" means preceded by an even number of
//! backslashes. That is why `a\=b=c` has the key `a=b` and the value `c`, while
//! `a\\=b` has the key `a\` and the value `b`.

use std::io::Read;

use crate::escape::{Error, unescape};

/// Reads a `.properties` document from bytes.
///
/// The bytes are decoded as ISO-8859-1, which is what `java.util.Properties`
/// assumes when it is handed a stream. Use [`parse_str`] for text you have
/// already decoded.
///
/// # Errors
///
/// Returns an error if a `\uXXXX` escape is malformed.
pub fn parse(bytes: &[u8]) -> Result<Vec<(String, String)>, Error> {
    let text: String = bytes.iter().map(|byte| char::from(*byte)).collect();
    parse_str(&text)
}

/// Reads a `.properties` document from text that has already been decoded.
///
/// # Errors
///
/// Returns an error if a `\uXXXX` escape is malformed.
///
/// # Example
///
/// ```
/// use javaproperties::parse_str;
///
/// let entries = parse_str("a = one\nb: two\n").unwrap();
/// assert_eq!(entries, vec![
///     ("a".to_string(), "one".to_string()),
///     ("b".to_string(), "two".to_string()),
/// ]);
/// ```
pub fn parse_str(text: &str) -> Result<Vec<(String, String)>, Error> {
    let lines = split_lines(text);
    let mut today = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let source = &lines[index];
        index += 1;

        if is_comment(source) || is_blank(source) {
            continue;
        }

        let mut line = trim_ends(source);

        // A trailing unescaped backslash joins the next line, whose leading
        // whitespace is dropped.
        while ends_with_continuation(&line) {
            line.pop();
            let next = lines.get(index).map_or("", String::as_str);
            index += 1;
            line.extend(trim_ends(next));
        }

        if line.is_empty() {
            continue;
        }

        match split_key_value(&line) {
            Some((key, value)) => {
                today.push((unescape(&key)?, unescape(&value)?));
            }
            None => {
                let whole: String = line.iter().collect();
                today.push((unescape(&whole)?, String::new()));
            }
        }
    }

    Ok(today)
}

/// Reads a document from anything readable.
///
/// # Errors
///
/// Returns an error if the reader fails or a `\uXXXX` escape is malformed.
pub fn load(mut reader: impl Read) -> Result<Vec<(String, String)>, Error> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).map_err(|_| Error::Io)?;
    parse(&buffer)
}

/// The key/value pairs of a document, in the order they appeared.
///
/// A key that appears more than once is kept each time; [`Properties::get`]
/// returns the last, which is what Java does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Properties {
    pairs: Vec<(String, String)>,
}

impl Properties {
    /// Builds from pairs that have already been parsed.
    #[must_use]
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Self {
        Self { pairs }
    }

    /// Parses bytes as ISO-8859-1.
    ///
    /// # Errors
    ///
    /// Returns an error if a `\uXXXX` escape is malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Self::from_pairs(parse(bytes)?))
    }

    /// The number of entries, counting duplicates.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether there are no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// The value for a key, taking the last if it is repeated.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .rev()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value.as_str())
    }

    /// The entries in order, including duplicates.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.pairs
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }
}

/// The characters that count as whitespace in this format.
///
/// Not `\r` or `\n`: those end lines and are handled before this matters.
fn is_space(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\u{000c}'
}

/// A line is a comment when its first non-blank character is `#` or `!`.
fn is_comment(source: &str) -> bool {
    let mut chars = source.chars();
    for c in chars.by_ref() {
        if is_space(c) {
            continue;
        }
        return c == '#' || c == '!';
    }
    false
}

/// A line with nothing but whitespace, with or without its terminator.
fn is_blank(source: &str) -> bool {
    trim_ends(source).is_empty()
}

/// Removes leading whitespace and the line terminator.
fn trim_ends(source: &str) -> Vec<char> {
    let start = source
        .char_indices()
        .find(|(_, c)| !is_space(*c))
        .map_or(source.len(), |(index, _)| index);
    source[start..]
        .trim_end_matches(['\r', '\n'])
        .chars()
        .collect()
}

/// Whether the line ends with an odd number of backslashes.
///
/// An even number means the last backslash is itself escaped, so the line does
/// not continue.
fn ends_with_continuation(line: &[char]) -> bool {
    let mut count = 0;
    for c in line.iter().rev() {
        if *c == '\\' {
            count += 1;
        } else {
            break;
        }
    }
    count % 2 == 1
}

/// Splits a line into its key and value.
///
/// Returns `None` when there is no separator, in which case the whole line is
/// the key and the value is empty.
fn split_key_value(line: &[char]) -> Option<(String, String)> {
    let (start, end) = find_separator(line)?;
    let key: String = line[..start].iter().collect();
    let value: String = line[end..].iter().collect();
    Some((key, value))
}

/// Finds the separator, returning where the key ends and the value begins.
///
/// The separator is the first run of whitespace, or the first `=` or `:`
/// together with the whitespace before and after it, that is not preceded by an
/// odd number of backslashes.
fn find_separator(line: &[char]) -> Option<(usize, usize)> {
    for start in 0..line.len() {
        // A separator is not separated by a backslash, so the run of escaped
        // backslashes before it must be even. Starting on a backslash whose
        // predecessor is one would make the run odd, so those starts are skipped.
        if start > 0 && line[start - 1] == '\\' {
            continue;
        }
        let mut position = start;
        loop {
            if let Some(end) = match_separator(line, position) {
                return Some((position, end));
            }
            // One more pair of escaped backslashes may sit before the separator.
            if position + 1 < line.len() && line[position] == '\\' && line[position + 1] == '\\' {
                position += 2;
            } else {
                break;
            }
        }
    }
    None
}

/// Tries to match the separator starting exactly at `position`.
///
/// Returns where the value begins, which is past the separator and any
/// whitespace after it.
fn match_separator(line: &[char], position: usize) -> Option<usize> {
    // `=:` after optional whitespace.
    let mut cursor = position;
    while cursor < line.len() && is_space(line[cursor]) {
        cursor += 1;
    }
    if cursor < line.len() && (line[cursor] == '=' || line[cursor] == ':') {
        cursor += 1;
        while cursor < line.len() && is_space(line[cursor]) {
            cursor += 1;
        }
        return Some(cursor);
    }
    // Or a single whitespace character.
    if position < line.len() && is_space(line[position]) {
        let mut cursor = position + 1;
        while cursor < line.len() && is_space(line[cursor]) {
            cursor += 1;
        }
        return Some(cursor);
    }
    None
}

/// Splits text into lines, keeping the terminator.
///
/// `\n`, `\r\n` and a lone `\r` all end a line, which matches how Java reads a
/// file whose line endings are not the host's.
fn split_lines(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut index = 0;

    while index < chars.len() {
        let c = chars[index];
        if c == '\n' {
            current.push('\n');
            lines.push(std::mem::take(&mut current));
        } else if c == '\r' {
            current.push('\r');
            if chars.get(index + 1) == Some(&'\n') {
                current.push('\n');
                index += 1;
            }
            lines.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
        index += 1;
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_keep_their_terminators() {
        assert_eq!(split_lines("a\nb\n"), vec!["a\n", "b\n"]);
        assert_eq!(split_lines("a"), vec!["a"]);
        assert_eq!(split_lines(""), Vec::<String>::new());
        assert_eq!(split_lines("a\r\nb"), vec!["a\r\n", "b"]);
        assert_eq!(split_lines("a\rb"), vec!["a\r", "b"]);
    }

    #[test]
    fn the_separator_is_the_first_unescaped_one() {
        let split = |text: &str| {
            let line: Vec<char> = text.chars().collect();
            split_key_value(&line)
        };
        assert_eq!(split("a=b"), Some(("a".into(), "b".into())));
        assert_eq!(split("a=b=c"), Some(("a".into(), "b=c".into())));
        // The key is returned as written; the escapes are decoded afterwards,
        // so the backslash is still there at this point.
        assert_eq!(split(r"a\=b=c"), Some((r"a\=b".into(), "c".into())));
        assert_eq!(split(r"a\\=b"), Some((r"a\\".into(), "b".into())));
        assert_eq!(split("a b=c"), Some(("a".into(), "b=c".into())));
        assert_eq!(split("a"), None);
    }

    #[test]
    fn continuation_needs_an_odd_number_of_backslashes() {
        let chars = |text: &str| text.chars().collect::<Vec<_>>();
        // One backslash continues; two do not, because the second escapes the
        // first; none does not.
        assert!(ends_with_continuation(&chars(r"a=b\")));
        assert!(!ends_with_continuation(&chars(r"a=b\\")));
        assert!(!ends_with_continuation(&chars("a=b")));
    }
}
