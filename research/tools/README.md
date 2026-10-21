# Tools: Rust crate gap research

The scripts used for the research in `../rust-crate-5-final.md`. All use public
APIs without a key. They need Python 3 and no external dependencies.

| Script | Purpose |
|---|---|
| `crates.py` | crates.io API wrapper (cache + retry). Usable directly: `python3 crates.py semver glob` |
| `npmfetch.py` | Fetch weekly downloads and metadata from npm. Batching is safe for scoped names |
| `demand.py` | npm and PyPI demand figures for a curated list of candidates |
| `pipeline.py` | The `pypi`, `crates-npm` and `crates-pypi` stages |
| `probe.py` | Probe ~263 curated capabilities against crates.io |
| `revdeps.py` | **The most important one**: reverse dependencies and incumbent repo activity |
| `incumbents.py` | Competitor detail for each candidate |
| `verify2.py` | Check 30 further areas (transit, health, finance, and so on) |
| `gen_hijri_table.py` | Regenerate the Umm al-Qura table and test fixtures for the `hijridate` crate. Needs `pip install hijridate`. Run from the repository root: `python3 research/tools/gen_hijri_table.py crates/hijridate` |
| `namecheck.py` | **Run this before writing code**: is the crate name still free? `python3 namecheck.py iso20022 xmlkit` |

## Operational warnings

- **Do not run two scripts at once.** crates.io rate-limits requests; running them
  in parallel triggers HTTP 429 and produces false empty results — a gap that
  looks real but is not.
- The npm bulk endpoint rejects a mix of scoped and unscoped names (HTTP 400).
  Keep the two apart, at most 32 per call.
- `revdeps.py` is the true measure of Rust demand. PyPI and npm download figures
  measure demand in Python and JavaScript, not in Rust.
- **Run `namecheck.py` before writing code, not after.** A crate name cannot be
  changed after publication. `iso20022` looked like an obvious gap until it turned
  out the name was taken by a crate dead since 2020.
