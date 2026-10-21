//! Indonesia and the Philippines.

use crate::{Error, Result, checksum};

/// Indonesia: NPWP and NIK.
pub(super) mod indonesia {
    use super::*;

    /// Indonesian NPWP: 15 digits, of which the first two identify the taxpayer
    /// category and the remainder a registration sequence.
    ///
    /// The NPWP carries no check digit, so only length, character set and the
    /// leading category digits are validated.
    pub(in crate::id) fn validate_npwp(s: &str) -> Result<()> {
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
    pub(in crate::id) fn validate_nik(s: &str) -> Result<()> {
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
}

/// Philippines: PhilSys.
pub(super) mod philippines {
    use super::*;

    /// Philippines PhilSys number: 12 digits with a Luhn check digit.
    pub(in crate::id) fn validate_philsys(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 12)?;
        if !checksum::luhn_matches(&digits[..11], digits[11]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::southeast_asia::{indonesia, philippines};

    #[test]
    fn npwp_accepts_plausible_numbers() {
        assert!(indonesia::validate_npwp("012345678901000").is_ok());
        assert!(indonesia::validate_npwp("999999999999000").is_ok());
    }

    #[test]
    fn npwp_rejects_bad_shapes() {
        // A zero category is never issued.
        assert_eq!(
            indonesia::validate_npwp("000000000000000"),
            Err(Error::OutOfRange)
        );
        assert_eq!(
            indonesia::validate_npwp("12345678901234"),
            Err(Error::WrongLength {
                expected: 15,
                found: 14
            })
        );
        assert!(indonesia::validate_npwp("12345678901234A").is_err());
    }

    #[test]
    fn nik_accepts_valid_dates() {
        assert!(indonesia::validate_nik("3174011708450001").is_ok());
        // 31 December is fine.
        assert!(indonesia::validate_nik("3174013112990001").is_ok());
    }

    #[test]
    fn nik_accepts_female_day_offset() {
        // Day 41 is 1 with the female offset of 40.
        assert!(indonesia::validate_nik("3174014108450001").is_ok());
        // Day 71 is 31, the last valid offset day.
        assert!(indonesia::validate_nik("3174017112990001").is_ok());
        // Day 72 would be 32, which does not exist.
        assert!(indonesia::validate_nik("3174017212990001").is_err());
    }

    #[test]
    fn nik_rejects_impossible_dates() {
        // Month 13.
        assert!(indonesia::validate_nik("3174011713450001").is_err());
        // Month 00.
        assert!(indonesia::validate_nik("3174011700450001").is_err());
        // 30 February.
        assert!(indonesia::validate_nik("3174013002450001").is_err());
        // 31 April.
        assert!(indonesia::validate_nik("3174013104450001").is_err());
        // Day 00.
        assert!(indonesia::validate_nik("3174010008450001").is_err());
    }

    #[test]
    fn philsys_accepts_a_valid_luhn_number() {
        let payload = "12345678901";
        let check = crate::Luhn::check_digit_for(payload).unwrap();
        assert!(philippines::validate_philsys(&format!("{payload}{check}")).is_ok());
    }

    #[test]
    fn philsys_rejects_a_bad_check_digit() {
        let payload = "12345678901";
        let check = crate::Luhn::check_digit_for(payload).unwrap();
        let wrong = (check + 1) % 10;
        assert_eq!(
            philippines::validate_philsys(&format!("{payload}{wrong}")),
            Err(Error::BadChecksum)
        );
    }
}
