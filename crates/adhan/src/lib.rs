#![doc = include_str!("../README.md")]
mod astronomy;
mod calculation;

pub use calculation::{
    HighLatitudeRule, Madhab, Method, Parameters, Prayer, PrayerTimes, Rounding, SunnahTimes,
    prayer_times, qibla, sunnah_times,
};
