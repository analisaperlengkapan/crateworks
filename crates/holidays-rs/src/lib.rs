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
    /// A fixed date, except that when it falls on a given weekday a different
    /// day of the same month is used instead.
    ///
    /// The Netherlands uses this for Koningsdag: 27 April, or 26 April when
    /// the 27th is a Sunday.
    FixedUnlessWeekday {
        /// Month, 1-12.
        month: u8,
        /// Day that normally holds the holiday.
        day: u8,
        /// Weekday that triggers the alternative, 0 is Sunday.
        weekday: u8,
        /// Day of the same month to use instead.
        then_day: u8,
    },
    /// The fixed date when it falls on a given weekday, otherwise the first
    /// `weekday` on or after it.
    ///
    /// Ireland uses this for Saint Brigid's Day: 1 February when that is a
    /// Friday, otherwise the first Monday in February.
    FixedOnWeekdayElseWeekdayOnOrAfter {
        /// Month, 1-12.
        month: u8,
        /// The fixed date.
        day: u8,
        /// Weekday on which the fixed date is kept, 0 is Sunday.
        on_weekday: u8,
        /// Weekday to advance to otherwise, 0 is Sunday.
        weekday: u8,
    },
    /// The first `weekday` on or after a fixed date.
    ///
    /// The result may fall in the following month, which is required by the
    /// Nordic All Saints' Day: the Saturday between 31 October and
    /// 6 November.
    WeekdayOnOrAfter {
        /// Month, 1-12.
        month: u8,
        /// Earliest day the holiday may fall on.
        day: u8,
        /// Weekday, 0 is Sunday.
        weekday: u8,
    },
    /// The last `weekday` on or before a fixed date.
    ///
    /// The result may fall in the preceding month.
    WeekdayOnOrBefore {
        /// Month, 1-12.
        month: u8,
        /// Latest day the holiday may fall on.
        day: u8,
        /// Weekday, 0 is Sunday.
        weekday: u8,
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
            Self::FixedUnlessWeekday {
                month,
                day,
                weekday,
                then_day,
            } => {
                let date = Date::new(year, month, day)?;
                if date.weekday() == weekday {
                    Date::new(year, month, then_day)
                } else {
                    Some(date)
                }
            }
            Self::FixedOnWeekdayElseWeekdayOnOrAfter {
                month,
                day,
                on_weekday,
                weekday,
            } => {
                let date = Date::new(year, month, day)?;
                if date.weekday() == on_weekday {
                    Some(date)
                } else {
                    let resolved = date.next_or_same_weekday(weekday);
                    // Staying inside the month keeps a mistyped rule visible
                    // rather than silently inventing a holiday in the next one.
                    (resolved.month == month).then_some(resolved)
                }
            }
            Self::WeekdayOnOrAfter {
                month,
                day,
                weekday,
            } => {
                let date = Date::new(year, month, day)?;
                Some(date.next_or_same_weekday(weekday))
            }
            Self::WeekdayOnOrBefore {
                month,
                day,
                weekday,
            } => {
                let date = Date::new(year, month, day)?;
                let back = (date.weekday() as i64 - weekday as i64).rem_euclid(7) as i32;
                date.add_days(-back)
            }
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

    #[test]
    fn fixed_unless_weekday_switches_only_on_that_day() {
        let rule = Rule::FixedUnlessWeekday {
            month: 4,
            day: 27,
            weekday: 0,
            then_day: 26,
        };
        // 27 April 2025 is a Sunday, so Koningsdag moves to the 26th.
        assert_eq!(rule.resolve(2025).unwrap().day, 26);
        // 27 April 2026 is a Monday, so it stays.
        assert_eq!(rule.resolve(2026).unwrap().day, 27);
        // A Saturday does not trigger the switch.
        assert_eq!(rule.resolve(2030).unwrap().day, 27);
    }

    #[test]
    fn fixed_on_weekday_else_weekday_on_or_after() {
        let rule = Rule::FixedOnWeekdayElseWeekdayOnOrAfter {
            month: 2,
            day: 1,
            on_weekday: 5,
            weekday: 1,
        };
        // 1 February 2030 is a Friday, so the fixed date is kept.
        assert_eq!(rule.resolve(2030).unwrap().day, 1);
        // Otherwise the first Monday follows: 2026-02-02.
        let d = rule.resolve(2026).unwrap();
        assert_eq!((d.month, d.day, d.weekday()), (2, 2, 1));
    }

    #[test]
    fn weekday_on_or_after_follows_the_fixed_date() {
        let rule = Rule::WeekdayOnOrAfter {
            month: 6,
            day: 20,
            weekday: 6,
        };
        // Saturdays in June 2026: 6, 13, 20, 27.
        assert_eq!(rule.resolve(2026).unwrap().day, 20);
        // Saturdays in June 2027: 5, 12, 19, 26.
        assert_eq!(rule.resolve(2027).unwrap().day, 26);
    }

    #[test]
    fn weekday_on_or_after_crosses_into_the_next_month() {
        // Nordic All Saints' Day: the Saturday between 31 October and
        // 6 November, which in 2024 was 2 November.
        let rule = Rule::WeekdayOnOrAfter {
            month: 10,
            day: 31,
            weekday: 6,
        };
        let d = rule.resolve(2024).unwrap();
        assert_eq!((d.month, d.day), (11, 2));
        // In 2026 the 31st is already a Saturday.
        let d = rule.resolve(2026).unwrap();
        assert_eq!((d.month, d.day), (10, 31));
    }

    #[test]
    fn weekday_on_or_before_steps_back_from_the_fixed_date() {
        let rule = Rule::WeekdayOnOrBefore {
            month: 5,
            day: 24,
            weekday: 1,
        };
        // Victoria Day: the Monday on or before 24 May.
        assert_eq!(rule.resolve(2024).unwrap().day, 20);
        assert_eq!(rule.resolve(2027).unwrap().day, 24);
        // A date early in the month can step into the previous one.
        let crossing = Rule::WeekdayOnOrBefore {
            month: 5,
            day: 1,
            weekday: 1,
        };
        let d = crossing.resolve(2026).unwrap();
        assert_eq!((d.month, d.day), (4, 27));
    }
}
