# hijridate

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
published by the Saudi authorities. See [`NOTICE`](NOTICE).

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
