//! Encoding and decoding of field values and length prefixes.
//!
//! Values travel through the crate as strings, because that is how switch
//! specifications write them: an amount is `000000010000`, not the integer ten
//! thousand. A binary-coded decimal field is then a string of digits that
//! happens to occupy half a byte per character on the wire, and a binary field
//! is a string of hexadecimal digits.

use crate::field::{FieldSpec, LengthUnit};
use crate::{Error, Result};

/// How a field's data is encoded on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataEncoding {
    /// One byte per character, upper-cased ASCII.
    Ascii,
    /// Two digits per byte, so a value must have an even length or be padded.
    Bcd,
    /// A string of hexadecimal digits, two per byte.
    Binary,
    /// One byte per character in EBCDIC code page 500.
    Ebcdic,
}

/// How a length prefix is encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthEncoding {
    /// The length as ASCII digits.
    Ascii,
    /// The length as a binary number in the given number of bytes.
    Binary,
    /// The length as binary-coded decimal digits.
    Bcd,
    /// The length as EBCDIC digits.
    Ebcdic,
}

/// How to fill the last unit when a value does not divide evenly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Padding {
    /// The value must already fill whole units.
    None,
    /// Pad on the left with the given character.
    Left(char),
    /// Pad on the right with the given character.
    Right(char),
}

/// Encodes `value` into bytes according to `spec`.
///
/// # Errors
///
/// Returns [`Error::TooLong`], [`Error::WrongLength`] or
/// [`Error::InvalidValue`] when the value does not fit the field.
pub(crate) fn encode_data(field: u8, value: &str, spec: &FieldSpec) -> Result<Vec<u8>> {
    match spec.data {
        DataEncoding::Ascii | DataEncoding::Ebcdic => {
            let bytes = if spec.data == DataEncoding::Ascii {
                value.to_ascii_uppercase().into_bytes()
            } else {
                to_ebcdic(&value.to_ascii_uppercase())?
            };
            if bytes.len() > spec.max_len {
                return Err(Error::TooLong {
                    field,
                    max: spec.max_len,
                    found: bytes.len(),
                });
            }
            Ok(bytes)
        }
        DataEncoding::Bcd => {
            if !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err(Error::InvalidValue {
                    field,
                    reason: "binary-coded decimal field needs digits",
                });
            }
            if value.len() > spec.max_len {
                return Err(Error::TooLong {
                    field,
                    max: spec.max_len,
                    found: value.len(),
                });
            }
            let padded = pad_nibbles(field, value, spec)?;
            Ok(nibbles_to_bytes(&padded))
        }
        DataEncoding::Binary => {
            if !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Error::InvalidValue {
                    field,
                    reason: "binary field needs hexadecimal digits",
                });
            }
            // A binary field's maximum counts bytes, so the digit count is
            // twice the maximum.
            if value.len() > spec.max_len * 2 {
                return Err(Error::TooLong {
                    field,
                    max: spec.max_len * 2,
                    found: value.len(),
                });
            }
            let padded = pad_nibbles(field, value, spec)?;
            Ok(nibbles_to_bytes(&padded))
        }
    }
}

/// Decodes field data back into a string.
///
/// # Errors
///
/// Returns [`Error::InvalidValue`] if the bytes are not valid for the encoding.
pub(crate) fn decode_data(field: u8, bytes: &[u8], spec: &FieldSpec) -> Result<String> {
    match spec.data {
        DataEncoding::Ascii => Ok(String::from_utf8_lossy(bytes).into_owned()),
        DataEncoding::Ebcdic => from_ebcdic(field, bytes),
        DataEncoding::Bcd => {
            let mut out = String::with_capacity(bytes.len() * 2);
            for b in bytes {
                out.push(char::from(b'0' + (b >> 4)));
                out.push(char::from(b'0' + (b & 0x0f)));
            }
            Ok(out)
        }
        DataEncoding::Binary => {
            let mut out = String::with_capacity(bytes.len() * 2);
            for b in bytes {
                out.push(hex_digit(b >> 4));
                out.push(hex_digit(b & 0x0f));
            }
            Ok(out)
        }
    }
}

/// Applies the field's padding until the value fills whole units.
fn pad_nibbles(field: u8, value: &str, spec: &FieldSpec) -> Result<String> {
    if value.len().is_multiple_of(2) {
        return Ok(value.to_string());
    }
    match spec.padding {
        Padding::Left(c) => Ok(format!("{c}{value}")),
        Padding::Right(c) => Ok(format!("{value}{c}")),
        Padding::None => Err(Error::InvalidValue {
            field,
            reason: "odd number of nibbles and no padding rule",
        }),
    }
}

/// Packs a string of hexadecimal digits into bytes.
fn nibbles_to_bytes(digits: &str) -> Vec<u8> {
    digits
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16).unwrap_or(0);
            let lo = (pair[1] as char).to_digit(16).unwrap_or(0);
            ((hi << 4) | lo) as u8
        })
        .collect()
}

/// A single hexadecimal digit as a character.
fn hex_digit(value: u8) -> char {
    char::from_digit(u32::from(value), 16)
        .unwrap_or('0')
        .to_ascii_uppercase()
}

/// Encodes a length prefix.
///
/// # Errors
///
/// Returns [`Error::BadSpec`] when the length does not fit the prefix.
pub(crate) fn encode_length(length: usize, spec: &FieldSpec) -> Result<Vec<u8>> {
    let units = spec.kind.prefix_units();
    if units == 0 {
        return Ok(Vec::new());
    }
    match spec.length {
        LengthEncoding::Ascii => Ok(format!("{length:0>width$}", width = units).into_bytes()),
        LengthEncoding::Ebcdic => Ok(to_ebcdic(&format!("{length:0>width$}", width = units))?),
        LengthEncoding::Binary => {
            let mut out = vec![0u8; units];
            for (i, byte) in out.iter_mut().enumerate() {
                let shift = 8 * (units - 1 - i);
                if shift < usize::BITS as usize {
                    *byte = ((length >> shift) & 0xff) as u8;
                }
            }
            Ok(out)
        }
        LengthEncoding::Bcd => {
            // Each nibble holds one decimal digit, so two digits per byte.
            let width = units * 2;
            let text = format!("{length:0>width$}");
            Ok(nibbles_to_bytes(&text))
        }
    }
}

/// Decodes a length prefix, returning the length and the bytes consumed.
///
/// # Errors
///
/// Returns [`Error::Truncated`] if the prefix does not fit, or
/// [`Error::InvalidValue`] if it is not a valid number.
pub(crate) fn decode_length(field: u8, bytes: &[u8], spec: &FieldSpec) -> Result<(usize, usize)> {
    let units = spec.kind.prefix_units();
    if units == 0 {
        return Ok((spec.max_len, 0));
    }
    let needed = match spec.length {
        LengthEncoding::Bcd => units.div_ceil(2),
        _ => units,
    };
    if bytes.len() < needed {
        return Err(Error::Truncated {
            field,
            needed,
            available: bytes.len(),
        });
    }
    let window = &bytes[..needed];
    let text = match spec.length {
        LengthEncoding::Ascii => String::from_utf8_lossy(window).into_owned(),
        LengthEncoding::Ebcdic => from_ebcdic(field, window)?,
        LengthEncoding::Binary => {
            let mut value = 0usize;
            for b in window {
                value = value * 256 + usize::from(*b);
            }
            return Ok((value, needed));
        }
        LengthEncoding::Bcd => {
            let mut out = String::with_capacity(needed * 2);
            for b in window {
                out.push(char::from(b'0' + (b >> 4)));
                out.push(char::from(b'0' + (b & 0x0f)));
            }
            out
        }
    };
    let value = text
        .trim()
        .parse::<usize>()
        .map_err(|_| Error::InvalidValue {
            field,
            reason: "length prefix is not a number",
        })?;
    Ok((value, needed))
}

/// How many bytes a field of `length` units occupies.
pub(crate) fn units_to_bytes(length: usize, unit: LengthUnit) -> usize {
    match unit {
        LengthUnit::Bytes => length,
        LengthUnit::Nibbles => length.div_ceil(2),
    }
}

/// Translates one character to its EBCDIC code page 500 byte.
///
/// Only the printable characters a switch uses are mapped. Anything else is an
/// error rather than a substitution, so a value that cannot be represented
/// fails loudly instead of arriving corrupted at the other end.
fn ebcdic_byte(c: char) -> Option<u8> {
    let code = match c {
        ' ' => 0x40,
        '.' => 0x4b,
        '<' => 0x4c,
        '(' => 0x4d,
        '+' => 0x4e,
        '|' => 0x4f,
        '&' => 0x50,
        '!' => 0x5a,
        '$' => 0x5b,
        '*' => 0x5c,
        ')' => 0x5d,
        ';' => 0x5e,
        '-' => 0x60,
        '/' => 0x61,
        ',' => 0x6b,
        '%' => 0x6c,
        '_' => 0x6d,
        '>' => 0x6e,
        '?' => 0x6f,
        '`' => 0x79,
        ':' => 0x7a,
        '#' => 0x7b,
        '@' => 0x7c,
        '\'' => 0x7d,
        '=' => 0x7e,
        '"' => 0x7f,
        '0'..='9' => 0xf0 + (c as u8 - b'0'),
        'A'..='I' => 0xc1 + (c as u8 - b'A'),
        'J'..='R' => 0xd1 + (c as u8 - b'J'),
        'S'..='Z' => 0xe2 + (c as u8 - b'S'),
        _ => return None,
    };
    Some(code)
}

/// The inverse of [`ebcdic_byte`].
fn ebcdic_char(b: u8) -> Option<char> {
    let c = match b {
        0x40 => ' ',
        0x4b => '.',
        0x4c => '<',
        0x4d => '(',
        0x4e => '+',
        0x4f => '|',
        0x50 => '&',
        0x5a => '!',
        0x5b => '$',
        0x5c => '*',
        0x5d => ')',
        0x5e => ';',
        0x60 => '-',
        0x61 => '/',
        0x6b => ',',
        0x6c => '%',
        0x6d => '_',
        0x6e => '>',
        0x6f => '?',
        0x79 => '`',
        0x7a => ':',
        0x7b => '#',
        0x7c => '@',
        0x7d => '\'',
        0x7e => '=',
        0x7f => '"',
        0xf0..=0xf9 => char::from(b'0' + (b - 0xf0)),
        0xc1..=0xc9 => char::from(b'A' + (b - 0xc1)),
        0xd1..=0xd9 => char::from(b'J' + (b - 0xd1)),
        0xe2..=0xe9 => char::from(b'S' + (b - 0xe2)),
        _ => return None,
    };
    Some(c)
}

/// Translates ASCII text to EBCDIC code page 500.
fn to_ebcdic(text: &str) -> Result<Vec<u8>> {
    text.chars()
        .map(|c| {
            ebcdic_byte(c).ok_or(Error::InvalidValue {
                field: 0,
                reason: "character has no EBCDIC code page 500 mapping",
            })
        })
        .collect()
}

/// Translates EBCDIC code page 500 back to ASCII.
fn from_ebcdic(field: u8, bytes: &[u8]) -> Result<String> {
    bytes
        .iter()
        .map(|b| {
            ebcdic_char(*b).ok_or(Error::InvalidValue {
                field,
                reason: "byte has no EBCDIC code page 500 mapping",
            })
        })
        .collect()
}
