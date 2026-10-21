//! Brazilian CPF and CNPJ.

use crate::{Error, Result, checksum};

/// Brazilian CPF: 11 digits, validated with two mod-11 check digits.
pub(super) mod cpf {
    use super::*;

    /// Validates a CPF.
    pub(in crate::id) fn validate(s: &str) -> Result<()> {
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
}

/// Brazilian CNPJ: 14 digits, two mod-11 check digits with a different weighting.
pub(super) mod cnpj {
    use super::*;

    /// Validates a CNPJ.
    pub(in crate::id) fn validate(s: &str) -> Result<()> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpf_accepts_published_examples() {
        assert!(cpf::validate("39053344705").is_ok());
    }

    #[test]
    fn cpf_rejects_bad_check_digits() {
        assert_eq!(cpf::validate("39053344706"), Err(Error::BadChecksum));
        assert_eq!(cpf::validate("39053344715"), Err(Error::BadChecksum));
    }

    #[test]
    fn cpf_rejects_repeated_digits() {
        assert_eq!(cpf::validate("11111111111"), Err(Error::OutOfRange));
        assert_eq!(cpf::validate("00000000000"), Err(Error::OutOfRange));
    }

    #[test]
    fn cnpj_accepts_published_examples() {
        assert!(cnpj::validate("16727230000197").is_ok());
    }

    #[test]
    fn cnpj_rejects_bad_check_digits() {
        assert_eq!(cnpj::validate("16727230000198"), Err(Error::BadChecksum));
    }
}
