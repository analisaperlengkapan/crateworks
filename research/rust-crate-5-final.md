# 5 final candidates (round 3, 2026-10-21)

## Mistakes I corrected this round

Two fundamental things had thrown the earlier list off:

**1. My demand figures were not Rust demand.**
PyPI/npm measures demand in Python/JS, not in Rust. The real Rust signal is the
**reverse dependencies** of the incumbent crate (how many other crates depend on
it) and whether its repo is actually dead. I measured that now for every
candidate.

**2. Number 8 (subtitles) on the earlier list was WRONG.**
I concluded "all dead" from the `subparse`/`srt` that turned up in the search. In
fact there is **`subtitler` v2.9.0**: 25 versions, last release 2026-10-02,
supporting 17 formats (SRT, VTT, ASS/SSA, TTML, MicroDVD, EBU STL, Whisper JSON).
That is a mature, active crate. There is also `subtp` (18.9K recent) and
`revelo-parsers-text`. The subtitle field is already well served. I dropped this
candidate.

Lesson: a crate dominating the search results does not mean it is the best crate.
Newer crates that are not yet popular have to be checked too.

## Supporting data (measured today)

| Crate | recent | revdeps | last release | repo status |
|---|---|---|---|---|
| `holidays` | 7,866 | 1 | 2023-02, **only 1 version** | dead (last push 2024-07), 6 issues |
| `py-holidays-rs` | 911 | 0 | 2025-09 | active, 1 star |
| `stdnum` | 16 | 0 | 2026-07 | new, 0 stars |
| `ssn` / `br_tax_id` / `idsmith` | 138 / 294 / 186 | 0 | 2026 | fragmented per country |
| `formula` | 20 | 0 | 2022-11 | dead |
| `xlformula_engine` | 2,428 | 1 | 2023-05 | dead |
| `formula-lang` | 190 | 1 | 2026-10 | new, 0 stars |
| `iso20022` | 21 | 0 | 2020-03 | dead |
| `sepa` / `mt940` / `camt053` | 1,595 / 2,422 / 220 | 1 / 2 / 0 | 2026 | active but narrow |
| `wsdl` / `soap` / `rsoap` | 8 / 3 / 46 | **0 / 0 / 0** | 2023 / 2025 / 2026 | experiments |
| `en16931-formats` | 5,404 | 1 | 2026-10 | **very active** (11 versions / 3 months) |
| `musicxml` | 1,834 | **7** | 2024-11 | stalled |
| `subtitler` | 1,905 | 0 | 2026-10 | **active, 25 versions** |
| `libdav` | 14,557 | 4 | 2026-09 | active, healthy |

An important note on SOAP: **0 reverse dependencies across the whole SOAP space**.
That means the large demand is in Python/JS, but in Rust nobody has yet been shown
to use it. The widest gap and the least proven Rust demand at the same time.

## 5 candidates

### 1. `holidays-rs` - public holiday calendars
- **Use:** compute working days. HR counts leave and pay, banks compute T+2 due
  dates, ticketing systems compute SLAs. It needs to know the holiday dates per
  country, and that data changes every year.
- **Gap evidence:** `holidays` has only 1 version (2023), the repo has been dead
  since 2024, 6 open issues. The only active challenger is `py-holidays-rs` (911
  recent, 1 star). Demand: PyPI `holidays` 26.9 million/month.
- **Difficulty:** low. This is data mapping, not algorithm research. Open
  sources: python-holidays, workalendar, date-holidays.
- **First step:** port python-holidays for 10 countries first, then add
  "next working day" and "Nth working day" rules.

### 2. `stdnum-rs` - official national number validation
- **Use:** validate NPWP, NIK, European VAT, SSN, CPF, IBAN. Used in
  registration forms, KYC onboarding, tax invoice validation. Every country has
  its own checksum formula.
- **Gap evidence:** there is no umbrella crate. What exists is fragmented: `ssn`
  (USA), `br_tax_id` (Brazil), `dni` (Spain), `idnr` (Germany), `amka` (Greece),
  `vat_id_validator` (5.3K). `stdnum` itself is newborn (16 recent, 0 stars).
  Demand: PyPI `python-stdnum` 5.08 million/month.
- **Difficulty:** medium, but very measurable. Suited to incremental
  contribution: one country per pull request, one module per PR.
- **First step:** a `validate(country, number)` API plus the first 15 countries.

### 3. `spreadsheet-formula` - spreadsheet formula evaluator
- **Use:** read an Excel file whose cells hold formulas and compute the results,
  without opening Excel. For financial reports, data import, servers that process
  spreadsheets.
- **Gap evidence:** every attempt died. `formula` (2022), `xlformula_engine`
  (2023), `formula-lang` (0 stars). Meanwhile the read/write side is already good
  (`calamine` 5.7 million recent, `rust_xlsxwriter` 2.4 million) — what is
  missing is exactly the calculation engine. Demand: PyPI `openpyxl` 283
  million/month, npm `exceljs` 10.8 million/week.
- **Difficulty:** medium to high. Control the scope: start with 50 functions
  (SUM, AVERAGE, IF, VLOOKUP, date, text), no pivot or macro needed yet.
- **First step:** formula parser plus an evaluator for 20 functions, then test
  against real xlsx files from `calamine`.

### 4. `iso20022` - banking payment messages
- **Use:** payment reconciliation, automated transfers, accounting
  applications. MT940 = bank account statement; SEPA = euro transfer; ISO 20022 =
  pain/pacs/camt, the new standard replacing SWIFT MT.
- **Gap evidence:** all the players are small and separate. `iso20022` dead since
  2020, `sepa` 1.6K, `mt940` 2.4K, `camt053` 220. None dominates. Demand is
  driven by regulation: SWIFT MT is being retired and replaced by ISO 20022.
- **Difficulty:** medium. The XSD schemas are public, so the type model can be
  generated rather than written by hand.
- **First step:** generate types from the XSD for camt.053 and pain.001, then add
  a parser and a validator.

### 5. `soap-client` - SOAP/WSDL client
- **Use:** connect to old XML-style web services. Banks, government agencies,
  SAP and legacy systems still use them.
- **Gap evidence:** the emptiest on the list. `wsdl` 8 recent (2023), `soap` 3
  recent, `rsoap` 46 recent, `soap-server` 195 recent. No mature client. Demand:
  PyPI `zeep` 29.4 million/month, `suds-py3` 771K/month.
- **Risk to be aware of:** 0 reverse dependencies across the whole SOAP space.
  The demand is real in Python, but in Rust it is unproven. And this is the
  heaviest: it needs code generation from WSDL, WS-Security and MTOM.
- **First step:** a WSDL parser, type generation, and a basic SOAP 1.1 client.
  Postpone WS-Security until there are real users.

## What I removed and why

- **subtitles** - `subtitler` is already mature (25 versions, 17 formats).
  Misjudged in the previous round.
- **CalDAV** - `libdav` is healthy and active (4 revdeps). Already served.
- **en16931 e-invoice** - `en16931-formats` is being worked on very
  aggressively (11 versions in 3 months). Competing with a highly motivated
  author is a large risk.
- **MusicXML** - has 7 revdeps (real Rust users), but new arrivals already exist
  (`acorde-core` 8 revdeps, `music21-rs`, `guitarpro`, `croma-core`). No longer
  empty.
- **PlantUML** - `puml` has 23 open issues (a single author struggling), but a
  diagram layout engine is heavy work for a narrow result.
- **PDF table extraction, CycloneDX, XML Signature, Mermaid, JSON-LD, HAR, GTFS,
  SAML, FHIR, BibTeX, HCL, DICOM, IMAP, DKIM/SPF/DMARC** - all already have
  active players.

## How to choose among the five

- Want a fast, safe result: **1 or 2**.
- Want large impact and are ready to work longer: **3 or 5**.
- Want a market that grows on its own through regulation: **4**.

Before starting, repeat these two checks because the space moves fast:
`cargo search <name>` and GitHub `search?q=<topic>+language:rust`, then look at
the reverse dependencies tab on crates.io for the incumbent you are targeting.

---

# Addendum (2026-10-21, name and competitor check)

Added after the candidates above were started. This section does not change the
assessment above; it adds two things not checked at the time: **name availability
on crates.io** and **competitor movement**.

## 1. Name availability (never checked before)

Every candidate name was measured through `GET https://crates.io/api/v1/crates/<name>`.
A 404 means free. The results that changed a decision:

| Name | Status | Note |
|---|---|---|
| `iso20022` | **TAKEN** | keithnoguchi/iso20022-rs, created 2020-03-11, last 2020-03-16, 21 recent. Dead, but the name stays taken |
| `iso20022-rs` | free | chosen |
| `crateworks-xml` | free | changed for design reasons, not a clash |
| `holidays-rs` | free | the plain name `holidays` is taken by mapbox-jp (2023) |
| `stdnum-rs` | free | the plain name `stdnum` is taken by hey-jj (2026-07) |
| `soap-client` | free | the plain names `soap`/`wsdl` are taken |
| `spreadsheet-formula` | free | the plain names `spreadsheet`/`formula` are taken |
| `sepa` | taken | hupe1980/sepa |

**The mistake found: candidate 4 (`iso20022`) cannot be published under that
name.** The name was taken by a crate dead since 2020. This should have been part
of the check from the start: **checking name availability is the first step, not
the last.** Renaming after publication is impossible.

Lesson on the `-rs` convention: the suffix is used **when the plain name is
already taken**. `holidays` and `stdnum` are both taken, so `holidays-rs` and
`stdnum-rs` are the right decision, not a naming deficiency. `iso20022-rs`
follows the same pattern.

`crateworks-xml` was renamed to `xmlkit` not because of a clash, but because a
foundation crate shows up in every user's manifest. A brand prefix there forces
people to write `crateworks` in their dependency list for no benefit.

## 2. Competitor movement since the first measurement

| Crate | Latest status | Impact |
|---|---|---|
| `sepa` | v0.9.0, 1,595 recent, updated **2026-10-13** (8 days ago) | **Moved up a class.** Its description is now "SEPA identifiers and ISO 20022 payment messages: builders, bank-report". That is squarely candidate 4, and it is moving fast |
| `stdnum` | v0.2.0, 16 recent, created 2026-07-01 | still new, 0 stars. Its description is exactly candidate 2 |
| `iso20022` | 21 recent, dead since 2020 | unchanged |
| `calamine` | 5.67 million recent, v0.37.0, updated 2026-10-18 | the xlsx read/write side stays strong, the calculation engine stays empty |

**What changed from the initial assessment:** `sepa` is no longer "small and
narrow". At 1,595 recent and a release 8 days ago, it is a serious challenger in
the ISO 20022 space, not a fringe player. The assessment "none dominates" is
still correct, but the difficulty has gone up.

`stdnum` (hey-jj) is only 3 months old and still at 0 stars, so candidate 2 is
still open, but no longer empty.

## 3. What did not change

- **SOAP stays the emptiest.** `wsdl` 8 recent, `soap` 3 recent, `rsoap` 46
  recent, and **0 reverse dependencies** across the space. The widest gap, the
  least proven Rust demand.
- **The spreadsheet formula engine stays empty.** `formula` (2022, dead),
  `xlformula_engine` (2023, dead), `formula-lang` (0 stars). The read/write side
  is already good; what is missing is still the calculation engine.
- **`holidays` is still 1 version from 2023** with a dead repo.

## 4. Method notes

- Reverse dependencies remain the best signal. Raw downloads mislead: a dead
  crate such as `spreadsheet` (2016) still has 5,725 total downloads.
- The name availability check should be run **before** writing code, for all
  candidates at once. The result rarely changes, so one check is enough until
  there is a new candidate.
- This space moves fast. `sepa` went from "narrow" to an active challenger within
  months, so the measurement has to be repeated before starting the next
  candidate.

---

## Addendum 2026-10-21 (second): a probe round for new candidates

After the first 6 crates, I probed 19 more capabilities the same way (demand from
npm/PyPI, supply from crates.io). The aim was to find what is worth building next.
The result was disappointing in a useful way: most candidates turned out to be
**already served**, and I nearly built duplicates.

### What turned out to be already served (do not build)

| Capability | What I thought was a gap | What it actually is |
|---|---|---|
| Number to words (multi-language) | `num2words` (PyPI 7.8 million/month), `inflect` (22 million/month) | A Rust `num2words` already exists and is **already multi-language** (French and others), 94K recent. Not a gap. |
| SQL formatting and parsing | `sqlparse` (107 million/month), `sqlglot` (72 million/month) | `sqlformat` 4.3 million recent plus `sqlparser` 14.8 million recent, 435 revdeps. Not a gap. |
| Byte size to text | `bytes` (84 million/week), `pretty-bytes` (19 million/week) | `humansize` 12.5 million, `bytesize` 21.9 million, 500 revdeps. Not a gap. |
| ISO 8601 durations | `iso8601-duration`, `tinyduration` | `iso8601` 6.4 million, `duration-str` 3.7 million. Not a gap. |
| Fuzzy matching | `rapidfuzz` (116 million/month), `thefuzz` | `strsim` 249 million recent, `fuzzy-matcher` 9.2 million, a Rust `rapidfuzz` 4.3 million. Not a gap. |
| Phone number validation | `libphonenumber-js` (16 million/week), `phonenumbers` (34 million/month) | `phonenumber` 3.2 million recent, 65 revdeps. Not a gap. |
| Barcodes | `jsbarcode`, `python-barcode` | `rxing` 212K, `barcoders` 162K, both active. Not a gap. |
| Password strength | `zxcvbn` | Rust `zxcvbn` 1.8 million recent. Already served. |
| Credit cards | `card-validator` | `card-validate` 439K recent. Already served. |
| Hijri calendar **under the name `hijri`** | PyPI `hijri-converter` 2.8 million/month | The Rust `hijri` crate is only a **CLI**, 11 recent. This is the deceptive one: the name looks alike, the function is not a library. |

### The candidates that passed, and the one chosen

**Chosen and already built: `hijridate`.**

- Demand: PyPI `hijridate` 3.3 million/month, `hijri-converter` 2.8 million/month,
  `ummalqura` 1.2 million/month. All millions, not thousands.
- Supply: the `hijri` crate (CLI, 11 recent); `hijridate`/`hijri-converter`/
  `ummalqura`/`islamic-calendar` are **all free**. The name `hijri-date` is taken
  but dead since 2023 (857 recent).
- Gap quality: `holidays-rs` in this workspace itself already records that
  Indonesian Eid al-Fitr and Eid al-Adha are left out because they "depend on an
  official decision, not a formula". The Umm al-Qura table is where those dates
  are looked up. So this gap was documented before I went looking for it.
- The data is openly available and MIT (`dralshehri/hijridate`), so the dates did
  not have to be guessed.

**Still open, not yet done:**

- **Table extraction from PDF** (`pdfplumber` 40.7 million/month, `tabula-py`
  1.2 million/month, `camelot-py` 722K/month). In Rust: `pdfplumber` 40K recent
  (a single player, alpha), `pdf-table`/`camelot` are free. The largest remaining
  gap, but the algorithm is heavy.
- **Pure-Rust Hunspell**: `spellbook` 140K recent already exists and is active,
  so this is no longer a clean gap. `nspell` (npm) 147K/week.
- **vCard**: `vcard`, `vcard-rs`, `vobject`, `vcf` all exist but are small
  (1.1-1.5K recent). Half a gap, Rust demand unproven.

### Method notes (correction)

- **A similar-looking crate name can deceive.** `hijri` sounds like a calendar
  library; it is a CLI. Always read the description and `crate_type`, not just
  the name and download figures.
- `MONTH_STARTS` in the Python `hijridate` stores **RJD** (JDN minus 2,400,000),
  not JDN. I assumed wrongly at first and the table produced year -4647. The same
  lesson as the Gitea port: read the source, do not guess what a column means.
- Regenerating the table is idempotent, so a diff in the generated file means a
  new upstream release or a mistake — not a style change.

---

## Addendum 2026-10-21 (third): a further probe round, 40 candidates

I probed 40 more capabilities in four batches. The result reinforced the same
pattern: **most candidates turned out to be already served**, and some names that
sounded like gaps turned out to be active crates.

### IMPORTANT: new competitors for crates already in this workspace

This research found that two crates in this workspace already have neighbours:

| Workspace crate | Competitor found | Size |
|---|---|---|
| `iso20022-rs` | `mx20022` (50 recent), `rust_iso20022` (55), `iso20022-common` (**2,362 recent**, 2024), `mx20022-translate` | Existing, but all small except `iso20022-common` |
| `soap-client` | (unchanged) `wsdl` 8, `soap` 3, 0 revdeps | Still the emptiest |

That `iso20022-common` at 2,362 recent needs attention: I had earlier recorded
"only one crate from 2020". That is no longer accurate. The ISO 20022 candidate is
no longer empty, though there is still no dominant player.

### What turned out to be already served (do not build)

| Capability | Demand | Rust supply | Verdict |
|---|---|---|---|
| **SWIFT MT103/MT202** | PyPI `mt-940` 499K/month | **`swift-mt-message` 3,881 recent plus derive macros, active 2026-02** | Served. The names `mt103`/`swift-mt` are free but useless. |
| JSONLogic rules engine | npm `json-logic-js` 1.5 million/week, PyPI `json-logic` 2.2 million/month | `datalogic-rs` **85K recent**, `juspay_jsonlogic` 77K, `jsonlogic-rs` 14K | Strongly served. |
| Words to numbers | PyPI `word2number` 2 million/month, `number-parser` 622K | `text2num` 21,655 recent (multi-language), `words2num2-core` | Served. |
| Unit conversion | PyPI `pint` 5.2 million/month, npm `convert-units` 116K/week | `uom` **2.6 million recent** | Strongly served. |
| Transliteration/ASCII folding | PyPI `text-unidecode` 65 million/month | `deunicode` **17 million recent**, `unidecode` 1.4 million, `any_ascii` 2.1 million | Strongly served. |
| URL slugs | PyPI `python-slugify` 64 million/month, npm `slugify` 11 million/week | `slug` **8 million recent**, 157 revdeps | Strongly served. |
| Human-readable durations | PyPI `humanize` 46 million/month | `humantime` **70 million recent**, 1,359 revdeps | Strongly served. |
| Natural date parsing | PyPI `python-dateutil` 899 million/month | `dateparser` 2.1 million, `parse_datetime` 1.6 million, `dtparse` 365K | Served. |
| Timezone from coordinates | PyPI `timezonefinder` 5.1 million/month | `tzf-rs` 147K recent | Served. |
| Medical DICOM | PyPI `pydicom` 5.8 million/month | `dicom` **307K recent**, 13 revdeps | Served. |
| NMEA GPS | PyPI `pynmea2` 112K/month | `nmea-parser` 118K, `nmea` 43K | Served. |
| EDI X12 | PyPI `pyx12` 308K/month | `edi` 58K recent | Served. |
| EDIFACT | PyPI `pydifact` 171K/month | `edifact-rs` 1,931 recent (active 2026-10) | Served. |
| OFX | PyPI `ofxparse` 413K/month | `ofx-rs` 2,819 recent | Served. |
| Passport MRZ | PyPI `mrz` 52K/month | `mrz` 6,300 recent | Served. |
| MT940 bank statements | PyPI `mt-940` 499K/month | `mt940` 2,435 recent, active | Served. |
| HL7 v2 | PyPI `hl7apy` 758K/month | `hl7-parser` 6,645 recent, `hl7` 172, `hl7v2` 120 | Existing, but weak. |
| Number/date i18n | PyPI `babel` 101 million/month | `icu` 703K recent (ICU4X), `num-format` 14 million | Served. |
| Person names | PyPI `nameparser` 3.1 million/month | `human-name` 485K recent | Served. |
| Beancount/ledger | PyPI `beancount` 133K/month | `beancount-rs` 12 recent (2026-07) | Empty but the demand is small. |

### Gaps that are still real, in order

1. **ISO 8583** (card transaction messages). PyPI `pyiso8583` **403,648/month**. In
   Rust: `iso8583` **62 recent (2017, dead)**, `iso8583-rs` **73 recent (2020,
   dead)**, `iso8583-codec-rs` 3 recent. The names `iso8583-msg` and `iso8583kit`
   are **free**. This is the strongest gap: two dead attempts and real demand in
   the payments industry.
2. **Arabic text** (reshaping, harakat, normalisation). PyPI `arabic-reshaper`
   **4.8 million/month**. In Rust: `arabic-reshaper` 1,859 recent (2023),
   `ar-reshaper` 9,241 recent (2024), `arabic_text_utils` 46 recent. Large demand,
   old and fragmented supply. The names `arabic` and `tashkeel` are free.
3. **EN 16931 e-invoice** (Factur-X/ZUGFeRD/UBL/XRechnung). PyPI `factur-x`
   287K/month. In Rust: `en16931-formats` 5,417 recent (1 revdep, active),
   `en16931` 5,586 recent, `zugferd` 159 recent, `xrechnung` 5 recent. Medium
   demand, growing because of EU regulation.
4. **Quran text** (verse lookup, mushaf pages, tajwid). Small in every ecosystem
   (npm `quran-json` 2,827/week, PyPI `quran` 608/month), Rust `quran` 2 recent
   (2022). A real gap but a narrow niche. The names `quran-rs`, `tajweed`,
   `mushaf` are free.

### Method notes

- **Even `mt103`, which sounded certain to be empty, already exists.** The
  `swift-mt-message` crate only appeared after I searched for the phrase "swift mt
  message", not the crate name. Searching by **capability**, not by name, is
  mandatory.
- A free crate name does not mean the gap is real: `mt103` is free, but the MT103
  capability is already handled by `swift-mt-message`. Name availability is
  necessary, not sufficient.
- A pattern repeated across three rounds: **generic capability is almost always
  already served; what is empty is capability tied to a specific domain** (card
  payments, EU fiscal regulation, Arabic/Quran text). That is the right direction
  for the next round.

---

## Addendum 2026-10-21 (fourth): 5 final candidates

I probed ~45 more capabilities in five batches, with an emphasis on areas I had
**never** touched: financial protocols, transit, bioinformatics, chemistry,
automotive, medical codes, and non-Latin script processing.

### What fell away this round (do not build)

| Capability | Demand | Why it fell away |
|---|---|---|
| Cyrillic transliteration | PyPI `cyrtranslit` 103K/month | **`transliteratio` exists** (236 recent, active 2026-10-19, already covering the ISO **and** GOST tables), plus `translit` 2,651, `iuliia`, `cranberry`. Served. |
| Thai word segmentation | PyPI `pythainlp` 658K/month | `kham-core` (464 recent, pure Rust), `thaibreak` 125, `nlpo3` 5,155. Served. |
| EMVCo payment QR | npm `qrcode-generator` 2.9 million/week | `raast-qr` 227 recent (#![no_std]), `promptparse` 232, `brcode` 62, `pix-brcode-parser`. Served. |
| FIX protocol | PyPI `quickfix` 25K/month | The `minifix*` family (2026-10-08) plus the `quickfix` bindings 162K recent. Existing. |
| ACH/NACHA | PyPI `ach` 11K/month | Small demand (`nacha` PyPI 444/month). Not worth it. |
| GTFS transit | PyPI `gtfs-realtime-bindings` 647K/month | `gtfs-structures` 24K recent plus `gtfs-rt`. Served. |
| Shapefile GIS | PyPI `pyshp` 3.6 million/month | `shapefile` 160K recent, 37 revdeps. Served. |
| FASTA/FASTQ/VCF bio | PyPI `biopython` 4.4 million/month | `bio` 114K recent, `noodles` 113K, 141 revdeps. Strongly served. |
| SMILES chemistry | PyPI `rdkit` 4.7 million/month | `purr` 13K, `chemcore`, `rdkit` bindings. Half served, needs FFI. |
| CAN DBC automotive | PyPI `cantools` 881K/month | `can-dbc` 175K recent, 13 revdeps. Served. |
| ICD-10/SNOMED/FHIR | PyPI `fhir.resources` 2.5 million/month | `snomed` 900, `loinc` 227, `fhir` 530 (all active 2026-10). Served. |
| Mandarin pinyin | PyPI `pypinyin` 1.6 million/month | `pinyin` 297K recent, 34 revdeps. Served. |
| Mandarin word segmentation | PyPI `jieba` 4.8 million/month | `jieba-rs` **1.77 million recent**, 59 revdeps. Strongly served. |
| Simplified/traditional Hanzi | PyPI `opencc` 650K/month | `zhconv` 20K recent plus `opencc-rs`. Served. |
| Japanese kana/romaji | PyPI `pykakasi` 2.2 million/month | `kakasi` 10.5K recent, `furigana`, `romaji`. Served. |
| Korean Hangul | PyPI `hangul-romanize` 93K/month | `hangul` 4,750 recent. Served. |
| XBRL | PyPI `xbrl` 19K/month | `xbrl-rs` 258 recent (active 2026-10). Served. |
| SMPP SMS | PyPI `smpplib` 55K/month | `smpp34` 2,762 recent (pure Rust, active). Served. |
| RADIUS/Diameter | PyPI `pyrad` 256K/month | `radius` 15K recent, `diameter` 512. Served. |

### The 5 final candidates

Ordered by the ratio of demand to supply, then by how clear the gap is.

**1. ISO 8583 — card transaction messages (ATM/EDC/switching)**
- Demand: PyPI `pyiso8583` **403,648/month**.
- Supply: `iso8583` 62 recent (2017, dead), `iso8583-rs` 73 recent (2020, dead),
  `iso8583-codec-rs` 3 recent, `iso8583-core` 12 recent (2026-02). **Four
  attempts, all at or below 73 recent.** None is used.
- Names: `iso8583-msg` **free**, `iso8583kit` **free**.
- Why it fits: the specification is public and stable, table-driven (bitmap plus
  fixed/LLVAR/LLLVAR data types), with no "which authority" problem. Closest to
  `hijridate` in the shape of the work, but a much larger market.

**2. Arabic text — reshaping, harakat, normalisation**
- Demand: PyPI `arabic-reshaper` **4.8 million/month**; npm
  `arabic-persian-reshaper` 42,084/week plus `arabic-reshaper` 14,329/week.
- Supply: `ar-reshaper` 9,241 recent (2024), `arabic-reshaper` 1,859 (2023),
  `arabic_text_utils` 46, `dhad` 86, `arabic-normalize` 9. **Fragmented and old;
  none dominates.** Even the largest (9K) is 25x smaller than pinyin.
- Names: `arabic` **free**, `tashkeel` **free**.
- Why it fits: the largest demand of any gap I found, and it is synergistic with
  `hijridate` (one domain: Arabic text plus the Hijri calendar).

**3. EN 16931 e-invoice — Factur-X / ZUGFeRD / UBL / XRechnung**
- Demand: PyPI `factur-x` **286,877/month**.
- Supply: `en16931-formats` 5,417 recent (1 revdep, active), `en16931` 5,586,
  `zugferd` 159, `xrechnung` 5. **A single player, still small.**
- Name: `factur-x` **free**.
- Why it fits: EU regulation (EN 16931 is mandatory for public procurement)
  forces adoption. It needs `xmlkit` from this workspace, so there is internal
  synergy.

**4. European VAT number validation**
- Demand: npm `jsvat` **50,847/week**; `python-stdnum` (which contains it)
  **5.08 million/month**.
- Supply: the `vat` crate **4 recent (2016, dead)**, nothing else. The names
  `eu-vat` **free**, `vat-validator` **free**.
- Why it fits: **direct synergy with the existing `stdnum-rs`** — this is an
  added module, not a crate from scratch. Per-country checksums plus formats,
  purely a table of rules, std-only. The lightest work of the five.

**5. Indic script transliteration (Devanagari and others)**
- Demand: PyPI `indic-transliteration` **69,267/month**; npm `sanscript`
  84/week.
- Supply: **NONE.** There is no Indic transliteration crate in Rust at all
  (`indic-transliteration`, `devanagari`, `sanscript`, `indic` are all free and
  have no counterpart).
- Names: `indic-transliteration` **free**, `devanagari` **free**, `sanscript`
  **free**.
- Why it fits: the cleanest gap — genuinely empty, not "exists but old". Medium
  but real demand. Important note: Cyrillic transliteration **is already served**
  (`transliteratio` covers ISO/GOST), so do not generalise to "all transliteration
  is empty"; what is empty is specifically Indic.

### An additional note: Indonesian gaps with no competitor

`pph21`, `bpjs`, `pajak`, `ppn`, `efaktur` are **all free** with **zero**
competitors on crates.io. The problem: there is no npm/PyPI package that can be
used to measure demand (all of them `None`), so this is "build it and hope it gets
used" — not a measurable gap. Still relevant for the cipansor/pesantren context,
but I did not put it in the top five because the demand is unproven.

### Method notes

- **"No crate at all" is stronger than "a crate but old".** Cyrillic looked empty
  until I searched the phrase "gost transliteration" and found the active
  `transliteratio`. Indic stayed empty even after being searched.
- The search has to use **capability phrases**, not crate names.
  `iso8583-core` and `swift-mt-message` did not appear from a name search.
- The pattern of these three rounds is consistent: **generic served, domain-bound
  empty.** All five candidates above are domain-bound (card payments, Arabic
  script, EU fiscal regulation, Indic script).

---

## Addendum 2026-10-21 (fifth): critical dependencies, and three corrections

This round had a different focus: not "what capability do end users use", but
**critical dependencies** — the layer that has to exist before other crates can
work. The method is the same (demand from PyPI/npm, supply from crates.io with
`recent_downloads` plus reverse dependencies), but what is sought is
infrastructure rather than tools.

### Three corrections to earlier notes

| Earlier claim | Reality (measured today) |
|---|---|
| **Indic transliteration: "NONE"** | **Wrong.** `shlesha` **3,394 recent** (2026-07-06, "extensible transliteration library with hub"), `vidyut-lipi` 636, `lipilekhika` 31, `transliterator` 3. So there are players, though small. The names `indic`, `devanagari`, `sanscript`, `itrans`, `velthuis`, `aksharamukha` stay free. |
| **Package URL (purl) looked empty** | **Wrong.** `purl` **1,623,677 recent**, `packageurl` 148,497, `purl_validator` 1,361. Heavily served. (PyPI `packageurl-python` 32 million/month.) |
| **IPP/CUPS printing looked empty** | **Wrong.** `ipp` **25,800 recent** (2026-08-12, "asynchronous IPP print protocol"), `cups-rs` 9,105, `cups` 179. Already served. Only `ipp-client` (2 recent) is still empty. |

The same lesson repeats: **a similar-looking crate name deceives, and the search
has to use capability phrases.** `purl-rs` turned out to be an HTTP CLI, not
Package URL.

### What turned out to be already served (do not build)

| Capability | Demand | Rust supply | Verdict |
|---|---|---|---|
| XML Signature + XML Encryption | `xmlsec` 8.5 million/month, `signxml` 2.3 million/month | `xml-sec` **43,682 recent** (2026-10-21), `bergshamra-dsig` 187K, `bergshamra-enc` 101K | Strongly served |
| PKCS#11 / HSM | - | `cryptoki` **1.43 million recent**, 10 revdeps | Strongly served |
| Kerberos / GSSAPI | - | `krb5` 432 recent (2026-10-16, pure Rust), `kerberos-parser` 129K | Served |
| LDAP | - | `ldap3` **1.34 million recent**, 10 revdeps | Strongly served |
| eBPF / pcap | - | `rbpf` 62K, `pcap-file` 2.88 million, `pcap` 1.13 million | Served |
| XSD / RELAX NG / Schematron | - | `uppsala` **230,774 recent** (XSD), `relax-ng` 2,327, `schematron-engine` 2,234 | Served |
| XSLT / XPath | - | `xrust` 13,923, `oxixml-xpath` 23,739 | Served |
| HDF5 / NetCDF | - | `hdf5-metno` 714K, `hdf5-pure` 121K, `netcdf` 54K | Served |
| COBOL copybook / EBCDIC | - | `copybook` 220, `copybook-charset` 596 (2026-09-17) | Existing, small |
| COBOL copybook / EBCDIC | - | the `copybook-*` family, active 2026-09 | Existing |
| Modbus / OPC UA / DNP3 | - | `modbus` 34K, `async-opcua` 104K, `dnp3` 16,967 | Served |
| Unicode collation | - | `feruca` 14,615, `colligo` 7,782, `icu_collator` 5.36 million | Served |
| DICOM plus DIMSE networking | - | `dicom` 307K, `dicom-ul` 336K | Served |
| SM2/SM3/SM4 (China) | `gmssl` 116K/month | `gmcrypto-core` 1,889 (2026-10-09, pure Rust) | Existing, thin |
| GOST (Russia) | `gostcrypto` 12K/month | `gost-crypto` 288, `kuznyechik` 15,423, `magma` 13,765 | Existing |

### Two real critical-dependency gaps

**A. PDF/A plus XMP conformance.** This is **the missing half of `factur-x`**,
which already exists in this workspace. `factur-x` produces the XML; to actually
send the invoice, that XML has to be embedded in PDF/A-3 with correct XMP
metadata. Without that, our crate is only half useful.

- Demand: PyPI `pikepdf` **9.48 million/month**, `pypdf` **126 million/month**,
  `reportlab` **66 million/month**, `ocrmypdf` 1.08 million/month.
- Supply: `lopdf` 12.55 million recent and `pdf-writer` 3.44 million recent are
  indeed large, **but both are low-level PDF writers with no concept of PDF/A.**
  What handles conformance: `pdf-compliance` **235 recent**, `pdfpurr` 1,669,
  `page_validation` 716, `pdfluent` 138. None is used.
- The names `pdfa` and `pdf-a` are **free**.
- Note: the *checker* side (PDF/A profile rules over an already-parsed PDF) is far
  easier than the writer, and that is exactly what veraPDF provides. It can be
  built on top of `lopdf` or `pdf-writer`.

**B. WS-Security for SOAP.** This is **the missing half of `soap-client`** in this
workspace. Every bank, government and SAP SOAP endpoint requires WS-Security
(UsernameToken, X.509 signing, timestamp). Without it `soap-client` cannot talk to
them.

- Demand: PyPI `zeep` **29.4 million/month**, `xmlsec` 8.5 million/month,
  `signxml` 2.3 million/month.
- Supply: `xml-sec` 43,682 recent already provides XMLDSig, XMLEnc and C14N as the
  foundation. **WS-Security itself is empty:** `parco-ws-security` **11 recent** is
  the only attempt. The name `wssec` is **free**.
- This is the clearest critical dependency: the foundation exists, the layer above
  it does not.

### Three domain gaps that are still real

**C. HL7 v2 plus MLLP.** How every hospital and laboratory exchanges data.
- Demand: PyPI `hl7apy` **758K/month**.
- Supply is fragmented and weak: `hl7-parser` 6,645 recent (2025-02, its
  description admits it "does not do" parts), `hl7v2-parser` 106 (3 revdeps),
  `hl7` 172, `hl7-v2` 62, `hl7-mllp` 76, `hl7v2-mllp` 166, `vyna-hl7-mllp` 249.
  **No leader**, and the MLLP transport layer is spread across three small crates.
- Synergy: a delimiter-based format, very similar to the existing `iso8583-msg`.

**D. Indic script transliteration.** (Corrected: `shlesha` at 3,394 recent exists,
so this is no longer empty — but 3.4K is still small.)
- Demand: PyPI `indic-transliteration` 69,267/month.
- Supply: `shlesha` 3,394, `vidyut-lipi` 636, `lipilekhika` 31.
- Names free: `indic`, `devanagari`, `sanscript`, `itrans`, `velthuis`.
- Synergy: pure table work, like `arabic`.

**E. ZPL II (Zebra label printers).** Warehousing and logistics.
- Demand: PyPI `zpl` 46,313/month, `zebra` 13,050/month.
- Supply: `zpl_toolchain_core` 311, `zpl` 159, `zpl-builder` 154, all small and
  none dominant.
- The name `epl2` is **free**. A text-based command language, easy.

### Method notes (fifth)

- Reverse dependencies remain the best signal, but **a large crate does not mean
  the gap is closed**: `lopdf` at 12.55 million recent does not mean PDF/A is
  served, because `lopdf` does not handle conformance at all. Ask *which layer* is
  served, not *which category*.
- The pattern of four rounds holds: **generic served, domain-bound empty.** What
  is new this round: **a conformance/standards layer on top of an existing
  foundation** is also empty (`pdfa` on top of `lopdf`, `wssec` on top of
  `xml-sec`). That is the most productive direction for the next round.
