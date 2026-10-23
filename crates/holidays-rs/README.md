# holidays-rs

[![crates.io](https://img.shields.io/crates/v/holidays-rs.svg)](https://crates.io/crates/holidays-rs)
[![docs.rs](https://docs.rs/holidays-rs/badge.svg)](https://docs.rs/holidays-rs)

Public holiday calendars and working-day arithmetic.

```rust
use holidays_rs::{Calendar, Date};

let cal = Calendar::indonesia();
let d = Date::new(2026, 8, 17).unwrap();
assert!(cal.is_holiday(d));
assert_eq!(cal.next_working_day(d), Date::new(2026, 8, 18).unwrap());
```

Holidays are rules, not tables, so a calendar stays correct for any year.
Country-specific weekend substitution is `SubstituteStyle`, applied per country
when a year is resolved: the United States moves a Saturday holiday back to
Friday and the United Kingdom moves a weekend holiday forward to Monday.

Covered: Indonesia, United States, United Kingdom, Germany, France,
Netherlands, Spain, Italy, Portugal, Belgium, Austria, Poland, Sweden, Norway,
Denmark, Finland, Ireland, Canada and New Zealand. `Country::ALL` lists them.

The crate answers three questions:

1. Which dates are public holidays in a given country and year?
2. Is a particular date a working day?
3. What is the *n*-th working day from a given date?

## Scope

Calendars cover *national* holidays. A holiday is not always in force for every
year, so `Rule` can bound it: `Rule::Since` and `Rule::Until` for a holiday
introduced or abolished in a given year (Poland's Epiphany from 2011, Sweden's
Whit Monday until 2004), `Rule::Between` for a window (Italy's Republic Day,
movable from 1977 to 2000), `Rule::ExceptYears` for years that are skipped
(France's Whit Monday as a solidarity day 2005-2007), `Rule::Once` for a one-off
(New Zealand's 2022 Queen Elizabeth II Memorial Day), `Rule::EveryNYears` for a
recurring interval (the Netherlands' Liberation Day every five years),
`Rule::ObservedFrom` for a holiday that gained a substitute day late (New
Zealand's Mondayised Waitangi Day and Anzac Day from 2014) and `Rule::Table` for
dates that are published rather than computed (Matariki). Regional observances
are out of scope; add them with `Calendar::with_extra` when needed.

`holidays_in` reports the date a holiday is *observed*, not the date it falls.
A bank holiday that lands at a weekend therefore appears on its substitute date
and not on the raw one, which is the date a payroll or a calendar grid needs.

## Verification

Every country table is checked against the Python `holidays` package over
2000-2050 by `research/tools/verify_holidays.py`, which exits non-zero on any
difference that is not listed as deliberate. The deliberate differences are
Indonesia (Islamic and Chinese observances are set by annual decree and are
available through `Calendar::with_extra`), Spain (a national holiday that falls
on a Sunday is kept rather than dropped), Ireland (Saint Stephen's Day is
observed on the substitute Monday) and Canada (the federal-wide set is used).
Run it from the repository root:

```sh
python3 research/tools/verify_holidays.py
```

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
