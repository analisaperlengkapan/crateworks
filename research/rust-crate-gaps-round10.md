# Gap scan round 10: the specialist codes nobody has published (2026-10-21)

Round 9 closed with "the npm and PyPI top lists are both served; a gap is a
standard a regulated industry is obliged to implement". This round follows that
rule to its end and answers a narrower question: **which small, well-specified
codes does an industry exchange every day, where the demand is measured and the
Rust registry is empty?**

Every number was fetched on 2026-10-21: npm downloads from
`api.npmjs.org/downloads/point/last-month`, crate counts from
`crates.io/api/v1/crates?q=`. A name is called a gap only when the demand is
large and the Rust side is absent or has no dependents.

## Correction: X12 HIPAA is withdrawn

Round 9 named X12 HIPAA semantic validation as "the one live candidate". That is
wrong and is withdrawn here.

The X12 **TR3 implementation guides** (the 837, 835, 270/271 rulebooks) are
copyrighted by Washington Publishing Company and licensed per user, and CPT codes
are licensed by the AMA. A crate cannot ship the tables the validation needs
without redistributing licensed material, and an Apache-2.0 crate cannot do that.
The syntax is free, and `x12-types` (Apache-2.0, apimeister) and `pyx12` (BSD,
azoner/pyx12) already cover it. **Do not propose X12 HIPAA again.** EDIFACT
(UN/CEFACT) is free and remains open, but its demand in this workspace's
audience is unmeasured, so it is a watch item, not a build.

## What the popularity lists still say

Re-measured this round, all served on the Rust side:

| npm package | monthly | what it does | Rust side |
| --- | --- | --- | --- |
| `nanoid` | 1,105,552,892 | short ids | `nanoid` |
| `dompurify` | 281,912,819 | HTML sanitise | `ammonia`, `sanitize-html` |
| `html-entities` | 173,885,909 | entity decode | `htmlize`, `html-escape` |
| `libphonenumber-js` | 110,762,636 | phone parse | `phonenumber`, `rlibphonenumber` |
| `cron-parser` | 90,190,068 | cron parse | `cron` 27M |
| `csv-parse` | 83,043,421 | CSV | `csv` |
| `html-to-text` | 73,400,983 | HTML to text | `html2text` 6.7M |
| `papaparse` | 67,064,879 | CSV | `csv` |
| `pdf-parse` | 36,962,001 | PDF text | `pdf-extract` 6.8M |
| `date-holidays` | 3,945,731 | public holidays | `holidays` 20,781 — **present but thin** |

Two observations. First, **the popularity list is still fully served**; nothing
here is a gap. Second, `date-holidays` at 3.9M/month against a Rust leader of
20,781 downloads is the largest *ratio* on the list, and it is the one entry
worth watching rather than calling served. `holidays-rs` (this workspace) is a
candidate to grow into it, one country at a time.

## The actual gaps: codes the registry has never named

These are the openings. Each is a small, deterministic check — a check digit or a
format — that an industry exchanges in volume, with a measured npm package and an
empty or near-empty Rust registry.

| Standard | What it is | npm (monthly) | crates.io today | Verdict |
| --- | --- | --- | --- | --- |
| **ISO 6346** | Shipping container code: 4 letters + 6 digits + 1 check digit | `container-validator` 20,376; `is-container` 526; `container-number-validation` | `q=iso6346` → **0 crates**; only a 14-download API client exists | **Build.** The name `iso6346` is free. |
| **UN/LOCODE** | UN location codes for ports and trade | `un-locode` 491; several data packages | `q=unlocode` → **0 crates** | **Watch.** Empty, but the data set is large and the npm demand is modest. |
| **ISO 11649** | Structured creditor reference (RF) | `node-iso11649` 7,421; `iso-11649` 1,517 | `iso_11649` 3,066, `sepa` 2,093 | Present. Skip. |
| **ISSN** | Serial numbers | `issn` 475 | `library_stdnums`, `commonmeta` | Present. Skip. |
| **ISMN / ISNI** | Music and name identifiers | `@form-validation/validator-ismn` 15,374 | `idt`, `bwipp-rs` | Partly present; low direct demand. Skip. |
| **IMO ship number** | Seven digits + check digit | `imonumber`; low volume | none by name | Watch. Too small to build alone; fold into ISO 6346 if ever needed. |
| **CUSIP / ISIN / LEI** | Securities and entity identifiers | large | `cusip` 213,119, `isin` 240,519, `lei` 201,791 | Served. Skip. |
| **ISO 9362 (BIC/SWIFT)** | Bank identifier code | `iban-to-bic` large | `rust_iso9362` 24 downloads | Thin but present; a BIC is a format check with little to gain. Skip. |

## What this round concludes

1. **ISO 6346 is the one new build.** It is the clearest empty-and-wanted result
   this workspace has found since round 9: a mandated code with a ~20k/month npm
   validator, zero crates under its name, and a check digit that is a page of
   arithmetic. It fits the workspace shape exactly — small, testable against the
   npm reference, and free of any licence on the algorithm (the *code* is
   ISO's; the check-digit rule is arithmetic).
2. **UN/LOCODE is the runner-up to watch**, not to build: the registry is empty,
   but the value is in the data set, not the check, and the npm demand is small.
3. **X12 HIPAA stays withdrawn**, on licence grounds, and round 9's recommendation
   of it should not be acted on.
4. **`date-holidays` is the ratio to watch**: 3.9M npm downloads against a 20,781
   Rust leader. `holidays-rs` can close it a country at a time, which is a
   different kind of work from a new crate but the largest measured opening left.
