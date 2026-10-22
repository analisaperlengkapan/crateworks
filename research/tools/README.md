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
| `round8.py` | Round 8: the PyPI top list against crates.io. Prints the tables in `../rust-crate-gaps-round8.md`. Treats a 404 as `FREE` and a rate limit as an error, so the two are never confused |
| `gen_hijri_table.py` | Regenerate the Umm al-Qura table and test fixtures for the `hijridate` crate. Needs `pip install hijridate`. Run from the repository root: `python3 research/tools/gen_hijri_table.py crates/hijridate` |
| `namecheck.py` | **Run this before writing code**: is the crate name still free? `python3 namecheck.py iso20022 xmlkit` |
| `stale.py` | Round 12: **the abandoned incumbent, measured**. Pages the crates.io ranked list by recent downloads and reads each crate's newest live version from the sparse index, so "popular but nobody is maintaining it" becomes a scan rather than an anecdote. A yank is not a release, so a pulled version does not count as activity. `--github` adds an archived-repository check (needs `GITHUB_TOKEN`). Caches index reads under `.cache/` |

## Operational warnings

- **Do not run two scripts at once.** crates.io rate-limits requests; running them
  in parallel triggers HTTP 429 and produces false empty results — a gap that
  looks real but is not.
- **`revdeps.py` runs its whole scan on import**, which is exactly the trap: a
  script that does `import revdeps` to reuse `revdeps()` also fires 33 crates.io
  requests, gets rate-limited, and then every later search in that script returns
  empty. A capability that is actually served then reads as a gap. Round 5 lost a
  run to this and nearly recorded "Hashids: nothing found". It now has a
  `if __name__ == "__main__"` guard, so importing it is safe; `probe.py`,
  `incumbents.py`, `verify2.py` and `gen_hijri_table.py` do not, so never import
  those — run them, or copy the helper out.
- **Treat any "nothing found" as a rate limit until it is re-checked.** A second
  run a minute later is cheap; a wrong gap is not.
- The npm bulk endpoint rejects a mix of scoped and unscoped names (HTTP 400).
  Keep the two apart, at most 32 per call.
- `revdeps.py` is the true measure of Rust demand. PyPI and npm download figures
  measure demand in Python and JavaScript, not in Rust.
- **Run `namecheck.py` before writing code, not after.** A crate name cannot be
  changed after publication. `iso20022` looked like an obvious gap until it turned
  out the name was taken by a crate dead since 2020.
