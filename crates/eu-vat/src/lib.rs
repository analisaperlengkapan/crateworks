#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod rules;

use std::fmt;

/// A European Union member state, or Northern Ireland.
///
/// Variants are ordered by their VAT prefix, so [`Country::ALL`] is sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Country {
    /// Austria, prefix `AT`.
    Austria,
    /// Belgium, prefix `BE`.
    Belgium,
    /// Bulgaria, prefix `BG`.
    Bulgaria,
    /// Cyprus, prefix `CY`.
    Cyprus,
    /// Czechia, prefix `CZ`.
    Czechia,
    /// Germany, prefix `DE`.
    Germany,
    /// Denmark, prefix `DK`.
    Denmark,
    /// Estonia, prefix `EE`.
    Estonia,
    /// Greece, prefix `EL` (also written `GR`).
    Greece,
    /// Spain, prefix `ES`.
    Spain,
    /// Finland, prefix `FI`.
    Finland,
    /// France, prefix `FR`.
    France,
    /// Croatia, prefix `HR`.
    Croatia,
    /// Hungary, prefix `HU`.
    Hungary,
    /// Ireland, prefix `IE`.
    Ireland,
    /// Italy, prefix `IT`.
    Italy,
    /// Lithuania, prefix `LT`.
    Lithuania,
    /// Luxembourg, prefix `LU`.
    Luxembourg,
    /// Latvia, prefix `LV`.
    Latvia,
    /// Malta, prefix `MT`.
    Malta,
    /// Netherlands, prefix `NL`.
    Netherlands,
    /// Poland, prefix `PL`.
    Poland,
    /// Portugal, prefix `PT`.
    Portugal,
    /// Romania, prefix `RO`.
    Romania,
    /// Sweden, prefix `SE`.
    Sweden,
    /// Slovenia, prefix `SI`.
    Slovenia,
    /// Slovakia, prefix `SK`.
    Slovakia,
    /// Northern Ireland, prefix `XI`.
    NorthernIreland,
}

impl Country {
    /// Every jurisdiction this crate can validate, ordered by prefix.
    pub const ALL: [Self; 28] = [
        Self::Austria,
        Self::Belgium,
        Self::Bulgaria,
        Self::Cyprus,
        Self::Czechia,
        Self::Germany,
        Self::Denmark,
        Self::Estonia,
        Self::Greece,
        Self::Spain,
        Self::Finland,
        Self::France,
        Self::Croatia,
        Self::Hungary,
        Self::Ireland,
        Self::Italy,
        Self::Lithuania,
        Self::Luxembourg,
        Self::Latvia,
        Self::Malta,
        Self::Netherlands,
        Self::Poland,
        Self::Portugal,
        Self::Romania,
        Self::Sweden,
        Self::Slovenia,
        Self::Slovakia,
        Self::NorthernIreland,
    ];

    /// The prefix used in the canonical form of a VAT number.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Austria => "AT",
            Self::Belgium => "BE",
            Self::Bulgaria => "BG",
            Self::Cyprus => "CY",
            Self::Czechia => "CZ",
            Self::Germany => "DE",
            Self::Denmark => "DK",
            Self::Estonia => "EE",
            Self::Greece => "EL",
            Self::Spain => "ES",
            Self::Finland => "FI",
            Self::France => "FR",
            Self::Croatia => "HR",
            Self::Hungary => "HU",
            Self::Ireland => "IE",
            Self::Italy => "IT",
            Self::Lithuania => "LT",
            Self::Luxembourg => "LU",
            Self::Latvia => "LV",
            Self::Malta => "MT",
            Self::Netherlands => "NL",
            Self::Poland => "PL",
            Self::Portugal => "PT",
            Self::Romania => "RO",
            Self::Sweden => "SE",
            Self::Slovenia => "SI",
            Self::Slovakia => "SK",
            Self::NorthernIreland => "XI",
        }
    }

    /// The English name of the jurisdiction.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Austria => "Austria",
            Self::Belgium => "Belgium",
            Self::Bulgaria => "Bulgaria",
            Self::Cyprus => "Cyprus",
            Self::Czechia => "Czechia",
            Self::Germany => "Germany",
            Self::Denmark => "Denmark",
            Self::Estonia => "Estonia",
            Self::Greece => "Greece",
            Self::Spain => "Spain",
            Self::Finland => "Finland",
            Self::France => "France",
            Self::Croatia => "Croatia",
            Self::Hungary => "Hungary",
            Self::Ireland => "Ireland",
            Self::Italy => "Italy",
            Self::Lithuania => "Lithuania",
            Self::Luxembourg => "Luxembourg",
            Self::Latvia => "Latvia",
            Self::Malta => "Malta",
            Self::Netherlands => "Netherlands",
            Self::Poland => "Poland",
            Self::Portugal => "Portugal",
            Self::Romania => "Romania",
            Self::Sweden => "Sweden",
            Self::Slovenia => "Slovenia",
            Self::Slovakia => "Slovakia",
            Self::NorthernIreland => "Northern Ireland",
        }
    }

    /// Looks up a jurisdiction by its two-letter prefix.
    ///
    /// `GR` is accepted as an alias for `EL`, since Greece's ISO 3166 code is
    /// the one most people reach for even though the VAT prefix is `EL`.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        let upper = code.to_ascii_uppercase();
        let code = match upper.as_str() {
            "GR" => "EL",
            other => other,
        };
        Self::ALL.into_iter().find(|c| c.code() == code)
    }
}

impl fmt::Display for Country {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Why a VAT number was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input was empty once separators were removed.
    Empty,
    /// The input contained a character that cannot appear in a VAT number.
    InvalidCharacter(char),
    /// The prefix is not a jurisdiction this crate knows.
    UnknownCountry(String),
    /// The national number has the wrong number of characters.
    WrongLength {
        /// Length the jurisdiction requires.
        expected: usize,
        /// Length that was supplied.
        found: usize,
    },
    /// The national number is the right length but the wrong shape, such as a
    /// letter where the jurisdiction requires a digit.
    InvalidFormat,
    /// The check digit or checksum does not match the rest of the number.
    BadChecksum,
    /// A component of the number is outside its permitted range, such as a
    /// birth date that is not a real date.
    OutOfRange,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("number is empty"),
            Self::InvalidCharacter(c) => write!(f, "invalid character {c:?}"),
            Self::UnknownCountry(code) => write!(f, "unknown country prefix {code:?}"),
            Self::WrongLength { expected, found } => {
                write!(f, "expected {expected} characters, found {found}")
            }
            Self::InvalidFormat => f.write_str("number has the wrong shape for its country"),
            Self::BadChecksum => f.write_str("checksum does not match"),
            Self::OutOfRange => f.write_str("value is out of range"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// A validated VAT number, stored in canonical form.
///
/// The canonical form is the country prefix followed by the national number
/// with every separator removed, upper-cased: `NL004495445B01`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Vat {
    country: Country,
    number: String,
}

impl Vat {
    /// Parses and validates a VAT number.
    ///
    /// Separators (spaces, dots, hyphens, slashes, colons, commas and
    /// parentheses) are ignored and the input is upper-cased, so a number
    /// copied out of an invoice can be passed as-is.
    ///
    /// # Errors
    ///
    /// Returns the reason the number is not a valid VAT number.
    pub fn parse(input: &str) -> Result<Self> {
        let cleaned = normalize(input);
        if cleaned.is_empty() {
            return Err(Error::Empty);
        }
        let (country, national) = split(&cleaned)?;
        let national = rules::canonicalize(country, national);
        rules::validate(country, &national)?;
        Ok(Self {
            country,
            number: national,
        })
    }

    /// The jurisdiction that issued the number.
    #[must_use]
    pub fn country(&self) -> Country {
        self.country
    }

    /// The national part, without the country prefix.
    #[must_use]
    pub fn number(&self) -> &str {
        &self.number
    }
}

impl fmt::Display for Vat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.country.code(), self.number)
    }
}

/// Validates a VAT number without keeping the parsed value.
///
/// # Errors
///
/// Returns the reason the number is not a valid VAT number.
///
/// # Example
///
/// ```
/// use eu_vat::validate;
///
/// assert!(validate("DE 136 695 976").is_ok());
/// // One digit changed, so the checksum no longer matches.
/// assert!(validate("DE 136 695 970").is_err());
/// ```
pub fn validate(input: &str) -> Result<()> {
    Vat::parse(input).map(|_| ())
}

/// Every jurisdiction whose rules accept this number.
///
/// A number whose prefix is unknown can still be recognised this way, which is
/// what a form needs when the user forgets to type the country code. The result
/// is normally empty or a single element; more than one is possible in
/// principle because the checks are independent.
#[must_use]
pub fn guess_country(input: &str) -> Vec<Country> {
    let cleaned = normalize(input);
    if cleaned.is_empty() {
        return Vec::new();
    }
    Country::ALL
        .into_iter()
        .filter(|&c| rules::validate(c, &cleaned).is_ok())
        .collect()
}

/// Strips separators and upper-cases the remainder.
///
/// The set matches the separators the national authorities tolerate: the
/// whitespace, dash, dot, slash, colon and the parentheses that bracket the old
/// Belgian leading zero. A comma is not among them, because a number written
/// with one is not a number the authority would issue.
fn normalize(input: &str) -> String {
    input
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '.' | '/' | ':' | '(' | ')' | '\u{a0}'))
        .flat_map(char::to_uppercase)
        .collect()
}

/// Splits a cleaned number into its jurisdiction and national part.
fn split(cleaned: &str) -> Result<(Country, String)> {
    if cleaned.len() < 2 {
        return Err(Error::WrongLength {
            expected: 2,
            found: cleaned.len(),
        });
    }
    let prefix = &cleaned[..2];
    let country =
        Country::from_code(prefix).ok_or_else(|| Error::UnknownCountry(prefix.to_string()))?;
    let national = cleaned[2..].to_string();
    if national.is_empty() {
        return Err(Error::Empty);
    }
    Ok((country, national))
}

/// Shared check-digit arithmetic.
pub(crate) mod checksum {
    use super::{Error, Result};

    /// Enforces an exact character count.
    pub(crate) fn expect_len(value: &str, expected: usize) -> Result<()> {
        if value.chars().count() != expected {
            return Err(Error::WrongLength {
                expected,
                found: value.chars().count(),
            });
        }
        Ok(())
    }

    /// Whether every character is an ASCII digit.
    pub(crate) fn is_digits(value: &str) -> bool {
        !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
    }

    /// Enforces that every character is an ASCII digit.
    pub(crate) fn require_digits(value: &str) -> Result<()> {
        if !is_digits(value) {
            let bad = value.chars().find(|c| !c.is_ascii_digit()).unwrap_or('\0');
            return Err(Error::InvalidCharacter(bad));
        }
        Ok(())
    }

    /// Parses a string of ASCII digits into a number, ignoring anything else.
    ///
    /// The longest VAT number is thirteen digits, so an `i64` is ample, and
    /// signed arithmetic keeps the negative weights used by Poland's check.
    pub(crate) fn parse_int(value: &str) -> i64 {
        value
            .bytes()
            .filter(|b| b.is_ascii_digit())
            .fold(0, |acc, b| acc * 10 + i64::from(b - b'0'))
    }

    /// The weighted sum of `value` against `weights`.
    ///
    /// Weights are signed because Poland's NIP check subtracts its final digit
    /// rather than adding it.
    pub(crate) fn weighted(value: &str, weights: &[i64]) -> i64 {
        value
            .bytes()
            .zip(weights)
            .map(|(b, w)| i64::from(b - b'0') * w)
            .sum()
    }

    /// The Luhn checksum of `value`. A valid number checksums to zero.
    pub(crate) fn luhn(value: &str) -> u32 {
        let digits: Vec<u32> = value
            .bytes()
            .rev()
            .filter_map(|b| (b as char).to_digit(10))
            .collect();
        let mut sum = 0;
        for (i, d) in digits.iter().enumerate() {
            if i % 2 == 0 {
                sum += d;
            } else {
                let doubled = d * 2;
                sum += doubled / 10 + doubled % 10;
            }
        }
        sum % 10
    }

    /// Whether `value` satisfies the Luhn check.
    pub(crate) fn luhn_ok(value: &str) -> bool {
        luhn(value) == 0
    }

    /// Whether `value` satisfies the ISO 7064 Mod 11, 10 check.
    ///
    /// Used by the German and Croatian numbers.
    pub(crate) fn mod_11_10_ok(value: &str) -> bool {
        let mut check = 5u32;
        for b in value.bytes() {
            let digit = u32::from(b - b'0');
            // A running value of zero is treated as ten.
            let carried = if check == 0 { 10 } else { check };
            check = ((carried * 2) % 11 + digit) % 10;
        }
        check == 1
    }

    /// Whether `value` satisfies the ISO 7064 Mod 97, 10 check.
    ///
    /// Letters are read as their base-36 value and the whole string is then read
    /// as a decimal number; the remainder must be one. The expansion is folded
    /// as it is built, so no large integer is ever materialised.
    pub(crate) fn mod_97_10_ok(value: &str) -> bool {
        let mut remainder = 0u32;
        for c in value.chars() {
            let v = c.to_digit(36).unwrap_or(0);
            for digit in v.to_string().bytes() {
                remainder = (remainder * 10 + u32::from(digit - b'0')) % 97;
            }
        }
        remainder == 1
    }

    /// Whether `year`/`month`/`day` is a real Gregorian date.
    pub(crate) fn is_real_date(year: i64, month: u32, day: u32) -> bool {
        if day < 1 || !(1..=12).contains(&month) {
            return false;
        }
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let max = match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        day <= max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separators_are_ignored_and_input_is_upper_cased() {
        assert_eq!(normalize("nl 0044.95445-b01"), "NL004495445B01");
        assert_eq!(normalize("BE(0)403019261"), "BE0403019261");
    }

    #[test]
    fn country_codes_round_trip_and_gr_is_an_alias() {
        for country in Country::ALL {
            assert_eq!(Country::from_code(country.code()), Some(country));
        }
        assert_eq!(Country::from_code("gr"), Some(Country::Greece));
        assert_eq!(Country::from_code("ZZ"), None);
    }

    #[test]
    fn all_is_sorted_by_prefix() {
        let codes: Vec<&str> = Country::ALL.iter().map(|c| c.code()).collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        assert_eq!(codes, sorted);
    }

    #[test]
    fn empty_and_unknown_prefix_are_rejected() {
        assert_eq!(Vat::parse("   "), Err(Error::Empty));
        assert_eq!(
            Vat::parse("ZZ123"),
            Err(Error::UnknownCountry("ZZ".to_string()))
        );
        assert_eq!(Vat::parse("NL"), Err(Error::Empty));
    }

    #[test]
    fn luhn_matches_known_values() {
        assert!(checksum::luhn_ok("4539148803436467"));
        assert!(!checksum::luhn_ok("4539148803436468"));
    }

    #[test]
    fn mod_11_10_matches_the_german_number() {
        assert!(checksum::mod_11_10_ok("136695976"));
        assert!(!checksum::mod_11_10_ok("136695978"));
    }

    #[test]
    fn mod_97_10_accepts_the_dutch_number_that_uses_it() {
        // NL002455799B11 passes this check; NL004495445B01 does not, because it
        // is validated as a citizen service number instead.
        assert!(checksum::mod_97_10_ok("NL002455799B11"));
        assert!(!checksum::mod_97_10_ok("NL004495445B01"));
    }

    #[test]
    fn real_dates_account_for_leap_years() {
        assert!(checksum::is_real_date(2024, 2, 29));
        assert!(!checksum::is_real_date(2023, 2, 29));
        assert!(checksum::is_real_date(2000, 2, 29));
        assert!(!checksum::is_real_date(1900, 2, 29));
        assert!(!checksum::is_real_date(2024, 4, 31));
    }

    #[test]
    fn guess_country_identifies_a_prefixless_number() {
        assert_eq!(guess_country("004495445B01"), vec![Country::Netherlands]);
    }
}
