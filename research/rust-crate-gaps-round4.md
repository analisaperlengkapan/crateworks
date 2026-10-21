# Rust package gaps: re-verification and a new scan (2026-10-21)

A continuation of `rust-crate-5-final.md`. Every figure below was **measured
today** through the crates.io, npmjs.org and PyPI APIs — not recalled. Tooling:
`research/tools/`.

## Method

Three figures, not one:

1. **Demand in the source ecosystem** (npm/week or PyPI/month). This shows the
   problem is real and that many people look for it. It is **not** evidence that
   anyone looks for it in Rust.
2. **`recent_downloads` of the Rust incumbent.** If an incumbent has tens of
   millions of downloads, the gap is already closed — however good the idea is.
3. **`reverse_dependencies` of the incumbent.** This is the real Rust signal. 40
   million npm downloads a week in JavaScript means nothing if zero Rust crates
   depend on the counterpart.

Plus one quality check: **is the incumbent repo still alive.** A dead incumbent
with large demand is the best opportunity — the demand is proven and nobody is
serving it.

### Tooling correction

The npm bulk endpoint (`/downloads/point/last-week/a,b,c`) **silently returns 0**
if the batch is too large or contains scoped names. My first check reported
`object-hash = 0/week`, which was wrong; the figure is 40.75 million. Fetch per
package, or use small batches without scoped names.

## Scan results

130+ categories were checked. Most **are already well served** — worth knowing so
as not to build what exists:

| Category | Rust crate | recent | revdeps | Verdict |
|---|---|---|---|---|
| JSON Schema | `jsonschema` | 31.7M | 1569 | served |
| Cron | `cron` | 9.3M | 463 | served |
| Phone (libphonenumber) | `phonenumber` | 3.2M | 65 | served |
| IBAN | `iban` | 121K | 2 | **stale** |
| vCard | `calcard` | 32K | 10 | served |
| RTF | `rtf-parser` | 70K | 5 | served |
| Hashids | `hashids-rust` | 59 | 0 | served but thin |
| Canonical JSON (JCS) | `serde_jcs` | 3.8M | 157 | served |
| Human durations | `humantime` | 70.1M | 1359 | served |
| User-agent | `uaparser` | 2.9M | 7 | served |
| Prayer times | `salah` | 851 | 4 | **fragmented and thin** |

## The 3 candidates that survived

### 1. `object-hash` — deterministic structural hashing  ⭐ strongest

- **Use:** put a stable fingerprint on a structured value (a nested object,
  dict or struct), not on a string. Used for cache keys, ETags, content dedup and
  content addressing. The problem has nothing to do with the language: the same
  value has to produce the same hash whatever the field order and numeric types.
- **Demand evidence:** npm `object-hash` **40.75 million/week**. Around it:
  `fast-json-stable-stringify` 96.8M, `safe-stable-stringify` 39.1M,
  `canonicalize` 3.0M.
- **Gap evidence in Rust:** `objecthash` (the cryptosphere port) **exists** but
  has been **dead since 2017-01, 58 recent downloads, 0 revdeps**. The name
  `object-hash` is **free** on crates.io. Searches for "content addressable
  hash", "structural hash" and "deep hash" turn up nothing alive.
- **What exists and is not a gap:** JCS (RFC 8785) **is already served** —
  `serde_jcs` 3.8M recent / 157 revdeps, `serde_json_canonicalizer` 9.4M / 139.
  So do not build "canonical JSON" on its own; it exists and is healthy. What does
  not exist is **hashing an arbitrary value across encodings** (JSON, CBOR,
  struct), which is the object-hash use case.
- **Difficulty:** low to medium. The core algorithm is normalise value → canonical
  encoding → digest. The hard part is type equality (1 vs 1.0 vs "1", map order,
  -0.0, NaN), and that is precisely the part that can be tested hard.
- **References:** `object-hash` (npm, MIT), RFC 8785 (JCS),
  `cryptosphere/rust-objecthash`.
- **Honest risk:** zero revdeps for a dead incumbent means Rust demand is **not
  yet proven**. This is both the widest gap and the least proven — the same
  caution as the SOAP note in the earlier research.

### 2. `adhan` — prayer times  ⭐ the best fit for this workspace

- **Use:** compute prayer times from coordinates, a date and a madhab. Used by
  mosque apps, imsakiyah schedules and time reminders.
- **Demand evidence:** npm `adhan` 36.1K/week; `moment-hijri` 14.3K. PyPI `adhan`
  is active.
- **Gap evidence in Rust:** the name `adhan` is **free**. The space is fragmented
  and thin: `salah` 851 recent / 4 revdeps, `adhaan` 137, `praytimes` 42 (2023,
  dead), `prayer-times` 6, `salati` 0 (2022), `praytime-rs` 7. None dominates.
- **Why it fits:** the workspace already has `hijridate` (Hijri conversion) and
  `arabic`. Prayer times are the same piece of domain, and **nothing** fills this
  space well. It is the "quiet domain vocabulary" pattern already proven in
  `cipansor/packages/shared`.
- **Difficulty:** low. This is astronomy arithmetic that is already specified
  (sun position, 18°/20° dawn angles, Shafi'i vs Hanafi Asr, altitude
  correction). Not research; the output can be compared number by number against
  `adhan` (npm/PyPI).
- **References:** `batoulapps/adhan-js`, `batoulapps/adhan-java`, `praytimes.org`.
- **Note:** `hijridate` is **also still free** on crates.io — this workspace's
  crates have never been published. If publication is the intent, those names
  need to be secured.

### 3. `properties` — Java .properties files

- **Use:** read `application.properties`, Java i18n messages, Spring
  configuration. A simple format with odd rules: `\uXXXX` Unicode escapes, `\`
  line continuation, `=`/`:`/space separators, `#` and `!` comments.
- **Demand evidence:** npm `properties` 317K/week.
- **Gap evidence:** `java-properties` exists (868K recent, 38 revdeps) but was
  **last updated 2023-05**. The name `properties` is **free**.
- **Difficulty:** very low. Rule mapping, not an algorithm.
- **Verdict:** worthwhile but **small**. The incumbent is alive-enough. This is a
  "when the others are done" candidate, not the first one.

## What I dropped, and why

- **`format-duration`** — the name is free, npm `pretty-ms` 26.7M/week. But
  `humantime` (70.1M recent, 1359 revdeps) and `duration-str` (3.7M, 65) already
  serve the same use case. The gap is only in the API shape, not the capability.
- **`cuid2`** — already served (`cuid2` 321K recent, 2026-04).
- **`vcard`, `rtf`, `hashids`** — already served.
- **Canonical JSON / JCS** — already served and healthy (see candidate 1).
- **`iban`** — the incumbent is stale (121K recent, 2 revdeps) and the name
  `iban` is taken. `eu-vat` in this workspace already touches that area.

## Recommended order

1. **`object-hash`** — the largest demand, the name is free, and the only
   incumbent died in 2017. Do this first; the difficulty is medium and the result
   is easy to test (each type-equality rule is one test).
2. **`adhan`** — the lowest risk, the best fit for the workspace, and it completes
   `hijridate` + `arabic` into one domain set.
3. **`properties`** — small, quick, once the two above are running.

All three meet the workspace's conditions: the name is free, they can be
std-only, the output can be compared against an existing oracle (npm
`object-hash`, npm `adhan`, Java `Properties.load`), and Apache-2.0 can be used.

## An honest warning

The earlier research recorded this and it still holds: **npm demand is not Rust
demand.** For `object-hash`, a dead Rust incumbent means we have no evidence that
anyone looks for it in Rust — only evidence that the concept is big in JS. Before
committing, it is worth checking how many Rust projects hack this themselves
(searching for "cache key hash" in their code), and whether `serde_jcs` +
`blake3` is already enough for them. If it is, the gap is narrower than it looks.
