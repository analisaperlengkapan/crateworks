//! Prints every holiday in 2026 for every supported country, one per line.
//!
//! Run with `cargo run --example dump -p holidays-rs`.

use holidays_rs::{Calendar, Country};
fn main() {
    for &c in Country::ALL {
        let cal = Calendar::new(c);
        for (d, n) in cal.holidays_in(2026) {
            println!(
                "{}\t{:04}-{:02}-{:02}\t{}",
                c.name(),
                d.year,
                d.month,
                d.day,
                n
            );
        }
    }
}
