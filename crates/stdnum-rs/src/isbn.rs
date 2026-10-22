//! ISBN-10 and ISBN-13 book identifiers.

use crate::checksum;
use crate::{Error, Result};

/// An ISBN, in either its 10- or 13-digit form.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Isbn {
    digits: String,
    isbn13: bool,
}

impl Isbn {
    /// Validates `input`.
    ///
    /// Accepts an optional `ISBN` or `ISBN-13` prefix, spaces and hyphens. The
    /// final character of an ISBN-10 may be `X`, standing for ten.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`] or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let mut normalized = crate::normalize(input);
        for prefix in ["ISBN-13", "ISBN13", "ISBN"] {
            if let Some(rest) = normalized.strip_prefix(prefix) {
                normalized = rest.to_string();
                break;
            }
        }
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        match normalized.len() {
            10 => {
                if !Self::isbn10_checksum_ok(&normalized)? {
                    return Err(Error::BadChecksum);
                }
                Ok(Self {
                    digits: normalized,
                    isbn13: false,
                })
            }
            13 => {
                let digits = checksum::digits_only(&normalized)?;
                let (payload, check) = digits.split_at(12);
                if checksum::gs1_check_digit(payload) != check[0] {
                    return Err(Error::BadChecksum);
                }
                Ok(Self {
                    digits: normalized,
                    isbn13: true,
                })
            }
            other => Err(Error::WrongLength {
                expected: 13,
                found: other,
            }),
        }
    }

    fn isbn10_checksum_ok(s: &str) -> Result<bool> {
        let mut sum = 0u32;
        for (i, c) in s.chars().enumerate() {
            let value = if i == 9 && c == 'X' {
                10
            } else {
                c.to_digit(10).ok_or(Error::InvalidCharacter(c))?
            };
            // Weights run from 10 down to 1.
            sum += value * (10 - i as u32);
        }
        Ok(sum.is_multiple_of(11))
    }

    /// The normalised identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.digits
    }

    /// Whether this is the 13-digit form.
    #[must_use]
    pub fn is_isbn13(&self) -> bool {
        self.isbn13
    }

    /// Converts an ISBN-10 to its ISBN-13 equivalent.
    ///
    /// # Errors
    ///
    /// Returns an error only if the constructed number fails validation, which
    /// cannot happen for an already-valid ISBN-10.
    pub fn to_isbn13(&self) -> Result<Self> {
        if self.isbn13 {
            return Self::parse(&self.digits);
        }
        let body = &self.digits[..9];
        let payload = format!("978{body}");
        let digits = checksum::digits_only(&payload)?;
        let check = checksum::gs1_check_digit(&digits);
        Self::parse(&format!("{payload}{check}"))
    }

    /// The registration group element, roughly the language area.
    ///
    /// The element boundaries are defined by ranges rather than by a fixed
    /// length, so this returns the part up to the first hyphen in the input and
    /// cannot be exact for a hyphen-less number.
    #[must_use]
    pub fn group_prefix(&self) -> &str {
        let body = if self.isbn13 {
            &self.digits[3..]
        } else {
            &self.digits
        };
        &body[..1.min(body.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_known_isbn13() {
        let i = Isbn::parse("9780306406157").unwrap();
        assert!(i.is_isbn13());
        assert_eq!(i.as_str(), "9780306406157");
    }

    #[test]
    fn accepts_known_isbn10() {
        let i = Isbn::parse("0306406152").unwrap();
        assert!(!i.is_isbn13());
    }

    #[test]
    fn accepts_isbn10_ending_in_x() {
        // 097522980X is a published ISBN-10 with an X check character.
        assert!(Isbn::parse("097522980X").is_ok());
    }

    #[test]
    fn rejects_bad_checksum() {
        assert_eq!(Isbn::parse("9780306406158"), Err(Error::BadChecksum));
        assert_eq!(Isbn::parse("0306406153"), Err(Error::BadChecksum));
    }

    #[test]
    fn rejects_bad_length_and_characters() {
        assert!(matches!(
            Isbn::parse("12345"),
            Err(Error::WrongLength { .. })
        ));
        assert!(matches!(
            Isbn::parse("978030640615A"),
            Err(Error::InvalidCharacter('A'))
        ));
    }

    #[test]
    fn strips_the_isbn_prefix() {
        assert!(Isbn::parse("ISBN 978-0-306-40615-7").is_ok());
        assert!(Isbn::parse("ISBN-13: 978-0-306-40615-7").is_ok());
    }

    #[test]
    fn isbn10_converts_to_isbn13() {
        let ten = Isbn::parse("0306406152").unwrap();
        let thirteen = ten.to_isbn13().unwrap();
        assert_eq!(thirteen.as_str(), "9780306406157");
        assert!(thirteen.is_isbn13());
    }

    #[test]
    fn converting_isbn13_is_idempotent() {
        let i = Isbn::parse("9780306406157").unwrap();
        assert_eq!(i.to_isbn13().unwrap().as_str(), "9780306406157");
    }
}
