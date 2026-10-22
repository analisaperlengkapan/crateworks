# hijridate

[![crates.io](https://img.shields.io/crates/v/hijridate.svg)](https://crates.io/crates/hijridate)
[![docs.rs](https://docs.rs/hijridate/badge.svg)](https://docs.rs/hijridate)

Hijri (Islamic) and Gregorian date conversion, based on the Umm al-Qura
calendar. No dependencies.

```rust
use hijridate::{Gregorian, Hijri};

// 1 Ramadan 1445 fell on 11 March 2024.
let h = Hijri::new(1445, 9, 1).unwrap();
assert_eq!(h.to_gregorian().to_string(), "2024-03-11");
assert_eq!(h.month_name(), "Ramadan");

let g = Gregorian::new(2024, 3, 11).unwrap();
assert_eq!(g.to_hijri().to_string(), "1445-09-01");
```

The Hijri year is about eleven days shorter than the Gregorian year and a month
begins at the crescent sighting, so the calendar cannot be computed from a
formula. Umm al-Qura is the Saudi sighting-based calendar that most software
treats as the reference. This crate carries its published month starts for Hijri
years 1343 to 1500 (Gregorian 1924 to 2077), so conversions in that range are
exact, and `Hijri::month_length` returns the real 29 or 30 day month rather than
a guess.

Outside that range the crate falls back to the classical arithmetic calendar and
sets `Hijri::is_tabular`, which can be a day or two off. Treat those dates as a
hint, not a fact.

## Regional differences

Umm al-Qura is one authority among several. Indonesia (Kemenag), Malaysia and
many communities announce their own start of the month, so a date from this crate
can differ by a day from what a country actually observes. That is not a bug to
fix, it is a fact about the calendar; expect to override the date when a local
authority publishes its own.

## Data provenance

The month-start table and the test fixtures are generated from the
[`hijridate`](https://github.com/dralshehri/hijridate) Python package by Mohammed
Alshehri, which is MIT licensed and reproduces the Umm al-Qura calendar as
published by the Saudi authorities. See [`NOTICE`](https://github.com/cipansor/crateworks/blob/main/crates/hijridate/NOTICE).

The Hijri year is about eleven days shorter than the Gregorian year, and a
Hijri month begins when the crescent is sighted, so the calendar cannot be
derived from a formula the way the Gregorian one can. Umm al-Qura is the
calendar Saudi Arabia publishes from actual sighting data, and it is what
most software uses as the reference.

## Scope

The table covers Hijri years 1343 to 1500 (Gregorian 1924-08-01 to
2077-11-16). Outside that range conversion falls back to a tabular
arithmetic approximation, which can be a day or two out; see
`Hijri::is_tabular` for how to tell.

## Regional differences

Umm al-Qura is one authority among several. Indonesia (Kemenag), Malaysia
and many communities determine the start of a month by their own sighting,
so a date here can differ by a day from the date a country observes. For
example 1 Shawwal 1445 is 2024-04-10 in Umm al-Qura, while Indonesia
observed Eid al-Fitr on 2024-04-10 as well, but this agreement is not
guaranteed for every year. Use `Hijri::month_name` and this table to get
the reference date, and expect to override it when a local authority
announces something different.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
