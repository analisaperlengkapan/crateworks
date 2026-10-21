//! Asian identifiers: India, Turkey, China, Korea and Mexico.

use crate::{Error, Result, checksum};

/// India: Aadhaar.
pub(super) mod india {
    use super::*;

    /// Validates an Indian Aadhaar number.
    pub(in crate::id) fn validate_aadhaar(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 12)?;
        // Aadhaar never starts with 0 or 1.
        if digits[0] < 2 {
            return Err(Error::OutOfRange);
        }
        if !verhoeff_is_valid(&digits) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Turkey: T.C. Kimlik No.
pub(super) mod turkey {
    use super::*;

    /// Validates a Turkish identification number.
    pub(in crate::id) fn validate_tckn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        if digits[0] == 0 {
            return Err(Error::OutOfRange);
        }
        // The tenth digit is derived from the odd and even positions of the
        // first nine, and the eleventh from the sum of the first ten.
        let odd: u32 = digits[..9].iter().step_by(2).map(|&d| u32::from(d)).sum();
        let even: u32 = digits[1..8].iter().step_by(2).map(|&d| u32::from(d)).sum();
        let tenth = (odd * 7).wrapping_sub(even) % 10;
        if tenth != u32::from(digits[9]) {
            return Err(Error::BadChecksum);
        }
        let first_ten: u32 = digits[..10].iter().map(|&d| u32::from(d)).sum();
        if first_ten % 10 != u32::from(digits[10]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// China: resident identity card.
pub(super) mod china {
    use super::*;

    /// Validates a Chinese resident identity card number.
    pub(in crate::id) fn validate_ric(s: &str) -> Result<()> {
        checksum::expect_len_str(s, 18)?;
        let bytes = s.as_bytes();
        if !bytes[..17].iter().all(u8::is_ascii_digit) {
            let bad = bytes
                .iter()
                .find(|b| !b.is_ascii_digit())
                .copied()
                .unwrap_or(bytes[0]);
            return Err(Error::InvalidCharacter(char::from(bad)));
        }
        let check = bytes[17];
        if !check.is_ascii_digit() && check != b'X' {
            return Err(Error::InvalidCharacter(char::from(check)));
        }
        // The check character is ISO 7064 MOD 11-2 over the first seventeen
        // digits, using the weights 7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5,
        // 8, 4, 2 and the remainder table 1, 0, X, 9, 8, 7, 6, 5, 4, 3, 2.
        const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
        const TABLE: &[u8] = b"10X98765432";
        let sum: u32 = bytes[..17]
            .iter()
            .zip(WEIGHTS)
            .map(|(&b, w)| u32::from(b - b'0') * w)
            .sum();
        let expected = TABLE[(sum % 11) as usize];
        if expected != check {
            return Err(Error::BadChecksum);
        }
        // The digits in positions seven to fourteen are the birth date. The
        // issuing place is also encoded in the first six digits, but resolving
        // it needs a location table this crate does not carry, so it is left
        // unchecked.
        let year = u32::from(bytes[6] - b'0') * 1000
            + u32::from(bytes[7] - b'0') * 100
            + u32::from(bytes[8] - b'0') * 10
            + u32::from(bytes[9] - b'0');
        let month = u32::from(bytes[10] - b'0') * 10 + u32::from(bytes[11] - b'0');
        let day = u32::from(bytes[12] - b'0') * 10 + u32::from(bytes[13] - b'0');
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// South Korea: resident registration number.
pub(super) mod korea {
    use super::*;

    /// Validates a South Korean resident registration number.
    ///
    /// The seventh digit selects the century, and the eighth and ninth are a
    /// place-of-birth code that must not exceed 96.
    pub(in crate::id) fn validate_rrn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 13)?;
        let yy = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        let year = match digits[6] {
            1 | 2 | 5 | 6 => yy + 1900,
            3 | 4 | 7 | 8 => yy + 2000,
            _ => yy + 1800,
        };
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        let place = u32::from(digits[7]) * 10 + u32::from(digits[8]);
        if place > 96 {
            return Err(Error::OutOfRange);
        }
        const WEIGHTS: [u32; 12] = [2, 3, 4, 5, 6, 7, 8, 9, 2, 3, 4, 5];
        let sum: u32 = digits[..12]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        let check = (11 - sum % 11) % 10;
        if check as u8 != digits[12] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Mexico: RFC.
pub(super) mod mexico {
    use super::*;

    /// The alphabet used for the RFC check digit, in value order.
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMN&OPQRSTUVWXYZ \xD1";

    /// Validates a Mexican RFC.
    ///
    /// The company form has twelve characters, of which the last three are the
    /// homoclave, and the personal form has thirteen, where the last character
    /// is a check character over the preceding twelve. Only the longer form
    /// carries a verifiable check, so the shorter one is validated structurally.
    pub(in crate::id) fn validate_rfc(s: &str) -> Result<()> {
        if s.len() != 12 && s.len() != 13 {
            return Err(Error::WrongLength {
                expected: 13,
                found: s.len(),
            });
        }
        let bytes = s.as_bytes();
        let letters = if s.len() == 13 { 4 } else { 3 };
        // The four or three leading letters, six digits of date, then the
        // homoclave.
        if !bytes[..letters].iter().all(|b| b.is_ascii_uppercase()) {
            let bad = bytes
                .iter()
                .find(|b| !b.is_ascii_uppercase())
                .copied()
                .unwrap_or(bytes[0]);
            return Err(Error::InvalidCharacter(char::from(bad)));
        }
        if !bytes[letters..letters + 6].iter().all(u8::is_ascii_digit) {
            let bad = bytes[letters..letters + 6]
                .iter()
                .find(|b| !b.is_ascii_digit())
                .copied()
                .unwrap_or(bytes[letters]);
            return Err(Error::InvalidCharacter(char::from(bad)));
        }
        if !bytes[letters + 6..].iter().all(u8::is_ascii_alphanumeric) {
            let bad = bytes[letters + 6..]
                .iter()
                .find(|b| !b.is_ascii_alphanumeric())
                .copied()
                .unwrap_or(bytes[letters + 6]);
            return Err(Error::InvalidCharacter(char::from(bad)));
        }
        // The six digits are a date. The century is not stored, so it is taken
        // as 2000, which matches the reference implementation.
        let start = letters;
        let year = 2000 + u32::from(bytes[start] - b'0') * 10 + u32::from(bytes[start + 1] - b'0');
        let month = u32::from(bytes[start + 2] - b'0') * 10 + u32::from(bytes[start + 3] - b'0');
        let day = u32::from(bytes[start + 4] - b'0') * 10 + u32::from(bytes[start + 5] - b'0');
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        if s.len() == 12 {
            // The company form ends with the same check character as the
            // personal form, but over eleven preceding characters, so it is
            // left-padded with one blank before the weights are applied.
            let blank = ALPHABET.iter().position(|a| *a == b' ').unwrap_or(0) as u32;
            let mut sum = blank * 13;
            for (i, c) in bytes[..11].iter().enumerate() {
                let Some(index) = ALPHABET.iter().position(|a| a == c) else {
                    return Err(Error::InvalidCharacter(char::from(*c)));
                };
                sum += index as u32 * (12 - i as u32);
            }
            let expected = ALPHABET[((11 - sum % 11) % 11) as usize];
            if expected != bytes[11] {
                return Err(Error::BadChecksum);
            }
            return Ok(());
        }
        // The check character is computed over the first twelve characters.
        let mut sum = 0u32;
        for (i, c) in bytes[..12].iter().enumerate() {
            let Some(index) = ALPHABET.iter().position(|a| a == c) else {
                return Err(Error::InvalidCharacter(char::from(*c)));
            };
            sum += index as u32 * (13 - i as u32);
        }
        let expected = ALPHABET[((11 - sum % 11) % 11) as usize];
        if expected != bytes[12] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// The Verhoeff tables, shared by Aadhaar and the public check-digit helper.
pub(super) const VERHOEFF_D: [[u8; 10]; 10] = [
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

/// The Verhoeff permutation table.
pub(super) const VERHOEFF_P: [[u8; 10]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
    [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
    [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
    [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
    [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
    [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
    [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
];

/// The Verhoeff multiplicative inverse table.
pub(super) const VERHOEFF_INV: [u8; 10] = [0, 4, 3, 2, 1, 5, 6, 7, 8, 9];

/// Whether the Verhoeff check over `digits` succeeds.
pub(super) fn verhoeff_is_valid(digits: &[u8]) -> bool {
    let mut c = 0u8;
    for (i, &d) in digits.iter().rev().enumerate() {
        c = VERHOEFF_D[c as usize][VERHOEFF_P[i % 8][d as usize] as usize];
    }
    c == 0
}

/// Appends the Verhoeff check digit to a payload.
///
/// Exposed so callers can generate numbers, and used by the tests.
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
    fn aadhaar_examples() {
        // Verhoeff check digits computed with the crate's own routine.
        let payload = [2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2];
        let check = crate::id::verhoeff_check_digit(&payload);
        let mut digits = payload.to_vec();
        digits.push(check);
        let text: String = digits.iter().map(|d| char::from(b'0' + d)).collect();
        assert!(india::validate_aadhaar(&text).is_ok(), "{text}");
    }

    #[test]
    fn aadhaar_rejects_a_leading_zero_or_one() {
        let payload = [1, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2];
        let check = crate::id::verhoeff_check_digit(&payload);
        let mut digits = payload.to_vec();
        digits.push(check);
        let text: String = digits.iter().map(|d| char::from(b'0' + d)).collect();
        assert_eq!(india::validate_aadhaar(&text), Err(Error::OutOfRange));
    }

    #[test]
    fn turkish_example() {
        assert!(turkey::validate_tckn("17291716060").is_ok());
        assert_eq!(
            turkey::validate_tckn("17291716050"),
            Err(Error::BadChecksum)
        );
        assert!(turkey::validate_tckn("07291716092").is_err());
    }

    #[test]
    fn chinese_example() {
        assert!(china::validate_ric("360426199101010071").is_ok());
        // A wrong final digit is rejected.
        assert_eq!(
            china::validate_ric("360426199101010072"),
            Err(Error::BadChecksum)
        );
        // An X check character is accepted where the table calls for it.
        assert!(china::validate_ric("11010519491231002X").is_ok());
    }

    #[test]
    fn korean_examples() {
        assert!(korea::validate_rrn("9710139019902").is_ok());
        assert_eq!(
            korea::validate_rrn("9710139019903"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn mexican_examples() {
        // The thirteen-character personal form carries a check character.
        for value in ["GODE561231GR8", "VACE460910SX5"] {
            assert!(mexico::validate_rfc(value).is_ok(), "{value}");
        }
        // The twelve-character company form also carries a check character,
        // computed over one fewer character.
        assert!(mexico::validate_rfc("MAB9307148T4").is_ok());
        // A wrong check character on either form is rejected.
        assert_eq!(
            mexico::validate_rfc("GODE561231GR9"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            mexico::validate_rfc("MAB9307148T5"),
            Err(Error::BadChecksum)
        );
        // A month of 13 is impossible.
        assert_eq!(
            mexico::validate_rfc("GODE561331GR8"),
            Err(Error::OutOfRange)
        );
    }
}
