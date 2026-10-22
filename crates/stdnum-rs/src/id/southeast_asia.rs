//! Indonesia and the Philippines.

use crate::{Error, Result, checksum};

/// Indonesia: NPWP and NIK.
pub(super) mod indonesia {
    use super::*;

    /// Indonesian NPWP: the tax number.
    ///
    /// Two shapes are accepted. The original fifteen-digit form carries a Luhn
    /// check digit in its ninth position. Since 2024 the number is sixteen
    /// digits: either the old number with a leading zero, in which case the
    /// check digit moves to the tenth position, or a NIK, which is recognised
    /// by not starting with zero and validated as one.
    ///
    /// The check digit is what makes this worth validating at all; without it
    /// any fifteen digits are accepted, which is what a typo produces.
    pub(in crate::id) fn validate_npwp(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        match digits.len() {
            15 => {
                if !checksum::luhn_matches(&digits[..8], digits[8]) {
                    return Err(Error::BadChecksum);
                }
                Ok(())
            }
            16 if digits[0] == 0 => {
                if !checksum::luhn_matches(&digits[..9], digits[9]) {
                    return Err(Error::BadChecksum);
                }
                Ok(())
            }
            16 => validate_nik(s),
            found => Err(Error::WrongLength {
                expected: 15,
                found,
            }),
        }
    }

    /// Indonesian NIK: 16 digits encoding region, birth date and sequence.
    ///
    /// The NIK carries no check digit. The date field is validated, because a
    /// plausible-looking but impossible date is the most common data-entry error.
    pub(in crate::id) fn validate_nik(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 16)?;
        // The first two digits are the province. Only the codes that have been
        // assigned are accepted, which catches a transposed digit that would
        // otherwise look like a valid date.
        let province = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        if !NIK_PROVINCES.contains(&province) {
            return Err(Error::OutOfRange);
        }
        let day = u32::from(digits[6]) * 10 + u32::from(digits[7]);
        let month = u32::from(digits[8]) * 10 + u32::from(digits[9]);
        // For women the day has 40 added, so the field runs 1-31 and 41-71 and
        // the offset is a modulo, not a subtraction: 40 is day 0, not day 40.
        let day = day % 40;
        if !(1..=12).contains(&month) || day == 0 {
            return Err(Error::OutOfRange);
        }
        // The century is not stored, so the year is read as 1900-1999 first and
        // as 2000-2099 second, and the date is accepted if it exists in either.
        // 29 February therefore passes for the year 00 (2000 was a leap year)
        // and fails for the year 10 (neither 1910 nor 2010 was).
        let year = u32::from(digits[10]) * 10 + u32::from(digits[11]);
        if !checksum::is_real_date(1900 + year, month, day)
            && !checksum::is_real_date(2000 + year, month, day)
        {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// The province codes the Indonesian statistics agency has assigned.
    const NIK_PROVINCES: [u32; 33] = [
        11, 12, 13, 14, 15, 16, 17, 18, 19, 21, 31, 32, 33, 34, 35, 36, 51, 52, 53, 61, 62, 63, 64,
        71, 72, 73, 74, 75, 76, 81, 82, 91, 94,
    ];
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
    fn npwp_checks_the_luhn_digit() {
        // Every one of these was produced by python-stdnum's `luhn` helper, so
        // the check digit is correct by construction.
        for number in [
            "013121660091000",
            "012345674091000",
            "999999998091000",
            "123456782091000",
        ] {
            assert!(
                indonesia::validate_npwp(number).is_ok(),
                "{number} should be valid"
            );
        }
        // One digit changed in the payload is caught by the check digit.
        assert_eq!(
            indonesia::validate_npwp("013121669091000"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn npwp_accepts_the_sixteen_digit_form() {
        // Since 2024 the number is sixteen digits: the old number with a zero
        // in front, where the check digit has moved to the tenth position.
        assert!(indonesia::validate_npwp("0131216608091000").is_ok());
        assert!(indonesia::validate_npwp("0123456782091000").is_ok());
        assert_eq!(
            indonesia::validate_npwp("0131216609091000"),
            Err(Error::BadChecksum)
        );
        // A sixteen-digit number that does not start with zero is a NIK.
        assert!(indonesia::validate_npwp("3174011708450001").is_ok());
    }

    #[test]
    fn npwp_rejects_bad_shapes() {
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
        // The day is taken modulo forty: 41 is 1 with the female offset, 70 is
        // 30, and 40 is day 0, which no month has.
        assert!(indonesia::validate_nik("3174014108450001").is_ok());
        assert!(indonesia::validate_nik("3174017008450001").is_ok());
        assert!(indonesia::validate_nik("3174014008450001").is_err());
        // Day 71 reduces to 31, and August has one, so it stands.
        assert!(indonesia::validate_nik("3174017108450001").is_ok());
        // The same day in April does not.
        assert!(indonesia::validate_nik("3174017104450001").is_err());
    }

    #[test]
    fn nik_requires_an_assigned_province() {
        // The first two digits are the province, and only the assigned codes
        // are accepted, so a transposed digit is caught.
        assert!(indonesia::validate_nik("3174011708450001").is_ok());
        assert!(indonesia::validate_nik("0174011708450001").is_err());
    }

    #[test]
    fn nik_treats_february_twenty_ninth_as_a_leap_year_test() {
        // The century is not stored, so the date stands if it exists in either
        // 19xx or 20xx. 2000 was a leap year and 1910 was not.
        assert!(indonesia::validate_nik("3174012902000001").is_ok());
        assert!(indonesia::validate_nik("3174012902100001").is_err());
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
