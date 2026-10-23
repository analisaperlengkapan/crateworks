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
    /// Netherlands.
    Netherlands,
    /// Spain.
    Spain,
    /// Italy.
    Italy,
    /// Portugal.
    Portugal,
    /// Belgium.
    Belgium,
    /// Austria.
    Austria,
    /// Poland.
    Poland,
    /// Sweden.
    Sweden,
    /// Norway.
    Norway,
    /// Denmark.
    Denmark,
    /// Finland.
    Finland,
    /// Ireland.
    Ireland,
    /// Canada (federal holidays).
    Canada,
    /// New Zealand.
    NewZealand,
}

impl Country {
    /// Every country this crate knows, for iterating over the whole set.
    pub const ALL: &'static [Self] = &[
        Self::Indonesia,
        Self::UnitedStates,
        Self::UnitedKingdom,
        Self::Germany,
        Self::France,
        Self::Netherlands,
        Self::Spain,
        Self::Italy,
        Self::Portugal,
        Self::Belgium,
        Self::Austria,
        Self::Poland,
        Self::Sweden,
        Self::Norway,
        Self::Denmark,
        Self::Finland,
        Self::Ireland,
        Self::Canada,
        Self::NewZealand,
    ];

    /// The holiday rules for this country, in no particular order.
    #[must_use]
    pub fn holidays(self) -> &'static [Holiday] {
        match self {
            Self::Indonesia => INDONESIA,
            Self::UnitedStates => UNITED_STATES,
            Self::UnitedKingdom => UNITED_KINGDOM,
            Self::Germany => GERMANY,
            Self::France => FRANCE,
            Self::Netherlands => NETHERLANDS,
            Self::Spain => SPAIN,
            Self::Italy => ITALY,
            Self::Portugal => PORTUGAL,
            Self::Belgium => BELGIUM,
            Self::Austria => AUSTRIA,
            Self::Poland => POLAND,
            Self::Sweden => SWEDEN,
            Self::Norway => NORWAY,
            Self::Denmark => DENMARK,
            Self::Finland => FINLAND,
            Self::Ireland => IRELAND,
            Self::Canada => CANADA,
            Self::NewZealand => NEW_ZEALAND,
        }
    }

    /// How this country handles holidays that fall on a weekend.
    #[must_use]
    pub const fn substitute_style(self) -> SubstituteStyle {
        match self {
            Self::UnitedStates => SubstituteStyle::NearestWeekday,
            Self::UnitedKingdom | Self::Ireland | Self::Canada | Self::NewZealand => {
                SubstituteStyle::NextMonday
            }
            Self::Indonesia
            | Self::Germany
            | Self::France
            | Self::Netherlands
            | Self::Spain
            | Self::Italy
            | Self::Portugal
            | Self::Belgium
            | Self::Austria
            | Self::Poland
            | Self::Sweden
            | Self::Norway
            | Self::Denmark
            | Self::Finland => SubstituteStyle::None,
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
            Self::Netherlands => "Netherlands",
            Self::Spain => "Spain",
            Self::Italy => "Italy",
            Self::Portugal => "Portugal",
            Self::Belgium => "Belgium",
            Self::Austria => "Austria",
            Self::Poland => "Poland",
            Self::Sweden => "Sweden",
            Self::Norway => "Norway",
            Self::Denmark => "Denmark",
            Self::Finland => "Finland",
            Self::Ireland => "Ireland",
            Self::Canada => "Canada",
            Self::NewZealand => "New Zealand",
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

    /// Builds the calendar for Germany.
    #[must_use]
    pub const fn germany() -> Self {
        Self::new(Country::Germany)
    }

    /// Builds the calendar for France.
    #[must_use]
    pub const fn france() -> Self {
        Self::new(Country::France)
    }

    /// Builds the calendar for the Netherlands.
    #[must_use]
    pub const fn netherlands() -> Self {
        Self::new(Country::Netherlands)
    }

    /// Builds the calendar for Spain.
    #[must_use]
    pub const fn spain() -> Self {
        Self::new(Country::Spain)
    }

    /// Builds the calendar for Italy.
    #[must_use]
    pub const fn italy() -> Self {
        Self::new(Country::Italy)
    }

    /// Builds the calendar for Portugal.
    #[must_use]
    pub const fn portugal() -> Self {
        Self::new(Country::Portugal)
    }

    /// Builds the calendar for Belgium.
    #[must_use]
    pub const fn belgium() -> Self {
        Self::new(Country::Belgium)
    }

    /// Builds the calendar for Austria.
    #[must_use]
    pub const fn austria() -> Self {
        Self::new(Country::Austria)
    }

    /// Builds the calendar for Poland.
    #[must_use]
    pub const fn poland() -> Self {
        Self::new(Country::Poland)
    }

    /// Builds the calendar for Sweden.
    #[must_use]
    pub const fn sweden() -> Self {
        Self::new(Country::Sweden)
    }

    /// Builds the calendar for Norway.
    #[must_use]
    pub const fn norway() -> Self {
        Self::new(Country::Norway)
    }

    /// Builds the calendar for Denmark.
    #[must_use]
    pub const fn denmark() -> Self {
        Self::new(Country::Denmark)
    }

    /// Builds the calendar for Finland.
    #[must_use]
    pub const fn finland() -> Self {
        Self::new(Country::Finland)
    }

    /// Builds the calendar for Ireland.
    #[must_use]
    pub const fn ireland() -> Self {
        Self::new(Country::Ireland)
    }

    /// Builds the calendar for Canada.
    #[must_use]
    pub const fn canada() -> Self {
        Self::new(Country::Canada)
    }

    /// Builds the calendar for New Zealand.
    #[must_use]
    pub const fn new_zealand() -> Self {
        Self::new(Country::NewZealand)
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
    ///
    /// # Example
    ///
    /// ```
    /// use holidays_rs::{Calendar, Date};
    ///
    /// let cal = Calendar::indonesia();
    /// let election = Date::new(2026, 6, 3).unwrap();
    ///
    /// // A one-off observance is not in the calendar until it is added.
    /// assert!(!cal.clone().is_holiday(election));
    /// let with_election = cal.with_extra(election);
    /// assert!(with_election.is_holiday(election));
    /// ```
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
        // The next year is resolved too, because its 1 January can be observed
        // on 31 December of this one when that is a Saturday, which is what the
        // United States does. Dates observed outside `year` are dropped below.
        let mut falls: Vec<(Date, &'static str, bool, bool)> = self
            .country
            .holidays()
            .iter()
            .flat_map(|h| {
                [year, year + 1].into_iter().filter_map(move |y| {
                    h.rule.resolve(y).map(|d| {
                        (
                            d,
                            h.name,
                            h.rule.substitutes_in(y),
                            h.rule.substitute_may_share(),
                        )
                    })
                })
            })
            .collect();
        // Earliest falling date first, so the holiday that keeps its slot is
        // the one that actually falls earlier.
        falls.sort_unstable();

        let style = self.country.substitute_style();
        // A country with no substitution keeps every holiday on the date it
        // falls, even when two share a date, so the collision rule does not
        // apply at all.
        if style == SubstituteStyle::None {
            let mut out: Vec<(Date, &'static str)> = falls
                .into_iter()
                .filter(|(d, _, _, _)| d.year == year)
                .map(|(d, name, _, _)| (d, name))
                .collect();
            out.sort_unstable();
            out.dedup();
            return out;
        }

        // A holiday that falls on a weekday keeps its date; only a weekend
        // holiday moves, and it moves to the next free day. The weekday
        // holidays must therefore be placed first: in the United Kingdom in
        // 2022 Boxing Day fell on a Monday and kept the 26th, so Christmas Day
        // (a Sunday) rolled past it to the 27th rather than taking the 26th.
        let mut weekday: Vec<(Date, &'static str)> = Vec::new();
        let mut weekend: Vec<(Date, &'static str, bool)> = Vec::new();
        for (date, name, substitutes, may_share) in falls {
            if substitutes && date.is_weekend() {
                weekend.push((date, name, may_share));
            } else {
                weekday.push((date, name));
            }
        }
        let mut taken: Vec<Date> = Vec::with_capacity(weekday.len() + weekend.len());
        let mut out: Vec<(Date, &'static str)> = Vec::with_capacity(weekday.len() + weekend.len());
        for (date, name) in weekday {
            taken.push(date);
            if date.year == year {
                out.push((date, name));
            }
        }
        for (date, name, may_share) in weekend {
            let mut observed = style.apply(date);
            while !may_share && taken.contains(&observed) {
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
        out.dedup();
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
    h(
        "Labour Day",
        Rule::Since {
            year: 2014,
            base: &Rule::Fixed { month: 5, day: 1 },
        },
    ),
    h(
        "Pancasila Day",
        Rule::Since {
            year: 2016,
            base: &Rule::Fixed { month: 6, day: 1 },
        },
    ),
    h("Independence Day", Rule::Fixed { month: 8, day: 17 }),
    h("Christmas Day", Rule::Fixed { month: 12, day: 25 }),
    h("Good Friday", Rule::EasterOffset(-2)),
    h("Ascension Day of Jesus Christ", Rule::EasterOffset(39)),
];

/// United States federal holidays.
///
/// Juneteenth became a federal holiday on 17 June 2021, so the date is an
/// ordinary working day before then.
static UNITED_STATES: &[Holiday] = &[
    h("New Year's Day", Rule::Fixed { month: 1, day: 1 }),
    h(
        "Juneteenth",
        Rule::Since {
            year: 2021,
            base: &Rule::Fixed { month: 6, day: 19 },
        },
    ),
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
        Rule::ExceptYears {
            base: &Rule::NthWeekday {
                month: 5,
                weekday: 1,
                nth: 1,
            },
            years: &[2020],
        },
    ),
    h(
        "Spring Bank Holiday",
        // Moved from the last Monday in May into early June in the jubilee
        // years, so the jubilee day could be a separate bank holiday.
        Rule::ExceptYears {
            base: &Rule::NthWeekday {
                month: 5,
                weekday: 1,
                nth: 5,
            },
            years: &[2002, 2012, 2022],
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
    // One-off bank holidays. 8 May 2020 was the moved early May bank holiday
    // for the 75th anniversary of VE Day; 3 June 2022 was the Platinum Jubilee
    // and 19 September 2022 the state funeral of Elizabeth II.
    h(
        "Early May Bank Holiday (VE Day)",
        Rule::Once {
            year: 2020,
            month: 5,
            day: 8,
        },
    ),
    h(
        "Platinum Jubilee of Elizabeth II",
        Rule::Once {
            year: 2022,
            month: 6,
            day: 3,
        },
    ),
    h(
        "State Funeral of Queen Elizabeth II",
        Rule::Once {
            year: 2022,
            month: 9,
            day: 19,
        },
    ),
    h(
        "Spring Bank Holiday (Golden Jubilee)",
        Rule::Once {
            year: 2002,
            month: 6,
            day: 4,
        },
    ),
    h(
        "Spring Bank Holiday (Diamond Jubilee)",
        Rule::Once {
            year: 2012,
            month: 6,
            day: 4,
        },
    ),
    h(
        "Spring Bank Holiday (Platinum Jubilee)",
        Rule::Once {
            year: 2022,
            month: 6,
            day: 2,
        },
    ),
    h(
        "Silver Jubilee of Elizabeth II",
        Rule::Once {
            year: 1977,
            month: 6,
            day: 7,
        },
    ),
    h(
        "Wedding of Charles and Diana",
        Rule::Once {
            year: 1981,
            month: 7,
            day: 29,
        },
    ),
    h(
        "Millennium Celebrations",
        Rule::Once {
            year: 1999,
            month: 12,
            day: 31,
        },
    ),
    h(
        "Golden Jubilee of Elizabeth II",
        Rule::Once {
            year: 2002,
            month: 6,
            day: 3,
        },
    ),
    h(
        "Wedding of William and Catherine",
        Rule::Once {
            year: 2011,
            month: 4,
            day: 29,
        },
    ),
    h(
        "Diamond Jubilee of Elizabeth II",
        Rule::Once {
            year: 2012,
            month: 6,
            day: 5,
        },
    ),
    h(
        "Coronation of Charles III",
        Rule::Once {
            year: 2023,
            month: 5,
            day: 8,
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
    // A one-off national holiday for the 500th anniversary of the Reformation.
    h(
        "Reformationstag",
        Rule::Once {
            year: 2017,
            month: 10,
            day: 31,
        },
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
    // Whit Monday was worked as a solidarity day from 2005 to 2007.
    h(
        "Lundi de Pentecôte",
        Rule::ExceptYears {
            base: &Rule::EasterOffset(50),
            years: &[2005, 2006, 2007],
        },
    ),
];

/// Dutch national holidays. King's Day moves to 26 April when the 27th is a
/// Sunday. Liberation Day is a public holiday only every fifth year, which is
/// what [`Rule::EveryNYears`] expresses.
static NETHERLANDS: &[Holiday] = &[
    h("Nieuwjaarsdag", Rule::Fixed { month: 1, day: 1 }),
    h("Goede Vrijdag", Rule::EasterOffset(-2)),
    h("Eerste Paasdag", Rule::EasterOffset(0)),
    h("Tweede Paasdag", Rule::EasterOffset(1)),
    h(
        "Koninginnedag",
        Rule::Until {
            year: 2013,
            base: &Rule::FixedUnlessWeekday {
                month: 4,
                day: 30,
                weekday: 0,
                then_day: 29,
            },
        },
    ),
    h(
        "Koningsdag",
        Rule::Since {
            year: 2014,
            base: &Rule::FixedUnlessWeekday {
                month: 4,
                day: 27,
                weekday: 0,
                then_day: 26,
            },
        },
    ),
    h(
        "Bevrijdingsdag",
        Rule::EveryNYears {
            step: 5,
            month: 5,
            day: 5,
        },
    ),
    h("Hemelvaartsdag", Rule::EasterOffset(39)),
    h("Eerste Pinksterdag", Rule::EasterOffset(49)),
    h("Tweede Pinksterdag", Rule::EasterOffset(50)),
    h("Eerste Kerstdag", Rule::Fixed { month: 12, day: 25 }),
    h("Tweede Kerstdag", Rule::Fixed { month: 12, day: 26 }),
];

/// Spanish national holidays. Regional holidays such as the Basque Country's
/// and Catalonia's are not included.
static SPAIN: &[Holiday] = &[
    h("Año Nuevo", Rule::Fixed { month: 1, day: 1 }),
    h("Epifanía del Señor", Rule::Fixed { month: 1, day: 6 }),
    h("Viernes Santo", Rule::EasterOffset(-2)),
    h("Fiesta del Trabajo", Rule::Fixed { month: 5, day: 1 }),
    h("Asunción de la Virgen", Rule::Fixed { month: 8, day: 15 }),
    h(
        "Fiesta Nacional de España",
        Rule::Fixed { month: 10, day: 12 },
    ),
    h("Todos los Santos", Rule::Fixed { month: 11, day: 1 }),
    h("Día de la Constitución", Rule::Fixed { month: 12, day: 6 }),
    h("Inmaculada Concepción", Rule::Fixed { month: 12, day: 8 }),
    h("Navidad", Rule::Fixed { month: 12, day: 25 }),
];

/// Italian national holidays. Regional and patron-saint days are not included.
static ITALY: &[Holiday] = &[
    h("Capodanno", Rule::Fixed { month: 1, day: 1 }),
    h("Epifania", Rule::Fixed { month: 1, day: 6 }),
    h("Lunedì dell'Angelo", Rule::EasterOffset(1)),
    h("Festa della Liberazione", Rule::Fixed { month: 4, day: 25 }),
    h("Festa dei Lavoratori", Rule::Fixed { month: 5, day: 1 }),
    h(
        "Festa della Repubblica",
        // Established in 1947, movable to the first Sunday in June from 1977 to
        // 2000, and fixed to 2 June again from 2001.
        Rule::Between {
            from: 1947,
            to: 1976,
            base: &Rule::Fixed { month: 6, day: 2 },
        },
    ),
    h(
        "Festa della Repubblica",
        Rule::Between {
            from: 1977,
            to: 2000,
            base: &Rule::NthWeekday {
                month: 6,
                weekday: 0,
                nth: 1,
            },
        },
    ),
    h(
        "Festa della Repubblica",
        Rule::Since {
            year: 2001,
            base: &Rule::Fixed { month: 6, day: 2 },
        },
    ),
    h(
        "Anniversario dell'Unità d'Italia",
        Rule::Once {
            year: 2011,
            month: 3,
            day: 17,
        },
    ),
    h(
        "Festa nazionale di San Francesco d'Assisi, patrono d'Italia",
        Rule::Since {
            year: 2026,
            base: &Rule::Fixed { month: 10, day: 4 },
        },
    ),
    h("Ferragosto", Rule::Fixed { month: 8, day: 15 }),
    h("Tutti i Santi", Rule::Fixed { month: 11, day: 1 }),
    h("Immacolata Concezione", Rule::Fixed { month: 12, day: 8 }),
    h("Natale", Rule::Fixed { month: 12, day: 25 }),
    h("Santo Stefano", Rule::Fixed { month: 12, day: 26 }),
];

/// Portuguese national holidays. Municipal holidays are not included.
///
/// Four holidays were suspended from 2013 to 2015 as an austerity measure and
/// restored in 2016, which is what [`Rule::ExceptYears`] records.
static PORTUGAL: &[Holiday] = &[
    h("Ano Novo", Rule::Fixed { month: 1, day: 1 }),
    h("Sexta-feira Santa", Rule::EasterOffset(-2)),
    h("Páscoa", Rule::EasterOffset(0)),
    h("Dia da Liberdade", Rule::Fixed { month: 4, day: 25 }),
    h("Dia do Trabalhador", Rule::Fixed { month: 5, day: 1 }),
    h(
        "Corpo de Deus",
        Rule::ExceptYears {
            base: &Rule::EasterOffset(60),
            years: &[2013, 2014, 2015],
        },
    ),
    h("Dia de Portugal", Rule::Fixed { month: 6, day: 10 }),
    h(
        "Assunção de Nossa Senhora",
        Rule::Fixed { month: 8, day: 15 },
    ),
    h(
        "Implantação da República",
        Rule::ExceptYears {
            base: &Rule::Fixed { month: 10, day: 5 },
            years: &[2013, 2014, 2015],
        },
    ),
    h(
        "Todos os Santos",
        Rule::ExceptYears {
            base: &Rule::Fixed { month: 11, day: 1 },
            years: &[2013, 2014, 2015],
        },
    ),
    h(
        "Restauração da Independência",
        Rule::ExceptYears {
            base: &Rule::Fixed { month: 12, day: 1 },
            years: &[2013, 2014, 2015],
        },
    ),
    h("Imaculada Conceição", Rule::Fixed { month: 12, day: 8 }),
    h("Natal", Rule::Fixed { month: 12, day: 25 }),
];

/// Belgian national holidays. Belgium has no weekend substitution: a holiday
/// that falls on a Sunday is simply lost.
static BELGIUM: &[Holiday] = &[
    h("Nieuwjaar", Rule::Fixed { month: 1, day: 1 }),
    h("Paasmaandag", Rule::EasterOffset(1)),
    h("Dag van de Arbeid", Rule::Fixed { month: 5, day: 1 }),
    h("Onze-Lieve-Heer-Hemelvaart", Rule::EasterOffset(39)),
    h("Pinkstermaandag", Rule::EasterOffset(50)),
    h("Nationale feestdag", Rule::Fixed { month: 7, day: 21 }),
    h(
        "Onze-Lieve-Vrouw-Hemelvaart",
        Rule::Fixed { month: 8, day: 15 },
    ),
    h("Allerheiligen", Rule::Fixed { month: 11, day: 1 }),
    h("Wapenstilstand", Rule::Fixed { month: 11, day: 11 }),
    h("Kerstmis", Rule::Fixed { month: 12, day: 25 }),
];

/// Austrian national holidays. Saint Leopold's Day is a holiday in only two
/// states, so it is not included.
static AUSTRIA: &[Holiday] = &[
    h("Neujahr", Rule::Fixed { month: 1, day: 1 }),
    h("Heilige Drei Könige", Rule::Fixed { month: 1, day: 6 }),
    h("Ostermontag", Rule::EasterOffset(1)),
    h("Staatsfeiertag", Rule::Fixed { month: 5, day: 1 }),
    h("Christi Himmelfahrt", Rule::EasterOffset(39)),
    h("Pfingstmontag", Rule::EasterOffset(50)),
    h("Fronleichnam", Rule::EasterOffset(60)),
    h("Mariä Himmelfahrt", Rule::Fixed { month: 8, day: 15 }),
    h("Nationalfeiertag", Rule::Fixed { month: 10, day: 26 }),
    h("Allerheiligen", Rule::Fixed { month: 11, day: 1 }),
    h("Mariä Empfängnis", Rule::Fixed { month: 12, day: 8 }),
    h("Christtag", Rule::Fixed { month: 12, day: 25 }),
    h("Stefanitag", Rule::Fixed { month: 12, day: 26 }),
];

/// Polish national holidays. Note that 1 January, 6 January, 1 May, 3 May and
/// the Christmas days are also statutory days free of work, which the
/// working-day queries already treat as non-working.
static POLAND: &[Holiday] = &[
    h("Nowy Rok", Rule::Fixed { month: 1, day: 1 }),
    h(
        "Święto Trzech Króli",
        Rule::Since {
            year: 2011,
            base: &Rule::Fixed { month: 1, day: 6 },
        },
    ),
    h("Wielkanoc", Rule::EasterOffset(0)),
    h("Poniedziałek Wielkanocny", Rule::EasterOffset(1)),
    h("Święto Pracy", Rule::Fixed { month: 5, day: 1 }),
    h(
        "Święto Konstytucji Trzeciego Maja",
        Rule::Fixed { month: 5, day: 3 },
    ),
    h("Zielone Świątki", Rule::EasterOffset(49)),
    h("Boże Ciało", Rule::EasterOffset(60)),
    h(
        "Wniebowzięcie Najświętszej Maryi Panny",
        Rule::Fixed { month: 8, day: 15 },
    ),
    h("Wszystkich Świętych", Rule::Fixed { month: 11, day: 1 }),
    h(
        "Narodowe Święto Niepodległości",
        Rule::Fixed { month: 11, day: 11 },
    ),
    h("Boże Narodzenie", Rule::Fixed { month: 12, day: 25 }),
    h(
        "Drugi dzień Bożego Narodzenia",
        Rule::Fixed { month: 12, day: 26 },
    ),
    h(
        "Wigilia Bożego Narodzenia",
        Rule::Since {
            year: 2025,
            base: &Rule::Fixed { month: 12, day: 24 },
        },
    ),
    h(
        "Narodowe Święto Niepodległości - 100-lecie",
        Rule::Once {
            year: 2018,
            month: 11,
            day: 12,
        },
    ),
];

/// Swedish public holidays. Midsummer Day and All Saints' Day are the
/// Saturdays that follow the fixed dates.
static SWEDEN: &[Holiday] = &[
    h("Nyårsdagen", Rule::Fixed { month: 1, day: 1 }),
    h("Trettondedag jul", Rule::Fixed { month: 1, day: 6 }),
    h("Långfredagen", Rule::EasterOffset(-2)),
    h("Påskdagen", Rule::EasterOffset(0)),
    h("Annandag påsk", Rule::EasterOffset(1)),
    h("Första maj", Rule::Fixed { month: 5, day: 1 }),
    h("Kristi himmelsfärdsdag", Rule::EasterOffset(39)),
    h("Pingstdagen", Rule::EasterOffset(49)),
    h(
        "Nationaldagen",
        Rule::Since {
            year: 2005,
            base: &Rule::Fixed { month: 6, day: 6 },
        },
    ),
    // Whit Monday was a public holiday until it was replaced by National Day.
    h(
        "Annandag pingst",
        Rule::Until {
            year: 2004,
            base: &Rule::EasterOffset(50),
        },
    ),
    h(
        "Midsommardagen",
        Rule::WeekdayOnOrAfter {
            month: 6,
            day: 20,
            weekday: 6,
        },
    ),
    h(
        "Alla helgons dag",
        Rule::WeekdayOnOrAfter {
            month: 10,
            day: 31,
            weekday: 6,
        },
    ),
    h("Juldagen", Rule::Fixed { month: 12, day: 25 }),
    h("Annandag jul", Rule::Fixed { month: 12, day: 26 }),
];

/// Norwegian public holidays.
static NORWAY: &[Holiday] = &[
    h("Første nyttårsdag", Rule::Fixed { month: 1, day: 1 }),
    h("Skjærtorsdag", Rule::EasterOffset(-3)),
    h("Langfredag", Rule::EasterOffset(-2)),
    h("Første påskedag", Rule::EasterOffset(0)),
    h("Andre påskedag", Rule::EasterOffset(1)),
    h("Arbeidernes dag", Rule::Fixed { month: 5, day: 1 }),
    h("Grunnlovsdag", Rule::Fixed { month: 5, day: 17 }),
    h("Kristi himmelfartsdag", Rule::EasterOffset(39)),
    h("Første pinsedag", Rule::EasterOffset(49)),
    h("Andre pinsedag", Rule::EasterOffset(50)),
    h("Første juledag", Rule::Fixed { month: 12, day: 25 }),
    h("Andre juledag", Rule::Fixed { month: 12, day: 26 }),
];

/// Danish public holidays. Great Prayer Day was abolished after 2023, so it is
/// kept until then and resolves to nothing from 2024 on.
static DENMARK: &[Holiday] = &[
    h("Nytårsdag", Rule::Fixed { month: 1, day: 1 }),
    h("Skærtorsdag", Rule::EasterOffset(-3)),
    h("Langfredag", Rule::EasterOffset(-2)),
    h("Påskedag", Rule::EasterOffset(0)),
    h("Anden påskedag", Rule::EasterOffset(1)),
    h(
        "Store bededag",
        Rule::Until {
            year: 2023,
            base: &Rule::EasterOffset(26),
        },
    ),
    h("Kristi himmelfartsdag", Rule::EasterOffset(39)),
    h("Pinsedag", Rule::EasterOffset(49)),
    h("Anden pinsedag", Rule::EasterOffset(50)),
    h("Juledag", Rule::Fixed { month: 12, day: 25 }),
    h("Anden juledag", Rule::Fixed { month: 12, day: 26 }),
];

/// Finnish public holidays.
///
/// Midsummer Eve and Christmas Eve are de facto public holidays: they are not
/// in the Act on Public Holidays but every collective agreement treats them as
/// days off, which is how the reference package models them too.
static FINLAND: &[Holiday] = &[
    h("Uudenvuodenpäivä", Rule::Fixed { month: 1, day: 1 }),
    h("Loppiainen", Rule::Fixed { month: 1, day: 6 }),
    h("Pitkäperjantai", Rule::EasterOffset(-2)),
    h("Pääsiäispäivä", Rule::EasterOffset(0)),
    h("Toinen pääsiäispäivä", Rule::EasterOffset(1)),
    h("Vappu", Rule::Fixed { month: 5, day: 1 }),
    h("Helatorstai", Rule::EasterOffset(39)),
    h("Helluntaipäivä", Rule::EasterOffset(49)),
    h(
        "Juhannusaatto",
        Rule::WeekdayOnOrAfter {
            month: 6,
            day: 19,
            weekday: 5,
        },
    ),
    h(
        "Juhannuspäivä",
        Rule::WeekdayOnOrAfter {
            month: 6,
            day: 20,
            weekday: 6,
        },
    ),
    h(
        "Pyhäinpäivä",
        Rule::WeekdayOnOrAfter {
            month: 10,
            day: 31,
            weekday: 6,
        },
    ),
    h("Itsenäisyyspäivä", Rule::Fixed { month: 12, day: 6 }),
    h("Jouluaatto", Rule::Fixed { month: 12, day: 24 }),
    h("Joulupäivä", Rule::Fixed { month: 12, day: 25 }),
    h("Tapaninpäivä", Rule::Fixed { month: 12, day: 26 }),
];

/// Irish public holidays. Saint Brigid's Day is 1 February when that is a
/// Friday and the first Monday in February otherwise; it was introduced in
/// 2023, so it resolves to nothing before then.
static IRELAND: &[Holiday] = &[
    h("New Year's Day", Rule::Fixed { month: 1, day: 1 }),
    h(
        "Saint Brigid's Day",
        Rule::Since {
            year: 2023,
            base: &Rule::FixedOnWeekdayElseWeekdayOnOrAfter {
                month: 2,
                day: 1,
                on_weekday: 5,
                weekday: 1,
            },
        },
    ),
    h("Saint Patrick's Day", Rule::Fixed { month: 3, day: 17 }),
    h("Easter Monday", Rule::EasterOffset(1)),
    h(
        "May Day",
        Rule::NthWeekday {
            month: 5,
            weekday: 1,
            nth: 1,
        },
    ),
    h(
        "June Bank Holiday",
        Rule::NthWeekday {
            month: 6,
            weekday: 1,
            nth: 1,
        },
    ),
    h(
        "August Bank Holiday",
        Rule::NthWeekday {
            month: 8,
            weekday: 1,
            nth: 1,
        },
    ),
    h(
        "October Bank Holiday",
        Rule::NthWeekday {
            month: 10,
            weekday: 1,
            nth: 5,
        },
    ),
    h("Christmas Day", Rule::Fixed { month: 12, day: 25 }),
    h("Saint Stephen's Day", Rule::Fixed { month: 12, day: 26 }),
    // One-off public holidays declared by the Oireachtas, transcribed from the
    // reference package rather than derived from a rule.
    h(
        "Millennium Celebrations",
        Rule::Once {
            year: 1999,
            month: 12,
            day: 31,
        },
    ),
    h(
        "National Day of Mourning",
        Rule::Once {
            year: 2011,
            month: 9,
            day: 14,
        },
    ),
    h(
        "Day of Remembrance and Recognition",
        Rule::Once {
            year: 2022,
            month: 3,
            day: 18,
        },
    ),
];

/// Canadian federal holidays. Provincial holidays such as Family Day and
/// Saint-Jean-Baptiste Day are not included. Remembrance Day is a federal
/// holiday but not a day off in every province.
static CANADA: &[Holiday] = &[
    h("New Year's Day", Rule::Fixed { month: 1, day: 1 }),
    h("Good Friday", Rule::EasterOffset(-2)),
    h(
        "Victoria Day",
        Rule::WeekdayOnOrBefore {
            month: 5,
            day: 24,
            weekday: 1,
        },
    ),
    h("Canada Day", Rule::Fixed { month: 7, day: 1 }),
    h(
        "Labour Day",
        Rule::NthWeekday {
            month: 9,
            weekday: 1,
            nth: 1,
        },
    ),
    h(
        "National Day for Truth and Reconciliation",
        Rule::Since {
            year: 2021,
            base: &Rule::Fixed { month: 9, day: 30 },
        },
    ),
    h(
        "Thanksgiving",
        Rule::NthWeekday {
            month: 10,
            weekday: 1,
            nth: 2,
        },
    ),
    h("Remembrance Day", Rule::Fixed { month: 11, day: 11 }),
    h("Christmas Day", Rule::Fixed { month: 12, day: 25 }),
    h("Boxing Day", Rule::Fixed { month: 12, day: 26 }),
];

/// New Zealand public holidays. Regional anniversary days are not included.
/// Matariki follows an official calendar set by the Matariki Advisory Group
/// rather than a formula, so it is transcribed into [`MATARIKI_DATES`] instead
/// of being computed; a formula would be a guess that looks precise.
static NEW_ZEALAND: &[Holiday] = &[
    h("New Year's Day", Rule::Fixed { month: 1, day: 1 }),
    h("Day after New Year's Day", Rule::Fixed { month: 1, day: 2 }),
    h(
        "Waitangi Day",
        Rule::ObservedFrom {
            year: 2014,
            shared: false,
            base: &Rule::Fixed { month: 2, day: 6 },
        },
    ),
    h("Good Friday", Rule::EasterOffset(-2)),
    h("Easter Monday", Rule::EasterOffset(1)),
    h(
        "ANZAC Day",
        Rule::ObservedFrom {
            year: 2014,
            shared: true,
            base: &Rule::Fixed { month: 4, day: 25 },
        },
    ),
    h(
        "King's Birthday",
        Rule::NthWeekday {
            month: 6,
            weekday: 1,
            nth: 1,
        },
    ),
    h(
        "Matariki",
        Rule::Table {
            dates: MATARIKI_DATES,
        },
    ),
    h(
        "Labour Day",
        Rule::NthWeekday {
            month: 10,
            weekday: 1,
            nth: 4,
        },
    ),
    h(
        "Queen Elizabeth II Memorial Day",
        Rule::Once {
            year: 2022,
            month: 9,
            day: 26,
        },
    ),
    h("Christmas Day", Rule::Fixed { month: 12, day: 25 }),
    h("Boxing Day", Rule::Fixed { month: 12, day: 26 }),
];

/// The official Matariki dates, 2022 to 2052, as published by the New Zealand
/// government. Matariki is a public holiday from 2022; the dates are fixed by
/// statute from the advisory group's calendar, which is why they are a table.
static MATARIKI_DATES: &[(i32, u8, u8)] = &[
    (2022, 6, 24),
    (2023, 7, 14),
    (2024, 6, 28),
    (2025, 6, 20),
    (2026, 7, 10),
    (2027, 6, 25),
    (2028, 7, 14),
    (2029, 7, 6),
    (2030, 6, 21),
    (2031, 7, 11),
    (2032, 7, 2),
    (2033, 6, 24),
    (2034, 7, 7),
    (2035, 6, 29),
    (2036, 7, 18),
    (2037, 7, 10),
    (2038, 6, 25),
    (2039, 7, 15),
    (2040, 7, 6),
    (2041, 7, 19),
    (2042, 7, 11),
    (2043, 7, 3),
    (2044, 6, 24),
    (2045, 7, 7),
    (2046, 6, 29),
    (2047, 7, 19),
    (2048, 7, 3),
    (2049, 6, 25),
    (2050, 7, 15),
    (2051, 6, 30),
    (2052, 6, 21),
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
        // 25 December 2022 was a Sunday and 26 December a Monday, so Boxing
        // Day keeps the Monday and Christmas Day is observed on the Tuesday.
        let names: Vec<&str> = cal
            .holidays_in(2022)
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        assert!(names.contains(&"Christmas Day"));
        assert!(names.contains(&"Boxing Day"));
        assert_eq!(cal.holiday_name(d(2022, 12, 26)), Some("Boxing Day"));
        assert_eq!(cal.holiday_name(d(2022, 12, 27)), Some("Christmas Day"));
    }

    #[test]
    fn united_kingdom_easter_monday() {
        let cal = Calendar::united_kingdom();
        // Easter 2026 is 5 April, so Easter Monday is 6 April.
        assert_eq!(cal.holiday_name(d(2026, 4, 6)), Some("Easter Monday"));
    }

    /// The full England-and-Wales bank-holiday list, transcribed from
    /// `python-holidays` with `subdiv="England"`. The crate reports the date a
    /// holiday is *observed*, so a bank holiday that falls at a weekend appears
    /// on its substitute date and not on the raw one; the reference lists both,
    /// and only the observed set is compared here. The whole set is checked,
    /// not a spot check, because the substitute dates are exactly what a bug in
    /// the roll-forward logic moves.
    #[test]
    fn united_kingdom_bank_holidays_match_the_reference() {
        let cal = Calendar::united_kingdom();
        let expected: &[(i32, u8, u8, &str)] = &[
            // 2020: the early May holiday was moved to 8 May for VE Day, and
            // Boxing Day (a Saturday) rolls to Monday the 28th.
            (2020, 1, 1, "New Year's Day"),
            (2020, 4, 10, "Good Friday"),
            (2020, 4, 13, "Easter Monday"),
            (2020, 5, 8, "Early May Bank Holiday (VE Day)"),
            (2020, 5, 25, "Spring Bank Holiday"),
            (2020, 8, 31, "Summer Bank Holiday"),
            (2020, 12, 25, "Christmas Day"),
            (2020, 12, 28, "Boxing Day"),
            // 2021: Christmas is a Saturday and Boxing Day a Sunday, so they
            // roll to the 27th and the 28th.
            (2021, 1, 1, "New Year's Day"),
            (2021, 4, 2, "Good Friday"),
            (2021, 4, 5, "Easter Monday"),
            (2021, 5, 3, "Early May Bank Holiday"),
            (2021, 5, 31, "Spring Bank Holiday"),
            (2021, 8, 30, "Summer Bank Holiday"),
            (2021, 12, 27, "Christmas Day"),
            (2021, 12, 28, "Boxing Day"),
            // 2022: New Year's Day is a Saturday; the spring bank holiday moved
            // to 2 June for the Platinum Jubilee, which is why both appear.
            // Christmas is a Sunday and Boxing Day the Monday, so Boxing Day
            // keeps the 26th and Christmas Day is observed on the 27th.
            (2022, 1, 3, "New Year's Day"),
            (2022, 4, 15, "Good Friday"),
            (2022, 4, 18, "Easter Monday"),
            (2022, 5, 2, "Early May Bank Holiday"),
            (2022, 6, 2, "Spring Bank Holiday (Platinum Jubilee)"),
            (2022, 6, 3, "Platinum Jubilee of Elizabeth II"),
            (2022, 8, 29, "Summer Bank Holiday"),
            (2022, 9, 19, "State Funeral of Queen Elizabeth II"),
            (2022, 12, 26, "Boxing Day"),
            (2022, 12, 27, "Christmas Day"),
            // 2026: Boxing Day is a Saturday and rolls to the 28th.
            (2026, 1, 1, "New Year's Day"),
            (2026, 4, 3, "Good Friday"),
            (2026, 4, 6, "Easter Monday"),
            (2026, 5, 4, "Early May Bank Holiday"),
            (2026, 5, 25, "Spring Bank Holiday"),
            (2026, 8, 31, "Summer Bank Holiday"),
            (2026, 12, 25, "Christmas Day"),
            (2026, 12, 28, "Boxing Day"),
        ];
        for year in [2020, 2021, 2022, 2026] {
            let got: Vec<(i32, u8, u8, &str)> = cal
                .holidays_in(year)
                .into_iter()
                .map(|(date, name)| (date.year, date.month, date.day, name))
                .collect();
            let want: Vec<(i32, u8, u8, &str)> = expected
                .iter()
                .copied()
                .filter(|(y, _, _, _)| *y == year)
                .collect();
            assert_eq!(got, want, "United Kingdom {year}");
        }
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
    fn netherlands_kings_day_moves_off_sunday() {
        let cal = Calendar::netherlands();
        // 27 April 2025 is a Sunday, so King's Day is the 26th.
        assert_eq!(cal.holiday_name(d(2025, 4, 26)), Some("Koningsdag"));
        assert_eq!(cal.holiday_name(d(2025, 4, 27)), None);
        // 27 April 2026 is a Monday and keeps the date.
        assert_eq!(cal.holiday_name(d(2026, 4, 27)), Some("Koningsdag"));
    }

    #[test]
    fn netherlands_pentecost_days() {
        let cal = Calendar::netherlands();
        // Easter 2026 is 5 April, so Whitsunday is 24 May and Whit Monday the 25th.
        assert_eq!(cal.holiday_name(d(2026, 5, 24)), Some("Eerste Pinksterdag"));
        assert_eq!(cal.holiday_name(d(2026, 5, 25)), Some("Tweede Pinksterdag"));
    }

    #[test]
    fn spain_national_day_and_constitution_day() {
        let cal = Calendar::spain();
        assert!(cal.is_holiday(d(2026, 10, 12)));
        assert!(cal.is_holiday(d(2026, 12, 6)));
        assert!(cal.is_holiday(d(2026, 1, 6)));
    }

    #[test]
    fn italy_republic_day_and_saint_stephen() {
        let cal = Calendar::italy();
        assert!(cal.is_holiday(d(2026, 6, 2)));
        assert!(cal.is_holiday(d(2026, 12, 26)));
        // Easter Monday 2026 is 6 April.
        assert_eq!(cal.holiday_name(d(2026, 4, 6)), Some("Lunedì dell'Angelo"));
    }

    #[test]
    fn portugal_corpus_christi_and_restoration() {
        let cal = Calendar::portugal();
        // Corpus Christi is 60 days after Easter: 4 June 2026.
        assert_eq!(cal.holiday_name(d(2026, 6, 4)), Some("Corpo de Deus"));
        assert!(cal.is_holiday(d(2026, 12, 1)));
        assert!(cal.is_holiday(d(2026, 10, 5)));
    }

    #[test]
    fn belgium_national_day_and_no_substitution() {
        let cal = Calendar::belgium();
        assert!(cal.is_holiday(d(2026, 7, 21)));
        // Belgium keeps a Sunday holiday on the Sunday and does not roll it
        // forward, so the Monday is an ordinary working day.
        assert_eq!(cal.holiday_name(d(2026, 11, 1)), Some("Allerheiligen"));
        assert!(cal.is_working_day(d(2026, 11, 2)));
    }

    #[test]
    fn austria_national_day() {
        let cal = Calendar::austria();
        assert!(cal.is_holiday(d(2026, 10, 26)));
        // Corpus Christi 2026 is 4 June.
        assert!(cal.is_holiday(d(2026, 6, 4)));
        assert!(cal.is_holiday(d(2026, 1, 6)));
    }

    #[test]
    fn poland_constitution_day() {
        let cal = Calendar::poland();
        assert!(cal.is_holiday(d(2026, 5, 3)));
        assert!(cal.is_holiday(d(2026, 11, 11)));
        assert!(cal.is_holiday(d(2026, 1, 6)));
    }

    #[test]
    fn sweden_midsummer_is_the_following_saturday() {
        let cal = Calendar::sweden();
        // Midsummer Day is the Saturday between 20 and 26 June.
        assert_eq!(cal.holiday_name(d(2026, 6, 20)), Some("Midsommardagen"));
        assert_eq!(cal.holiday_name(d(2027, 6, 26)), Some("Midsommardagen"));
        assert!(cal.is_holiday(d(2026, 6, 6)));
    }

    #[test]
    fn norway_constitution_day_and_maundy_thursday() {
        let cal = Calendar::norway();
        assert!(cal.is_holiday(d(2026, 5, 17)));
        // Maundy Thursday is three days before Easter: 2 April 2026.
        assert_eq!(cal.holiday_name(d(2026, 4, 2)), Some("Skjærtorsdag"));
    }

    #[test]
    fn denmark_has_no_great_prayer_day_after_2024() {
        let cal = Calendar::denmark();
        // Great Prayer Day was abolished from 2024; Easter 2026 + 26 is 1 May.
        assert_eq!(cal.holiday_name(d(2026, 5, 1)), None);
        assert!(cal.is_holiday(d(2026, 12, 26)));
    }

    #[test]
    fn finland_independence_day_and_midsummer() {
        let cal = Calendar::finland();
        assert!(cal.is_holiday(d(2026, 12, 6)));
        assert_eq!(cal.holiday_name(d(2026, 6, 20)), Some("Juhannuspäivä"));
        // All Saints' Day is the Saturday between 31 October and 6 November.
        assert_eq!(cal.holiday_name(d(2026, 10, 31)), Some("Pyhäinpäivä"));
    }

    #[test]
    fn ireland_saint_brigid_day() {
        let cal = Calendar::ireland();
        // 1 February 2026 is a Sunday, so the first Monday, the 2nd, is used.
        assert_eq!(cal.holiday_name(d(2026, 2, 2)), Some("Saint Brigid's Day"));
        assert!(cal.is_holiday(d(2026, 3, 17)));
    }

    #[test]
    fn ireland_saint_patrick_moves_to_monday() {
        let cal = Calendar::ireland();
        // 17 March 2024 was a Sunday, so the bank holiday was Monday the 18th.
        assert_eq!(
            cal.holiday_name(d(2024, 3, 18)),
            Some("Saint Patrick's Day")
        );
        assert_eq!(cal.holiday_name(d(2024, 3, 17)), None);
    }

    #[test]
    fn canada_victoria_day_is_the_monday_on_or_before_the_24th() {
        let cal = Calendar::canada();
        assert_eq!(cal.holiday_name(d(2024, 5, 20)), Some("Victoria Day"));
        assert_eq!(cal.holiday_name(d(2026, 5, 18)), Some("Victoria Day"));
        // 24 May 2027 is a Monday, so it is Victoria Day itself.
        assert_eq!(cal.holiday_name(d(2027, 5, 24)), Some("Victoria Day"));
    }

    #[test]
    fn canada_day_moves_to_monday() {
        let cal = Calendar::canada();
        // 1 July 2023 was a Saturday, observed on Monday the 3rd.
        assert_eq!(cal.holiday_name(d(2023, 7, 3)), Some("Canada Day"));
        assert!(cal.is_holiday(d(2026, 9, 30)));
    }

    #[test]
    fn new_zealand_labour_day_is_the_fourth_monday() {
        let cal = Calendar::new_zealand();
        assert_eq!(cal.holiday_name(d(2026, 10, 26)), Some("Labour Day"));
        assert_eq!(cal.holiday_name(d(2025, 10, 27)), Some("Labour Day"));
        // King's Birthday is the first Monday in June.
        assert_eq!(cal.holiday_name(d(2026, 6, 1)), Some("King's Birthday"));
    }

    #[test]
    fn new_zealand_new_year_substitution() {
        let cal = Calendar::new_zealand();
        // 1 January 2022 was a Saturday and 2 January a Sunday, so the observed
        // days were Monday the 3rd and Tuesday the 4th.
        assert_eq!(cal.holiday_name(d(2022, 1, 3)), Some("New Year's Day"));
        assert_eq!(
            cal.holiday_name(d(2022, 1, 4)),
            Some("Day after New Year's Day")
        );
    }

    #[test]
    fn every_country_resolves_a_full_year_without_panicking() {
        // Catches a rule that resolves to nothing, which would silently drop a
        // holiday from the calendar rather than fail loudly. A rule that is
        // deliberately tied to a year (`Once`, `ExceptYears`) is expected to
        // resolve to nothing outside it, and is checked in its own test.
        for &country in Country::ALL {
            let cal = Calendar::new(country);
            for year in [2024, 2025, 2026, 2027] {
                for holiday in country.holidays() {
                    let resolves = holiday.rule.resolve(year).is_some();
                    if !holiday.rule.is_year_restricted() {
                        assert!(
                            resolves,
                            "{}: {} does not resolve in {year}",
                            country.name(),
                            holiday.name
                        );
                    }
                }
                let holidays = cal.holidays_in(year);
                assert!(
                    !holidays.is_empty(),
                    "{} produced no holidays in {year}",
                    country.name()
                );
                assert!(
                    holidays.iter().all(|(date, _)| date.year == year),
                    "{} produced an out-of-year date in {year}",
                    country.name()
                );
                let mut unique = holidays.clone();
                unique.dedup();
                assert_eq!(
                    unique.len(),
                    holidays.len(),
                    "{} observed two holidays on one date in {year}",
                    country.name()
                );
            }
        }
    }

    #[test]
    fn each_country_observes_every_rule_that_applies_in_that_year() {
        // No holiday may silently vanish. A rule that resolves in a year must
        // appear in that year's list or, when a substitution pushes it over the
        // year boundary, in the next year's; a rule tied to particular years
        // contributes only in those years.
        for &country in Country::ALL {
            let cal = Calendar::new(country);
            for year in [2024, 2025, 2026, 2027] {
                let names: Vec<&str> = cal
                    .holidays_in(year)
                    .into_iter()
                    .chain(cal.holidays_in(year + 1))
                    .map(|(_, name)| name)
                    .collect();
                for holiday in country.holidays() {
                    if holiday.rule.resolve(year).is_some() {
                        assert!(
                            names.contains(&holiday.name),
                            "{} lost {} in {year}",
                            country.name(),
                            holiday.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn country_metadata_covers_every_variant() {
        for &country in Country::ALL {
            assert!(!country.name().is_empty());
            assert!(!country.holidays().is_empty());
        }
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
