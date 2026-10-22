//! A reader for ASN.1 DER, the encoding everything a PAdES signature contains
//! is written in.
//!
//! Only what a signature needs is here: definite lengths, the universal types
//! that appear in CMS and X.509, and the context-specific tags CMS uses for
//! optional fields. BER's indefinite lengths are rejected, because a CMS object
//! is DER and accepting indefinite lengths would mean accepting a second
//! encoding of the same value, which is exactly the ambiguity a signature must
//! not have.
//!
//! Every read is bounds-checked and returns [`Error`] rather than panicking, so a
//! truncated or hostile blob is an error and never an out-of-range slice.

use std::fmt;

/// What can go wrong while reading DER.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input ended in the middle of a value.
    Truncated,
    /// A length used a form this reader does not accept, such as the indefinite
    /// length of BER, or a length that does not fit in a `usize`.
    BadLength,
    /// A tag was not the one the caller expected.
    UnexpectedTag {
        /// The tag that was found.
        found: u8,
        /// The tag that was required.
        expected: u8,
    },
    /// A value was the right tag but not a valid instance of that type, such as
    /// an integer with no bytes or an object identifier that ends mid-arc.
    BadValue,
    /// There were bytes left over after a value that should have filled them.
    Trailing,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => write!(f, "the value ends before its length says it does"),
            Self::BadLength => write!(f, "the length is not one DER allows"),
            Self::UnexpectedTag { found, expected } => {
                write!(f, "expected tag {expected:#04x}, found {found:#04x}")
            }
            Self::BadValue => write!(f, "the value is not a valid instance of its type"),
            Self::Trailing => write!(f, "bytes remain after the value"),
        }
    }
}

impl std::error::Error for Error {}

/// A result from this module.
pub type Result<T> = std::result::Result<T, Error>;

/// `BOOLEAN`.
pub const BOOLEAN: u8 = 0x01;
/// `INTEGER`.
pub const INTEGER: u8 = 0x02;
/// `BIT STRING`.
pub const BIT_STRING: u8 = 0x03;
/// `OCTET STRING`.
pub const OCTET_STRING: u8 = 0x04;
/// `NULL`.
pub const NULL: u8 = 0x05;
/// `OBJECT IDENTIFIER`.
pub const OID: u8 = 0x06;
/// `UTF8String`.
pub const UTF8_STRING: u8 = 0x0C;
/// `PrintableString`.
pub const PRINTABLE_STRING: u8 = 0x13;
/// `IA5String`.
pub const IA5_STRING: u8 = 0x16;
/// `UTCTime`.
pub const UTC_TIME: u8 = 0x17;
/// `GeneralizedTime`.
pub const GENERALIZED_TIME: u8 = 0x18;
/// `SEQUENCE`, and the constructed form generally.
pub const SEQUENCE: u8 = 0x30;
/// `SET`, and the constructed form generally.
pub const SET: u8 = 0x31;

/// The context-specific tag with the given number, in its primitive form.
#[must_use]
pub const fn context(number: u8) -> u8 {
    0x80 | number
}

/// The context-specific tag with the given number, in its constructed form.
#[must_use]
pub const fn context_constructed(number: u8) -> u8 {
    0xA0 | number
}

/// One tag-length-value: the tag, and the bytes of the value without the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element<'a> {
    /// The identifier octet. Multi-byte tags are not used by CMS or X.509.
    pub tag: u8,
    /// The value, with its own header removed.
    pub value: &'a [u8],
}

impl<'a> Element<'a> {
    /// A reader positioned at the start of this value, for a constructed type.
    #[must_use]
    pub fn reader(&self) -> Reader<'a> {
        Reader::new(self.value)
    }

    /// The value as an integer, rejecting a non-minimal encoding.
    ///
    /// DER requires the fewest bytes that hold the value, so a leading `0x00`
    /// followed by a byte below `0x80` is a second encoding of a number that has
    /// one already. Signatures must not be malleable, so that is an error.
    ///
    /// # Errors
    ///
    /// [`Error::BadValue`] when the value is empty or not minimally encoded.
    pub fn integer(&self) -> Result<u64> {
        if self.value.is_empty() {
            return Err(Error::BadValue);
        }
        if self.value.len() > 1 && self.value[0] == 0 && self.value[1] < 0x80 {
            return Err(Error::BadValue);
        }
        if self.value[0] == 0 && self.value.len() > 9 {
            return Err(Error::BadValue);
        }
        let mut value = 0u64;
        for &byte in self.value {
            value = (value << 8) | u64::from(byte);
        }
        Ok(value)
    }

    /// The value as an unsigned big-endian integer of arbitrary width, with the
    /// sign byte removed.
    ///
    /// An RSA modulus and an RSA signature are integers wider than a `u64`, so
    /// they are kept as bytes. The leading `0x00` DER adds to keep the value
    /// positive is stripped, because the arithmetic downstream is unsigned.
    #[must_use]
    pub fn unsigned_bytes(&self) -> &'a [u8] {
        let mut bytes = self.value;
        while bytes.len() > 1 && bytes[0] == 0 {
            bytes = &bytes[1..];
        }
        bytes
    }

    /// The value as an object identifier, written the usual dotted way.
    ///
    /// # Errors
    ///
    /// [`Error::BadValue`] when the encoding ends in the middle of an arc.
    pub fn oid(&self) -> Result<String> {
        let mut out = String::new();
        let mut first = true;
        let mut arc: u64 = 0;
        let mut pending = false;
        for &byte in self.value {
            // The high bit means "another byte belongs to this arc".
            arc = (arc << 7) | u64::from(byte & 0x7F);
            pending = byte & 0x80 != 0;
            if !pending {
                if first {
                    // The first byte holds the first two arcs: 40 * a + b.
                    let (a, b) = if arc < 40 {
                        (0, arc)
                    } else if arc < 80 {
                        (1, arc - 40)
                    } else {
                        (2, arc - 80)
                    };
                    out.push_str(&format!("{a}.{b}"));
                    first = false;
                } else {
                    out.push_str(&format!(".{arc}"));
                }
                arc = 0;
            }
        }
        if pending || first {
            return Err(Error::BadValue);
        }
        Ok(out)
    }

    /// The bytes of a `BIT STRING`, with the unused-bits count removed.
    ///
    /// # Errors
    ///
    /// [`Error::BadValue`] when the unused-bits count is missing or out of range.
    pub fn bit_string(&self) -> Result<&'a [u8]> {
        let (&unused, rest) = self.value.split_first().ok_or(Error::BadValue)?;
        if unused > 7 {
            return Err(Error::BadValue);
        }
        Ok(rest)
    }

    /// The value as text, when it is a string type DER encodes as the bytes
    /// themselves.
    ///
    /// # Errors
    ///
    /// [`Error::BadValue`] when the bytes are not valid UTF-8.
    pub fn text(&self) -> Result<&'a str> {
        std::str::from_utf8(self.value).map_err(|_| Error::BadValue)
    }

    /// The value as a boolean.
    #[must_use]
    pub fn boolean(&self) -> bool {
        self.value.iter().any(|&byte| byte != 0)
    }
}

/// Reads tag-length-value elements from a slice, in order.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    /// A reader over `bytes`.
    #[must_use]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    /// Whether every byte has been read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.position >= self.bytes.len()
    }

    /// The bytes that have not been read.
    #[must_use]
    pub fn remaining(&self) -> &'a [u8] {
        self.bytes.get(self.position..).unwrap_or_default()
    }

    /// Reads the next element, whatever its tag.
    ///
    /// # Errors
    ///
    /// [`Error::Truncated`] when the header or the value runs past the end, and
    /// [`Error::BadLength`] for an indefinite or oversized length.
    pub fn read(&mut self) -> Result<Element<'a>> {
        let header = self.bytes.get(self.position..).ok_or(Error::Truncated)?;
        let (&tag, rest) = header.split_first().ok_or(Error::Truncated)?;
        if tag & 0x1F == 0x1F {
            // A multi-byte tag number. Neither CMS nor X.509 uses one.
            return Err(Error::BadValue);
        }
        let (&first_length, rest) = rest.split_first().ok_or(Error::Truncated)?;
        let (length, consumed) = if first_length & 0x80 == 0 {
            (usize::from(first_length), 1)
        } else {
            let count = usize::from(first_length & 0x7F);
            if count == 0 {
                // Indefinite length, which is BER. DER requires a definite one.
                return Err(Error::BadLength);
            }
            if count > 8 {
                return Err(Error::BadLength);
            }
            let digits = rest.get(..count).ok_or(Error::Truncated)?;
            let mut length = 0usize;
            for &byte in digits {
                length = (length << 8) | usize::from(byte);
            }
            (length, 1 + count)
        };
        let start = self.position + 1 + consumed;
        let value = self
            .bytes
            .get(start..start + length)
            .ok_or(Error::Truncated)?;
        self.position = start + length;
        Ok(Element { tag, value })
    }

    /// Reads the next element and requires it to carry `tag`.
    ///
    /// # Errors
    ///
    /// Whatever [`Reader::read`] returns, and [`Error::UnexpectedTag`] when the
    /// tag is not the expected one.
    pub fn expect(&mut self, tag: u8) -> Result<Element<'a>> {
        let element = self.read()?;
        if element.tag != tag {
            return Err(Error::UnexpectedTag {
                found: element.tag,
                expected: tag,
            });
        }
        Ok(element)
    }

    /// Reads the next element if it carries `tag`, and leaves the reader where it
    /// was otherwise.
    ///
    /// # Errors
    ///
    /// [`Error::Truncated`] and [`Error::BadLength`] from reading the header.
    pub fn optional(&mut self, tag: u8) -> Result<Option<Element<'a>>> {
        let saved = self.position;
        match self.read() {
            Ok(element) if element.tag == tag => Ok(Some(element)),
            Ok(_) => {
                self.position = saved;
                Ok(None)
            }
            Err(Error::Truncated) => {
                // Running out of input is how "the optional field is absent"
                // looks at the end of a sequence.
                self.position = saved;
                Ok(None)
            }
            Err(other) => Err(other),
        }
    }

    /// Requires that every byte has been read.
    ///
    /// # Errors
    ///
    /// [`Error::Trailing`] when bytes remain.
    pub fn finish(&self) -> Result<()> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(Error::Trailing)
        }
    }
}

/// Reads a single element that fills the whole slice.
///
/// # Errors
///
/// [`Error::Trailing`] when the element does not fill the slice, plus whatever
/// the read itself reports.
pub fn one(bytes: &[u8]) -> Result<Element<'_>> {
    let mut reader = Reader::new(bytes);
    let element = reader.read()?;
    reader.finish()?;
    Ok(element)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_short_length() {
        let mut reader = Reader::new(&[0x02, 0x01, 0x05]);
        let element = reader.expect(INTEGER).unwrap();
        assert_eq!(element.integer().unwrap(), 5);
        reader.finish().unwrap();
    }

    #[test]
    fn reads_a_long_length() {
        // An OCTET STRING of 200 bytes, so the length needs two bytes.
        let mut bytes = vec![0x04, 0x81, 0xC8];
        bytes.extend(std::iter::repeat_n(0xAB, 200));
        let mut reader = Reader::new(&bytes);
        let element = reader.expect(OCTET_STRING).unwrap();
        assert_eq!(element.value.len(), 200);
    }

    #[test]
    fn rejects_an_indefinite_length() {
        let mut reader = Reader::new(&[0x30, 0x80, 0x00, 0x00]);
        assert_eq!(reader.read(), Err(Error::BadLength));
    }

    #[test]
    fn rejects_a_truncated_value() {
        let mut reader = Reader::new(&[0x02, 0x05, 0x01]);
        assert_eq!(reader.read(), Err(Error::Truncated));
    }

    #[test]
    fn reads_an_object_identifier() {
        // 1.2.840.113549.1.7.2, the signedData content type.
        let element = one(&[
            0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02,
        ])
        .unwrap();
        assert_eq!(element.oid().unwrap(), "1.2.840.113549.1.7.2");
    }

    #[test]
    fn reads_the_other_first_arcs() {
        // 2.5.4.3, the common name attribute. The first byte is 0x55 = 85 = 2*40+5.
        let element = one(&[0x06, 0x03, 0x55, 0x04, 0x03]).unwrap();
        assert_eq!(element.oid().unwrap(), "2.5.4.3");
        // 0.9.2342, where the first arc is 0.
        let element = one(&[0x06, 0x03, 0x09, 0x92, 0x26]).unwrap();
        assert_eq!(element.oid().unwrap(), "0.9.2342");
    }

    #[test]
    fn rejects_an_oid_that_ends_mid_arc() {
        let element = one(&[0x06, 0x02, 0x2A, 0x86]).unwrap();
        assert_eq!(element.oid(), Err(Error::BadValue));
    }

    #[test]
    fn strips_the_sign_byte_from_a_large_integer() {
        let element = one(&[0x02, 0x03, 0x00, 0x80, 0x01]).unwrap();
        assert_eq!(element.unsigned_bytes(), &[0x80, 0x01]);
    }

    #[test]
    fn rejects_a_non_minimal_integer() {
        // 0x00 0x7F is a second encoding of 127.
        let element = one(&[0x02, 0x02, 0x00, 0x7F]).unwrap();
        assert_eq!(element.integer(), Err(Error::BadValue));
    }

    #[test]
    fn reads_a_bit_string() {
        let element = one(&[0x03, 0x03, 0x00, 0xAA, 0xBB]).unwrap();
        assert_eq!(element.bit_string().unwrap(), &[0xAA, 0xBB]);
    }

    #[test]
    fn reads_optional_fields() {
        // A SEQUENCE holding an INTEGER then an OCTET STRING.
        let bytes = [0x30, 0x06, 0x02, 0x01, 0x01, 0x04, 0x01, 0xFF];
        let mut reader = Reader::new(&bytes);
        let sequence = reader.expect(SEQUENCE).unwrap();
        let mut inner = sequence.reader();
        assert!(inner.optional(NULL).unwrap().is_none());
        assert_eq!(inner.expect(INTEGER).unwrap().integer().unwrap(), 1);
        assert_eq!(inner.expect(OCTET_STRING).unwrap().value, &[0xFF]);
        inner.finish().unwrap();
    }

    #[test]
    fn reports_trailing_bytes() {
        let element = one(&[0x02, 0x01, 0x05, 0xFF]);
        assert_eq!(element, Err(Error::Trailing));
    }
}
