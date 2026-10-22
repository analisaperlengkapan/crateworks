# Gap scan round 12: the abandoned incumbent, measured at last, and the clearance layer (2026-10-21)

Round 11 found one abandoned incumbent by hand — `fuzzy-matcher`, nine and a
half million recent downloads, last release 2020, repository archived — and
called it the strongest opening it had seen. A single example noticed by eye is
not a method. This round does two things:

1. **Measures the abandoned incumbent across the whole registry**, so the answer
   stops depending on someone noticing. The tool is `tools/stale.py`, committed
   so the scan can be repeated.
2. **Turns the scan forward.** Rounds 1–11 all looked at what is already popular
   elsewhere and asked whether Rust has it. That question is answered. The
   complementary question — *what will a regulated industry be obliged to
   implement, on a date, that Rust does not have* — had not been asked.

Every figure was fetched on 2026-10-21 from `crates.io/api/v1`,
`index.crates.io` (the sparse index, which carries a publish time per version
where the API's `updated_at` does not), `api.npmjs.org`, `pypistats.org` and
`registry.npmjs.org`.

## Part one: the abandoned incumbent, measured

`tools/stale.py` walks the ranked crates.io list by recent (90-day) downloads,
reads each crate's newest **live** version from the sparse index, and reports
every crate whose last live release is older than a threshold. A yank is not a
release, so a crate whose newest version was pulled is reported as older than it
looks. The repository check is a second, optional pass against `api.github.com`.

The result at ≥200,000 recent downloads and ≥730 days without a live release:

| | |
| --- | --- |
| crates in the ranked list examined | 3,000 |
| crates matching the age and download filter | **569** |
| of those, repositories **archived** on GitHub (≥500k downloads) | **45** |

569 crates is too many to be a list of openings, and that is itself the finding.
An old last-release date is the normal state of a finished crate, not a signal.
Sorting the 45 archived ones by what they are separates two populations:

**Infrastructure that is finished, and correctly frozen.** Most of the 45.
`winapi` (109M, 2020) is the Windows API bound by generation — it is complete
because Windows 10's surface stopped moving. `foreign-types` (98M, 2019),
`nohash-hasher` (31M, 2020), `paste` (116M, 2024, superseded by `macro_rules`
improvements), `proc-macro-error` (44M, 2020, superseded by `proc-macro-error2`)
and the `-sys` crates are glue that either works or does not. Archiving them is
housekeeping. **None of these is an opening**, and a scan that reported them as
one would be worse than no scan.

**General capabilities with a large live audience.** A short list, and this is
the part worth keeping:

| Crate | recent downloads | last release | repo state | what it does |
| --- | --- | --- | --- | --- |
| `serde_yaml` | **97,228,781** | 2024-03-25 | archived, 1,016 stars, 50 open issues | YAML for serde |
| `unsafe-libyaml` | 81,117,519 | 2024-03-17 | archived | libyaml transpiled by c2rust |
| `rustls-pemfile` | 77,627,787 | 2024-09-30 | archived | PEM parsing |
| `smartstring` | 13,069,177 | 2022-03-24 | archived, 543 stars, 23 open issues | inline small strings |
| `fuzzy-matcher` | 9,428,270 | 2020-10-04 | archived, 295 stars | fuzzy scoring |
| `multipart` | 5,674,786 | 2021-05-29 | archived, 193 stars, 21 open issues | multipart/form-data |
| `im` | 4,753,789 | 2022-04-29 | archived, 1,583 stars, 53 open issues | immutable collections |

The ranking has an obvious shape: **the largest abandoned capabilities are data
formats, and the largest of all is YAML.** `serde_yaml` at 97M recent downloads
with an archived repository, 50 open issues and a last release in March 2024 is
the single biggest audience-with-nobody-home found in twelve rounds — 2.5 times
`fuzzy-matcher`'s ratio against its successor. It is also the hardest: the
`unsafe-libyaml` dependency is archived too, so a replacement has to carry a YAML
parser as well as the serde layer. Recorded as the strongest measured opening,
not as a small one.

`smartstring` (13M, 543 stars) and `im` (4.8M, 1,583 stars) are the next two: a
large audience, a maintained-looking repository, and no release in years.

## Part two: the clearance layer, where the scan points forward

Rounds 1–11 established that the European EN 16931 core is **served**:
`en16931` (5,761 recent downloads) and `en16931-formats` (5,592) cover the
semantic model, UBL, CII and ZUGFeRD, and this workspace's own `factur-x` and
`pdfa` sit in the same space. That is not a gap.

What is empty is the layer *above* it: the national clearance and reporting
formats that a tax authority mandates, in the country's own XML, on a date the
law names. These are not EN 16931 profiles; they are separate documents with
their own namespaces, catalogues, signatures and QR codes.

The demand is measured on PyPI, and the Rust side is empty:

| Format | Country | PyPI monthly | npm weekly | crates.io |
| --- | --- | --- | --- | --- |
| **CFDI 4.0** | Mexico | `satcfdi` **35,374** | `facturapi` **11,740** | `cfdi` — **16 recent**, deserialise-only, 0 reverse deps |
| **NF-e / NFC-e** | Brazil | `erpbrasil.edoc` 23,870, `pynfe` 9,596 | — | `fiscal-*` — 200–500 recent, present but young |
| **FatturaPA / SDI** | Italy | `fatturapa` 165 | `fatturapa` 32 | **0 crates** |
| EN 16931 core | EU | `factur-x` **286,877** | `factur-x` 261 | `en16931` 5,761, `en16931-formats` 5,592 — **served** |
| ZATCA / Fatoora | Saudi Arabia | — | — | `fatoora-core` 220 — served |
| SUNAT | Peru | — | — | `kembec-sunat-*` — served |
| PINT-MY / MyInvois | Malaysia | `myinvois` 17 | — | `core-invoice*` — served |
| KSeF | Poland | `pyksef` 3,586 | — | `ksef`, `ksef-client` — present |
| Veri\*Factu | Spain | `verifactu` 75 | — | `verifactu` 47 — present |

Three rows matter and the rest are recorded so they are not re-scanned.

**Mexico CFDI 4.0 is the one real gap.** `satcfdi` does 35,374 downloads a month
on PyPI and `facturapi` 11,740 a week on npm — the largest measured demand in the
table after the EN 16931 core itself. The Rust side has exactly one crate, `cfdi`
at 16 recent downloads, whose own GitHub description is *"Crate para Deserializar
un XML de un CFDI en structs nativos en Rust"* — it deserialises, and nothing
else. Its repository has **one star and zero reverse dependencies**, and it last
moved in December 2024. There is no generator, no validation, no `cadena
original`, no CSD signing and no timbrado client anywhere in Rust.

The mandate is not optional and it is not new: CFDI 4.0 became mandatory in
April 2023, every tax-relevant transaction is a signed XML, and the 2026
Miscelánea Fiscal added further cancellation and payment-complement rules. This
is the same shape as `iso6346` in round 10 — a mandated format, a measured
reference, an empty registry — except the format is much larger, which is why it
is a multi-crate body of work rather than a check digit.

**Italy FatturaPA is the runner-up and is smaller than it looks.** `fatturapa`
on PyPI does 165 downloads a month and 32 a week on npm; the format is real and
mandatory but the open-source demand is modest, because most Italian invoicing
runs through commercial intermediaries. The name is free on crates.io. Watch it;
do not start there.

**Brazil NF-e is already being built, by someone else.** `fiscal-core` (511),
`fiscal-sefaz` (338), `fiscal` (217) and `nfe-parser` (40) are a young,
coordinated family released in July 2026. Two independent Rust efforts in one
jurisdiction is the state `iso6346` was in before it closed. Leave it.

## What this round concludes

1. **The abandoned incumbent is a real signal but a noisy one.** 569 crates pass
   a naive age-and-downloads filter; 45 have archived repositories; perhaps six
   are general capabilities worth replacing. The filter that works is not the
   date, it is *archived repository plus a general capability*, and it now has a
   committed tool (`tools/stale.py`) rather than an anecdote.
2. **`serde_yaml` is the largest opening measured in twelve rounds** — 97M recent
   downloads, archived repository, no release in nineteen months, and the
   `unsafe-libyaml` underneath it is archived too. It is also the largest job.
   `smartstring` and `im` are the smaller, cleaner follow-ups.
3. **`fuzzy-matcher` is confirmed**, not new: round 11 found it by hand and this
   round finds it in the systematic scan at 9.4M recent downloads. It remains a
   good candidate and is now one of six rather than the only one.
4. **The forward-looking question found exactly one new build: Mexico CFDI 4.0.**
   The EN 16931 core is served, and Brazil, Saudi Arabia, Peru, Malaysia and
   Poland are already taken. CFDI has 35k/month of measured Python demand, a
   single-star deserialise-only crate in Rust, and a legal mandate behind it.
5. **Do not re-scan the national e-invoicing space for "is there a crate".** The
   answer is now recorded row by row above. The remaining work there is depth in
   one jurisdiction, not discovery of another.

## Method note

`tools/stale.py` is the tool this round added. It caches sparse-index reads under
`tools/.cache/`, pages the crates.io ranked list with the API's own `seek`
cursor, and treats a yank as not-a-release. `--github` adds the archived check
and needs `GITHUB_TOKEN` to stay under the unauthenticated rate limit. The
warnings in `tools/README.md` still apply: do not run two research scripts at
once, and treat any "nothing found" as a rate limit until it is re-checked.
