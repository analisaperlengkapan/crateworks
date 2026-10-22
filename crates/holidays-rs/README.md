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

Calendars cover *national* holidays, including one-off national bank holidays
(`Rule::Once`, such as the United Kingdom's 2022 Platinum Jubilee) and a holiday
that a one-off date replaced (`Rule::ExceptYears`, such as the early May bank
holiday moved to 8 May 2020 for VE Day). Regional observances are out of scope;
add them with `Calendar::with_extra` when needed.

`holidays_in` reports the date a holiday is *observed*, not the date it falls.
A bank holiday that lands at a weekend therefore appears on its substitute date
and not on the raw one, which is the date a payroll or a calendar grid needs.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
