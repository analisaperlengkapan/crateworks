# Gap scan round 8: the PyPI list, and the two niches that survive it (2026-10-21)

A continuation of `rust-crate-gaps-round7.md`. Every figure below was measured
today through the crates.io, npm, PyPI and pypistats APIs. Tooling:
`research/tools/`.

Round 7 closed the npm top-400 and concluded that the remaining openings are
standards and data sets, not general capability. This round tests that
conclusion against a source round 7 did **not** sweep: the **PyPI top list**.
The npm list is dominated by JavaScript build tooling, which Rust never had to
match. Python is where the *domain* packages live, so it is the harder test.

## Method

Unchanged, and it is the reason most names are a dead end:

1. **Demand in the source ecosystem** (PyPI/month via `pypistats`, npm/week).
2. **`recent_downloads`** of the best Rust crate in the area (90 days).
3. **`reverse_dependencies`** of that crate. The only real Rust signal.

A gap is real when the best crate has `recent_downloads` under about 20,000
**and** near-zero reverse dependencies, while the source counterpart has real
volume. A crate with real downloads but zero dependents is a CLI or a
copy-paste dependency, not a library the ecosystem has adopted.

## The PyPI top list is served, exactly like the npm top-400

160 of the highest-download PyPI packages were looked up, and for each one the
plausible Rust counterpart was measured. Every area has an incumbent at scale:

| Area | PyPI counterpart | Best Rust crate | recent |
|---|---|---|---|
| Recurrence rules | `python-dateutil` | `rrule` | 639,823 |
| Phone numbers | `phonenumbers` | `phonenumber` | 3,196,305 |
| Time zones | `pytz` | `chrono-tz` | 45,118,358 |
| Holidays | `holidays` | `holidays` | 7,829 (+ this workspace's `holidays-rs`) |
| Barcodes | `python-barcode` | `barcoders` | 163,235 |
| Fuzzy match | `fuzzywuzzy` | `fuzzy-matcher` | served |
| Slugify | `python-slugify` | `slug` | 8,118,193 |
| Markdown | `markdown` | `pulldown-cmark` | 59,083,635 |
| Sanitize HTML | `bleach` | `ammonia` | 6,132,319 |
| Units | `pint` | `uom` | 2,645,463 |
| Money | `py-moneyed` | `rust_decimal` | 39,852,658 |
| Date parsing | `dateparser` | `dateparser` | 2,080,698 |
| Spreadsheets | `openpyxl` | `calamine` / `rust_xlsxwriter` | 5,775,596 / 2,473,268 |
| DOCX | `python-docx` | `docx-rs` | 791,317 |
| PPTX | `python-pptx` (60.2M/month) | `pptx` | 17,762 |
| EPUB | `ebooklib` | `epub` | 104,135 |
| DICOM | `pydicom` | `dicom` | 305,170 |
| FHIR | `fhir.resources` | `fhir` | 639 |
| Sympy | `sympy` | `symbolica` | 3,474 |
| NetworkX | `networkx` | `petgraph` | 126,904,513 |
| Arrow | `pyarrow` | `arrow` | 23,850,307 |
| Plotting | `matplotlib` | `plotters` | 59,349,022 |
| ASN.1 | `asn1crypto` (133M/month) | `rasn` / `der` | 3,081,269 / 4,008,937 |

Two entries are worth writing down because they look like gaps and are not:

- **`python-pptx` at 60.2M/month against `pptx` at 17,762.** The Rust number is
  low because the Rust audience writes reports, not slide decks. This is a
  demand-signal mismatch, not a missing capability: the crate exists, is
  maintained, and covers the API.
- **`sympy` at 126M/month against `symbolica` at 3,474.** A CAS is a decade of
  work and `symbolica` is a commercial-backed incumbent. Do not enter.

**Conclusion, now stable across rounds 4-8: there is no popular PyPI or npm
package that Rust lacks.** The rest of this round is the exceptions.

## Candidate A: PAdES / PDF signing with long-term validation

This is the strongest demand signal found in any round, and the weakest Rust
answer.

| | |
|---|---|
| Demand | PyPI `pyhanko` **5,042,503/month**, `pikepdf` 9,480,221/month, `endesive` 115,253/month. npm `@signpdf/signpdf` **168,731/week**, `node-signpdf` 10,287/week. |
| Best Rust crate | `pdf_signer` (10,860 recent, **0 revdeps**, v0.3.2, 2026-09-25). Then `pdf_signing` (1,771, **0 revdeps**, last updated 2024-11-08), `underskrift` (1,060, **0 revdeps**), `pdfluent-sign` (235, 2 revdeps). |
| Shape of the incumbents | `pdf_signer` covers PAdES B-B through B-LTA. `underskrift` advertises the same range. Both are **single-maintainer, zero-dependency** crates with no downstream adoption. `oxidize-pdf` (116,438 recent) lists "signatures" among a dozen features, which means signatures are a feature, not the product. |
| Names free | `pades`, `pdf-pades`, `pdf-signature`, `signpdf`, `pdfsign`, `dsig` - all **FREE**. (`pdf-sign` is taken by an 11-download date extractor; `pdf_signing` is taken by the 2024 crate.) |
| Names taken | `pdf_signer`, `pdf_signing`, `underskrift`, `pdfluent-sign`, `oxidize-pdf` |

The gap is not "no crate". It is that the crate that exists has **no dependents
at all** while the Python equivalent is downloaded five million times a month.
That is the signature of an unadopted library: it works for its author and
nobody has been able to build on it. The specific openings are the parts a
signing library needs and the incumbents skip:

- **A separate verification crate.** `pdfluent-sign` (235 recent) is the only
  pure verifier, and it is brand new. Verifying a signed PDF is a distinct job
  from producing one, and it is what an auditor, an archive or a validator
  actually runs.
- **RFC 3161 timestamping and DSS/LTV.** `tsp-ltv` (195,612 recent) does
  timestamping but is not wired to any PDF crate. The two halves of B-LTA do not
  meet.
- **A round-trip corpus.** Sign with `pdf_signer`, verify with the crate, and
  check the result against `pyhanko`'s output. This is the differential method
  already used for `stdnum-rs`, `hijridate` and `adhan`, and no Rust crate in
  this space advertises it.

The workspace already holds the two neighbours: `pdfa` (PDF/A validation) and
`pdfmake` (PDF generation). A PAdES crate would sit between them and share the
CMS and X.509 work that `cms` (5,541,796 recent) and `x509-cert` (19,677,984
recent) already provide.

## Candidate B: X12 HIPAA healthcare EDI (837 / 835 / 271)

| | |
|---|---|
| Demand | PyPI `pyx12` **307,648/month** (HIPAA X12 validator, parser and converter). npm `x12-parser` **12,721/week**. |
| Best Rust crate | `edi` (58,243 recent, **0 revdeps**, last updated 2025-08-12) is a generic X12 file parser. `x12-types` (387, 2 revdeps) is definition bindings. `serde_x12` (49, 0 revdeps) has not moved since 2023. `x12-parser-client` (13, 0 revdeps) is a **network client** for a paid RapidAPI service, not a library. |
| Shape of the incumbents | `edi` parses the envelope and the segments. What a HIPAA transaction needs is the **transaction semantics**: which segments are required for an 837P claim, which are conditional, which loop they belong to, and what a valid 835 remittance carries. None of that exists in Rust. `edifabric-x12` (15 recent) is a binding to a commercial native library. |
| Names free | `x12-edi`, `x12-rs`, `x12-837`, `hipaa-edi`, `edi-837` - all **FREE**. |
| Names taken | `edi`, `x12`, `x12-types`, `serde_x12`, `edi-cli`, `edifabric-x12` |

The shape is exactly the one that worked for `ebics` and `iso8583-msg` in this
workspace: a fixed published grammar (ASC X12 005010X222A1 for 837, X221A1 for
835), a large regulated industry currently on Python and Java, and an oracle
(`pyx12`) that can label a corpus for differential testing. The generic parser
is served; the **transaction validation** is not, and validation is what a
clearinghouse actually needs.

## Rejected this round, with the number that rejected them

- **ODF (OpenDocument) authoring.** PyPI `odfpy` 5,197,259/month. This was the
  most tempting rejection. Rust has `spreadsheet-ods` (16,560 recent, **7
  revdeps**) and `lo_odf` (3,413, 7 revdeps), so ODS read/write is adopted, and
  `ferrodoc-odt` (387) covers ODT. The demand is real but the capability is
  present and has dependents. Recorded so it is not re-proposed.
- **National e-invoicing regimes.** ZATCA Fatoora (Saudi), MyInvois (Malaysia),
  Coretax/e-Faktur (Indonesia), KSeF (Poland), Verifactu (Spain), SDI/FatturaPA
  (Italy), DIAN (Colombia), SUNAT (Peru), Chile SII. Most names are **free**
  (`zatca`, `fatoora`, `myinvois`, `efaktur`, `coretax`, `fatturapa`, `sdi`,
  `sunat`, `dian`), and round 6 already flagged FatturaPA. This round rejects
  them on **demand**: PyPI `fatoora` is 69/month, `myinvois` 17/month,
  `fatturapa` 165/month, `ksef-client` 5,127/month. Empty *and* no demand is the
  case round 6 rejected with MICR and PCL. FatturaPA stays the one exception
  worth watching, because Italy's SDI is genuinely mandatory at scale.
- **`python-pptx`.** 60.2M/month against `pptx` at 17,762. The crate exists; the
  Rust audience does not write slides. Demand-signal mismatch, not a gap.
- **`sympy`.** 126M/month against `symbolica`. A CAS is out of scope for a
  single-purpose crate.
- **HL7 CDA / C-CDA.** Names free (`hl7-cda`, `cda-rs`, `ccda`, `cda-r2`), but
  the `hl7-rust` org is actively building the whole family: `hl7` (172 recent),
  `hl7-3` (242), `hl7-3-soap` (116), all updated 2026-10-16. Entering now means
  racing a moving incumbent, the case round 5 warned about. This workspace's
  `mllp` covers the transport side, which is the part that is settled.
- **XML Signature / XAdES.** PyPI `signxml` 2,292,365/month, npm `xml-crypto`
  2,970,240/week. Served: `xml-sec` (44,053 recent), `bergshamra-dsig` (191,426),
  `xades` (89), plus this workspace's own `wssec`.
- **NCPDP (pharmacy claims).** crates.io search total **0**, names free. But no
  measurable PyPI or npm demand either - the same empty-and-unwanted case.
- **DLMS/COSEM (smart metering).** PyPI `dlms-cosem` 46,619/month. Served by
  `dlms_cosem` (279 recent) and `spodes-rs` (25); the name `dlms` is free but the
  capability is present. The audience is metering vendors, not crate users.

## Recommendation

Ranked by what is defensible, using the same standard as round 7 (is the oracle
unambiguous, is the incumbent absent rather than merely young):

1. **PAdES verification with LTV, as a crate separate from signing.** The
   largest demand measured in any round (5M/month Python, 168k/week npm), and
   the Rust side has a producer (`pdf_signer`, 10,860 recent) with **zero
   dependents** and no independent verifier. The oracle is `pyhanko`, which can
   both produce signatures to verify and verify signatures this crate produces.
   It extends the workspace's PDF line (`pdfa`, `pdfmake`) and reuses `cms` and
   `x509-cert` rather than reimplementing them. Scope it to **verification** to
   avoid competing with the two existing signers.
2. **X12 HIPAA transaction validation (837/835/271).** 307,648/month on PyPI, a
   published grammar, a regulated industry on Python and Java, and an oracle
   (`pyx12`). The generic parser exists (`edi`); the transaction semantics do
   not. Same shape as `ebics` and `iso8583-msg`.

Both are stronger than round 7's candidates, because in both cases the demand is
an order of magnitude larger and the Rust incumbent has **zero dependents**,
which round 7's candidates did not have measured.

## Note on the pattern across rounds

Rounds 4-8 now agree, and the agreement is worth stating plainly: in 2026 there
is no popular npm or PyPI package that Rust lacks. The remaining openings are
where a **regulated, mandatory** format has a large Python or Java incumbent and
the Rust crates in the space have no dependents. The test is not "is there a
crate" - there almost always is - but "does anything depend on it". A crate with
10,000 downloads and 0 reverse dependencies is a gap wearing a crate's name.
