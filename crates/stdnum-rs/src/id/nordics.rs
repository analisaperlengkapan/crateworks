//! Nordic identifiers: Sweden, Norway, Denmark and Finland.

use crate::{Error, Result, checksum};

/// Sweden: personnummer.
pub(super) mod sweden {
    use super::*;

    /// Validates a Swedish personnummer, with or without the century.
    ///
    /// The number is ten or twelve digits, optionally with a separator before
    /// the last four, and the Luhn check covers the last ten digits.
    pub(in crate::id) fn validate_personnummer(s: &str) -> Result<()> {
        let digits: Vec<u8> = s
            .chars()
            .filter(|c| c.is_ascii_digit())
            .map(|c| c as u8 - b'0')
            .collect();
        if !(10..=12).contains(&digits.len()) {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        // The check covers the ten digits of the date and the serial, so a
        // twelve-digit form drops the century.
        let tail = &digits[digits.len() - 10..];
        if !checksum::luhn_matches(&tail[..9], tail[9]) {
            return Err(Error::BadChecksum);
        }
        // The date part must be a real date.
        let month = tail[2] * 10 + tail[3];
        let day = tail[4] * 10 + tail[5];
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Norway: fødselsnummer and organisasjonsnummer.
pub(super) mod norway {
    use super::*;

    /// Validates a Norwegian birth number, which has two check digits.
    pub(in crate::id) fn validate_birth_number(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        const W1: [u32; 9] = [3, 7, 6, 1, 8, 9, 4, 5, 2];
        const W2: [u32; 10] = [5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
        if !mod11_matches(&digits[..9], &W1, digits[9]) {
            return Err(Error::BadChecksum);
        }
        if !mod11_matches(&digits[..10], &W2, digits[10]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Norwegian organisation number, which has one check digit.
    pub(in crate::id) fn validate_org_number(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 9)?;
        // The check digit is included in the weighting, so a valid number
        // weights to a multiple of eleven.
        const WEIGHTS: [u32; 9] = [3, 2, 7, 6, 5, 4, 3, 2, 1];
        let sum: u32 = digits
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        if !sum.is_multiple_of(11) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Whether the mod-11 check digit for `payload` equals `expected`.
    ///
    /// A raw remainder of one would give a check digit of ten, which has no
    /// digit representation, so such a number cannot be valid.
    fn mod11_matches(payload: &[u8], weights: &[u32], expected: u8) -> bool {
        let sum: u32 = payload
            .iter()
            .zip(weights)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        let check = 11 - sum % 11;
        let check = if check == 11 { 0 } else { check };
        check != 10 && check as u8 == expected
    }
}

/// Denmark: CPR.
pub(super) mod denmark {
    use super::*;

    /// Validates a Danish CPR number.
    ///
    /// The number carries a birth date but no reliable check digit: the mod-11
    /// scheme was abandoned, so only the date and the century marker are
    /// examined.
    pub(in crate::id) fn validate_cpr(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 10)?;
        let day = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let yy = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        // The seventh digit selects the century, together with the year.
        let marker = digits[6];
        let year = if matches!(marker, 5..=8) && yy >= 58 {
            yy + 1800
        } else if matches!(marker, 0..=3) || (matches!(marker, 4 | 9) && yy >= 37) {
            yy + 1900
        } else {
            yy + 2000
        };
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        // A number cannot belong to someone who has not been born yet.
        if !checksum::is_not_in_the_future(year, month, day) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Finland: HETU.
pub(super) mod finland {
    use super::*;

    /// The check characters, indexed by the nine-digit date and serial modulo 31.
    const TABLE: &[u8] = b"0123456789ABCDEFHJKLMNPRSTUVWXY";

    /// Validates a Finnish HETU.
    pub(in crate::id) fn validate_hetu(s: &str) -> Result<()> {
        if s.len() != 11 {
            return Err(Error::WrongLength {
                expected: 11,
                found: s.len(),
            });
        }
        let bytes = s.as_bytes();
        // ddmmyy, then a century sign, then a three-digit serial and a check
        // character.
        if !bytes[..6].iter().all(u8::is_ascii_digit)
            || !bytes[7..10].iter().all(u8::is_ascii_digit)
        {
            let bad = bytes
                .iter()
                .find(|b| !b.is_ascii_digit())
                .copied()
                .unwrap_or(bytes[0]);
            return Err(Error::InvalidCharacter(char::from(bad)));
        }
        if !b"+-ABCDEFYXWVU".contains(&bytes[6]) {
            return Err(Error::InvalidCharacter(char::from(bytes[6])));
        }
        let day = u32::from(bytes[0] - b'0') * 10 + u32::from(bytes[1] - b'0');
        let month = u32::from(bytes[2] - b'0') * 10 + u32::from(bytes[3] - b'0');
        if !checksum::is_plausible_day_month(day, month) {
            return Err(Error::OutOfRange);
        }
        let serial = u32::from(bytes[7] - b'0') * 100
            + u32::from(bytes[8] - b'0') * 10
            + u32::from(bytes[9] - b'0');
        // Historical identifiers start at 002, and 900 to 999 is reserved for
        // temporary identifiers.
        if serial < 2 || (900..=999).contains(&serial) {
            return Err(Error::OutOfRange);
        }
        // The check is taken over the date and serial with the century sign
        // removed, so 131052-308 becomes 131052308.
        let mut value = 0u32;
        for &b in bytes[..6].iter().chain(bytes[7..10].iter()) {
            value = value * 10 + u32::from(b - b'0');
        }
        let expected = TABLE[(value % 31) as usize];
        if expected != bytes[10] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swedish_examples() {
        for value in ["811228-9841", "8112289841", "880320-0016", "890102-3286"] {
            assert!(sweden::validate_personnummer(value).is_ok(), "{value}");
        }
        assert_eq!(
            sweden::validate_personnummer("880320-0018"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn norwegian_examples() {
        assert!(norway::validate_birth_number("15108695088").is_ok());
        assert_eq!(
            norway::validate_birth_number("15108695077"),
            Err(Error::BadChecksum)
        );
        assert!(norway::validate_org_number("988077917").is_ok());
        assert_eq!(
            norway::validate_org_number("988077918"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn danish_examples() {
        assert!(denmark::validate_cpr("0101501234").is_ok());
        // Month 13 is impossible.
        assert_eq!(denmark::validate_cpr("0113501234"), Err(Error::OutOfRange));
        // 31 February is impossible.
        assert_eq!(denmark::validate_cpr("3102501234"), Err(Error::OutOfRange));
    }

    #[test]
    fn finnish_examples() {
        for value in ["131052-308T", "131052A308T"] {
            assert!(finland::validate_hetu(value).is_ok(), "{value}");
        }
        assert_eq!(
            finland::validate_hetu("131052-308U"),
            Err(Error::BadChecksum)
        );
        // An impossible date is rejected before the check character is examined.
        assert!(finland::validate_hetu("310252-308Y").is_err());
    }
}
