# Gap scan round 9: npm and PyPI demand against the Rust registry (2026-10-21)

Round 8 worked from the PyPI list. This round adds the **npm** side, which the
earlier rounds did not measure, and re-checks round 8's two survivors against the
registry as it stands today.

Every number below was fetched on 2026-10-21: weekly downloads from
`api.npmjs.org`, monthly from `pypistats.org`, and crate age/downloads from
`crates.io/api/v1/crates`. A name is only called a gap if the demand is large
**and** the Rust side is absent or has no dependents.

## What the npm top list says

| npm package | weekly | what it does | Rust side today |
| --- | --- | --- | --- |
| `ajv` | 233,123,786 | JSON Schema validation | `jsonschema` 107M downloads. Served. |
| `fast-xml-parser` | 47,022,431 | XML parse | `quick-xml`. Served. |
| `handlebars` | 24,592,894 | templates | `handlebars`. Served. |
| `node-forge` | 23,072,243 | crypto | `rsa`/`sha2`/`p256`. Served. |
| `qrcode` | 18,476,870 | QR generation | `qrcode-generator` 5.7M. Served. |
| `i18next` | 14,054,696 | i18n | `rust-i18n`. Served. |
| `csv-parse` | 11,567,708 | CSV | `csv`. Served. |
| `pdf-lib` | 11,473,039 | PDF authoring | `pdfmake` (this workspace), `lopdf`. Served. |
| `exceljs` | 10,786,375 | XLSX read/write | `calamine` 15.9M, `rust_xlsxwriter`. Served. |
| `xmlbuilder2` | 10,637,478 | XML build | `quick-xml` writer. Served. |
| `jspdf` | 9,412,068 | PDF in the browser | browser-side; not a Rust problem. |
| `pdfkit` | 6,603,086 | PDF generation | `pdfmake`. Served. |
| `mammoth` | 6,153,762 | DOCX to HTML | `undoc` 26,802, `zavora-docx-html`. Served. |
| `openapi3-ts` | 6,555,744 | OpenAPI types | `utoipa`, `openapi`. Served. |
| `mailparser` | 3,395,866 | email parse | `mail-parser` 4.6M. Served. |
| `docx` | 4,345,180 | DOCX authoring | `docx-rs` 3.7M, `zavora-docx`. Served. |
| `node-cron` | 2,983,185 | cron | `cron`, `tokio-cron-scheduler`. Served. |
| `xml-crypto` | 2,970,240 | XML DSig | `bergshamra-dsig` 224,202, `xml-sec`. Served. |
| `pptxgenjs` | 2,360,543 | PPTX authoring | `ppt-rs` 102,072, `pptx` 21,595. Served. |
| `i18n-iso-countries` | 1,595,921 | ISO 3166 names | `isocountry` 6.1M. Served. |
| `node-sql-parser` | 1,247,590 | SQL parse | `sqlparser` 100M+. Served. |
| `docxtemplater` | 674,815 | DOCX templating | `docxtplrs` 709 (2026-10-22), `xlsx-handlebars`. Thin but present. |
| `ical-generator` | 405,838 | iCal | `icalendar` 994,569, `ical` 1.67M. Served. |
| `html-to-docx` | 140,364 | HTML to DOCX | thin. Niche. |
| `iban` | 99,902 | IBAN validation | `iban_validate`; `stdnum-rs` covers it. Served. |
| `edifact` | 2,864 | EDIFACT parse | `edifact-rs` 3,171, `edifact-types` 10,769. Present, small. |

**Conclusion from npm: the list is served.** Every package with millions of weekly
downloads has a Rust counterpart, usually a mature one. This is the same result
round 8 got from PyPI, and it is worth stating plainly: "the npm top 100 has no
Rust crate" is not true, and looking there is a dead end.

## Round 8's two survivors, re-checked today

### PAdES verification — now built

Round 8's #1 is implemented in this workspace as `crates/pades` (commit 956f7ee).
Re-checking the incumbent: `pdf_signer` and `underskrift` produce signatures,
`podpis-rs` (51 downloads, 2026-10-06) verifies both XAdES and PAdES. The
demand is the largest measured anywhere in these rounds (PyPI `pyhanko`
5,042,503/month, `signxml` 2,292,365/month; npm `xml-crypto` 2,970,240/week).
That combination — real demand, one tiny incumbent — is what made it #1, and
building it did not change the reasoning.

### X12 HIPAA — the picture changed

Round 8 said "the generic parser exists (`edi`), the transaction semantics do
not." That is now only half true:

| crate | downloads | updated | what it is |
| --- | --- | --- | --- |
| `x12-types` | 25,818 | 2026-06-25 | Bindings for the ASC X12 EDI definitions |
| `autogen-stedi` | 16,552 | 2026-09-17 | Auto-generated client for a commercial healthcare EDI API |
| `edi-cli` | 7,572 | 2026-06-24 | CLI over EDI files |

So the **grammar and the type bindings are now covered**, and a commercial vendor
(Stedi) publishes generated Rust. The remaining gap is narrower and clearer than
round 8 stated: **semantic validation of 837/835/271** — does this claim satisfy
the HIPAA implementation guide, are the loops and segments required for this
transaction present, is the code set correct. That is a validation crate, not a
parser crate, and it is the same shape as this workspace's `iso8583-msg` and
`ebics`. Demand is PyPI `pyx12`-class (the round 8 number, 307,648/month) and the
audience is regulated and on Python and Java.

## Four areas this round tested that earlier rounds did not

| area | demand | Rust side | verdict |
| --- | --- | --- | --- |
| SAS / Stata / SPSS files | PyPI `pyreadstat`-class | `sas7bdat` 6,786, `polars-readstat-rs` 4,024 | Present, and Polars is behind it. Not a gap. |
| PDF table extraction | npm `pdf-table-extract`-class | `pdf-extract` **6,790,423**, `pdf-text-extract` 41,526, `pdfsink-rs` 11,784 | Served by a very popular crate. Not a gap. |
| ZPL label printing | npm `zpl` small | `zpl` 160, `zpl_toolchain_*` | Present but tiny; demand is also tiny. Empty-and-unwanted. |
| XBRL | PyPI `arelle` 3,063/month | `xbrl-rs` 411, `crabrl` 1,416, `xbrlkit` 29 | Present but young, and demand is low. Watch, do not build. |

Two of these are worth recording as **rejections** so they are not re-proposed:
PDF table extraction (a 6.8M-download incumbent) and SAS/Stata/SPSS (Polars
covers it).

## What this round concludes

1. **The npm and PyPI top lists are both served.** A gap is not found by scanning
   a popularity list; it is found by taking a *standard* that a regulated
   industry is obliged to implement and checking whether Rust has it. Every gap
   this workspace has filled (`ebics`, `iso8583-msg`, `factur-x`, `pades`) was
   found that way, not by download rank.
2. **The one live candidate from round 8 is X12 HIPAA semantic validation**, and
   its shape is now precise: `x12-types` gives the grammar, so build the
   implementation-guide validation on top rather than a parser from scratch.
3. **XBRL is the runner-up to watch**, not to build: real standard, real
   obligation (SEC and EU filings), but the demand measured so far is small and
   three young crates have already entered.
