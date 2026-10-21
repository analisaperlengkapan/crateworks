//! Country calendars and the query API built on top of them.

use crate::{Date, Holiday, Rule};

/// What a country does when a holiday falls on a weekend.
///
/// This is a per-country property, not a per-holiday one, because countries
/// differ in a way that cannot be expressed in a single rule. Indonesia and
/// France observe the holiday on its actual date, the United States observes
/// Saturday holidays on the preceding Friday, and the United Kingdom rolls
/// both weekend days forward to the next Monday.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstituteStyle {
    /// Keep the holiday on the date it falls, even at a weekend.
    None,
    /// Saturday holidays move back to Friday; Sunday holidays move to Monday.
    /// This is the United States federal rule.
    NearestWeekday,
    /// Any weekend holiday moves to the following Monday. This is the United
    /// Kingdom bank holiday rule.
    NextMonday,
}

impl SubstituteStyle {
    /// Applies this style to a date.
    #[must_use]
    pub fn apply(self, date: Date) -> Date {
        match self {
            Self::None => date,
            Self::NearestWeekday => match date.weekday() {
                6 => date.add_days(-1).unwrap_or(date),
                0 => date.add_days(1).unwrap_or(date),
                _ => date,
            },
            Self::NextMonday => match date.weekday() {
                6 => date.add_days(2).unwrap_or(date),
                0 => date.add_days(1).unwrap_or(date),
                _ => date,
            },
        }
    }
}

/// A country with a built-in holiday calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Country {
    /// Indonesia.
    Indonesia,
    /// United States (federal holidays).
    UnitedStates,
    /// United Kingdom (England and Wales, bank holidays).
    UnitedKingdom,
    /// Germany.
    Germany,
    /// France.
    France,
}

impl Country {
    /// The holiday rules for this country, in no particular order.
    #[must_use]
    pub fn holidays(self) -> &'static [Holiday] {
        match self {
            Self::Indonesia => INDONESIA,
            Self::UnitedStates => UNITED_STATES,
            Self::UnitedKingdom => UNITED_KINGDOM,
            Self::Germany => GERMANY,
            Self::France => FRANCE,
        }
    }

    /// How this country handles holidays that fall on a weekend.
    #[must_use]
    pub const fn substitute_style(self) -> SubstituteStyle {
        match self {
            Self::UnitedStates => SubstituteStyle::NearestWeekday,
            Self::UnitedKingdom => SubstituteStyle::NextMonday,
            Self::Indonesia | Self::Germany | Self::France => SubstituteStyle::None,
        }
    }

    /// The English name of the country.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Indonesia => "Indonesia",
            Self::UnitedStates => "United States",
            Self::UnitedKingdom => "United Kingdom",
            Self::Germany => "Germany",
            Self::France => "France",
        }
    }
}

/// A country calendar, optionally extended with extra dates.
#[derive(Debug, Clone)]
pub struct Calendar {
    country: Country,
    extra: Vec<Date>,
}

impl Calendar {
    /// Builds the calendar for a country.
    #[must_use]
    pub const fn new(country: Country) -> Self {
        Self {
            country,
            extra: Vec::new(),
        }
    }

    /// Builds the Indonesia calendar.
    #[must_use]
    pub const fn indonesia() -> Self {
        Self::new(Country::Indonesia)
    }

    /// Builds the United States calendar.
    #[must_use]
    pub const fn united_states() -> Self {
        Self::new(Country::UnitedStates)
    }

    /// Builds the United Kingdom calendar.
    #[must_use]
    pub const fn united_kingdom() -> Self {
        Self::new(Country::UnitedKingdom)
    }

    /// The underlying country.
    #[must_use]
    pub const fn country(&self) -> Country {
        self.country
    }

    /// Returns a copy with an additional holiday date.
    ///
    /// Use this for one-off observances such as election days, which no rule
    /// can express.
    #[must_use]
    pub fn with_extra(mut self, date: Date) -> Self {
        self.extra.push(date);
        self
    }

    /// Every holiday in `year` as (observed date, name), sorted by date.
    ///
    /// Two things can move a holiday off the date it falls on. First the
    /// country's [`SubstituteStyle`] shifts weekend holidays. Then, because
    /// substitution can make two holidays collide, a holiday whose observed
    /// date is already taken moves to the next free day. This is what turns
    /// Christmas Day and Boxing Day 2022, which fell on a Sunday and a Monday,
    /// into Monday 26 and Tuesday 27 December.
    ///
    /// A substitution can push a holiday into the next year, as happens when
    /// 1 January falls on a Sunday in the United Kingdom. That date belongs to
    /// the next year's list, so it is not returned here.
    #[must_use]
    pub fn holidays_in(&self, year: i32) -> Vec<(Date, &'static str)> {
        let mut falls: Vec<(Date, &'static str)> = self
            .country
            .holidays()
            .iter()
            .filter_map(|h| h.rule.resolve(year).map(|d| (d, h.name)))
            .collect();
        // Earliest falling date first, so the holiday that keeps its slot is
        // the one that actually falls earlier.
        falls.sort_unstable();

        let style = self.country.substitute_style();
        let mut taken: Vec<Date> = Vec::with_capacity(falls.len());
        let mut out: Vec<(Date, &'static str)> = Vec::with_capacity(falls.len());
        for (date, name) in falls {
            let mut observed = style.apply(date);
            while taken.contains(&observed) {
                let Some(next) = observed.add_days(1) else {
                    break;
                };
                observed = next;
            }
            taken.push(observed);
            if observed.year == year {
                out.push((observed, name));
            }
        }
        out.sort_unstable();
        out
    }

    /// Returns whether `date` is a public holiday.
    #[must_use]
    pub fn is_holiday(&self, date: Date) -> bool {
        self.extra.contains(&date) || self.holiday_name(date).is_some()
    }

    /// Returns the name of the holiday observed on `date`, if any.
    ///
    /// This resolves the whole year, because a weekend substitution or a
    /// collision can move a holiday onto a date that no rule produces
    /// directly.
    #[must_use]
    pub fn holiday_name(&self, date: Date) -> Option<&'static str> {
        self.holidays_in(date.year)
            .into_iter()
            .find(|(d, _)| *d == date)
            .map(|(_, name)| name)
    }

    /// Returns whether `date` is a working day: not a weekend and not a
    /// holiday.
    #[must_use]
    pub fn is_working_day(&self, date: Date) -> bool {
        !date.is_weekend() && !self.is_holiday(date)
    }

    /// All holiday dates in `year`, including extras, for repeated lookups.
    fn year_holidays(&self, year: i32) -> Vec<Date> {
        let mut dates: Vec<Date> = self.holidays_in(year).into_iter().map(|(d, _)| d).collect();
        dates.extend(self.extra.iter().copied().filter(|d| d.year == year));
        dates
    }

    /// Walks `steps` days in the given direction, counting working days, and
    /// returns the date of the `n`-th one.
    ///
    /// The holiday set is resolved once per year rather than once per day.
    fn walk_working_days(&self, date: Date, n: u32, forward: bool) -> Option<Date> {
        let step: i32 = if forward { 1 } else { -1 };
        let mut current = date;
        let mut remaining = n;
        let mut cached_year = i32::MIN;
        let mut cached: Vec<Date> = Vec::new();
        for _ in 0..3_660 {
            current = current.add_days(step)?;
            if current.year != cached_year {
                cached = self.year_holidays(current.year);
                cached_year = current.year;
            }
            if !current.is_weekend() && !cached.contains(&current) {
                remaining -= 1;
                if remaining == 0 {
                    return Some(current);
                }
            }
        }
        None
    }

    /// The next working day strictly after `date`.
    #[must_use]
    pub fn next_working_day(&self, date: Date) -> Date {
        self.nth_working_day_after(date, 1)
    }

    /// The working day `n` positions after `date`, skipping weekends and
    /// holidays.
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero, or if no working day is found within ten years,
    /// which would mean the calendar declares an implausible run of holidays.
    #[must_use]
    pub fn nth_working_day_after(&self, date: Date, n: u32) -> Date {
        assert!(n > 0, "n must be at least 1");
        // Ten years of daily steps is far beyond any real holiday run, so
        // reaching the bound indicates a broken calendar rather than bad input.
        self.walk_working_days(date, n, true)
            .unwrap_or_else(|| panic!("no working day found within ten years of {date}"))
    }

    /// The working day `n` positions before `date`.
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero or no working day is found within ten years.
    #[must_use]
    pub fn nth_working_day_before(&self, date: Date, n: u32) -> Date {
        assert!(n > 0, "n must be at least 1");
        self.walk_working_days(date, n, false)
            .unwrap_or_else(|| panic!("no working day found within ten years before {date}"))
    }

    /// Number of working days in the inclusive range, useful for invoicing.
    #[must_use]
    pub fn working_days_between(&self, from: Date, to: Date) -> u32 {
        let mut count = 0;
        let mut current = from;
        let mut cached_year = i32::MIN;
        let mut cached: Vec<Date> = Vec::new();
        while current <= to {
            if current.year != cached_year {
                cached = self.year_holidays(current.year);
                cached_year = current.year;
            }
            if !current.is_weekend() && !cached.contains(&current) {
                count += 1;
            }
            let Some(next) = current.add_days(1) else {
                break;
            };
            current = next;
        }
        count
    }
}

const fn h(name: &'static str, rule: Rule) -> Holiday {
    Holiday { name, rule }
}

/// Indonesian national holidays.
///
/// The Islamic and Chinese-derived observances (Idul Fitri, Idul Adha, Imlek,
/// Nyepi, Waisak) depend on official yearly decrees rather than a formula, so
/// they are intentionally absent. Add them per year with
/// [`Calendar::with_extra`] once the government decree is published.
static INDONESIA: &[Holiday] = &[
    h("New Year's Day", Rule::Fixed { month: 1, day: 1 }),
    h("Labour Day", Rule::Fixed { month: 5, day: 1 }),
    h("Independence Day", Rule::Fixed { month: 8, day: 17 }),
    h("Christmas Day", Rule::Fixed { month: 12, day: 25 }),
    h("Good Friday", Rule::EasterOffset(-2)),
    h("Ascension Day of Jesus Christ", Rule::EasterOffset(39)),
];

/// United States federal holidays.
static UNITED_STATES: &[Holiday] = &[
    h("New Year's Day", Rule::Fixed { month: 1, day: 1 }),
    h("Juneteenth", Rule::Fixed { month: 6, day: 19 }),
    h("Independence Day", Rule::Fixed { month: 7, day: 4 }),
    h("Veterans Day", Rule::Fixed { month: 11, day: 11 }),
    h("Christmas Day", Rule::Fixed { month: 12, day: 25 }),
    h(
        "Martin Luther King Jr. Day",
        Rule::NthWeekday {
            month: 1,
            weekday: 1,
            nth: 3,
        },
    ),
    h(
        "Washington's Birthday",
        Rule::NthWeekday {
            month: 2,
            weekday: 1,
            nth: 3,
        },
    ),
    h(
        "Memorial Day",
        Rule::NthWeekday {
            month: 5,
            weekday: 1,
            nth: 5,
        },
    ),
    h(
        "Labor Day",
        Rule::NthWeekday {
            month: 9,
            weekday: 1,
            nth: 1,
        },
    ),
    h(
        "Columbus Day",
        Rule::NthWeekday {
            month: 10,
            weekday: 1,
            nth: 2,
        },
    ),
    h(
        "Thanksgiving Day",
        Rule::NthWeekday {
            month: 11,
            weekday: 4,
            nth: 4,
        },
    ),
];

/// United Kingdom bank holidays for England and Wales.
static UNITED_KINGDOM: &[Holiday] = &[
    h("New Year's Day", Rule::Fixed { month: 1, day: 1 }),
    h("Christmas Day", Rule::Fixed { month: 12, day: 25 }),
    h("Boxing Day", Rule::Fixed { month: 12, day: 26 }),
    h("Good Friday", Rule::EasterOffset(-2)),
    h("Easter Monday", Rule::EasterOffset(1)),
    h(
        "Early May Bank Holiday",
        Rule::NthWeekday {
            month: 5,
            weekday: 1,
            nth: 1,
        },
    ),
    h(
        "Spring Bank Holiday",
        Rule::NthWeekday {
            month: 5,
            weekday: 1,
            nth: 5,
        },
    ),
    h(
        "Summer Bank Holiday",
        Rule::NthWeekday {
            month: 8,
            weekday: 1,
            nth: 5,
        },
    ),
];

/// German national holidays. Regional holidays such as Epiphany and Corpus
/// Christi vary by state and are not included.
static GERMANY: &[Holiday] = &[
    h("Neujahr", Rule::Fixed { month: 1, day: 1 }),
    h("Tag der Arbeit", Rule::Fixed { month: 5, day: 1 }),
    h(
        "Tag der Deutschen Einheit",
        Rule::Fixed { month: 10, day: 3 },
    ),
    h("Erster Weihnachtstag", Rule::Fixed { month: 12, day: 25 }),
    h("Zweiter Weihnachtstag", Rule::Fixed { month: 12, day: 26 }),
    h("Karfreitag", Rule::EasterOffset(-2)),
    h("Ostermontag", Rule::EasterOffset(1)),
    h("Christi Himmelfahrt", Rule::EasterOffset(39)),
    h("Pfingstmontag", Rule::EasterOffset(50)),
];

/// French national holidays.
static FRANCE: &[Holiday] = &[
    h("Jour de l'an", Rule::Fixed { month: 1, day: 1 }),
    h("Fête du Travail", Rule::Fixed { month: 5, day: 1 }),
    h("Victoire 1945", Rule::Fixed { month: 5, day: 8 }),
    h("Fête nationale", Rule::Fixed { month: 7, day: 14 }),
    h("Assomption", Rule::Fixed { month: 8, day: 15 }),
    h("Toussaint", Rule::Fixed { month: 11, day: 1 }),
    h("Armistice 1918", Rule::Fixed { month: 11, day: 11 }),
    h("Noël", Rule::Fixed { month: 12, day: 25 }),
    h("Lundi de Pâques", Rule::EasterOffset(1)),
    h("Ascension", Rule::EasterOffset(39)),
    h("Lundi de Pentecôte", Rule::EasterOffset(50)),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::new(y, m, day).unwrap()
    }

    #[test]
    fn indonesia_fixed_holidays() {
        let cal = Calendar::indonesia();
        assert!(cal.is_holiday(d(2026, 8, 17)));
        assert!(cal.is_holiday(d(2026, 1, 1)));
        assert!(cal.is_holiday(d(2026, 12, 25)));
        assert!(!cal.is_holiday(d(2026, 8, 18)));
    }

    #[test]
    fn indonesia_easter_relative_holidays() {
        let cal = Calendar::indonesia();
        // Easter 2026 is 5 April, so Good Friday is 3 April.
        assert!(cal.is_holiday(d(2026, 4, 3)));
        // Ascension is 39 days after Easter, which is 14 May 2026.
        assert!(cal.is_holiday(d(2026, 5, 14)));
    }

    #[test]
    fn indonesia_does_not_substitute_weekends() {
        let cal = Calendar::indonesia();
        // 17 August 2025 is a Sunday and is still the holiday on that date.
        assert_eq!(cal.holiday_name(d(2025, 8, 17)), Some("Independence Day"));
        assert_eq!(cal.holiday_name(d(2025, 8, 18)), None);
    }

    #[test]
    fn united_states_thanksgiving_is_fourth_thursday() {
        let cal = Calendar::united_states();
        assert_eq!(cal.holiday_name(d(2026, 11, 26)), Some("Thanksgiving Day"));
        assert_eq!(cal.holiday_name(d(2026, 11, 19)), None);
    }

    #[test]
    fn united_states_memorial_day_is_last_monday() {
        let cal = Calendar::united_states();
        assert_eq!(cal.holiday_name(d(2026, 5, 25)), Some("Memorial Day"));
    }

    #[test]
    fn united_states_saturday_holiday_moves_to_friday() {
        let cal = Calendar::united_states();
        // 4 July 2026 is a Saturday, observed on Friday the 3rd.
        assert_eq!(cal.holiday_name(d(2026, 7, 3)), Some("Independence Day"));
        assert_eq!(cal.holiday_name(d(2026, 7, 4)), None);
    }

    #[test]
    fn united_states_sunday_holiday_moves_to_monday() {
        let cal = Calendar::united_states();
        // 25 December 2022 was a Sunday, observed on Monday the 26th.
        assert_eq!(cal.holiday_name(d(2022, 12, 26)), Some("Christmas Day"));
    }

    #[test]
    fn united_kingdom_weekend_holiday_moves_to_monday() {
        let cal = Calendar::united_kingdom();
        // 25 December 2022 was a Sunday, so both Christmas and Boxing Day roll
        // forward: Christmas to the 26th and Boxing Day to the 27th.
        let names: Vec<&str> = cal
            .holidays_in(2022)
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        assert!(names.contains(&"Christmas Day"));
        assert!(names.contains(&"Boxing Day"));
        assert_eq!(cal.holiday_name(d(2022, 12, 26)), Some("Christmas Day"));
        assert_eq!(cal.holiday_name(d(2022, 12, 27)), Some("Boxing Day"));
    }

    #[test]
    fn united_kingdom_easter_monday() {
        let cal = Calendar::united_kingdom();
        // Easter 2026 is 5 April, so Easter Monday is 6 April.
        assert_eq!(cal.holiday_name(d(2026, 4, 6)), Some("Easter Monday"));
    }

    #[test]
    fn germany_reunification_day() {
        let cal = Calendar::new(Country::Germany);
        assert!(cal.is_holiday(d(2026, 10, 3)));
        assert!(cal.is_holiday(d(2026, 5, 25))); // Pfingstmontag, Easter 2026 + 50
    }

    #[test]
    fn france_bastille_day() {
        let cal = Calendar::new(Country::France);
        assert!(cal.is_holiday(d(2026, 7, 14)));
        assert!(cal.is_holiday(d(2026, 5, 8)));
    }

    #[test]
    fn holidays_in_is_sorted_and_year_bounded() {
        let cal = Calendar::united_states();
        let list = cal.holidays_in(2026);
        assert!(!list.is_empty());
        let mut sorted = list.clone();
        sorted.sort_unstable();
        assert_eq!(list, sorted);
        assert!(list.iter().all(|(date, _)| date.year == 2026));
    }

    #[test]
    fn substitute_styles_are_distinct() {
        // Saturday 2026-07-04.
        let saturday = d(2026, 7, 4);
        assert_eq!(SubstituteStyle::None.apply(saturday), saturday);
        assert_eq!(
            SubstituteStyle::NearestWeekday.apply(saturday),
            d(2026, 7, 3)
        );
        assert_eq!(SubstituteStyle::NextMonday.apply(saturday), d(2026, 7, 6));
        // Sunday 2026-07-05.
        let sunday = d(2026, 7, 5);
        assert_eq!(SubstituteStyle::NearestWeekday.apply(sunday), d(2026, 7, 6));
        assert_eq!(SubstituteStyle::NextMonday.apply(sunday), d(2026, 7, 6));
    }

    #[test]
    fn weekend_is_not_a_working_day() {
        let cal = Calendar::indonesia();
        assert!(!cal.is_working_day(d(2026, 10, 17))); // Saturday
        assert!(!cal.is_working_day(d(2026, 8, 17))); // Monday but a holiday
        assert!(cal.is_working_day(d(2026, 10, 19))); // ordinary Monday
    }

    #[test]
    fn next_working_day_skips_weekend() {
        let cal = Calendar::indonesia();
        // Friday 16 October 2026 -> Monday 19 October.
        assert_eq!(cal.next_working_day(d(2026, 10, 16)), d(2026, 10, 19));
    }

    #[test]
    fn next_working_day_skips_holiday() {
        let cal = Calendar::indonesia();
        // 17 August 2026 is a Monday holiday, so next working day is Tuesday.
        assert_eq!(cal.next_working_day(d(2026, 8, 17)), d(2026, 8, 18));
    }

    #[test]
    fn nth_working_day_after_and_before() {
        let cal = Calendar::united_states();
        // Monday 2026-11-23, skipping Thanksgiving Thursday and the weekend.
        let start = d(2026, 11, 23);
        assert_eq!(cal.nth_working_day_after(start, 1), d(2026, 11, 24));
        assert_eq!(cal.nth_working_day_after(start, 2), d(2026, 11, 25));
        assert_eq!(cal.nth_working_day_after(start, 3), d(2026, 11, 27));
        assert_eq!(
            cal.nth_working_day_before(d(2026, 11, 30), 1),
            d(2026, 11, 27)
        );
    }

    #[test]
    fn working_days_between_counts_inclusive() {
        let cal = Calendar::united_states();
        // Mon 2026-11-23 through Fri 2026-11-27, minus Thanksgiving Thursday.
        assert_eq!(
            cal.working_days_between(d(2026, 11, 23), d(2026, 11, 27)),
            4
        );
    }

    #[test]
    fn extra_dates_are_honoured() {
        let cal = Calendar::indonesia().with_extra(d(2026, 11, 25));
        assert!(cal.is_holiday(d(2026, 11, 25)));
        assert!(!cal.is_working_day(d(2026, 11, 25)));
        assert_eq!(cal.holiday_name(d(2026, 11, 25)), None);
    }

    #[test]
    fn country_metadata() {
        assert_eq!(Calendar::indonesia().country(), Country::Indonesia);
        assert_eq!(Country::Germany.name(), "Germany");
        assert_eq!(
            Country::UnitedStates.substitute_style(),
            SubstituteStyle::NearestWeekday
        );
    }

    #[test]
    #[should_panic(expected = "n must be at least 1")]
    fn zero_working_days_panics() {
        let cal = Calendar::indonesia();
        let _ = cal.nth_working_day_after(d(2026, 1, 1), 0);
    }
}
