#![doc = include_str!("../README.md")]
mod astronomy;
mod calculation;

pub use calculation::{
    HighLatitudeRule, Madhab, Method, Parameters, PrayerTimes, Rounding, prayer_times,
};
