//! IBAN validation.
//!
//! An IBAN is validated in two independent ways: the country-specific length
//! and structure, and the ISO 7064 MOD-97-10 check over the rearranged
//! characters. Both must pass, because the check alone would accept a number
//! of the wrong length for its country.

use crate::{Error, Result};

/// An IBAN.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Iban(String);

/// Country code and expected IBAN length, from the ISO 13616 registry.
///
/// Only members of the registry appear here. An IBAN whose country is absent
/// is rejected as [`Error::UnknownCountry`] rather than guessed at.
static IBAN_LENGTHS: &[(&str, usize)] = &[
    ("AD", 24),
    ("AE", 23),
    ("AL", 28),
    ("AT", 20),
    ("AZ", 28),
    ("BA", 20),
    ("BE", 16),
    ("BG", 22),
    ("BH", 22),
    ("BR", 29),
    ("BY", 28),
    ("CH", 21),
    ("CR", 22),
    ("CY", 28),
    ("CZ", 24),
    ("DE", 22),
    ("DK", 18),
    ("DO", 28),
    ("EE", 20),
    ("EG", 29),
    ("ES", 24),
    ("FI", 18),
    ("FO", 18),
    ("FR", 27),
    ("GB", 22),
    ("GE", 22),
    ("GI", 23),
    ("GL", 18),
    ("GR", 27),
    ("GT", 28),
    ("HR", 21),
    ("HU", 28),
    ("IE", 22),
    ("IL", 23),
    ("IQ", 23),
    ("IS", 26),
    ("IT", 27),
    ("JO", 30),
    ("KW", 30),
    ("KZ", 20),
    ("LB", 28),
    ("LC", 32),
    ("LI", 21),
    ("LT", 20),
    ("LU", 20),
    ("LV", 21),
    ("MC", 27),
    ("MD", 24),
    ("ME", 22),
    ("MK", 19),
    ("MR", 27),
    ("MT", 31),
    ("MU", 30),
    ("NL", 18),
    ("NO", 15),
    ("PK", 24),
    ("PL", 28),
    ("PS", 29),
    ("PT", 25),
    ("QA", 29),
    ("RO", 24),
    ("RS", 22),
    ("SA", 24),
    ("SC", 31),
    ("SE", 24),
    ("SI", 19),
    ("SK", 24),
    ("SM", 27),
    ("ST", 25),
    ("SV", 28),
    ("TL", 23),
    ("TN", 24),
    ("TR", 26),
    ("UA", 29),
    ("VA", 22),
    ("VG", 24),
    ("XK", 20),
];

fn expected_length(country: &str) -> Option<usize> {
    IBAN_LENGTHS
        .iter()
        .find(|(code, _)| *code == country)
        .map(|(_, len)| *len)
}

impl Iban {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::WrongLength`],
    /// [`Error::UnknownCountry`], [`Error::InvalidCharacter`] or
    /// [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        if normalized.len() < 4 {
            return Err(Error::WrongLength {
                expected: 4,
                found: normalized.len(),
            });
        }
        if !normalized.chars().all(|c| c.is_ascii_alphanumeric()) {
            let bad = normalized
                .chars()
                .find(|c| !c.is_ascii_alphanumeric())
                .unwrap_or('?');
            return Err(Error::InvalidCharacter(bad));
        }
        let country = &normalized[..2];
        if !country.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(Error::UnknownCountry(country.to_string()));
        }
        let Some(expected) = expected_length(country) else {
            return Err(Error::UnknownCountry(country.to_string()));
        };
        if normalized.len() != expected {
            return Err(Error::WrongLength {
                expected,
                found: normalized.len(),
            });
        }
        if !normalized[2..4].chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::BadChecksum);
        }
        if !Self::mod97_is_zero(&normalized) {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// Rearranges the IBAN, converts letters to numbers and checks that the
    /// result is congruent to 1 modulo 97.
    fn mod97_is_zero(iban: &str) -> bool {
        // Move the first four characters to the end, then interpret the whole
        // string as a base-36 number and reduce it modulo 97 digit by digit.
        let rearranged = format!("{}{}", &iban[4..], &iban[..4]);
        let mut remainder = 0u32;
        for c in rearranged.chars() {
            let value = if let Some(d) = c.to_digit(10) {
                d
            } else if c.is_ascii_uppercase() {
                c as u32 - 'A' as u32 + 10
            } else {
                return false;
            };
            // A letter expands to two decimal digits, so it shifts by 100.
            let base = if value >= 10 { 100 } else { 10 };
            remainder = (remainder * base + value) % 97;
        }
        remainder == 1
    }

    /// The normalised IBAN.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The two-letter country code.
    #[must_use]
    pub fn country_code(&self) -> &str {
        &self.0[..2]
    }

    /// The two check digits.
    #[must_use]
    pub fn check_digits(&self) -> &str {
        &self.0[2..4]
    }

    /// The country-specific part after the check digits.
    #[must_use]
    pub fn bban(&self) -> &str {
        &self.0[4..]
    }

    /// Formats the IBAN in groups of four characters, the usual printed form.
    #[must_use]
    pub fn format_grouped(&self) -> String {
        let mut out = String::with_capacity(self.0.len() + self.0.len() / 4);
        for (i, c) in self.0.chars().enumerate() {
            if i > 0 && i % 4 == 0 {
                out.push(' ');
            }
            out.push(c);
        }
        out
    }

    /// Computes the check digits for a country code and BBAN, so callers can
    /// build an IBAN from a national account number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownCountry`] for a country outside the registry, or
    /// [`Error::InvalidCharacter`] for a non-alphanumeric BBAN.
    pub fn check_digits_for(country_code: &str, bban: &str) -> Result<String> {
        let country = country_code.to_uppercase();
        if country.len() != 2 || !country.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(Error::UnknownCountry(country_code.to_string()));
        }
        let bban = crate::normalize(bban);
        if !bban.chars().all(|c| c.is_ascii_alphanumeric()) {
            let bad = bban
                .chars()
                .find(|c| !c.is_ascii_alphanumeric())
                .unwrap_or('?');
            return Err(Error::InvalidCharacter(bad));
        }
        if let Some(expected) = expected_length(&country) {
            let found = country.len() + 2 + bban.len();
            if found != expected {
                return Err(Error::WrongLength { expected, found });
            }
        }
        // With check digits "00", find the value that makes the whole thing
        // congruent to 1 modulo 97.
        let probe = format!("{bban}{country}00");
        let mut remainder = 0u32;
        for c in probe.chars() {
            let value = if let Some(d) = c.to_digit(10) {
                d
            } else {
                c as u32 - 'A' as u32 + 10
            };
            let base = if value >= 10 { 100 } else { 10 };
            remainder = (remainder * base + value) % 97;
        }
        let check = 98 - remainder;
        Ok(format!("{check:02}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_published_examples() {
        // Examples from the ISO 13616 registry and common test data.
        for iban in [
            "DE89370400440532013000",
            "GB82WEST12345698765432",
            "FR1420041010050500013M02606",
            "NL91ABNA0417164300",
            "ES9121000418450200051332",
            "IT60X0542811101000000123456",
            "BE68539007547034",
            "CH9300762011623852957",
            "AT611904300234573201",
            "PL61109010140000071219812874",
            "SA0380000000608010167519",
            "AE070331234567890123456",
        ] {
            assert!(Iban::parse(iban).is_ok(), "{iban} should be valid");
        }
    }

    #[test]
    fn rejects_wrong_check_digits() {
        assert_eq!(
            Iban::parse("DE89370400440532013001"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            Iban::parse("GB83WEST12345698765432"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn rejects_wrong_length_for_country() {
        // Germany requires 22 characters.
        let err = Iban::parse("DE8937040044053201300").unwrap_err();
        assert_eq!(
            err,
            Error::WrongLength {
                expected: 22,
                found: 21
            }
        );
    }

    #[test]
    fn rejects_unknown_country() {
        assert_eq!(
            Iban::parse("ZZ89370400440532013000"),
            Err(Error::UnknownCountry("ZZ".into()))
        );
    }

    #[test]
    fn rejects_non_alphanumeric() {
        assert!(matches!(
            Iban::parse("DE89 3704 0044 0532 0130 0#"),
            Err(Error::InvalidCharacter('#'))
        ));
    }

    #[test]
    fn rejects_too_short() {
        assert!(matches!(Iban::parse("DE8"), Err(Error::WrongLength { .. })));
    }

    #[test]
    fn accessors_split_the_iban() {
        let i = Iban::parse("DE89370400440532013000").unwrap();
        assert_eq!(i.country_code(), "DE");
        assert_eq!(i.check_digits(), "89");
        assert_eq!(i.bban(), "370400440532013000");
    }

    #[test]
    fn groups_of_four_for_display() {
        let i = Iban::parse("DE89370400440532013000").unwrap();
        assert_eq!(i.format_grouped(), "DE89 3704 0044 0532 0130 00");
    }

    #[test]
    fn separators_and_lowercase_are_tolerated() {
        let i = Iban::parse("de89 3704 0044 0532 0130 00").unwrap();
        assert_eq!(i.as_str(), "DE89370400440532013000");
    }

    #[test]
    fn computes_check_digits() {
        // Rebuilding the published German example must reproduce "89".
        assert_eq!(
            Iban::check_digits_for("DE", "370400440532013000").unwrap(),
            "89"
        );
        assert_eq!(
            Iban::check_digits_for("GB", "WEST12345698765432").unwrap(),
            "82"
        );
        assert_eq!(
            Iban::check_digits_for("NL", "ABNA0417164300").unwrap(),
            "91"
        );
    }

    #[test]
    fn check_digits_for_rejects_bad_input() {
        assert!(matches!(
            Iban::check_digits_for("DE", "37040044053201300"),
            Err(Error::WrongLength { .. })
        ));
        assert!(matches!(
            Iban::check_digits_for("1X", "1234"),
            Err(Error::UnknownCountry(_))
        ));
    }
}
