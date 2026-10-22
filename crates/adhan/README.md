# adhan

[![crates.io](https://img.shields.io/crates/v/adhan.svg)](https://crates.io/crates/adhan)
[![docs.rs](https://docs.rs/adhan/badge.svg)](https://docs.rs/adhan)

Prayer times from a date and coordinates.

Given a Gregorian date, a latitude and a longitude, this computes Fajr, sunrise,
Dhuhr, Asr, Maghrib and Isha. It is the arithmetic that every prayer app, mosque
schedule and Ramadan calendar needs, and it sits next to `hijridate` and `arabic`
in this workspace.

```rust
use adhan::{Madhab, Method, Parameters, prayer_times};

let times = prayer_times(
    2026, 10, 21,          // the date
    21.4225, 39.8262,      // Makkah
    &Parameters::for_method(Method::MuslimWorldLeague).with_madhab(Madhab::Shafi),
);

let (hour, minute, _second) = times.dhuhr.unwrap();
println!("Dhuhr at {hour:02}:{minute:02} UTC");
```

## What it does, and what it leaves to the caller

The times are in UTC. A local clock is a timezone question, and this crate does
not guess one from a longitude: a place can be an hour or more from its solar
time, sometimes for political reasons. The caller adds the offset, which keeps a
wrong local time the caller's decision rather than a surprise.

The six times are returned as whole hours, minutes and seconds. The seconds are
there because `Rounding::None` keeps them and the reference does the same; under
the default `Rounding::Nearest` the second is always zero. Nothing here formats
them,
reads a clock, or knows about time zones; that is the caller's business.

## The two choices that actually matter

Fajr and Isha are defined by how far below the horizon the sun is, and the angle
is a matter of convention rather than of astronomy. The crate ships the usual
authorities:

| Method | Fajr angle | Isha |
|---|---|---|
| Muslim World League | 18 | 17 degrees |
| Egyptian General Authority | 19.5 | 17.5 degrees |
| Karachi | 18 | 18 degrees |
| Umm al-Qura, Makkah | 18.5 | 90 minutes after sunset |
| North America (ISNA) | 15 | 15 degrees |
| Tehran | 17.7 | 14 degrees, with a Maghrib angle |
| and Dubai, Kuwait, Qatar, Singapore, Turkey, Moonsighting Committee | | |

Asr is defined by the length of a shadow, and here the schools differ. The
majority take one shadow length plus the noon shadow; the Hanafi school takes
two. Both are there as `Madhab::Shafi` and `Madhab::Hanafi`.

## High latitudes, where the honest answer is that there is no time

Inside the polar circles the sun may not set in summer or rise in winter. Then
sunrise, sunset and the twilight times do not exist, and the reference
implementation returns nothing for them rather than a number. So does this one:

```rust
use adhan::{HighLatitudeRule, Method, Parameters, prayer_times};

// Tromso in June: the sun does not set.
let times = prayer_times(
    2026, 6, 21, 69.6492, 18.9553,
    &Parameters::for_method(Method::MuslimWorldLeague)
        .with_high_latitude_rule(HighLatitudeRule::SeventhOfTheNight),
);
assert!(times.sunrise.is_none());
assert!(times.dhuhr.is_some(), "the times that do exist are still given");
```

Where a time has to be produced anyway, the caller picks a `HighLatitudeRule`:
half the night, a seventh of the night, or a fraction given by the twilight
angle. The Moonsighting Committee method also has its own seasonal twilight
curve, which is implemented as it defines it.

## How it is checked

The results are compared against the `adhan` JavaScript library, the
implementation most prayer apps use, over a grid of dates, locations, methods,
schools and high latitude rules: **37,440 times, all matching**.

On top of that grid there is a corpus of **6,000 random cases**: latitudes
weighted to the polar circles, longitudes near the date line, all thirteen
methods, both schools, all three high latitude rules, all three rounding modes,
and manual adjustments from five minutes before to five minutes after. It is
generated deterministically, so it can be rebuilt exactly; the generator is in
`scripts/fuzz/adhan/`.

That corpus found two real gaps the grid had missed, because the grid used no
adjustments and only the default rounding:

* The default is `Rounding::Nearest`, which zeroes the seconds, so a whole-minute
  result looked correct. `Rounding::None` keeps the second the reference's own
  arithmetic produces, and the times now carry seconds so it can be reported.
* The minutes a method applies on its own are separate from the minutes the
  caller adds, and the reference adds them together. Folding the two into one
  field made "two minutes on top of the method's five" impossible to express.

One more: at high latitude with `Rounding::None`, a Fajr that falls before
midnight UTC came out a second late, because truncating toward zero is not the
same as flooring when the reading is negative.

That comparison also settled two things worth recording. The `adhan` Python port
is wrong for eastern longitudes, giving Jakarta's Dhuhr about fourteen hours off,
so the JavaScript library is the reference rather than the Python one. And in the
polar cases the reference returns nothing rather than a number, which is why the
API here returns `Option`.

Given a Gregorian date, a latitude and a longitude, this computes Fajr,
sunrise, Dhuhr, Asr, Maghrib and Isha. It is the sort of arithmetic that
every prayer app, mosque schedule and Ramadan calendar needs, and that
`hijridate` and `arabic` in this workspace sit next to.

## Example

```
use adhan::{Madhab, Method, Parameters, prayer_times};

// Makkah, 21 October 2026, the Muslim World League method. Makkah is at
// UTC+3, and choosing a location east of Greenwich keeps every time inside
// the UTC day, which makes the ordering below easy to read.
let parameters = Parameters::for_method(Method::MuslimWorldLeague)
    .with_madhab(Madhab::Shafi);
let times = prayer_times(2026, 10, 21, 21.4225, 39.8262, &parameters);

// The result is in UTC. Add three hours for the local clock.
let (hour, _minute, _second) = times.dhuhr.unwrap();
assert_eq!((hour + 3) % 24, 12, "local Dhuhr is a little after noon");
assert!(times.fajr < times.sunrise);
assert!(times.sunrise < times.dhuhr);
assert!(times.dhuhr < times.asr);
assert!(times.asr < times.maghrib);
assert!(times.maghrib < times.isha);
```

## What it does, and what it leaves to the caller

The times are in UTC. A local clock is a timezone question, and this crate
does not guess one from a longitude: a place can be an hour or more from its
solar time, sometimes for political reasons. The caller adds the offset,
which keeps a wrong local time the caller's decision rather than a surprise.

The angle conventions come from the authorities that publish them, and the
astronomy follows Meeus. The results are checked against the `adhan`
JavaScript library, which is the implementation most prayer apps use, across
thousands of date, location, method and school combinations.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
