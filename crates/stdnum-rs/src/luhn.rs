//! The Luhn check, used by payment cards and several national identifiers.

use crate::checksum;
use crate::{Error, Result};

/// A number that is valid only if it satisfies the Luhn check.
///
/// This is the check used by payment card numbers, IMEIs, and the Canadian
/// Social Insurance Number. It detects any single-digit error and most
/// transpositions, but it says nothing about whether the number was ever
/// issued.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Luhn(String);

impl Luhn {
    /// Validates `input` and returns the normalised digits.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`] or
    /// [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        let digits = checksum::digits_only(&normalized)?;
        if digits.len() < 2 {
            return Err(Error::WrongLength {
                expected: 2,
                found: digits.len(),
            });
        }
        let (payload, check) = digits.split_at(digits.len() - 1);
        if !checksum::luhn_matches(payload, check[0]) {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The check digit.
    #[must_use]
    pub fn check_digit(&self) -> u8 {
        self.0.as_bytes()[self.0.len() - 1] - b'0'
    }

    /// Computes the check digit for a payload, so callers can generate numbers.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCharacter`] if `payload` is not all digits.
    pub fn check_digit_for(payload: &str) -> Result<u8> {
        let digits = checksum::digits_only(payload)?;
        for candidate in 0u8..=9 {
            if checksum::luhn_matches(&digits, candidate) {
                return Ok(candidate);
            }
        }
        unreachable!("exactly one candidate satisfies the Luhn check")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_known_valid_numbers() {
        // Published test card numbers.
        for n in [
            "4539148803436467",
            "4111111111111111",
            "5555555555554444",
            "79927398713",
        ] {
            assert!(Luhn::parse(n).is_ok(), "{n} should be valid");
        }
    }

    #[test]
    fn rejects_single_digit_error() {
        assert_eq!(Luhn::parse("4539148803436468"), Err(Error::BadChecksum));
    }

    #[test]
    fn rejects_transposition() {
        // Swapping the two digits before the check digit must be caught.
        assert_eq!(Luhn::parse("4539148803436647"), Err(Error::BadChecksum));
    }

    #[test]
    fn separators_are_ignored() {
        let a = Luhn::parse("4539 1488 0343 6467").unwrap();
        assert_eq!(a.as_str(), "4539148803436467");
        assert_eq!(a.check_digit(), 7);
    }

    #[test]
    fn computes_check_digit() {
        assert_eq!(Luhn::check_digit_for("453914880343646").unwrap(), 7);
        assert_eq!(Luhn::check_digit_for("7992739871").unwrap(), 3);
    }

    #[test]
    fn rejects_non_digits_and_short_input() {
        assert!(matches!(
            Luhn::parse("1234abcd"),
            Err(Error::InvalidCharacter('A'))
        ));
        assert!(matches!(Luhn::parse("7"), Err(Error::WrongLength { .. })));
        assert_eq!(Luhn::parse("  "), Err(Error::Empty));
    }
}
