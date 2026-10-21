"""Generate the Umm al-Qura table and test fixtures for the hijridate crate.

Requires the hijridate Python package:  pip install hijridate
Then from the repo root:                python3 research/tools/gen_hijri_table.py crates/hijridate

The generated files carry the data provenance in their headers; do not edit them
by hand. The values are published calendar observations, so a diff means either
a new upstream release or a mistake, never a stylistic change.

Source of truth: hijridate (MIT, (c) Mohammed Alshehri), which reproduces the
Umm al-Qura calendar as published by the Saudi authorities. Run from a checkout
of the crate root:  python3 gen.py <crate_dir>
"""
import sys, pathlib
from hijridate import ummalqura as u
from hijridate import Gregorian, Hijri

MS = u.MONTH_STARTS
RJD_TO_JDN = 2_400_000
BASE = MS[0] + RJD_TO_JDN
assert all(0 <= v - MS[0] <= 0xFFFF for v in MS), "does not fit u16 offsets"
assert len(MS) == 158 * 12 + 1, len(MS)
Y0, Y1 = u.HIJRI_RANGE[0][0], u.HIJRI_RANGE[1][0]
assert Y1 - Y0 + 1 == 158, (Y0, Y1)

crate = pathlib.Path(sys.argv[1])

# ---- table ----
rows = []
for i in range(0, len(MS), 12):
    rows.append("    " + " ".join(f"{v - MS[0]}," for v in MS[i:i + 12]))
table = "\n".join(rows)
out = f"""//! The Umm al-Qura month-start table.
//!
//! Generated from the `hijridate` Python package (MIT, (c) Mohammed Alshehri),
//! which reproduces the Umm al-Qura calendar published by the Saudi authorities.
//! Regenerate with `research/tools/gen_hijri_table.py` rather than editing by
//! hand: every value is a published observation, not something to guess at.
//!
//! Each entry is the Julian day number of the first day of a Hijri month,
//! stored as an offset from [`JDN_BASE`] so the whole table fits in
//! {len(MS)} x 2 bytes.

/// The lowest Julian day number in the table.
pub const JDN_BASE: u32 = {BASE};

/// The first Hijri year covered by [`MONTH_STARTS`].
pub const FIRST_HIJRI_YEAR: i32 = {Y0};

/// The last Hijri year covered by [`MONTH_STARTS`].
pub const LAST_HIJRI_YEAR: i32 = {Y1};

/// First day of each Hijri month from {Y0}-01 to {Y1}-12, plus one extra entry
/// so the length of the final month can be read without a bounds check.
///
/// Index a month with `(year - FIRST_HIJRI_YEAR) * 12 + (month - 1)`.
pub const MONTH_STARTS: [u16; {len(MS)}] = [
{table}
];
"""
(crate / "src" / "ummalqura.rs").write_text(out)
print("wrote src/ummalqura.rs", len(out), "bytes")

# ---- fixtures: every 1 Muharram, plus a spread of other dates ----
pairs = []
for y in range(Y0, Y1 + 1):
    for m in (1, 9, 12):
        h = Hijri(y, m, 1)
        pairs.append((y, m, 1, h.to_gregorian().isoformat()))
# month lengths for every month of a few years + all years sampled sparsely
lengths = []
for y in range(Y0, Y1 + 1):
    for m in range(1, 13):
        lengths.append((y, m, Hijri(y, m, 1).month_length()))
print("pairs", len(pairs), "lengths", len(lengths))

# round-trip gregorian->hijri samples (first of each gregorian year)
greg = []
for gy in range(u.GREGORIAN_RANGE[0][0] + 1, u.GREGORIAN_RANGE[1][0]):
    for gm, gd in ((1, 1), (6, 15), (12, 31)):
        try:
            g = Gregorian(gy, gm, gd)
            greg.append((gy, gm, gd, g.to_hijri().isoformat()))
        except Exception:
            pass
print("greg samples", len(greg))

def emit_pairs(name, items):
    body = "\n".join(f"    ({a}, {b}, {c}, \"{d}\")," for a, b, c, d in items)
    return f"pub const {name}: &[(i32, u8, u8, &str)] = &[\n{body}\n];\n"

fx = """//! Fixtures transcribed from the `hijridate` Python package (MIT,
//! (c) Mohammed Alshehri). Regenerate with `research/tools/gen_hijri_table.py`.

/// `(hijri year, hijri month, hijri day, expected Gregorian YYYY-MM-DD)`.
"""
fx += emit_pairs("HIJRI_TO_GREGORIAN", pairs)
fx += "\n/// `(hijri year, hijri month, length in days)`.\n"
fx += "pub const MONTH_LENGTHS: &[(i32, u8, u8)] = &[\n"
fx += "\n".join(f"    ({y}, {m}, {l})," for y, m, l in lengths)
fx += "\n];\n"
fx += "\n/// `(gregorian year, month, day, expected Hijri YYYY-MM-DD)`.\n"
fx += emit_pairs("GREGORIAN_TO_HIJRI", greg)
(crate / "tests" / "fixtures.rs").write_text(fx)
print("wrote tests/fixtures.rs")
