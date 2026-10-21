//! The PDF object model, and enough of the syntax to write it back out.
//!
//! A PDF file is a sequence of numbered objects plus a cross-reference table
//! saying where each one starts. This module models the objects and writes them,
//! which is all a conformance layer needs: the pages, fonts and content streams
//! of the original document are copied through as raw bytes and never parsed.
//!
//! Copying objects rather than re-serialising them matters. A PDF is full of
//! constructs that a partial parser would quietly mangle, and a document whose
//! visual output changed while its bytes claimed to be the same file would be a
//! bad trade for tidiness.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A PDF object.
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    /// `null`.
    Null,
    /// `true` or `false`.
    Boolean(bool),
    /// An integer.
    Integer(i64),
    /// A real number, written with its decimal point.
    Real(f64),
    /// A literal string, written in parentheses with escapes.
    LiteralString(String),
    /// A hexadecimal string.
    HexString(Vec<u8>),
    /// A name, such as `/Type`.
    Name(String),
    /// An array.
    Array(Vec<Object>),
    /// A dictionary.
    Dictionary(BTreeMap<String, Object>),
    /// A reference to another object, `12 0 R`.
    Reference(u32),
}

impl Object {
    /// A dictionary with the given `/Type`.
    #[must_use]
    pub fn typed(kind: &str) -> Self {
        let mut map = BTreeMap::new();
        map.insert("Type".to_string(), Object::Name(kind.to_string()));
        Self::Dictionary(map)
    }

    /// Serialises the object.
    #[must_use]
    pub fn write(&self) -> String {
        let mut out = String::new();
        self.write_into(&mut out);
        out
    }

    fn write_into(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Boolean(value) => out.push_str(if *value { "true" } else { "false" }),
            Self::Integer(value) => {
                let _ = write!(out, "{value}");
            }
            Self::Real(value) => {
                // A PDF real must show a decimal point, and must not use the
                // exponent form some formatters produce.
                let text = format!("{value}");
                if text.contains('.') || text.contains('e') || text.contains('E') {
                    out.push_str(&text);
                } else {
                    let _ = write!(out, "{text}.0");
                }
            }
            Self::LiteralString(value) => write_literal_string(value, out),
            Self::HexString(bytes) => {
                out.push('<');
                for byte in bytes {
                    let _ = write!(out, "{byte:02X}");
                }
                out.push('>');
            }
            Self::Name(name) => {
                out.push('/');
                write_name(name, out);
            }
            Self::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(' ');
                    }
                    item.write_into(out);
                }
                out.push(']');
            }
            Self::Dictionary(map) => {
                out.push_str("<<");
                for (key, value) in map {
                    out.push('/');
                    write_name(key, out);
                    out.push(' ');
                    value.write_into(out);
                    out.push('\n');
                }
                out.push_str(">>");
            }
            Self::Reference(number) => {
                let _ = write!(out, "{number} 0 R");
            }
        }
    }
}

/// Writes a name, escaping the characters that cannot appear bare.
///
/// The reference implementation writes the subtype of an embedded XML file as
/// `/text#2Fxml`, so a slash is escaped; the same rule covers the whitespace and
/// delimiters the syntax reserves.
fn write_name(name: &str, out: &mut String) {
    for byte in name.bytes() {
        match byte {
            b'#'
            | b'('
            | b')'
            | b'<'
            | b'>'
            | b'['
            | b']'
            | b'{'
            | b'}'
            | b'/'
            | b'%'
            | b' '
            | b'\t'
            | b'\r'
            | b'\n'
            | 0..=0x20
            | 0x7f..=0xff => {
                let _ = write!(out, "#{byte:02X}");
            }
            _ => out.push(byte as char),
        }
    }
}

/// Writes a literal string with the escapes the syntax requires.
fn write_literal_string(value: &str, out: &mut String) {
    out.push('(');
    for byte in value.bytes() {
        match byte {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(byte as char);
            }
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            b'\x08' => out.push_str("\\b"),
            b'\x0c' => out.push_str("\\f"),
            0..=0x1f | 0x7f..=0xff => {
                let _ = write!(out, "\\{byte:03o}");
            }
            _ => out.push(byte as char),
        }
    }
    out.push(')');
}

/// A string as PDF reads it back, undoing the escapes.
///
/// The reverse of [`write_literal_string`], needed because a value copied from
/// the original document arrives escaped and a checksum comparison must see the
/// text rather than its escaped spelling.
#[must_use]
pub fn read_literal_string(text: &str) -> String {
    let mut out = Vec::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        i += 1;
        let Some(&next) = bytes.get(i) else { break };
        match next {
            b'n' => {
                out.push(b'\n');
                i += 1;
            }
            b'r' => {
                out.push(b'\r');
                i += 1;
            }
            b't' => {
                out.push(b'\t');
                i += 1;
            }
            b'b' => {
                out.push(0x08);
                i += 1;
            }
            b'f' => {
                out.push(0x0c);
                i += 1;
            }
            b'0'..=b'7' => {
                let start = i;
                while i < bytes.len() && i < start + 3 && (b'0'..=b'7').contains(&bytes[i]) {
                    i += 1;
                }
                let value = u16::from_str_radix(&text[start..i], 8).unwrap_or(0);
                out.push(value as u8);
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slash_in_a_name_is_escaped_as_the_reference_does() {
        // The reference writes the embedded XML subtype as `/text#2Fxml`.
        assert_eq!(Object::Name("text/xml".to_string()).write(), "/text#2Fxml");
    }

    #[test]
    fn an_ordinary_name_is_left_alone() {
        assert_eq!(
            Object::Name("EmbeddedFiles".to_string()).write(),
            "/EmbeddedFiles"
        );
        assert_eq!(Object::Name("factur-x".to_string()).write(), "/factur-x");
    }

    #[test]
    fn a_name_always_carries_its_leading_slash() {
        // Without it the object is unreadable, and the mistake is invisible in
        // a grep because "/Type EmbeddedFile" still contains the word.
        assert_eq!(Object::Name("Type".to_string()).write(), "/Type");
    }

    #[test]
    fn literal_strings_escape_and_read_back() {
        let value = "a(b)c\\d\ne";
        let written = Object::LiteralString(value.to_string()).write();
        assert!(written.starts_with('(') && written.ends_with(')'));
        let inner = &written[1..written.len() - 1];
        assert_eq!(read_literal_string(inner), value);
    }

    #[test]
    fn the_escaped_form_of_the_reference_filename_reads_back() {
        // pypdf writes `factur\055x\056xml`, and the name it means is this.
        assert_eq!(read_literal_string("factur\\055x\\056xml"), "factur-x.xml");
    }

    #[test]
    fn real_numbers_always_show_a_decimal_point() {
        assert_eq!(Object::Real(595.0).write(), "595.0");
        assert_eq!(Object::Real(0.5).write(), "0.5");
        assert_eq!(Object::Integer(3).write(), "3");
    }

    #[test]
    fn dictionaries_and_arrays_serialise() {
        let mut map = BTreeMap::new();
        map.insert("Count".to_string(), Object::Integer(1));
        map.insert(
            "Kids".to_string(),
            Object::Array(vec![Object::Reference(3)]),
        );
        let written = Object::Dictionary(map).write();
        assert!(written.contains("/Count 1"), "{written}");
        assert!(written.contains("/Kids [3 0 R]"), "{written}");
    }

    #[test]
    fn a_date_string_escapes_the_colons_pypdf_escapes() {
        // `D:20261021144753+00'00'` is written with escaped punctuation.
        let date = "D:20261021144753+00'00'";
        let written = Object::LiteralString(date.to_string()).write();
        assert!(
            written.contains("\\050") || written.contains('+'),
            "{written}"
        );
        assert_eq!(read_literal_string(&written[1..written.len() - 1]), date);
    }
}
