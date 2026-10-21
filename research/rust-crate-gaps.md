# Popular npm/PyPI packages with no mature Rust equivalent

Research date: 2026-10-21. Every figure was read from the registry APIs (npm,
PyPI, crates.io) on that day, not recalled from memory.

## Method

1. Demand sources:
   - npm: the 1,000 "most depended-upon" packages (anvaka gist, names only) plus
     weekly download figures refetched from `api.npmjs.org` (the gist's own
     figures are a 2019 snapshot, so only its names were used).
   - PyPI: the top 15,000 packages from `top-pypi-packages-30-days.json`.
   - Monthly PyPI figures from `pypistats.org`.
2. Supply sources: crates.io `/api/v1/crates?q=...` and `/api/v1/crates/<name>`.
   The metric is `recent_downloads` (the last 90 days), not total downloads,
   because a total hides a crate that has died.
3. Candidates were filtered: packages that are specifically JavaScript-platform
   (React, Babel, ESLint, DOM, bundlers) were dropped. What remains is generic
   capability that ought to be able to live in Rust.
4. Each candidate was verified by hand against the exact crate name, because
   searching crates.io often gives a false negative.

Verdict rule: a gap counts as real when the best crate in the area has
`recent_downloads` under about 20,000 (effectively unused) or does not exist at
all, while its npm/PyPI counterpart has millions of downloads.

## Strongest candidates

| Capability | Strongest counterpart | Demand | Best Rust crate | recent | Verdict |
|---|---|---|---|---|---|
| Public holidays (per-country holiday calendars) | PyPI `holidays` 26.9M/month; `workalendar` 957K/month | large, stable | `holidays` 20K total (2023), `public-holidays` v0.0.0 (2024) | <200 | **LARGE GAP** |
| Table extraction from PDF | PyPI `tabula-py` 1.16M/month; `camelot-py` 722K/month | large | none; only `lopdf`/`pdf-extract` for raw text | 0 | **LARGE GAP** |
| Spreadsheet formula evaluator | npm `exceljs` 10.8M/week; `xlsx` 7.2M/week | large | `formula` 20 total (2022), `xlformula_engine` dead (2023) | <100 | **LARGE GAP** |
| European e-invoicing (ZUGFeRD/Factur-X, UBL, PEPPOL) | PyPI `factur-x` 287K/month; npm `zugferd` | medium, growing | `en16931-formats` | 5.4K | **GAP** |
| Pure-Rust Hunspell dictionaries | PyPI `hunspell`; npm `nspell` 147K/week | medium | `hunspell-sys`/`hunspell-rs` (C bindings, 2022), `spellbook` (partial) | <30K | **GAP** |
| zxcvbn-style password strength (Indonesian locale) | npm `zxcvbn` 783K/week | medium | `zxcvbn` already exists | 1.8M | already covered |
| iCal parser + RRULE expansion | PyPI `icalendar` + `recurring-ical-events` | medium | `rrule` (2025), `icalendar` 395K | 629K | partly covered |
| English inflection/pluralisation | npm `pluralize` 23.5M/week | large | `inflector` dead (2019), `cruet` fork, `pluralizer` | 1.8M | **FORK-AND-WIN** |
| User-agent detection | npm `ua-parser-js` 19M/week | large | `uaparser` (2024 data) | 2.9M | fork-and-win |
| Arabic harakat/tashkeel + Arabic text normalisation | - | niche | `ar-reshaper`, `arabic_text_utils`, `dengjen-tashkeel` | <10K | **NICHE GAP** |
| Hijri calendar (Umm al-Qura) + prayer times | PyPI `hijri-converter` 2.8M/month | medium | `hijri` (CLI), `salah` 848, `praytimes` (2023) | <1K | **NICHE GAP** |

## Important note: many apparent gaps already exist

This is what separates registry-based research from guessing:

- XLSX: `rust_xlsxwriter` (2.4M recent), `calamine` (5.7M recent),
  `umya-spreadsheet` are mature. Not a gap.
- DOCX: `docx-rs` (790K recent) is active. Not a gap.
- Generic PDF: `pdf-writer`, `printpdf`, `typst` (1.6M recent) are strong.
  Not a gap.
- OCR: `ocrs` (254K recent), `tesseract-rs` (42K recent) exist. Not a gap.
- Image processing: `image` 64.6M recent plus `imageproc`. Not a gap.
- YAML: `serde_yaml` is deprecated but `serde-yaml-ng` (11.3M recent) and
  `saphyr` have replaced it. Not a gap.
- Browser automation: `chromiumoxide` 2.1M recent, `headless_chrome`,
  `thirtyfour`, `fantoccini` exist. Not a gap.
- Job queues: `apalis` (394K recent) exists. Not a gap.
- State machines: `statig` (231K recent), `rust-fsm`, `sm` exist. Not a gap.
- Workflow engines: `dataflow-rs` (6.7K recent), `acts`, `aion-rs` exist,
  though there is no Temporal equivalent. Half a gap.
- OAuth/OIDC: `oauth2` 17M recent, `openidconnect` 4.9M recent. Not a gap.
- PII redaction: PyPI `presidio-analyzer` 5.2M/month, but Rust already has
  active players (`redactable` 16.6K recent, `redact-core`, `qubit-redact`).
  Not a large gap, though the market is still open.
- Embeddings/vector DBs: `fastembed` 2.3M recent, `pgvector` 5.2M recent.
  Not a gap.
- Indonesian NLP: there **already is** `sastrawi-rs`, `rustrawi`, `harmorp`.
  An important correction to the old assumption that this field was empty.
- The crate name `temporal-rs` is taken by `temporal_rs` (7.1M recent, the
  Temporal JS calendar), so do not use that name.

## Recommended order of work

1. `holidays-rs` - the clearest demand, effectively no competitor, and the data
   is openly available (workalendar, date-holidays and python-holidays are all
   open source). Low risk, high value.
2. Table extraction from PDF - large demand, no competitor at all, but the
   algorithm is heavy (line detection, word clustering, cell reconstruction).
3. Spreadsheet formula evaluator - large demand, two dead competitors. Scope can
   be limited at first to the common subset of formulas.
4. EN 16931 e-invoicing - medium demand but growing because of European Union
   regulation, and `en16931-formats` is still too small to count as a competitor.
5. English inflection - the fastest way to win is to fork `inflector`, fix the
   long-standing bugs and release. Not to write it from scratch.

## How to reproduce

The scripts are in `/tmp/gapresearch` (not committed):

- `crates.py` - crates.io API wrapper with cache and retry.
- `npmfetch.py` - fetch npm downloads and metadata (the bulk endpoint rejects a
  mix of scoped and unscoped names, so they have to be batched separately).
- `pipeline.py` - the `pypi`, `crates-npm` and `crates-pypi` stages.
- `probe.py` - probe 263 curated capabilities against crates.io.
- `demand.py` - npm and PyPI demand figures for a curated list of candidates.

Operational note: crates.io rate-limits requests. Running two scripts at once
triggers HTTP 429 and the results come back falsely empty. Run them one at a time.
