//! The timestamp format an EBICS header carries.
//!
//! The `Timestamp` element is UTC, to the millisecond, with a `Z` suffix:
//! `2026-10-22T11:44:26.627Z`. The bank keeps the nonce and timestamp of recent
//! requests so that a replayed message is refused, so a client whose clock is
//! wrong, or whose format is wrong, is rejected before the order is looked at.
//!
//! The workspace has no date crate and the policy is std-only.
//! [`std::time::SystemTime`] can tell how far from the epoch it is, but it cannot
//! turn that into a calendar date, so the civil-date conversion is here. It is
//! Howard Hinnant's `civil_from_days`, which is exact for the whole range of
//! `i64` days and has no lookup tables or leap-year special cases.

/// Formats a count of seconds and milliseconds since the Unix epoch.
///
/// # Panics
///
/// Panics if `millis` is negative, which cannot happen for a clock after 1970.
#[must_use]
pub fn format_epoch_millis(millis: i64) -> String {
    assert!(
        millis >= 0,
        "the epoch is 1970 and EBICS is later than that"
    );
    let seconds = millis / 1_000;
    let millis_part = millis % 1_000;
    let days = seconds / 86_400;
    let seconds_of_day = seconds % 86_400;

    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis_part:03}Z")
}

/// The current time in the format an EBICS header carries.
///
/// Returns `None` if the system clock is before the epoch, which is a clock
/// fault rather than something a caller can recover from.
#[must_use]
pub fn now() -> Option<String> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    let millis = i64::try_from(elapsed.as_millis()).ok()?;
    Some(format_epoch_millis(millis))
}

/// Days since 1970-01-01 to a proleptic Gregorian year, month and day.
///
/// Howard Hinnant's algorithm. The era is a 400-year cycle, which is exactly
/// 146,097 days, so the arithmetic lands on a date without any leap-year table.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Shift the epoch to 0000-03-01, which puts the leap day at the end of the
    // year and makes the month lengths a repeating pattern.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    // January and February were the tail of the shifted year.
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::{civil_from_days, format_epoch_millis, now};

    /// Values checked against the Unix `date` command and the known epoch
    /// boundaries.
    #[test]
    fn known_instants() {
        let cases = [
            (0, "1970-01-01T00:00:00.000Z"),
            (1_000, "1970-01-01T00:00:01.000Z"),
            (86_399_000, "1970-01-01T23:59:59.000Z"),
            (86_400_000, "1970-01-02T00:00:00.000Z"),
            // A leap day.
            (951_782_400_000, "2000-02-29T00:00:00.000Z"),
            // The day after a leap day in a non-leap year.
            (1_583_020_800_000, "2020-03-01T00:00:00.000Z"),
            (1_672_531_200_000, "2023-01-01T00:00:00.000Z"),
            (1_700_000_000_123, "2023-11-14T22:13:20.123Z"),
        ];
        for (millis, expected) in cases {
            assert_eq!(format_epoch_millis(millis), expected, "at {millis}");
        }
    }

    /// The month lengths must not drift: one day after each month's end is the
    /// first of the next.
    #[test]
    fn month_boundaries_in_a_leap_year() {
        // 2024-02-28, 2024-02-29, 2024-03-01.
        assert_eq!(
            format_epoch_millis(1_709_078_400_000),
            "2024-02-28T00:00:00.000Z"
        );
        assert_eq!(
            format_epoch_millis(1_709_164_800_000),
            "2024-02-29T00:00:00.000Z"
        );
        assert_eq!(
            format_epoch_millis(1_709_251_200_000),
            "2024-03-01T00:00:00.000Z"
        );
    }

    /// The 400-year cycle must round-trip, including a century that is not a
    /// leap year.
    #[test]
    fn century_rules() {
        // 1900 was not a leap year, so 1900-03-01 follows 1900-02-28.
        assert_eq!(civil_from_days(-25_509), (1900, 2, 28));
        assert_eq!(civil_from_days(-25_508), (1900, 3, 1));
    }

    #[test]
    fn now_is_well_formed() {
        let stamp = now().expect("the clock is after 1970");
        assert_eq!(stamp.len(), 24, "got {stamp}");
        assert!(stamp.ends_with('Z'));
        assert_eq!(&stamp[4..5], "-");
        assert_eq!(&stamp[10..11], "T");
        assert_eq!(&stamp[19..20], ".");
    }
}
