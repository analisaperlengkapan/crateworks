#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod ummalqura;

use std::fmt;

pub use ummalqura::{FIRST_HIJRI_YEAR, LAST_HIJRI_YEAR};

/// Errors returned when a date is outside the calendar's rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The Hijri year is not a plausible value.
    HijriYear(i32),
    /// The Hijri month was not in 1..=12.
    HijriMonth(u8),
    /// The Hijri day was not in 1..=30, or not valid for that month.
    HijriDay {
        /// The month the day was checked against.
        month: u8,
        /// The day that was rejected.
        day: u8,
    },
    /// The Gregorian date does not exist, such as 31 February.
    GregorianDate {
        /// Year as given.
        year: i32,
        /// Month as given.
        month: u8,
        /// Day as given.
        day: u8,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HijriYear(y) => write!(f, "Hijri year {y} is out of range"),
            Self::HijriMonth(m) => write!(f, "Hijri month {m} is not in 1..=12"),
            Self::HijriDay { month, day } => {
                write!(f, "Hijri day {day} is not valid in month {month}")
            }
            Self::GregorianDate { year, month, day } => {
                write!(
                    f,
                    "Gregorian date {year}-{month:02}-{day:02} does not exist"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

/// A date in the Hijri calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hijri {
    year: i32,
    month: u8,
    day: u8,
    tabular: bool,
}

/// A date in the proleptic Gregorian calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Gregorian {
    year: i32,
    month: u8,
    day: u8,
}

/// The names of the twelve Hijri months.
const MONTH_NAMES: [&str; 12] = [
    "Muharram",
    "Safar",
    "Rabi' al-Awwal",
    "Rabi' al-Thani",
    "Jumada al-Ula",
    "Jumada al-Akhirah",
    "Rajab",
    "Sha'ban",
    "Ramadan",
    "Shawwal",
    "Dhu al-Qa'dah",
    "Dhu al-Hijjah",
];

/// The names of the days of the week, Monday first.
const DAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

impl Hijri {
    /// Builds a Hijri date, checking the day against the month's real length.
    ///
    /// # Errors
    ///
    /// Returns [`Error::HijriMonth`] when `month` is not in 1..=12, and
    /// [`Error::HijriDay`] when `day` is not a valid day of that month.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self> {
        if !(1..=12).contains(&month) {
            return Err(Error::HijriMonth(month));
        }
        let length = month_length(year, month);
        if day == 0 || day > length {
            return Err(Error::HijriDay { month, day });
        }
        Ok(Self {
            year,
            month,
            day,
            tabular: !in_table(year),
        })
    }

    /// The Hijri year.
    #[must_use]
    pub fn year(self) -> i32 {
        self.year
    }

    /// The Hijri month, 1 for Muharram through 12 for Dhu al-Hijjah.
    #[must_use]
    pub fn month(self) -> u8 {
        self.month
    }

    /// The day of the month.
    #[must_use]
    pub fn day(self) -> u8 {
        self.day
    }

    /// Whether this date is outside the Umm al-Qura table and therefore
    /// converted with the arithmetic approximation.
    ///
    /// A tabular date can be a day or two away from what an authority would
    /// publish, so treat it as a hint rather than a fact.
    #[must_use]
    pub fn is_tabular(self) -> bool {
        self.tabular
    }

    /// The English name of the month, such as `"Ramadan"`.
    #[must_use]
    pub fn month_name(self) -> &'static str {
        MONTH_NAMES[usize::from(self.month) - 1]
    }

    /// The number of days in this month.
    #[must_use]
    pub fn month_length(self) -> u8 {
        month_length(self.year, self.month)
    }

    /// The English name of the weekday.
    #[must_use]
    pub fn day_name(self) -> &'static str {
        DAY_NAMES[self.weekday()]
    }

    /// The weekday, Monday as 0 through Sunday as 6.
    #[must_use]
    pub fn weekday(self) -> usize {
        // A Julian day number mod 7 gives the weekday with Monday as 0:
        // JDN 2451545 is 2000-01-01, a Saturday, and 2451545 % 7 is 5.
        self.to_julian().rem_euclid(7) as usize
    }

    /// The corresponding Gregorian date.
    ///
    /// # Example
    ///
    /// ```
    /// use hijridate::Hijri;
    ///
    /// // 1 Ramadan 1447 falls in February 2026.
    /// let hijri = Hijri::new(1447, 9, 1).unwrap();
    /// let gregorian = hijri.to_gregorian();
    /// assert_eq!(gregorian.year(), 2026);
    /// ```
    #[must_use]
    pub fn to_gregorian(self) -> Gregorian {
        from_julian(self.to_julian())
    }

    /// The Julian day number of this date.
    #[must_use]
    pub fn to_julian(self) -> i64 {
        if in_table(self.year) {
            i64::from(ummalqura::MONTH_STARTS[month_index(self.year, self.month)])
                + i64::from(ummalqura::JDN_BASE)
                + i64::from(self.day)
                - 1
        } else {
            tabular_julian(self.year, self.month, self.day)
        }
    }
}

impl fmt::Display for Hijri {
    /// Formats as `YYYY-MM-DD`, zero padded.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl Gregorian {
    /// Builds a Gregorian date, rejecting days that do not exist.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GregorianDate`] for a month outside 1..=12 or a day
    /// outside the length of that month.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self> {
        if !(1..=12).contains(&month) || day == 0 || day > gregorian_month_length(year, month) {
            return Err(Error::GregorianDate { year, month, day });
        }
        Ok(Self { year, month, day })
    }

    /// The Gregorian year.
    #[must_use]
    pub fn year(self) -> i32 {
        self.year
    }

    /// The Gregorian month, 1 through 12.
    #[must_use]
    pub fn month(self) -> u8 {
        self.month
    }

    /// The day of the month.
    #[must_use]
    pub fn day(self) -> u8 {
        self.day
    }

    /// The corresponding Hijri date.
    #[must_use]
    pub fn to_hijri(self) -> Hijri {
        let jdn = to_julian(self.year, self.month, self.day);
        // Search the table, then fall back to the approximation outside it.
        if let Some(h) = hijri_from_table(jdn) {
            return h;
        }
        let (year, month, day) = tabular_hijri(jdn);
        Hijri {
            year,
            month,
            day,
            tabular: true,
        }
    }

    /// The weekday, Monday as 0 through Sunday as 6.
    #[must_use]
    pub fn weekday(self) -> usize {
        to_julian(self.year, self.month, self.day).rem_euclid(7) as usize
    }

    /// The English name of the weekday.
    #[must_use]
    pub fn day_name(self) -> &'static str {
        DAY_NAMES[self.weekday()]
    }
}

impl fmt::Display for Gregorian {
    /// Formats as `YYYY-MM-DD`, zero padded.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Whether a Hijri year is covered by the Umm al-Qura table.
fn in_table(year: i32) -> bool {
    (ummalqura::FIRST_HIJRI_YEAR..=ummalqura::LAST_HIJRI_YEAR).contains(&year)
}

/// The index into [`ummalqura::MONTH_STARTS`] for a Hijri year and month.
fn month_index(year: i32, month: u8) -> usize {
    let months = (year - ummalqura::FIRST_HIJRI_YEAR) * 12 + i32::from(month) - 1;
    months as usize
}

/// The length of a Hijri month in days.
///
/// Inside the table this is the gap between two published month starts, which
/// is 29 or 30 depending on the year. Outside it, the approximation always
/// gives 29 or 30 on the same alternating pattern.
fn month_length(year: i32, month: u8) -> u8 {
    if in_table(year) {
        let start = ummalqura::MONTH_STARTS[month_index(year, month)];
        let next = ummalqura::MONTH_STARTS[month_index(year, month) + 1];
        (next - start) as u8
    } else if month % 2 == 1 {
        30
    } else {
        29
    }
}

/// Converts a Julian day number to a Gregorian date.
fn from_julian(jdn: i64) -> Gregorian {
    // Fliegel-Van Flandern, valid for the proleptic Gregorian calendar.
    let l = jdn + 68569;
    let n = 4 * l / 146_097;
    let l = l - (146_097 * n + 3) / 4;
    let i = 4000 * (l + 1) / 1_461_001;
    let l = l - 1461 * i / 4 + 31;
    let j = 80 * l / 2447;
    let day = (l - 2447 * j / 80) as u8;
    let l = j / 11;
    let month = (j + 2 - 12 * l) as u8;
    let year = (100 * (n - 49) + i + l) as i32;
    Gregorian { year, month, day }
}

/// Converts a Gregorian date to a Julian day number.
fn to_julian(year: i32, month: u8, day: u8) -> i64 {
    let a = i64::from(year);
    let m = i64::from(month);
    let d = i64::from(day);
    let a = if m <= 2 { a - 1 } else { a };
    let era = a.div_euclid(400);
    let yoe = a - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    // The sum is days since 1970-01-01, which is JDN 2440588.
    era * 146_097 + doe - 719_468 + 2_440_588
}

/// Whether a Gregorian year is a leap year.
fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// The number of days in a Gregorian month.
fn gregorian_month_length(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Finds the Hijri date for a Julian day number inside the table, if it is there.
fn hijri_from_table(jdn: i64) -> Option<Hijri> {
    let offset = jdn - i64::from(ummalqura::JDN_BASE);
    if offset < 0 {
        return None;
    }
    let offset = u16::try_from(offset).ok()?;
    // Binary search for the last month whose start is <= this day.
    let idx = ummalqura::MONTH_STARTS.partition_point(|&start| start <= offset);
    if idx == 0 {
        return None;
    }
    let idx = idx - 1;
    let start = ummalqura::MONTH_STARTS[idx];
    let day = u8::try_from(offset - start).ok()? + 1;
    let months = i32::try_from(idx).ok()?;
    let year = ummalqura::FIRST_HIJRI_YEAR + months / 12;
    let month = u8::try_from(months % 12).ok()? + 1;
    if !in_table(year) {
        return None;
    }
    Some(Hijri {
        year,
        month,
        day,
        tabular: false,
    })
}

/// The arithmetic (tabular) Hijri date for a Julian day number.
///
/// This is the classical 30-year cycle with 11 leap years, which is what the
/// fallback uses outside the Umm al-Qura table. It is an approximation.
fn tabular_hijri(jdn: i64) -> (i32, u8, u8) {
    // Epoch: 1 Muharram 1 AH is 16 July 622 CE (Julian), JDN 1948440.
    const EPOCH: i64 = 1_948_440;
    let days = jdn - EPOCH;
    let cycle = days.div_euclid(10_631);
    let mut rem = days.rem_euclid(10_631);
    let mut year_in_cycle = 0i64;
    let mut year_len = 354;
    while rem >= year_len {
        rem -= year_len;
        year_in_cycle += 1;
        year_len = if is_tabular_leap(year_in_cycle) {
            355
        } else {
            354
        };
    }
    let year = (30 * cycle + year_in_cycle + 1) as i32;
    let mut month = 1u8;
    let mut remaining = rem;
    loop {
        let len = i64::from(if month % 2 == 1 { 30u8 } else { 29u8 });
        if remaining < len {
            break;
        }
        remaining -= len;
        month += 1;
    }
    (year, month, remaining as u8 + 1)
}

/// Whether a year within the 30-year tabular cycle is a leap year.
fn is_tabular_leap(year_in_cycle: i64) -> bool {
    matches!(
        year_in_cycle,
        2 | 5 | 7 | 10 | 13 | 16 | 18 | 21 | 24 | 26 | 29
    )
}

/// The Julian day number of a tabular Hijri date.
fn tabular_julian(year: i32, month: u8, day: u8) -> i64 {
    const EPOCH: i64 = 1_948_440;
    let y = i64::from(year) - 1;
    let cycles = y.div_euclid(30);
    let year_in_cycle = y.rem_euclid(30);
    let mut days = cycles * 10_631;
    for i in 0..year_in_cycle {
        days += if is_tabular_leap(i) { 355 } else { 354 };
    }
    for m in 1..month {
        days += i64::from(if m % 2 == 1 { 30u8 } else { 29u8 });
    }
    EPOCH + days + i64::from(day) - 1
}
