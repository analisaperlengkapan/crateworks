//! Canadian and Australasian identifiers.

use crate::{Error, Result, checksum};

/// Canada: Social Insurance Number.
pub(super) mod canada {
    use super::*;

    /// Validates a Canadian SIN.
    pub(in crate::id) fn validate_sin(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 9)?;
        // A SIN never begins with 0 or 8.
        if matches!(digits[0], 0 | 8) {
            return Err(Error::OutOfRange);
        }
        if !checksum::luhn_matches(&digits[..8], digits[8]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Australia: ABN and TFN.
pub(super) mod australia {
    use super::*;

    /// Validates an Australian Business Number.
    pub(in crate::id) fn validate_abn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        // Subtract one from the leading digit before weighting, which is what
        // makes the total a multiple of 89. Signed arithmetic avoids an
        // underflow when the leading digit is small.
        const WEIGHTS: [i64; 11] = [10, 1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
        let sum: i64 = digits
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| i64::from(d) * w)
            .sum();
        if (sum - 10) % 89 != 0 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates an Australian Tax File Number.
    pub(in crate::id) fn validate_tfn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        if digits.len() != 8 && digits.len() != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: digits.len(),
            });
        }
        // A nine-digit number is weighted with a leading ten.
        const WEIGHTS: [u32; 9] = [1, 4, 3, 7, 5, 8, 6, 9, 10];
        let offset = WEIGHTS.len() - digits.len();
        let sum: u32 = digits
            .iter()
            .zip(&WEIGHTS[offset..])
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        if sum % 11 != 0 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// New Zealand: IRD number.
pub(super) mod new_zealand {
    use super::*;

    /// Validates a New Zealand IRD number.
    pub(in crate::id) fn validate_ird(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        if digits.len() != 8 && digits.len() != 9 {
            return Err(Error::WrongLength {
                expected: 8,
                found: digits.len(),
            });
        }
        let value = digits.iter().fold(0u64, |acc, &d| acc * 10 + u64::from(d));
        // The value must exceed ten million, which rules out a leading-zero
        // eight-digit form, and must stay below 150 million.
        if !(10_000_000..150_000_000).contains(&value) {
            return Err(Error::OutOfRange);
        }
        // The check digit is computed over the eight digits before it, padded
        // on the left for the shorter form.
        let payload = &digits[..digits.len() - 1];
        let padded: Vec<u8> = std::iter::repeat_n(0u8, 8 - payload.len())
            .chain(payload.iter().copied())
            .collect();
        const PRIMARY: [u32; 8] = [3, 2, 7, 6, 5, 4, 3, 2];
        const SECONDARY: [u32; 8] = [7, 4, 3, 2, 5, 2, 7, 6];
        // The check digit is 11 minus the remainder, or zero when that is
        // eleven. When it would be ten the secondary weights are used instead.
        let check = {
            let sum: u32 = padded
                .iter()
                .zip(PRIMARY)
                .map(|(&d, w)| u32::from(d) * w)
                .sum();
            let r = 11 - sum % 11;
            if r == 10 {
                let sum: u32 = padded
                    .iter()
                    .zip(SECONDARY)
                    .map(|(&d, w)| u32::from(d) * w)
                    .sum();
                let r = 11 - sum % 11;
                if r == 11 { 0 } else { r }
            } else if r == 11 {
                0
            } else {
                r
            }
        };
        if check as u8 != digits[digits.len() - 1] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canadian_examples() {
        assert!(canada::validate_sin("123456782").is_ok());
        assert_eq!(canada::validate_sin("823456785"), Err(Error::OutOfRange));
        assert_eq!(canada::validate_sin("046454286"), Err(Error::OutOfRange));
    }

    #[test]
    fn australian_abn_examples() {
        assert!(australia::validate_abn("51824753556").is_ok());
        assert_eq!(
            australia::validate_abn("99999999999"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn australian_tfn_examples() {
        assert!(australia::validate_tfn("123456782").is_ok());
        assert_eq!(
            australia::validate_tfn("999999999"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn new_zealand_ird_examples() {
        assert!(new_zealand::validate_ird("49098576").is_ok());
        // Nine digits, within range, but the check digit is wrong.
        assert_eq!(
            new_zealand::validate_ird("136410133"),
            Err(Error::BadChecksum)
        );
        // Nine digits, above the permitted maximum.
        assert_eq!(
            new_zealand::validate_ird("990985760"),
            Err(Error::OutOfRange)
        );
        // Eight digits whose value does not exceed ten million.
        assert_eq!(
            new_zealand::validate_ird("02179997"),
            Err(Error::OutOfRange)
        );
    }
}
