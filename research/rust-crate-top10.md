# Top 10 Rust crate candidates (re-verification, 2026-10-21)

Second round. Every figure was read from the registry APIs today. The supply
metric is still crates.io `recent_downloads` (90 days), but this time each
candidate was also checked on GitHub
(`api.github.com/search/repositories`) to find WIP projects, and the demand
figures were refetched from pypistats and npm.

## Important corrections to the first round

Three claims from the first round turned out to be wrong on re-checking:

1. **PDF table extraction is NOT a gap.** I called it "nothing at all" because a
   crates.io search came back empty. In fact GitHub has at least 5 active
   projects: `pdfsink-rs` (24 stars, updated 2026-10-13), `tablers` (15 stars,
   2026-10-19), `bolivar`, `distillpdf`, `pdfni`. crates.io also has
   `pdf-text-extract` (34.6K recent) and `pdfsink-rs` (6.9K recent). I removed
   this candidate from the list.
2. **CycloneDX SBOM is not a gap.** `cyclonedx-bom` has 1.34 million recent
   downloads and is active (2026-03). I nearly misjudged it by looking only at
   the empty `cyclonedx` crate.
3. **XML Signature is not a gap.** `bergshamra-dsig` 186K recent and `xml-sec`
   43K recent (pure Rust, 2026-10). Already served.

Methodological lesson: low `recent_downloads` does NOT mean no competitor. A new
crate that is not yet published, or that is used through a path dependency, is
invisible in the registry. A GitHub search is mandatory for the top candidates.

## The list of 10

Ordered by the combined strength (real demand x Rust emptiness x feasibility).
"Name free" was verified through the crates.io API.

### 1. `holidays-rs` - public holiday calendars
- Demand: PyPI `holidays` 26.9 million/month, `workalendar` 957K/month.
- Rust today: `holidays` 7.8K recent (v0.1.0, 2023), `public-holidays` v0.0.0.
  GitHub: `mapbox-jp/holidays-rs` 7 stars (2024).
- Why it is worth it: the data is already open (python-holidays, workalendar and
  date-holidays are all open source), so this is data mapping, not algorithm
  research. The names `holidays-rs`, `holiday-calendar`, `multicalendar` and
  `world-holidays` are all free.
- Difficulty: low. The safest choice.

### 2. `soap-client` - SOAP/WSDL client
- Demand: PyPI `zeep` 29.4 million/month, `suds-py3` 771K/month. npm `soap`
  453K/week.
- Rust today: practically empty. `wsdl` 8 recent (2023), `soap` 3 recent,
  `rsoap` 46 recent, `soap-server` 195 recent. No mature client.
- Why it is worth it: banks, government and ERP systems are still full of SOAP.
  There is no alternative at all, so this is the widest gap on the list.
- Difficulty: medium to high (needs code generation from WSDL, WS-Security, MTOM).
- Names free: `soap-client`, `wsdl-client`, `soap-rs`, `rustsoap`.

### 3. `stdnum-rs` - multi-country national ID/VAT/tax number validation
- Demand: PyPI `python-stdnum` 5.08 million/month.
- Rust today: `stdnum` only 16 recent. Fragmented: `dni` (Spain), `idnr`
  (Germany), `ssn` (USA), `amka` (Greece), `br_tax_id` (Brazil), `idsmith`,
  `vat_id_validator` 5.3K. No umbrella crate unifies them.
- Why it is worth it: every country needs NIK/NPWP/VAT validation and every
  project writes it itself. Very well suited to incremental contribution, one
  country per pull request.
- Difficulty: medium, but very measurable and easy to test.
- Names free: `stdnum-rs`, `checkdigits`, `national-id`, `vatcheck`.

### 4. `spreadsheet-formula` - spreadsheet formula evaluator
- Demand: npm `exceljs` 10.8 million/week, `xlsx` 7.2 million/week; PyPI
  `openpyxl` 283 million/month.
- Rust today: `formula` 20 recent (dead since 2022), `xlformula_engine` 2.4K
  (dead since 2023), `formula-lang` 190 recent, `xlstream` 47 recent. All dead
  or immature. Nothing evaluates the full formula language.
- Why it is worth it: `calamine` and `rust_xlsxwriter` are already good at
  reading and writing files, but both stop there. What is missing is the
  evaluation engine.
- Difficulty: medium to high. Start with a subset of functions.
- Names free: `spreadsheet-formula`, `excel-formula`, `formula-eval`,
  `calcengine`.

### 5. `en16931-rs` - European e-invoicing (ZUGFeRD/Factur-X, UBL, PEPPOL)
- Demand: PyPI `factur-x` 287K/month. Regulation: XRechnung is already mandatory
  for B2G invoices in Germany, and other European Union countries are following.
- Rust today: `en16931-formats` 5.5K recent (WIP, repo has only 3 stars),
  `zugferd` 159 recent.
- Why it is worth it: the demand is driven by legal obligation, not preference.
  A market that grows on its own without having to convince anyone.
- Difficulty: medium (data modelling plus business rules, not a hard algorithm).
- Names free: `en16931-rs`, `e-invoice`, `facturx`, `zugferd-rs`.

### 6. `plantuml-rs` - PlantUML renderer
- Demand: PyPI `plantuml` 333K/month, npm `plantuml-encoder` 133K/week.
- Rust today: `puml` 97 recent, `plantuml-little` 671 recent. Note: Mermaid in
  Rust is already served (`mermaid-rs-renderer` 160K recent, `merman` 137K
  recent) — so PlantUML is the only one left empty.
- Difficulty: medium to high (needs a diagram layout engine).
- Names free: `plantuml-rs`, `plantuml-render`.

### 7. `musicxml-rs` - MusicXML
- Demand: PyPI `music21` 522K/month.
- Rust today: `musicxml` 1.8K recent (2024). For MIDI there is already `midly`
  (176K recent), but MusicXML is empty.
- Difficulty: medium. The schema is large but clear.
- Names free: `musicxml-rs`, `music-notation`.

### 8. `subtitles` - SRT/ASS/VTT subtitle toolkit
- Demand: PyPI `srt` 970K/month, `pysubs2` 193K/month; npm `srt-parser-2`
  87K/week.
- Rust today: all dead. `subparse` 4.8K recent (last 2021), `srt` 599 recent
  (2017), `ass-parser` 134 recent, `srt-parser` 269 recent.
- Why it is worth it: an everyday need (media, video pipelines, AI tooling), and
  everything has been abandoned. This is the candidate with the lowest effort per
  value.
- Difficulty: low to medium. Good for a fast first release.
- Names free: `subtitles`, `subtitle-parser`.

### 9. `libdav` alternative / `caldav-client` - CalDAV + CardDAV client
- Demand: PyPI `caldav` 542K/month, `vdirsyncer` 27K/month.
- Rust today: `libdav` 14.5K recent is the only serious player, and that is the
  extent of it. `caldav` 47 recent (2018), `carddav` 9 recent (2018).
- Difficulty: medium.
- Name free: `dav-client`.

### 10. `iso20022` - ISO 20022 payment messages (pain/pacs/camt)
- Demand: PyPI `sepaxml` 312K/month; MT940/MT103 are still widely used in
  banking. The SWIFT migration to ISO 20022 adds pressure.
- Rust today: `sepa` 1.6K recent, `mt940` 2.4K recent, `camt053` 220 recent,
  `iso20022` 21 recent (dead since 2020). WIP: `mx20022` (7 stars), `quackiso`
  (6 stars).
- Difficulty: medium. The XSD schemas are public and can be generated from.
- Names free: `iso20022-rs`, `sepa-rs`, `pain-rs`, `camt`.

## What I deliberately removed

- PDF table extraction - busy WIP on GitHub (see the corrections above).
- CycloneDX SBOM, XML Signature, JSON-LD, HAR, Mermaid, XLSX, DOCX, generic PDF,
  OCR, browser automation, job queues, state machines, OAuth - all have active
  players with tens of thousands to millions of recent downloads.
- Pure-Rust Hunspell - `spellbook` is already 140K recent, so it is not an empty
  gap, even if it could still be improved.
- PII redaction, embeddings/vector DBs - already served.

## Suggested next steps

Start with 1 or 8 to build reputation quickly at low risk, then move to 2 or 4,
which are worth the most but are the heaviest.

Before starting, do two things for the candidate you pick:
1. Run `cargo search <name>` and check GitHub
   `search?q=<topic>+language:rust` again, because this space moves fast.
2. Check whether a crate has been `yank`ed or a repo archived without a
   replacement; that is the fastest adoption opportunity, because the users
   already exist and are looking.
