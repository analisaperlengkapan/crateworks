//! Validation of standard numbers: national identifiers, VAT numbers and
//! IBANs.
//!
//! Each number type lives in its own module so that a program only pays for
//! what it uses, and [`validate`] offers a uniform entry point when the type
//! is only known at runtime.
//!
//! # Example
//!
//! ```
//! use stdnum_rs::{Iban, validate, NumberType};
//!
//! // IBAN: country code, two check digits, then the national part.
//! assert!(Iban::parse("GB82 WEST 1234 5698 7654 32").is_ok());
//! assert!(Iban::parse("GB82 WEST 1234 5698 7654 33").is_err());
//!
//! assert!(validate(NumberType::Iban, "DE89370400440532013000").is_ok());
//! ```
//!
//! # Scope
//!
//! Only checksum and structural validation is performed. Nothing here contacts
//! a registry, so a number that passes may still be unissued. Treat this as a
//! first filter in a form, never as proof that a number exists.

#![forbid(unsafe_code)]

pub mod ean;
pub mod iban;
pub mod id;
pub mod isbn;
pub mod luhn;

pub use ean::Ean;
pub use iban::Iban;
pub use id::{Country, IdNumber};
pub use isbn::Isbn;
pub use luhn::Luhn;

/// Why a number failed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input was empty once separators were removed.
    Empty,
    /// The input contained a character that is not allowed for this type.
    InvalidCharacter(char),
    /// The number has the wrong number of digits.
    WrongLength {
        /// Length the type requires.
        expected: usize,
        /// Length that was supplied.
        found: usize,
    },
    /// The check digit or checksum does not match the rest of the number.
    BadChecksum,
    /// The country code is not one this crate knows.
    UnknownCountry(String),
    /// The prefix does not correspond to a valid issuing country.
    InvalidPrefix,
    /// A component of the number is outside its permitted range.
    OutOfRange,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("number is empty"),
            Self::InvalidCharacter(c) => write!(f, "invalid character {c:?}"),
            Self::WrongLength { expected, found } => {
                write!(f, "expected {expected} characters, found {found}")
            }
            Self::BadChecksum => f.write_str("checksum does not match"),
            Self::UnknownCountry(code) => write!(f, "unknown country code {code:?}"),
            Self::InvalidPrefix => f.write_str("prefix does not identify a valid issuer"),
            Self::OutOfRange => f.write_str("value is out of range"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Strips spaces, hyphens and dots, and upper-cases the remainder.
///
/// Standard numbers are written with separators in documents but stored
/// without them, so every parser starts here.
#[must_use]
pub fn normalize(input: &str) -> String {
    input
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '.' | ':' | '\u{a0}'))
        .flat_map(char::to_uppercase)
        .collect()
}

/// The kind of number to validate when it is not known at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NumberType {
    /// An IBAN.
    Iban,
    /// An ISBN, either 10 or 13 digits.
    Isbn,
    /// An EAN or UPC barcode.
    Ean,
    /// A number that only uses the Luhn check, such as a payment card.
    Luhn,
    /// A national identifier for a specific country.
    National(Country),
}

/// Validates a number of a runtime-known type.
///
/// # Errors
///
/// Returns the underlying [`Error`] from the relevant parser.
pub fn validate(kind: NumberType, input: &str) -> Result<()> {
    match kind {
        NumberType::Iban => Iban::parse(input).map(|_| ()),
        NumberType::Isbn => Isbn::parse(input).map(|_| ()),
        NumberType::Ean => Ean::parse(input).map(|_| ()),
        NumberType::Luhn => Luhn::parse(input).map(|_| ()),
        NumberType::National(country) => IdNumber::parse(country, input).map(|_| ()),
    }
}

/// Shared helpers for check-digit arithmetic.
pub(crate) mod checksum {
    use super::{Error, Result};

    /// Returns whether the Luhn check digit for `digits` equals `expected`.
    ///
    /// `digits` is the payload *without* the check digit, ordered from left to
    /// right.
    pub(crate) fn luhn_matches(digits: &[u8], expected: u8) -> bool {
        let mut sum = u32::from(expected);
        let len = digits.len();
        for (i, &d) in digits.iter().enumerate() {
            // Counting from the right of the full number, the check digit sits
            // at distance zero, so a payload digit is doubled when its distance
            // from the right end is odd.
            let double = (len - i) % 2 == 1;
            let mut v = u32::from(d) * if double { 2 } else { 1 };
            if v > 9 {
                v -= 9;
            }
            sum += v;
        }
        sum % 10 == 0
    }

    /// Converts ASCII digits to their numeric values.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCharacter`] for any non-digit.
    pub(crate) fn digits_only(s: &str) -> Result<Vec<u8>> {
        s.chars()
            .map(|c| {
                c.to_digit(10)
                    .map(|d| d as u8)
                    .ok_or(Error::InvalidCharacter(c))
            })
            .collect()
    }

    /// Enforces an exact digit count.
    pub(crate) fn expect_len(digits: &[u8], expected: usize) -> Result<()> {
        if digits.len() != expected {
            return Err(Error::WrongLength {
                expected,
                found: digits.len(),
            });
        }
        Ok(())
    }

    /// The check digit that completes a GS1 payload, used by EAN, UPC and
    /// ISBN-13.
    ///
    /// The 1/3 weighting is anchored at the right, so the digit immediately
    /// left of the check digit is tripled. Anchoring matters: a 12-digit UPC-A
    /// payload and a 12-digit EAN payload weigh their digits the same way only
    /// because the anchor is the right end, not the left.
    pub(crate) fn gs1_check_digit(payload: &[u8]) -> u8 {
        let len = payload.len();
        let sum: u32 = payload
            .iter()
            .enumerate()
            .map(|(i, &d)| u32::from(d) * if (len - 1 - i) % 2 == 0 { 3 } else { 1 })
            .sum();
        ((10 - (sum % 10)) % 10) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_separators_and_upper_cases() {
        assert_eq!(normalize("gb82 west-1234.5698"), "GB82WEST12345698");
        assert_eq!(
            normalize("  DE89 3704 0044 0532 0130 00 "),
            "DE89370400440532013000"
        );
    }

    #[test]
    fn gs1_check_digit_matches_known_isbn13() {
        // ISBN-13 9780306406157: the payload is the first twelve digits.
        let payload = [9, 7, 8, 0, 3, 0, 6, 4, 0, 6, 1, 5];
        assert_eq!(checksum::gs1_check_digit(&payload), 7);
    }

    #[test]
    fn validate_dispatches_on_type() {
        assert!(validate(NumberType::Iban, "DE89370400440532013000").is_ok());
        assert!(validate(NumberType::Isbn, "9780306406157").is_ok());
        assert!(validate(NumberType::Ean, "5901234123457").is_ok());
    }

    #[test]
    fn empty_input_is_rejected_everywhere() {
        for kind in [
            NumberType::Iban,
            NumberType::Isbn,
            NumberType::Ean,
            NumberType::Luhn,
        ] {
            assert_eq!(validate(kind, "   "), Err(Error::Empty), "{kind:?}");
        }
    }
}
