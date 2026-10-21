//! EAN and UPC barcodes.

use crate::checksum;
use crate::{Error, Result};

/// An EAN-8, EAN-13, UPC-A or UPC-E barcode.
///
/// All four use the same alternating 1/3 weighted check digit, and differ only
/// in length: EAN-8 is 8 digits, UPC-E is 8, UPC-A is 12 and EAN-13 is 13.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ean(String);

impl Ean {
    /// Validates `input` and returns the normalised digits.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`] or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        let digits = checksum::digits_only(&normalized)?;
        if !matches!(digits.len(), 8 | 12 | 13) {
            return Err(Error::WrongLength {
                expected: 13,
                found: digits.len(),
            });
        }
        let (payload, check) = digits.split_at(digits.len() - 1);
        if check[0] != checksum::gs1_check_digit(payload) {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The number of digits.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false`; a valid barcode has at least eight digits.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Whether this is a 13-digit EAN.
    #[must_use]
    pub fn is_ean13(&self) -> bool {
        self.0.len() == 13
    }

    /// Whether this is a 12-digit UPC-A.
    #[must_use]
    pub fn is_upca(&self) -> bool {
        self.0.len() == 12
    }

    /// The GS1 prefix, which identifies the issuing member organisation. The
    /// first three digits are a registration authority, not a country of
    /// manufacture.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.0[..3.min(self.0.len())]
    }

    /// Converts a 12-digit UPC-A into the equivalent 13-digit EAN.
    ///
    /// # Errors
    ///
    /// Returns [`Error::WrongLength`] if this is not a UPC-A.
    pub fn to_ean13(&self) -> Result<Self> {
        if !self.is_upca() {
            return Err(Error::WrongLength {
                expected: 12,
                found: self.0.len(),
            });
        }
        Self::parse(&format!("0{}", self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_known_ean13() {
        let e = Ean::parse("5901234123457").unwrap();
        assert_eq!(e.as_str(), "5901234123457");
        assert!(e.is_ean13());
        assert_eq!(e.prefix(), "590");
    }

    #[test]
    fn accepts_known_upca() {
        let e = Ean::parse("036000291452").unwrap();
        assert!(e.is_upca());
        assert_eq!(e.len(), 12);
    }

    #[test]
    fn accepts_known_ean8() {
        assert!(Ean::parse("96385074").is_ok());
    }

    #[test]
    fn rejects_bad_check_digit() {
        assert_eq!(Ean::parse("5901234123458"), Err(Error::BadChecksum));
        assert_eq!(Ean::parse("036000291453"), Err(Error::BadChecksum));
    }

    #[test]
    fn rejects_unsupported_lengths() {
        assert!(matches!(
            Ean::parse("123456"),
            Err(Error::WrongLength { .. })
        ));
        assert!(matches!(
            Ean::parse("12345678901234567"),
            Err(Error::WrongLength { .. })
        ));
    }

    #[test]
    fn upca_converts_to_ean13() {
        let e = Ean::parse("036000291452").unwrap().to_ean13().unwrap();
        assert_eq!(e.as_str(), "0036000291452");
        assert!(e.is_ean13());
    }

    #[test]
    fn separators_are_ignored() {
        assert!(Ean::parse("590 1234 123457").is_ok());
    }
}
