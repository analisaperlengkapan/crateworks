# holidays-rs

Public holiday calendars and working-day arithmetic.

```rust
use holidays_rs::{Calendar, Date};

let cal = Calendar::indonesia();
let d = Date::new(2026, 8, 17).unwrap();
assert!(cal.is_holiday(d));
assert_eq!(cal.next_working_day(d), Date::new(2026, 8, 18).unwrap());
```

Holidays are rules, not tables, so a calendar stays correct for any year.
Country-specific weekend substitution is handled, including the United States
rule where a Saturday holiday moves back to Friday and the United Kingdom rule
where a weekend holiday moves forward to Monday.

Covered: Indonesia, United States, United Kingdom, Germany, France.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
