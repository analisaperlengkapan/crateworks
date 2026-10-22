#![doc = include_str!("../README.md")]
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
///
/// # Example
///
/// ```
/// use stdnum_rs::{validate, NumberType};
///
/// assert!(validate(NumberType::Iban, "DE89370400440532013000").is_ok());
/// // The last digit of an IBAN is a check digit, so changing it fails.
/// assert!(validate(NumberType::Iban, "DE89370400440532013001").is_err());
/// ```
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

    /// Enforces an exact length on a string that may contain non-digits.
    pub(crate) fn expect_len_str(s: &str, expected: usize) -> Result<()> {
        if s.chars().count() != expected {
            return Err(Error::WrongLength {
                expected,
                found: s.chars().count(),
            });
        }
        Ok(())
    }

    /// Whether `day`/`month` is a real date. The year is not known, so
    /// February always allows 29.
    pub(crate) fn is_plausible_day_month(day: u32, month: u32) -> bool {
        if !(1..=12).contains(&month) {
            return false;
        }
        let max = match month {
            2 => 29,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        (1..=max).contains(&day)
    }

    /// Whether `year`/`month`/`day` is a real calendar date.
    pub(crate) fn is_real_date(year: u32, month: u32, day: u32) -> bool {
        if day < 1 || !(1..=12).contains(&month) {
            return false;
        }
        let leap =
            (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
        let max = match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        day <= max
    }

    /// Days since 1970-01-01 for a proleptic Gregorian date.
    ///
    /// This is the standard civil-from-days transform run backwards, which lets
    /// a birth date be compared against today without pulling in a date crate.
    fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
        let year = if month <= 2 { year - 1 } else { year };
        let era = if year >= 0 { year } else { year - 399 } / 400;
        let year_of_era = year - era * 400;
        let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// Days since 1970-01-01 for today, in UTC.
    fn today_days() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| (elapsed.as_secs() / 86_400) as i64)
    }

    /// Whether the date is today or earlier.
    ///
    /// Identifiers encode a birth date, and a number cannot belong to someone
    /// who has not been born yet. The comparison uses UTC, so a number can be
    /// rejected for at most one day around midnight.
    pub(crate) fn is_not_in_the_future(year: u32, month: u32, day: u32) -> bool {
        days_from_civil(i64::from(year), i64::from(month), i64::from(day)) <= today_days()
    }

    /// Whether the first day of `year` has already arrived.
    pub(crate) fn year_has_started(year: u32) -> bool {
        days_from_civil(i64::from(year), 1, 1) <= today_days()
    }

    /// Whether `digits` satisfies the ISO 7064 Mod 11, 10 check.
    ///
    /// This is the check used by the German tax identification number. A valid
    /// number runs the recurrence over every digit and lands on one.
    pub(crate) fn iso7064_mod_11_10_is_valid(digits: &[u8]) -> bool {
        let mut check = 5u32;
        for &d in digits {
            // A running value of zero is treated as ten.
            let carried = if check == 0 { 10 } else { check };
            check = ((carried * 2) % 11 + u32::from(d)) % 10;
        }
        check == 1
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
            .map(|(i, &d)| {
                u32::from(d)
                    * if (len - 1 - i).is_multiple_of(2) {
                        3
                    } else {
                        1
                    }
            })
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
