//! Continental European identifiers.

use crate::{Error, Result, checksum};

/// Spain: DNI for citizens, NIE for foreigners and NIF for legal entities.
pub(super) mod spain {
    use super::*;

    const LETTERS: &[u8] = b"TRWAGMYFPDXBNJZSQVHLCKE";

    /// Validates a Spanish DNI, NIE or NIF.
    pub(in crate::id) fn validate_dni(s: &str) -> Result<()> {
        if s.len() != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: s.len(),
            });
        }
        let bytes = s.as_bytes();
        let check = bytes[8];
        // A personal number ends in a letter; a company number may end in a
        // digit or in the letter standing for it.
        if !check.is_ascii_uppercase() && !check.is_ascii_digit() {
            return Err(Error::InvalidCharacter(char::from(check)));
        }
        let leading = bytes[0];
        // The check character of a personal number is a letter over the number
        // taken modulo 23. A NIE writes X, Y or Z in front of eight digits, and
        // the letter stands for 0, 1 or 2; a K, L or M number stands for no
        // digit at all, so it is read as a zero. A plain DNI is the eight
        // digits on their own.
        let letter_check = |prefix: u32, body: &str| -> Result<()> {
            let digits = checksum::digits_only(body)?;
            let mut value = prefix;
            for &d in &digits {
                value = value * 10 + u32::from(d);
            }
            if LETTERS[(value % 23) as usize] != check {
                return Err(Error::BadChecksum);
            }
            Ok(())
        };
        match leading {
            b'X' | b'Y' | b'Z' => return letter_check(u32::from(leading - b'X'), &s[1..8]),
            b'K' | b'L' | b'M' => return letter_check(0, &s[1..8]),
            // A CIF, a legal entity's number. Its check character is a Luhn
            // digit over the middle seven, written either as the digit or as
            // the letter standing for it. Only certain letters may head one.
            b'A'..=b'H' | b'J' | b'N' | b'P'..=b'S' | b'U'..=b'W' => {
                let digits = checksum::digits_only(&s[1..8])?;
                let Some(digit) = checksum::luhn_check_digit(&digits) else {
                    return Err(Error::InvalidCharacter(char::from(check)));
                };
                let as_letter = b"JABCDEFGHI"[digit as usize];
                if check != b'0' + digit && check != as_letter {
                    return Err(Error::BadChecksum);
                }
                return Ok(());
            }
            _ if leading.is_ascii_alphabetic() => {
                return Err(Error::InvalidCharacter(char::from(leading)));
            }
            _ => {}
        }
        letter_check(0, &s[..8])
    }
}

/// Germany: Steuerliche Identifikationsnummer.
pub(super) mod germany {
    use super::*;

    /// Validates a German tax identification number.
    ///
    /// The check digit uses ISO 7064 Mod 11, 10, not a plain digit sum.
    pub(in crate::id) fn validate_tax_id(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        if digits[0] == 0 {
            return Err(Error::OutOfRange);
        }
        // Among the first ten digits exactly one value must appear twice or
        // three times, and no value more often.
        let mut counts = [0u8; 10];
        for &d in &digits[..10] {
            counts[d as usize] += 1;
        }
        let repeated = counts.iter().filter(|c| **c > 1).count();
        if repeated != 1 || counts.iter().any(|c| *c > 3) {
            return Err(Error::OutOfRange);
        }
        if !checksum::iso7064_mod_11_10_is_valid(&digits) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Netherlands: Burgerservicenummer.
pub(super) mod netherlands {
    use super::*;

    /// Validates a Dutch BSN using the eleven-test.
    pub(in crate::id) fn validate_bsn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 9)?;
        // Weights 9 down to 2 over the first eight digits, and the check digit
        // is subtracted rather than added, so a valid number sums to a multiple
        // of eleven.
        let weighted: i64 = digits[..8]
            .iter()
            .enumerate()
            .map(|(i, &d)| i64::from(d) * (9 - i as i64))
            .sum();
        if (weighted - i64::from(digits[8])) % 11 != 0 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Belgium: National Number.
pub(super) mod belgium {
    use super::*;

    /// Validates a Belgian national number.
    ///
    /// The check number is 97 minus the remainder, not the remainder itself.
    /// Numbers issued from 2000 onwards prepend a 2 to the nine-digit base
    /// first, but that reading is only considered once the implied year has
    /// arrived. An unknown birth month or day is encoded as zero and allowed.
    pub(in crate::id) fn validate(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        let base = digits[..9]
            .iter()
            .fold(0i64, |acc, &d| acc * 10 + i64::from(d));
        let check = i64::from(digits[9]) * 10 + i64::from(digits[10]);
        let yy = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let before_2000 = 97 - base % 97;
        let after_2000 = 97 - (base + 2_000_000_000) % 97;
        let matches_1900s = check == before_2000;
        let matches_2000s = check == after_2000 && checksum::year_has_started(yy + 2000);
        if !matches_1900s && !matches_2000s {
            return Err(Error::BadChecksum);
        }
        // A number whose year, month and day are all zero marks an unknown
        // birth date, so only the remaining forms are date-checked.
        if digits[..6].iter().all(|d| *d == 0) {
            return Ok(());
        }
        // The month field is 1 to 12, offset by 20 or 40 for later centuries,
        // and a zero month means the month was unknown when the number was
        // issued.
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let month = month % 20;
        if month > 12 {
            return Err(Error::OutOfRange);
        }
        if month == 0 {
            return Ok(());
        }
        // The year is not known, so February always allows 29. A zero day is
        // allowed for the same reason as a zero month.
        let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        if day != 0 && !checksum::is_plausible_day_month(day, month) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Portugal: Número de Identificação Fiscal.
pub(super) mod portugal {
    use super::*;

    /// Validates a Portuguese NIF.
    pub(in crate::id) fn validate_nif(s: &str) -> Result<()> {
        // The number is written with the country prefix, so `PT 501 964 843`
        // and `501964843` are the same number.
        let s = s.strip_prefix("PT").unwrap_or(s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 9)?;
        if digits[0] == 0 {
            return Err(Error::OutOfRange);
        }
        let sum: u32 = digits[..8]
            .iter()
            .enumerate()
            .map(|(i, &d)| u32::from(d) * (9 - i as u32))
            .sum();
        // 11 - (sum mod 11), with 10 and 11 both collapsing to 0.
        let check = ((11 - sum % 11) % 11 % 10) as u8;
        if check != digits[8] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// France: SIREN and SIRET.
pub(super) mod france {
    use super::*;

    /// Validates a French SIREN, the nine-digit company identifier.
    pub(in crate::id) fn validate_siren(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 9)?;
        if !checksum::luhn_matches(&digits[..8], digits[8]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a French SIRET, the fourteen-digit establishment identifier.
    pub(in crate::id) fn validate_siret(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 14)?;
        // La Poste establishments other than its head office do not use the
        // Luhn check; their digits must instead sum to a multiple of five.
        if s.starts_with("356000000") && s != "35600000000048" {
            let sum: u32 = digits.iter().map(|&d| u32::from(d)).sum();
            if !sum.is_multiple_of(5) {
                return Err(Error::BadChecksum);
            }
        } else if !checksum::luhn_matches(&digits[..13], digits[13]) {
            return Err(Error::BadChecksum);
        }
        validate_siren(&s[..9])
    }
}

/// Italy: codice fiscale.
pub(super) mod italy {
    use super::*;

    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    /// Value contributed by a character in an odd position, counting from one.
    const ODD_VALUES: [u8; 36] = [
        1, 0, 5, 7, 9, 13, 15, 17, 19, 21, 1, 0, 5, 7, 9, 13, 15, 17, 19, 21, 2, 4, 18, 20, 11, 3,
        6, 8, 12, 14, 16, 10, 22, 25, 24, 23,
    ];

    /// Validates an Italian codice fiscale, either the sixteen-character
    /// personal code or the eleven-digit VAT form.
    pub(in crate::id) fn validate_codice_fiscale(s: &str) -> Result<()> {
        if s.len() == 11 {
            // The VAT form is an eleven-digit number with an ordinary Luhn
            // check, plus a province code and a non-zero first seven digits.
            let digits = checksum::digits_only(s)?;
            if digits[..7].iter().all(|d| *d == 0) {
                return Err(Error::OutOfRange);
            }
            let province =
                u32::from(digits[7]) * 100 + u32::from(digits[8]) * 10 + u32::from(digits[9]);
            let province_ok =
                (1..=100).contains(&province) || matches!(province, 120 | 121 | 888 | 999);
            if !province_ok {
                return Err(Error::OutOfRange);
            }
            if !checksum::luhn_matches(&digits[..10], digits[10]) {
                return Err(Error::BadChecksum);
            }
            return Ok(());
        }
        if s.len() != 16 {
            return Err(Error::WrongLength {
                expected: 16,
                found: s.len(),
            });
        }
        let written = s.as_bytes();
        // Layout: six letters of name, two digits of year, one letter of month,
        // two digits of day, one letter and three digits of birthplace, one
        // check letter. Under the omocodia scheme, when a plain number would
        // collide with another, the digits of the year, day and birthplace are
        // replaced by the letters L through V, standing for 0 through 9. The
        // letters are decoded for the shape and date checks only; the check
        // character is taken over the number as written.
        let mut decoded: [u8; 16] = written
            .try_into()
            .map_err(|_| Error::InvalidCharacter('?'))?;
        for slot in [6, 7, 9, 10, 12, 13, 14] {
            if let Some(digit) = omocodia_digit(decoded[slot]) {
                decoded[slot] = b'0' + digit;
            }
        }
        // The birth-day digits run to 31 for men and to 71 for women, and the
        // birthplace letter stands in for a municipality code, so neither is
        // checked beyond its character class.
        let shape_ok = decoded[..6].iter().all(u8::is_ascii_uppercase)
            && decoded[6..8].iter().all(u8::is_ascii_digit)
            && b"ABCDEHLMPRST".contains(&decoded[8])
            && decoded[9..11].iter().all(u8::is_ascii_digit)
            && decoded[11].is_ascii_uppercase()
            && decoded[12..15].iter().all(u8::is_ascii_digit)
            && decoded[15].is_ascii_uppercase();
        if !shape_ok {
            return Err(Error::OutOfRange);
        }
        // The month letter selects a month, and the day digits select a day
        // within it, allowing for the forty added to a woman's day.
        let month = b"ABCDEHLMPRST"
            .iter()
            .position(|m| *m == decoded[8])
            .map_or(0, |i| i as u32 + 1);
        let day = (u32::from(decoded[9] - b'0') * 10 + u32::from(decoded[10] - b'0')) % 40;
        if !checksum::is_plausible_day_month(day, month) {
            return Err(Error::OutOfRange);
        }
        let expected = check_char(&s[..15])?;
        if expected != written[15] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// The digit an omocodia letter stands for, or `None` when the byte is not
    /// one of the substitution letters.
    ///
    /// `stdnum.it.codicefiscale`.
    fn omocodia_digit(b: u8) -> Option<u8> {
        match b {
            b'L' => Some(0),
            b'M' => Some(1),
            b'N' => Some(2),
            b'P' => Some(3),
            b'Q' => Some(4),
            b'R' => Some(5),
            b'S' => Some(6),
            b'T' => Some(7),
            b'U' => Some(8),
            b'V' => Some(9),
            _ => None,
        }
    }

    /// The check letter for the first fifteen characters.
    fn check_char(payload: &str) -> Result<u8> {
        let mut sum = 0u32;
        for (i, c) in payload.bytes().enumerate() {
            let Some(index) = ALPHABET.iter().position(|a| *a == c) else {
                return Err(Error::InvalidCharacter(char::from(c)));
            };
            // Positions are counted from one, so even indices are odd
            // positions. At an odd position a digit is worth its face value and
            // a letter is worth its position in the alphabet; at an even
            // position both are worth their index in the shared table.
            let value = if i % 2 == 0 {
                u32::from(ODD_VALUES[index])
            } else if c.is_ascii_digit() {
                u32::from(c - b'0')
            } else {
                u32::from(c - b'A')
            };
            sum += value;
        }
        Ok(b'A' + (sum % 26) as u8)
    }
}

/// Romania: Codul Numeric Personal.
pub(super) mod romania {
    use super::*;

    /// The two-digit county codes, from 01 to 48 for the counties and 51 and 52
    /// for Bucharest. The remaining accepted values cover later additions and
    /// foreign residents.
    const COUNTIES: &[u8] = &[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48,
        51, 52, 70, 80, 81, 82, 83,
    ];

    /// Validates a Romanian CNP.
    pub(in crate::id) fn validate_cnp(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 13)?;
        // The first digit marks the century and sex and runs from 1 to 9.
        if digits[0] == 0 {
            return Err(Error::OutOfRange);
        }
        // The date is years two and three, then month, then day.
        let century = match digits[0] {
            1 | 2 => 1900,
            3 | 4 => 1800,
            5 | 6 => 2000,
            _ => 1900,
        };
        let year = century + u32::from(digits[1]) * 10 + u32::from(digits[2]);
        let month = u32::from(digits[3]) * 10 + u32::from(digits[4]);
        let day = u32::from(digits[5]) * 10 + u32::from(digits[6]);
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        // The next two digits are the county of birth.
        let county = u32::from(digits[7]) * 10 + u32::from(digits[8]);
        if !COUNTIES.contains(&(county as u8)) {
            return Err(Error::OutOfRange);
        }
        const WEIGHTS: [u32; 12] = [2, 7, 9, 1, 4, 6, 3, 5, 8, 2, 7, 9];
        let sum: u32 = digits[..12]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        // A remainder of ten becomes one rather than zero.
        let check = match sum % 11 {
            10 => 1,
            r => r as u8,
        };
        if check != digits[12] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Poland: PESEL.
pub(super) mod poland {
    use super::*;

    /// Validates a Polish PESEL.
    pub(in crate::id) fn validate_pesel(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        const WEIGHTS: [u32; 10] = [1, 3, 7, 9, 1, 3, 7, 9, 1, 3];
        let sum: u32 = digits[..10]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        let check = ((10 - sum % 10) % 10) as u8;
        if check != digits[10] {
            return Err(Error::BadChecksum);
        }
        // The month field carries a century offset in steps of twenty, from
        // 1800 up to 2200, and the remainder must be a real month.
        let encoded = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let year = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let year = year
            + match encoded / 20 {
                0 => 1900,
                1 => 2000,
                2 => 2100,
                3 => 2200,
                _ => 1800,
            };
        let month = encoded % 20;
        let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spanish_examples() {
        for value in ["54362315K", "12345678Z", "X2482300W", "Y1234567X"] {
            assert!(spain::validate_dni(value).is_ok(), "{value}");
        }
        assert_eq!(spain::validate_dni("54362315Z"), Err(Error::BadChecksum));
        // A NIE prefix is not a valid check letter on its own.
        assert!(spain::validate_dni("54362315").is_err());
    }

    /// A CIF is a legal entity's number, and its check character is the Luhn
    /// digit over the middle seven. It may be written as that digit or as the
    /// letter that stands for it, and only a fixed set of letters may head it.
    /// These come from `stdnum.es.nif` and `stdnum.es.cif`.
    #[test]
    fn spanish_legal_entities_and_personal_numbers() {
        for value in [
            "B64717838",
            "B6471783H",
            "J99216582",
            "A13585625",
            "M1234567L",
            "K1234567L",
        ] {
            assert!(spain::validate_dni(value).is_ok(), "{value}");
        }
        // A letter that may not head a CIF is not a Spanish number at all.
        assert!(spain::validate_dni("O1234567L").is_err());
        assert_eq!(spain::validate_dni("B64717839"), Err(Error::BadChecksum));
        // The letter standing for the check digit must stand for the right one.
        assert_eq!(spain::validate_dni("B6471783J"), Err(Error::BadChecksum));
    }

    #[test]
    fn german_example() {
        // The example from the ISO 7064 Mod 11, 10 specification.
        assert!(germany::validate_tax_id("36574261809").is_ok());
        assert_eq!(
            germany::validate_tax_id("36574261890"),
            Err(Error::BadChecksum)
        );
        assert!(germany::validate_tax_id("02476291356").is_err());
    }

    #[test]
    fn german_rejects_a_wrong_repeat_count() {
        // Two digits repeat, so the number is malformed even if the check digit
        // were right.
        assert_eq!(
            germany::validate_tax_id("11234567893"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn dutch_bsn_examples() {
        assert!(netherlands::validate_bsn("111222333").is_ok());
        assert_eq!(
            netherlands::validate_bsn("111252333"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn belgian_examples() {
        // A number issued before 2000, and one after.
        assert!(belgium::validate("85073003328").is_ok());
        assert!(belgium::validate("17073003384").is_ok());
        assert_eq!(belgium::validate("12345678901"), Err(Error::BadChecksum));
    }

    #[test]
    fn portuguese_examples() {
        assert!(portugal::validate_nif("501964843").is_ok());
        assert_eq!(portugal::validate_nif("501964842"), Err(Error::BadChecksum));
        assert!(portugal::validate_nif("012345678").is_err());
    }

    #[test]
    fn french_siren_and_siret() {
        for value in ["404833048", "552008443"] {
            assert!(france::validate_siren(value).is_ok(), "{value}");
        }
        assert_eq!(france::validate_siren("404833047"), Err(Error::BadChecksum));
        assert!(france::validate_siret("73282932000074").is_ok());
        assert_eq!(
            france::validate_siret("73282932000079"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn french_siret_la_poste_uses_the_digit_sum() {
        // 35600000000048 is the head office and does use Luhn.
        assert!(france::validate_siret("35600000000048").is_ok());
        // Another La Poste establishment whose digits sum to a multiple of five.
        let candidate = "35600000000010";
        let sum: u32 = candidate.bytes().map(|b| u32::from(b - b'0')).sum();
        if sum.is_multiple_of(5) {
            assert!(france::validate_siret(candidate).is_ok(), "{candidate}");
        }
    }

    #[test]
    fn italian_examples() {
        for value in ["RCCMNL83S18D969H", "00743110157"] {
            assert!(italy::validate_codice_fiscale(value).is_ok(), "{value}");
        }
        assert_eq!(
            italy::validate_codice_fiscale("00743110158"),
            Err(Error::BadChecksum)
        );
        assert!(italy::validate_codice_fiscale("RCCMNL83S18D969").is_err());
    }

    /// Under the omocodia scheme the year, day and birthplace digits may be
    /// replaced by the letters L through V. An earlier version rejected every
    /// such number with an out-of-range error.
    #[test]
    fn italian_omocodia_letters_are_accepted() {
        for value in [
            "RSSMRA45C12F20RX",
            "RSSMRA45C12F2L5N",
            "RSSMRA45C12F2LRI",
            "RSSMRAQRCMNFNLRG",
        ] {
            assert!(italy::validate_codice_fiscale(value).is_ok(), "{value}");
        }
        // The check character is taken over the letters as written, so a
        // number with a substituted digit still fails when it is wrong.
        assert_eq!(
            italy::validate_codice_fiscale("RCCMNL83S18D969X"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn romanian_example() {
        assert!(romania::validate_cnp("1630615123457").is_ok());
        assert_eq!(
            romania::validate_cnp("1630615123458"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn polish_examples() {
        for value in ["44051401359", "02122401358", "02211307589"] {
            assert!(poland::validate_pesel(value).is_ok(), "{value}");
        }
        assert_eq!(
            poland::validate_pesel("44051401358"),
            Err(Error::BadChecksum)
        );
    }
}
