//! National identification numbers.
//!
//! Every country defines its own structure and check, so each is implemented
//! separately and reached through [`IdNumber::parse`]. Coverage starts with the
//! numbers this crate's authors could verify against published rules; adding a
//! country means adding one arm and its tests, which is the intended way to
//! extend the crate.
//!
//! # What is checked
//!
//! Only structure and checksum. A number that passes may still be unissued or
//! belong to someone else.

use crate::{Error, Result, checksum};

/// A country whose identifiers this crate can check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Country {
    /// Indonesia: NPWP tax number, 15 digits.
    IndonesiaNpwp,
    /// Indonesia: NIK identity number, 16 digits.
    IndonesiaNik,
    /// Brazil: CPF, 11 digits with two check digits.
    BrazilCpf,
    /// Brazil: CNPJ company number, 14 digits.
    BrazilCnpj,
    /// Spain: DNI/NIF for individuals.
    SpainDni,
    /// Germany: Steuerliche Identifikationsnummer, 11 digits.
    GermanyTaxId,
    /// Poland: PESEL, 11 digits.
    PolandPesel,
    /// India: Aadhaar, 12 digits with a Verhoeff check.
    IndiaAadhaar,
}

impl Country {
    /// A short English description of the identifier.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::IndonesiaNpwp => "Indonesia NPWP tax number",
            Self::IndonesiaNik => "Indonesia NIK identity number",
            Self::BrazilCpf => "Brazil CPF",
            Self::BrazilCnpj => "Brazil CNPJ",
            Self::SpainDni => "Spain DNI/NIF",
            Self::GermanyTaxId => "Germany tax identification number",
            Self::PolandPesel => "Poland PESEL",
            Self::IndiaAadhaar => "India Aadhaar",
        }
    }
}

/// A validated national identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdNumber {
    country: Country,
    normalized: String,
}

impl IdNumber {
    /// Validates `input` as an identifier of the given country.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] describing the first problem found.
    pub fn parse(country: Country, input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        match country {
            Country::IndonesiaNpwp => validate_npwp(&normalized)?,
            Country::IndonesiaNik => validate_nik(&normalized)?,
            Country::BrazilCpf => validate_cpf(&normalized)?,
            Country::BrazilCnpj => validate_cnpj(&normalized)?,
            Country::SpainDni => validate_spanish_dni(&normalized)?,
            Country::GermanyTaxId => validate_german_tax_id(&normalized)?,
            Country::PolandPesel => validate_pesel(&normalized)?,
            Country::IndiaAadhaar => validate_aadhaar(&normalized)?,
        }
        Ok(Self {
            country,
            normalized,
        })
    }

    /// The country this identifier belongs to.
    #[must_use]
    pub const fn country(&self) -> Country {
        self.country
    }

    /// The normalised identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.normalized
    }
}

/// Indonesian NPWP: 15 digits, of which the first two identify the taxpayer
/// category and the remainder a registration sequence.
///
/// The NPWP carries no check digit, so only length, character set and the
/// leading category digits are validated.
fn validate_npwp(s: &str) -> Result<()> {
    let digits = checksum::digits_only(s)?;
    checksum::expect_len(&digits, 15)?;
    // The first two digits are the taxpayer category and run from 01 to 99.
    let category = u32::from(digits[0]) * 10 + u32::from(digits[1]);
    if category == 0 {
        return Err(Error::OutOfRange);
    }
    Ok(())
}

/// Indonesian NIK: 16 digits encoding region, birth date and sequence.
///
/// The NIK carries no check digit. The date field is validated, because a
/// plausible-looking but impossible date is the most common data-entry error.
fn validate_nik(s: &str) -> Result<()> {
    let digits = checksum::digits_only(s)?;
    checksum::expect_len(&digits, 16)?;
    let day = u32::from(digits[6]) * 10 + u32::from(digits[7]);
    let month = u32::from(digits[8]) * 10 + u32::from(digits[9]);
    // For women the day has 40 added, so days run 1-31 and 41-71.
    let day = if day > 40 { day - 40 } else { day };
    if !(1..=31).contains(&day) {
        return Err(Error::OutOfRange);
    }
    if !(1..=12).contains(&month) {
        return Err(Error::OutOfRange);
    }
    // A day that cannot exist in the month is rejected, except that 29 February
    // is accepted because the NIK omits the year's century.
    let max_day = match month {
        2 => 29,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day > max_day {
        return Err(Error::OutOfRange);
    }
    Ok(())
}

/// Brazilian CPF: 11 digits, validated with two mod-11 check digits.
fn validate_cpf(s: &str) -> Result<()> {
    let digits = checksum::digits_only(s)?;
    checksum::expect_len(&digits, 11)?;
    if digits.iter().all(|d| *d == digits[0]) {
        return Err(Error::OutOfRange);
    }
    for (i, &expected) in [digits[9], digits[10]].iter().enumerate() {
        let count = 9 + i;
        let sum: u32 = digits[..count]
            .iter()
            .enumerate()
            .map(|(j, &d)| u32::from(d) * (count as u32 + 1 - j as u32))
            .sum();
        let remainder = sum % 11;
        let check = if remainder < 2 { 0 } else { 11 - remainder } as u8;
        if check != expected {
            return Err(Error::BadChecksum);
        }
    }
    Ok(())
}

/// Brazilian CNPJ: 14 digits, two mod-11 check digits with a different weighting.
fn validate_cnpj(s: &str) -> Result<()> {
    let digits = checksum::digits_only(s)?;
    checksum::expect_len(&digits, 14)?;
    if digits.iter().all(|d| *d == digits[0]) {
        return Err(Error::OutOfRange);
    }
    let weights: [[u32; 13]; 2] = [
        [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2, 0],
        [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2],
    ];
    for (i, &expected) in [digits[12], digits[13]].iter().enumerate() {
        let count = 12 + i;
        let sum: u32 = digits[..count]
            .iter()
            .zip(weights[i])
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        let remainder = sum % 11;
        let check = if remainder < 2 { 0 } else { 11 - remainder } as u8;
        if check != expected {
            return Err(Error::BadChecksum);
        }
    }
    Ok(())
}

/// Spanish DNI and NIF: eight digits followed by a check letter, or a
/// leading letter for legal entities.
fn validate_spanish_dni(s: &str) -> Result<()> {
    const LETTERS: &[u8] = b"TRWAGMYFPDXBNJZSQVHLCKE";
    if !(s.len() == 9) {
        return Err(Error::WrongLength {
            expected: 9,
            found: s.len(),
        });
    }
    let bytes = s.as_bytes();
    let check = bytes[8];
    if !check.is_ascii_uppercase() {
        return Err(Error::InvalidCharacter(char::from(check)));
    }
    // X, Y and Z mark a NIE, a number issued to foreigners. The letter is
    // replaced by 0, 1 or 2 and the result is treated as an eight-digit DNI.
    let leading = bytes[0];
    if matches!(leading, b'X' | b'Y' | b'Z') {
        let prefix = u32::from(leading - b'X');
        let digits = checksum::digits_only(&s[1..8])?;
        // The body is a seven-digit number and the letter contributes a single
        // digit, so the two are added rather than folded together.
        let body = digits.iter().fold(0u32, |acc, &d| acc * 10 + u32::from(d));
        let value = prefix * 10_000_000 + body;
        if LETTERS[(value % 23) as usize] != check {
            return Err(Error::BadChecksum);
        }
        return Ok(());
    }
    if leading.is_ascii_alphabetic() {
        // Any other leading letter is a legal-entity NIF, whose check character
        // is a digit.
        let body = &s[1..8];
        let digits = checksum::digits_only(body)?;
        let sum: u32 = digits.iter().map(|&d| u32::from(d)).sum();
        let expected = (sum % 10) as u8;
        if check != b'0' + expected {
            return Err(Error::BadChecksum);
        }
        return Ok(());
    }
    let body = &s[..8];
    let digits = checksum::digits_only(body)?;
    let value: u32 = digits.iter().fold(0u32, |acc, &d| acc * 10 + u32::from(d));
    if LETTERS[(value % 23) as usize] != check {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// German tax identification number: 11 digits with one check digit, and a
/// requirement that exactly one digit repeats and the first is never zero.
fn validate_german_tax_id(s: &str) -> Result<()> {
    let digits = checksum::digits_only(s)?;
    checksum::expect_len(&digits, 11)?;
    if digits[0] == 0 {
        return Err(Error::OutOfRange);
    }
    // Among the first ten digits exactly one value must appear twice or three
    // times, and no value more often.
    let mut counts = [0u8; 10];
    for &d in &digits[..10] {
        counts[d as usize] += 1;
    }
    let repeated = counts.iter().filter(|c| **c > 1).count();
    if repeated != 1 || counts.iter().any(|c| *c > 3) {
        return Err(Error::OutOfRange);
    }
    // Weighted sum with the alternating product method, weights 1..10.
    let mut sum = 0u32;
    for (i, &d) in digits[..10].iter().enumerate() {
        let mut product = u32::from(d) * (i as u32 + 1);
        if product > 9 {
            product = product / 10 + product % 10;
        }
        sum += product;
    }
    let check = ((10 - (sum % 10)) % 10) as u8;
    if check != digits[10] {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Polish PESEL: 11 digits whose check digit uses weights 1, 3, 7, 9 repeating.
fn validate_pesel(s: &str) -> Result<()> {
    let digits = checksum::digits_only(s)?;
    checksum::expect_len(&digits, 11)?;
    const WEIGHTS: [u32; 10] = [1, 3, 7, 9, 1, 3, 7, 9, 1, 3];
    let sum: u32 = digits[..10]
        .iter()
        .zip(WEIGHTS)
        .map(|(&d, w)| u32::from(d) * w)
        .sum();
    let check = ((10 - (sum % 10)) % 10) as u8;
    if check != digits[10] {
        return Err(Error::BadChecksum);
    }
    // The encoded birth month is offset by a century marker.
    let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
    let month = match month {
        1..=12 | 21..=32 | 41..=52 | 61..=72 | 81..=92 => {
            if month > 80 {
                month - 80
            } else if month > 60 {
                month - 60
            } else if month > 40 {
                month - 40
            } else if month > 20 {
                month - 20
            } else {
                month
            }
        }
        _ => return Err(Error::OutOfRange),
    };
    if !(1..=12).contains(&month) {
        return Err(Error::OutOfRange);
    }
    let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
    if !(1..=31).contains(&day) {
        return Err(Error::OutOfRange);
    }
    Ok(())
}

/// Indian Aadhaar: 12 digits whose last digit is a Verhoeff check.
fn validate_aadhaar(s: &str) -> Result<()> {
    let digits = checksum::digits_only(s)?;
    checksum::expect_len(&digits, 12)?;
    if digits[0] == 0 || digits[0] == 1 {
        // Aadhaar numbers never begin with 0 or 1.
        return Err(Error::OutOfRange);
    }
    if !verhoeff_is_valid(&digits) {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// The Verhoeff multiplication and permutation tables.
const VERHOEFF_D: [[u8; 10]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
    [2, 3, 4, 0, 1, 7, 8, 9, 5, 6],
    [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
    [4, 0, 1, 2, 3, 9, 5, 6, 7, 8],
    [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
    [6, 5, 9, 8, 7, 1, 0, 4, 3, 2],
    [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
    [8, 7, 6, 5, 9, 3, 2, 1, 0, 4],
    [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
];

const VERHOEFF_P: [[u8; 10]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
    [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
    [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
    [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
    [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
    [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
    [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
];

const VERHOEFF_INV: [u8; 10] = [0, 4, 3, 2, 1, 5, 6, 7, 8, 9];

fn verhoeff_is_valid(digits: &[u8]) -> bool {
    let mut c = 0u8;
    for (i, &d) in digits.iter().rev().enumerate() {
        c = VERHOEFF_D[c as usize][VERHOEFF_P[i % 8][d as usize] as usize];
    }
    c == 0
}

/// Appends the Verhoeff check digit to a payload. Exposed for tests and for
/// callers that generate numbers.
#[must_use]
pub fn verhoeff_check_digit(payload: &[u8]) -> u8 {
    let mut c = 0u8;
    for (i, &d) in payload.iter().rev().enumerate() {
        c = VERHOEFF_D[c as usize][VERHOEFF_P[(i + 1) % 8][d as usize] as usize];
    }
    VERHOEFF_INV[c as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npwp_accepts_plausible_numbers() {
        assert!(IdNumber::parse(Country::IndonesiaNpwp, "01.234.567.8-901.000").is_ok());
        assert!(IdNumber::parse(Country::IndonesiaNpwp, "012345678901000").is_ok());
    }

    #[test]
    fn npwp_rejects_bad_shapes() {
        assert!(matches!(
            IdNumber::parse(Country::IndonesiaNpwp, "012345678901"),
            Err(Error::WrongLength {
                expected: 15,
                found: 12
            })
        ));
        assert_eq!(
            IdNumber::parse(Country::IndonesiaNpwp, "001234567890100"),
            Err(Error::OutOfRange)
        );
        // Category 00 is never assigned.
        assert_eq!(
            IdNumber::parse(Country::IndonesiaNpwp, "001234567890123"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn nik_accepts_valid_dates() {
        // A male born 17 August in a 34 region code.
        assert!(IdNumber::parse(Country::IndonesiaNik, "3174011708450001").is_ok());
    }

    #[test]
    fn nik_accepts_female_day_offset() {
        // Day 57 means 17 with the female offset of 40.
        assert!(IdNumber::parse(Country::IndonesiaNik, "3174015708450001").is_ok());
    }

    #[test]
    fn nik_rejects_impossible_dates() {
        // Day 32 and month 13 are impossible.
        assert_eq!(
            IdNumber::parse(Country::IndonesiaNik, "3174013208450001"),
            Err(Error::OutOfRange)
        );
        assert_eq!(
            IdNumber::parse(Country::IndonesiaNik, "3174011713450013"),
            Err(Error::OutOfRange)
        );
        // 31 April does not exist.
        assert_eq!(
            IdNumber::parse(Country::IndonesiaNik, "3174013104450001"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn cpf_accepts_published_examples() {
        for cpf in ["52998224725", "11144477735", "39053344705"] {
            assert!(IdNumber::parse(Country::BrazilCpf, cpf).is_ok(), "{cpf}");
        }
    }

    #[test]
    fn cpf_rejects_bad_check_digits() {
        assert_eq!(
            IdNumber::parse(Country::BrazilCpf, "52998224726"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            IdNumber::parse(Country::BrazilCpf, "11111111111"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn cnpj_accepts_published_examples() {
        for cnpj in ["11222333000181", "34028316000103", "45543915000181"] {
            assert!(IdNumber::parse(Country::BrazilCnpj, cnpj).is_ok(), "{cnpj}");
        }
    }

    #[test]
    fn cnpj_rejects_bad_check_digits() {
        assert_eq!(
            IdNumber::parse(Country::BrazilCnpj, "11222333000182"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn spanish_dni_accepts_published_examples() {
        for dni in ["12345678Z", "54362315K", "X2482300W", "Y1234567X"] {
            assert!(IdNumber::parse(Country::SpainDni, dni).is_ok(), "{dni}");
        }
    }

    #[test]
    fn spanish_dni_rejects_bad_letter() {
        assert_eq!(
            IdNumber::parse(Country::SpainDni, "12345678A"),
            Err(Error::BadChecksum)
        );
        assert!(matches!(
            IdNumber::parse(Country::SpainDni, "1234567"),
            Err(Error::WrongLength { .. })
        ));
    }

    #[test]
    fn german_tax_id_accepts_a_well_formed_number() {
        // Check digit 0 for the payload 1234567891, one value repeated, and a
        // non-zero leading digit.
        assert!(IdNumber::parse(Country::GermanyTaxId, "12345678910").is_ok());
    }

    #[test]
    fn german_tax_id_rejects_bad_checksum_and_repeats() {
        // Correct shape, wrong check digit.
        assert_eq!(
            IdNumber::parse(Country::GermanyTaxId, "12345678911"),
            Err(Error::BadChecksum)
        );
        // No value is repeated, which the specification forbids.
        assert_eq!(
            IdNumber::parse(Country::GermanyTaxId, "12345678901"),
            Err(Error::OutOfRange)
        );
        // A leading zero is not allowed.
        assert_eq!(
            IdNumber::parse(Country::GermanyTaxId, "02476291356"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn pesel_accepts_published_example() {
        // 44051401359 is the standard example.
        assert!(IdNumber::parse(Country::PolandPesel, "44051401359").is_ok());
    }

    #[test]
    fn pesel_rejects_bad_checksum_and_month() {
        assert_eq!(
            IdNumber::parse(Country::PolandPesel, "44051401358"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            IdNumber::parse(Country::PolandPesel, "44991401350"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn verhoeff_check_digit_is_correct() {
        // Reference vectors from the Verhoeff algorithm description.
        assert_eq!(verhoeff_check_digit(&[1, 2, 3, 4, 5]), 1);
        assert_eq!(verhoeff_check_digit(&[1, 4, 2, 8, 5, 7]), 0);
    }

    #[test]
    fn aadhaar_accepts_valid_verhoeff() {
        let payload = [2u8, 3, 4, 1, 2, 3, 4, 5, 6, 7, 8];
        let check = verhoeff_check_digit(&payload);
        let candidate: String = payload.iter().map(|d| char::from(b'0' + d)).collect();
        let full = format!("{candidate}{check}");
        assert!(IdNumber::parse(Country::IndiaAadhaar, &full).is_ok());
    }

    #[test]
    fn aadhaar_rejects_bad_check_and_leading_digit() {
        let payload = [2u8, 3, 4, 1, 2, 3, 4, 5, 6, 7, 8];
        let check = verhoeff_check_digit(&payload);
        let candidate: String = payload.iter().map(|d| char::from(b'0' + d)).collect();
        let wrong = (check + 1) % 10;
        assert_eq!(
            IdNumber::parse(Country::IndiaAadhaar, &format!("{candidate}{wrong}")),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            IdNumber::parse(Country::IndiaAadhaar, "034123456789"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn empty_is_rejected_for_every_country() {
        for country in [
            Country::IndonesiaNpwp,
            Country::IndonesiaNik,
            Country::BrazilCpf,
            Country::BrazilCnpj,
            Country::SpainDni,
            Country::GermanyTaxId,
            Country::PolandPesel,
            Country::IndiaAadhaar,
        ] {
            assert_eq!(
                IdNumber::parse(country, "  "),
                Err(Error::Empty),
                "{country:?}"
            );
        }
    }

    #[test]
    fn accessors_and_descriptions() {
        let id = IdNumber::parse(Country::BrazilCpf, "529.982.247-25").unwrap();
        assert_eq!(id.as_str(), "52998224725");
        assert_eq!(id.country(), Country::BrazilCpf);
        assert!(Country::IndonesiaNik.description().contains("NIK"));
    }
}
