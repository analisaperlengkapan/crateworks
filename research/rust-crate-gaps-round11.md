# Gap scan round 11: the demand signal from inside Rust, and the identity-code space (2026-10-21)

Rounds 1-10 scanned from the outside in: npm, PyPI, then curated lists of
standards. Round 10 closed with "the npm and PyPI top lists are both served; a
gap is a standard a regulated industry is obliged to implement".

This round changes the direction of the signal. Instead of asking what is
popular elsewhere, it asks **what Rust users themselves have written down that
they want and cannot get**. Every figure below was fetched on 2026-10-21.

## Method

1. **Demand signal from the Rust side.** Three sources no earlier round used:
   - `not-yet-awesome-rust` (NYAR), the community's own gap list and issue
     tracker, 1,419 stars.
   - Exact-name availability on crates.io (`404` means free).
   - `recent_downloads` and `reverse_dependencies` of the incumbent.
2. A gap is real when the Rust side is empty or has under about 20,000 recent
   downloads with near-zero reverse dependencies, while a source ecosystem
   shows real volume.

## The Rust-side signal has gone quiet

NYAR is the obvious place to look, and it no longer carries information.

| Source | What it holds today | Verdict |
| --- | --- | --- |
| `not-yet-awesome-rust` README list | 115 bullet entries, last meaningfully updated years ago; several are struck through as closed | Stale. Most entries now have crates (`vobject`, `bevy`, `godot-rust`, `miri` are all cited *in the list itself*). |
| NYAR open issues | **27 open**, of which only 4 carry more than 2 reactions. Top item is "hostapd and wpa_supplicant re-implementations" at 6. | Too little volume to rank candidates. |

A list whose most-voted open request has six reactions is not a demand signal.
Recorded so it is not scanned again: **NYAR is not a useful source.**

## The identity-code space: measured, and it closed recently

The strongest pattern from rounds 9-10 was "a code an industry exchanges every
day". This round tested the largest remaining family in that shape, the
identity and document codes, and found the window has just been taken.

| Code | npm / PyPI demand | crates.io | Verdict |
| --- | --- | --- | --- |
| **ICAO 9303 MRZ** | npm `mrz` **265,353/month**, PyPI `mrz` 52,660/month, `passporteye` 29,122/month | `mrz` **6,466** recent, v0.9.1 (2026-09-29) — TD1/TD2/TD3 + MRV-A/MRV-B, parser, emitter, check digits | **Served.** A complete zero-dependency crate landed last month. Do not build. |
| **AAMVA DL/ID** | npm `aamva-parser` **73,179/month** | `aamva` 181 recent v0.3.5, `aamva-parser-rs` 7 | Served, thinly. |
| **ISO/IEC 18013-5 mDL** | growing (EU wallet mandate) | `mdl-verify` v0.0.0 only | Too early to call a gap; watch. |
| SEDOL / LEI | large | `sedol` 41,956; `lei` 42,516 | Served. |
| NHS number | `nhs-number-validator` 7,880/month | `nhs-number` 11,553 | Served. |
| VIN | `vin-validator` 12,945/month | `vin-info` 2,990, `vin_parser` 191 | Served, thinly. |
| NPI / DEA / DUNS / CAGE / IMO | small | near-empty, and the demand is small too | Too small to carry a crate. |
| GS1 (GTIN/SSCC/AI) | `bwip-js` 4.5M/month (generation) | `barcoders` 163,842, `bwipp-rs` 1,088 | Served. |

The MRZ row is the one worth keeping. `mrz` at v0.9.1 with six minor versions in
the last year is a live, maintained crate; building a second one now would be
repeating work, not filling a gap. **MRZ is withdrawn as a candidate.**

## What the rounds now agree on

Across eleven rounds the same shape keeps appearing, and round 11 adds a
fourth confirmation:

1. **Popularity lists are fully served** (rounds 6, 7, 8, 10). Do not scan them.
2. **A popular code with an empty registry is a window, not a gap** — it closes
   within months. `iso6346` (round 10) and MRZ (this round) are the two ends of
   that: one was still open, the other had just been taken.
3. **The Rust-side signal is not a shortcut.** NYAR is stale, and the issue
   tracker that replaced it has too little traffic to rank anything.
4. **The largest measured opening is still a ratio, not a hole:**
   `date-holidays` at **3,945,731 npm downloads/month** against the Rust leader
   at **20,781** recent. Nothing else measured in eleven rounds has a ratio that
   large against a served-but-thin incumbent.

## A fourth shape: the abandoned incumbent

Rounds 1-10 only ever asked "is the registry empty?". This round asked the
complementary question: **where is the incumbent present but dead?** That is a
different and, for a builder, a better opening — the users, the name recognition
and the reverse dependencies already exist, and nobody is maintaining them.

The signal is a crate with real recent downloads whose last release is years old.
Measured today:

| Crate | recent downloads | last release | state |
| --- | --- | --- | --- |
| `fuzzy-matcher` | **9,428,270** | 2020-10-04 | repo **archived**; 295 stars, 5 open issues frozen |
| `sublime_fuzzy` | 697,946 | 2020-12-19 | dormant |
| `fst` | 8,068,887 | 2021-06-06 | dormant |
| `genpdf` | 133,101 | 2021-06-17 | dormant |
| `simple_excel_writer` | 25,814 | 2022-03-11 | dormant |
| `holidays` | 7,838 | 2023-02-02 | v0.1.0, effectively a stub |
| `tesseract` | 53,111 | 2025-04-19 | dormant, but the OCR demand is small |

Two are worth separating from the rest.

**`fuzzy-matcher` is the strongest abandoned incumbent measured in any round.**
Nine and a half million recent downloads, a released version from 2020, and the
repository archived — so no bug report or pull request can land. `strsim`
(253M recent) covers the *edit-distance* half of fuzzy matching and is healthy,
and `nucleo` (706K) covers interactive filtering for editors, so the space is not
empty. What is missing is a maintained scorer for the `fuzzy-matcher` use case:
rank a list of candidates against a query. A new crate here inherits a large,
unserved audience. This is the one candidate round 11 found that is not a
standard and not a data set.

**`holidays` (the crate) is the ratio, and it is the one this workspace can take.**
At 7,838 recent downloads and v0.1.0 it is barely a crate, while the npm
counterpart `date-holidays` does **3,945,731 downloads/month**. The workspace's
own `holidays-rs` is already at 0.1.1 and covers 19 countries. The work here is
not a new crate; it is closing a 500:1 ratio country by country.

## What this round concludes

1. **Do not build an MRZ crate.** The space went from empty to served between
   round 10 and today.
2. **NYAR and its tracker are not worth scanning again**, and that is a result:
   it means the only reliable demand signals remain npm/PyPI volume plus the
   crates.io reverse-dependency count.
3. **A new angle is worth keeping: the abandoned incumbent.** `fuzzy-matcher`
   (9.4M recent, archived repo, 2020) is the clearest example found so far of a
   large audience with nobody maintaining it, and it is a general capability
   rather than a standard or a data set.
4. **`holidays` remains the best fit for this workspace.** Round 10 said the
   same; round 11 measured the incumbent directly and found it thinner than
   round 10 assumed — v0.1.0, not a maintained competitor. `holidays-rs` growing
   country by country is still the highest-ratio work available, and it is
   already started.
