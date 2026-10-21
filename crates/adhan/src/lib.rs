//! Prayer times from a date and coordinates.
//!
//! Given a Gregorian date, a latitude and a longitude, this computes Fajr,
//! sunrise, Dhuhr, Asr, Maghrib and Isha. It is the sort of arithmetic that
//! every prayer app, mosque schedule and Ramadan calendar needs, and that
//! `hijridate` and `arabic` in this workspace sit next to.
//!
//! # Example
//!
//! ```
//! use adhan::{Madhab, Method, Parameters, prayer_times};
//!
//! // Makkah, 21 October 2026, the Muslim World League method. Makkah is at
//! // UTC+3, and choosing a location east of Greenwich keeps every time inside
//! // the UTC day, which makes the ordering below easy to read.
//! let parameters = Parameters::for_method(Method::MuslimWorldLeague)
//!     .with_madhab(Madhab::Shafi);
//! let times = prayer_times(2026, 10, 21, 21.4225, 39.8262, &parameters);
//!
//! // The result is in UTC. Add three hours for the local clock.
//! let (hour, _minute, _second) = times.dhuhr.unwrap();
//! assert_eq!((hour + 3) % 24, 12, "local Dhuhr is a little after noon");
//! assert!(times.fajr < times.sunrise);
//! assert!(times.sunrise < times.dhuhr);
//! assert!(times.dhuhr < times.asr);
//! assert!(times.asr < times.maghrib);
//! assert!(times.maghrib < times.isha);
//! ```
//!
//! # What it does, and what it leaves to the caller
//!
//! The times are in UTC. A local clock is a timezone question, and this crate
//! does not guess one from a longitude: a place can be an hour or more from its
//! solar time, sometimes for political reasons. The caller adds the offset,
//! which keeps a wrong local time the caller's decision rather than a surprise.
//!
//! The angle conventions come from the authorities that publish them, and the
//! astronomy follows Meeus. The results are checked against the `adhan`
//! JavaScript library, which is the implementation most prayer apps use, across
//! thousands of date, location, method and school combinations.

mod astronomy;
mod calculation;

pub use calculation::{
    HighLatitudeRule, Madhab, Method, Parameters, PrayerTimes, Rounding, prayer_times,
};
