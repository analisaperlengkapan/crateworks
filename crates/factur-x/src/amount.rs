//! Monetary amounts and currency codes.

use std::fmt::{self, Display};

use crate::{Error, Result};

/// A three-letter currency code.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Currency(String);

impl Currency {
    /// Wraps a currency code, upper-casing it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValue`] unless the code is three ASCII letters.
    pub fn parse(code: &str) -> Result<Self> {
        let upper = code.trim().to_uppercase();
        if upper.len() != 3 || !upper.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(Error::InvalidValue {
                field: "BT-5".to_string(),
                found: code.to_string(),
            });
        }
        Ok(Self(upper))
    }

    /// The code as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A monetary amount.
///
/// The value is kept both as the text that appeared in the document and as
/// hundredths of a unit, because EN 16931 fixes amounts at two decimal places
/// and the business rules compare sums. Keeping the text means a generated
/// document is byte-for-byte what the sender wrote; keeping the integer means
/// the comparisons are exact rather than floating point.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount {
    /// The value in hundredths of a unit.
    cents: i64,
    /// The text as written, which is what gets emitted.
    text: String,
}

impl Amount {
    /// Parses an amount from its decimal text.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValue`] if the text is not a decimal number with
    /// at most two fraction digits.
    pub fn parse(text: &str) -> Result<Self> {
        let trimmed = text.trim();
        let (sign, rest) = match trimmed.strip_prefix('-') {
            Some(rest) => (-1i64, rest),
            None => (1i64, trimmed.strip_prefix('+').unwrap_or(trimmed)),
        };
        let mut parts = rest.split('.');
        let whole = parts.next().unwrap_or("");
        let fraction = parts.next().unwrap_or("");
        if parts.next().is_some() {
            return Err(Self::bad(text));
        }
        if whole.is_empty() && fraction.is_empty() {
            return Err(Self::bad(text));
        }
        if !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
            || fraction.len() > 2
        {
            return Err(Self::bad(text));
        }
        let whole_value: i64 = if whole.is_empty() {
            0
        } else {
            whole.parse().map_err(|_| Self::bad(text))?
        };
        let fraction_value: i64 = match fraction.len() {
            0 => 0,
            1 => fraction.parse::<i64>().map_err(|_| Self::bad(text))? * 10,
            _ => fraction.parse().map_err(|_| Self::bad(text))?,
        };
        let cents = whole_value
            .checked_mul(100)
            .and_then(|v| v.checked_add(fraction_value))
            .ok_or_else(|| Self::bad(text))?;
        Ok(Self {
            cents: sign * cents,
            text: trimmed.to_string(),
        })
    }

    /// An amount from a whole number of hundredths, formatted with two
    /// decimals.
    #[must_use]
    pub fn from_cents(cents: i64) -> Self {
        Self {
            cents,
            text: Self::format_cents(cents),
        }
    }

    /// The value in hundredths of a unit.
    #[must_use]
    pub fn cents(&self) -> i64 {
        self.cents
    }

    /// The text as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Formats hundredths as a decimal string with two fraction digits.
    fn format_cents(cents: i64) -> String {
        let sign = if cents < 0 { "-" } else { "" };
        let magnitude = cents.unsigned_abs();
        format!("{sign}{}.{:02}", magnitude / 100, magnitude % 100)
    }

    fn bad(text: &str) -> Error {
        Error::InvalidValue {
            field: "amount".to_string(),
            found: text.to_string(),
        }
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_parse_and_keep_their_text() {
        let a = Amount::parse("1190.00").unwrap();
        assert_eq!(a.cents(), 119000);
        assert_eq!(a.as_str(), "1190.00");
    }

    #[test]
    fn one_fraction_digit_is_scaled() {
        assert_eq!(Amount::parse("10.5").unwrap().cents(), 1050);
    }

    #[test]
    fn a_whole_number_has_no_fraction() {
        assert_eq!(Amount::parse("42").unwrap().cents(), 4200);
        assert_eq!(Amount::parse("42").unwrap().as_str(), "42");
    }

    #[test]
    fn negative_amounts_are_supported() {
        assert_eq!(Amount::parse("-5.25").unwrap().cents(), -525);
    }

    #[test]
    fn more_than_two_fraction_digits_is_rejected() {
        assert!(Amount::parse("1.234").is_err());
    }

    #[test]
    fn non_numeric_text_is_rejected() {
        for bad in ["", "abc", "1.2.3", "1,00", "--1"] {
            assert!(Amount::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn from_cents_formats_two_decimals() {
        assert_eq!(Amount::from_cents(119000).as_str(), "1190.00");
        assert_eq!(Amount::from_cents(-525).as_str(), "-5.25");
        assert_eq!(Amount::from_cents(5).as_str(), "0.05");
    }

    #[test]
    fn a_currency_code_must_be_three_letters() {
        assert!(Currency::parse("eur").is_ok());
        assert_eq!(Currency::parse("eur").unwrap().as_str(), "EUR");
        assert!(Currency::parse("EU").is_err());
        assert!(Currency::parse("E1R").is_err());
    }
}
