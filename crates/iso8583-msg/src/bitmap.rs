//! Bitmaps: the 64-bit maps that say which fields follow.
//!
//! The first bitmap covers fields 1 to 64. Bit 1 is special: when it is set, a
//! second bitmap follows and covers fields 65 to 128. In that second bitmap bit
//! 65 plays the same role for a third bitmap covering fields 129 to 192.

use std::collections::BTreeSet;

use crate::codec::{decode_data, encode_data};
use crate::field::FieldSpec;
use crate::{Error, Result};

/// The field number whose presence announces the secondary bitmap.
pub(crate) const SECONDARY_MARKER: u8 = 1;
/// The field number whose presence announces the tertiary bitmap.
pub(crate) const TERTIARY_MARKER: u8 = 65;

/// The number of bits one bitmap carries.
const BITS_PER_BITMAP: u8 = 64;

/// Encodes the primary bitmap, plus a secondary and tertiary one if the fields
/// require them.
///
/// `present` must not contain the marker bits themselves; they are derived from
/// the highest field number present.
///
/// # Errors
///
/// Returns an error if a field number is outside 1-192 or the bitmap cannot be
/// encoded with the given specification.
pub(crate) fn encode(present: &BTreeSet<u8>, spec: &FieldSpec) -> Result<Vec<u8>> {
    let highest = present.iter().copied().max().unwrap_or(0);
    let bitmaps = match highest {
        0..=64 => 1,
        65..=128 => 2,
        _ => 3,
    };

    let mut out = Vec::new();
    for index in 0..bitmaps {
        let base = u16::from(index) * u16::from(BITS_PER_BITMAP);
        let mut bytes = [0u8; 8];
        for field in present {
            let field = u16::from(*field);
            if field <= base || field > base + u16::from(BITS_PER_BITMAP) {
                continue;
            }
            let bit = field - base; // 1-based within this bitmap
            let byte = (bit - 1) / 8;
            let offset = 7 - (bit - 1) % 8;
            bytes[byte as usize] |= 1 << offset;
        }
        // A bitmap that carries a following one has its first bit set.
        if index + 1 < bitmaps {
            bytes[0] |= 0x80;
        }
        out.extend(encode_data(field_id(index), &hex(&bytes), spec)?);
    }
    Ok(out)
}

/// Decodes the bitmap chain, returning the field numbers present and the number
/// of bytes consumed.
///
/// # Errors
///
/// Returns an error if the bytes run out or the bitmap is malformed.
pub(crate) fn decode(bytes: &[u8], spec: &FieldSpec) -> Result<(BTreeSet<u8>, usize)> {
    let mut present = BTreeSet::new();
    let mut consumed = 0usize;
    let mut base = 0u8;

    loop {
        let index = base / BITS_PER_BITMAP;
        let width = bitmap_bytes(spec);
        if bytes.len() < consumed + width {
            return Err(Error::Truncated {
                field: field_id(index),
                needed: width,
                available: bytes.len() - consumed,
            });
        }
        let raw = decode_data(field_id(index), &bytes[consumed..consumed + width], spec)?;
        consumed += width;

        let nibbles = parse_hex(field_id(index), &raw)?;
        if nibbles.len() < 8 {
            return Err(Error::InvalidValue {
                field: field_id(index),
                reason: "bitmap is shorter than eight bytes",
            });
        }
        for bit in 1..=BITS_PER_BITMAP {
            let byte = nibbles[((bit - 1) / 8) as usize];
            let offset = 7 - (bit - 1) % 8;
            if byte & (1 << offset) != 0 {
                let field = base + bit;
                // The marker bit is not itself a field.
                if field != SECONDARY_MARKER && field != TERTIARY_MARKER {
                    present.insert(field);
                }
            }
        }

        let more = nibbles[0] & 0x80 != 0;
        if !more {
            break;
        }
        if index == 2 {
            return Err(Error::BadSpec(
                "bitmap chain is longer than three bitmaps".to_string(),
            ));
        }
        base += BITS_PER_BITMAP;
    }

    Ok((present, consumed))
}

/// How many bytes one bitmap occupies under this specification.
fn bitmap_bytes(spec: &FieldSpec) -> usize {
    crate::codec::units_to_bytes(spec.max_len, spec.length_unit())
}

/// The field number to blame for a bitmap problem, so error messages point
/// somewhere useful.
fn field_id(index: u8) -> u8 {
    match index {
        0 => SECONDARY_MARKER,
        _ => TERTIARY_MARKER,
    }
}

/// Renders eight bytes as sixteen upper-case hexadecimal digits.
fn hex(bytes: &[u8; 8]) -> String {
    let mut out = String::with_capacity(16);
    for b in bytes {
        out.push(
            char::from_digit(u32::from(b >> 4), 16)
                .unwrap_or('0')
                .to_ascii_uppercase(),
        );
        out.push(
            char::from_digit(u32::from(b & 0x0f), 16)
                .unwrap_or('0')
                .to_ascii_uppercase(),
        );
    }
    out
}

/// Parses sixteen hexadecimal digits into eight bytes.
fn parse_hex(field: u8, text: &str) -> Result<Vec<u8>> {
    if text.len() % 2 != 0 {
        return Err(Error::InvalidValue {
            field,
            reason: "bitmap has an odd number of digits",
        });
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16);
            let lo = (pair[1] as char).to_digit(16);
            match (hi, lo) {
                (Some(hi), Some(lo)) => Ok(((hi << 4) | lo) as u8),
                _ => Err(Error::InvalidValue {
                    field,
                    reason: "bitmap is not hexadecimal",
                }),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::default_ascii;

    #[test]
    fn a_single_field_sets_the_expected_bit() {
        let spec = default_ascii();
        let mut present = BTreeSet::new();
        present.insert(2u8);
        let encoded = encode(&present, &spec.get(1).unwrap()).unwrap();
        // Field 2 is the second bit, so 0x40 in the first byte.
        assert_eq!(encoded, b"4000000000000000");
    }

    #[test]
    fn a_high_field_pulls_in_the_secondary_bitmap() {
        let spec = default_ascii();
        let mut present = BTreeSet::new();
        present.insert(2u8);
        present.insert(70u8);
        let encoded = encode(&present, &spec.get(1).unwrap()).unwrap();
        // Bit 1 is set for the secondary, bit 2 for field 2, and field 70 is
        // bit 6 of the second bitmap.
        assert_eq!(encoded, b"C0000000000000000400000000000000");
    }

    #[test]
    fn encode_then_decode_is_identity() {
        let spec = default_ascii();
        for fields in [
            vec![2u8],
            vec![2, 3, 4, 7, 11],
            vec![2, 70],
            vec![1u8; 0].into_iter().chain([64]).collect(),
            vec![130],
        ] {
            let present: BTreeSet<u8> = fields.iter().copied().collect();
            let encoded = encode(&present, &spec.get(1).unwrap()).unwrap();
            let (decoded, consumed) = decode(&encoded, &spec.get(1).unwrap()).unwrap();
            assert_eq!(decoded, present, "{fields:?}");
            assert_eq!(consumed, encoded.len());
        }
    }
}
