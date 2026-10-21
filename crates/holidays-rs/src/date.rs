//! A civil date with the arithmetic this crate needs.
//!
//! Only the Gregorian calendar is supported. Everything here is a pure
//! function of the fields, which keeps the holiday rules easy to test.

use std::fmt::{self, Display};

/// A Gregorian calendar date.
///
/// Construct with [`Date::new`], which rejects impossible dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// Proleptic Gregorian year.
    pub year: i32,
    /// Month, 1-12.
    pub month: u8,
    /// Day, 1-31.
    pub day: u8,
}

const MONTH_LENGTHS: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Returns whether `year` is a leap year in the Gregorian calendar.
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Returns the number of days in `month` of `year`.
#[must_use]
pub const fn days_in_month(year: i32, month: u8) -> u8 {
    if month == 2 && is_leap_year(year) {
        29
    } else {
        MONTH_LENGTHS[(month - 1) as usize]
    }
}

impl Date {
    /// Creates a date, returning `None` when the fields are out of range.
    #[must_use]
    pub const fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if month < 1 || month > 12 || day < 1 || day > days_in_month(year, month) {
            return None;
        }
        Some(Self { year, month, day })
    }

    /// Days since 1970-01-01. Negative for earlier dates.
    ///
    /// Uses Howard Hinnant's `days_from_civil` algorithm.
    #[must_use]
    pub const fn days_from_epoch(self) -> i64 {
        let y = if self.month <= 2 {
            self.year - 1
        } else {
            self.year
        } as i64;
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = self.month as i64;
        let d = self.day as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Builds a date from days since 1970-01-01.
    #[must_use]
    pub const fn from_days_from_epoch(days: i64) -> Self {
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        Self {
            year: (if m <= 2 { y + 1 } else { y }) as i32,
            month: m as u8,
            day: d as u8,
        }
    }

    /// Weekday, where 0 is Sunday and 6 is Saturday.
    #[must_use]
    pub const fn weekday(self) -> u8 {
        // 1970-01-01 was a Thursday, which is index 4.
        (self.days_from_epoch() + 4).rem_euclid(7) as u8
    }

    /// Returns whether this is a Saturday or Sunday.
    #[must_use]
    pub const fn is_weekend(self) -> bool {
        matches!(self.weekday(), 0 | 6)
    }

    /// Adds `days` to this date, returning `None` on overflow.
    #[must_use]
    pub const fn add_days(self, days: i32) -> Option<Self> {
        Some(Self::from_days_from_epoch(
            self.days_from_epoch() + days as i64,
        ))
    }

    /// The next date whose weekday equals `weekday`, or this date if it
    /// already matches.
    #[must_use]
    pub const fn next_or_same_weekday(self, weekday: u8) -> Self {
        let delta = (weekday as i64 - self.weekday() as i64).rem_euclid(7);
        Self::from_days_from_epoch(self.days_from_epoch() + delta)
    }

    /// The `nth` occurrence of `weekday` in `month` of `year`.
    ///
    /// `nth` is 1-based. A value of 5 means the last occurrence, which is what
    /// many holiday definitions mean by "the last Monday of May".
    #[must_use]
    pub fn nth_weekday(year: i32, month: u8, weekday: u8, nth: u8) -> Option<Self> {
        if weekday > 6 || nth == 0 || nth > 5 {
            return None;
        }
        let first_match = Self::new(year, month, 1)?.next_or_same_weekday(weekday);
        if nth == 5 {
            // Prefer the fifth occurrence; fall back to the fourth, which every
            // month is guaranteed to have.
            let fifth = first_match.add_days(28)?;
            return if fifth.month == month {
                Some(fifth)
            } else {
                first_match.add_days(21)
            };
        }
        let target = first_match.add_days(7 * i32::from(nth - 1))?;
        if target.month == month {
            Some(target)
        } else {
            None
        }
    }

    /// Easter Sunday in the Gregorian calendar, using the anonymous Gregorian
    /// computus (the algorithm also known as Meeus/Jones/Butcher).
    #[must_use]
    pub fn easter(year: i32) -> Option<Self> {
        let a = year % 19;
        let b = year / 100;
        let c = year % 100;
        let d = b / 4;
        let e = b % 4;
        let f = (b + 8) / 25;
        let g = (b - f + 1) / 3;
        let h = (19 * a + b - d - g + 15) % 30;
        let i = c / 4;
        let k = c % 4;
        let l = (32 + 2 * e + 2 * i - h - k) % 7;
        let m = (a + 11 * h + 22 * l) / 451;
        let month = (h + l - 7 * m + 114) / 31;
        let day = (h + l - 7 * m + 114) % 31 + 1;
        Self::new(year, u8::try_from(month).ok()?, u8::try_from(day).ok()?)
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_impossible_dates() {
        assert!(Date::new(2026, 0, 1).is_none());
        assert!(Date::new(2026, 13, 1).is_none());
        assert!(Date::new(2026, 1, 0).is_none());
        assert!(Date::new(2026, 4, 31).is_none());
        assert!(Date::new(2026, 2, 29).is_none());
        assert!(Date::new(2028, 2, 29).is_some());
    }

    #[test]
    fn leap_year_rules() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2025));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
    }

    #[test]
    fn epoch_round_trip() {
        for days in [-100_000_i64, -1, 0, 1, 19_000, 100_000] {
            let d = Date::from_days_from_epoch(days);
            assert_eq!(d.days_from_epoch(), days, "{d}");
        }
    }

    #[test]
    fn known_weekdays() {
        // 1970-01-01 was a Thursday.
        assert_eq!(Date::new(1970, 1, 1).unwrap().weekday(), 4);
        // 2000-01-01 was a Saturday.
        assert_eq!(Date::new(2000, 1, 1).unwrap().weekday(), 6);
        // 2026-10-21 is a Wednesday.
        assert_eq!(Date::new(2026, 10, 21).unwrap().weekday(), 3);
    }

    #[test]
    fn weekend_detection() {
        assert!(Date::new(2026, 10, 17).unwrap().is_weekend()); // Saturday
        assert!(Date::new(2026, 10, 18).unwrap().is_weekend()); // Sunday
        assert!(!Date::new(2026, 10, 19).unwrap().is_weekend()); // Monday
    }

    #[test]
    fn add_days_crosses_month_and_year() {
        let d = Date::new(2025, 12, 31).unwrap();
        assert_eq!(d.add_days(1), Date::new(2026, 1, 1));
        assert_eq!(d.add_days(-365), Date::new(2024, 12, 31));
    }

    #[test]
    fn next_or_same_weekday_is_idempotent_on_match() {
        let monday = Date::new(2026, 10, 19).unwrap();
        assert_eq!(monday.next_or_same_weekday(1), monday);
        assert_eq!(
            monday.next_or_same_weekday(0),
            Date::new(2026, 10, 25).unwrap()
        );
    }

    #[test]
    fn nth_weekday_ordinal_and_last() {
        // Thursdays in October 2026: 1, 8, 15, 22, 29.
        assert_eq!(Date::nth_weekday(2026, 10, 4, 1), Date::new(2026, 10, 1));
        assert_eq!(Date::nth_weekday(2026, 10, 4, 3), Date::new(2026, 10, 15));
        assert_eq!(Date::nth_weekday(2026, 10, 4, 5), Date::new(2026, 10, 29));
    }

    #[test]
    fn nth_weekday_last_when_only_four_occurrences() {
        // May 2026 has four Mondays: 4, 11, 18, 25. "Last Monday" is the 25th.
        assert_eq!(Date::nth_weekday(2026, 5, 1, 5), Date::new(2026, 5, 25));
        // Out-of-range ordinals are rejected rather than guessed at.
        assert!(Date::nth_weekday(2026, 5, 1, 6).is_none());
        assert!(Date::nth_weekday(2026, 5, 1, 0).is_none());
        assert!(Date::nth_weekday(2026, 5, 7, 1).is_none());
    }

    #[test]
    fn easter_dates_match_published_values() {
        // Reference values from the Western (Gregorian) computus.
        let cases = [
            (2020, 4, 12),
            (2021, 4, 4),
            (2022, 4, 17),
            (2023, 4, 9),
            (2024, 3, 31),
            (2025, 4, 20),
            (2026, 4, 5),
            (2027, 3, 28),
            (2030, 4, 21),
            (2038, 4, 25),
        ];
        for (year, month, day) in cases {
            assert_eq!(
                Date::easter(year),
                Date::new(year, month, day),
                "Easter {year}"
            );
        }
    }

    #[test]
    fn display_is_iso_style() {
        assert_eq!(Date::new(2026, 1, 5).unwrap().to_string(), "2026-01-05");
    }
}
