//! Public holiday calendars.
//!
//! The crate answers three questions:
//!
//! 1. Which dates are public holidays in a given country and year?
//! 2. Is a particular date a working day?
//! 3. What is the *n*-th working day from a given date?
//!
//! Holidays are described by [`Rule`]s rather than by hard-coded date tables,
//! so a calendar stays correct for any year instead of going stale. Only the
//! rules that need a calendar (the date of Easter) or a weekday are computed;
//! the rest are plain month/day pairs.
//!
//! Countries differ in what happens when a holiday falls on a weekend. That
//! behaviour is [`SubstituteStyle`], applied per country when a year is
//! resolved.
//!
//! # Example
//!
//! ```
//! use holidays_rs::{Calendar, Date};
//!
//! let cal = Calendar::indonesia();
//! let d = Date::new(2026, 8, 17).unwrap();
//! assert!(cal.is_holiday(d));
//!
//! // 17 August 2026 is a Monday, so the next working day is Tuesday.
//! let next = cal.next_working_day(d);
//! assert_eq!((next.year, next.month, next.day), (2026, 8, 18));
//! ```
//!
//! # Scope
//!
//! Calendars cover *national* holidays. Regional and one-off observances are
//! out of scope; add them with [`Calendar::with_extra`] when needed.

#![forbid(unsafe_code)]

mod countries;
mod date;

pub use countries::{Calendar, Country, SubstituteStyle};
pub use date::Date;

/// A rule that determines the date of a holiday within a year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    /// A fixed month and day, for example 17 August.
    Fixed {
        /// Month, 1-12.
        month: u8,
        /// Day, 1-31.
        day: u8,
    },
    /// A date relative to Easter Sunday, in days. Negative is before Easter.
    EasterOffset(i32),
    /// The *n*-th weekday of a month. `nth` of 5 means "last".
    NthWeekday {
        /// Month, 1-12.
        month: u8,
        /// Weekday, 0 is Sunday.
        weekday: u8,
        /// Which occurrence, 1-based. 5 means the last one in the month.
        nth: u8,
    },
}

impl Rule {
    /// Resolves this rule to the date on which the holiday *falls*, before any
    /// weekend substitution is applied.
    ///
    /// Returns `None` only when the rule describes an impossible date, which
    /// can happen for the fixed rules on a short month.
    #[must_use]
    pub fn resolve(&self, year: i32) -> Option<Date> {
        match *self {
            Self::Fixed { month, day } => Date::new(year, month, day),
            Self::EasterOffset(offset) => Date::easter(year)?.add_days(offset),
            Self::NthWeekday {
                month,
                weekday,
                nth,
            } => Date::nth_weekday(year, month, weekday, nth),
        }
    }
}

/// A named holiday.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holiday {
    /// Local name of the holiday.
    pub name: &'static str,
    /// Rule that produces the date.
    pub rule: Rule,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_rule_resolves() {
        assert_eq!(
            Rule::Fixed { month: 8, day: 17 }.resolve(2026),
            Date::new(2026, 8, 17)
        );
    }

    #[test]
    fn fixed_rule_rejects_impossible_dates() {
        assert!(Rule::Fixed { month: 2, day: 30 }.resolve(2026).is_none());
        assert!(Rule::Fixed { month: 2, day: 29 }.resolve(2026).is_none());
        assert!(Rule::Fixed { month: 2, day: 29 }.resolve(2028).is_some());
    }

    #[test]
    fn easter_offsets() {
        // Easter Sunday 2024 was 31 March; Good Friday is two days before.
        let good_friday = Rule::EasterOffset(-2).resolve(2024).unwrap();
        assert_eq!((good_friday.month, good_friday.day), (3, 29));
    }

    #[test]
    fn nth_weekday_handles_last_occurrence() {
        // Last Monday of May 2026 is the 25th.
        let d = Rule::NthWeekday {
            month: 5,
            weekday: 1,
            nth: 5,
        }
        .resolve(2026)
        .unwrap();
        assert_eq!(d.day, 25);
    }

    #[test]
    fn nth_weekday_handles_last_when_month_has_five() {
        // Thursdays in October 2026: 1, 8, 15, 22, 29.
        let d = Rule::NthWeekday {
            month: 10,
            weekday: 4,
            nth: 5,
        }
        .resolve(2026)
        .unwrap();
        assert_eq!(d.day, 29);
    }
}
