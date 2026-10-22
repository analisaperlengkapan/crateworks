# Gap scan round 6: the tail of the npm list, and the regulated-document niche (2026-10-21)

A continuation of `rust-crate-gaps-round5.md`. Every figure below was measured
today through the crates.io, npm and PyPI APIs. Tooling: `research/tools/`.

This round went at the question from two directions that the earlier rounds did
not use:

1. **The whole npm top-400 by dependent count**, not a curated shortlist. The
   list came from the `npm-high-impact` package (17,338 names), and each name was
   looked up on crates.io under its exact spelling.
2. **A narrow sweep of the regulated-document and industrial-protocol space**,
   which is where this workspace already lives (`factur-x`, `iso20022-rs`,
   `stdnum-rs`, `pdfa`, `soap-client`).

## Method

Unchanged from round 4, and it is the reason most of the npm list is a dead end:

1. **Demand in the source ecosystem** (npm/week or PyPI/month). Shows the problem
   is real; shows nothing about Rust.
2. **`recent_downloads` of the best Rust crate in the area** (90 days).
3. **`reverse_dependencies` of that crate.** The only real Rust signal.

A gap is real when the best crate has `recent_downloads` under about 20,000
**and** near-zero reverse dependencies, while the source-ecosystem counterpart
has real volume.

## Result of the npm top-400 scan: nothing

211 of the 400 top npm names have **no crate under the exact same name**. That
number is a trap, and it is worth writing down so it is not re-run:

- Many are served under a different name. `minimatch` -> `globset` (2.7M recent),
  `brace-expansion` -> the same, `lru-cache` -> `lru` (LruCache is std now),
  `chalk`/`ansi-styles` -> `anstyle` (839M recent), `picomatch` -> `globset`,
  `debug` -> `log` + `env_logger`, `ajv`/`jsonschema` -> `jsonschema`
  (107M recent), `emoji-regex` -> `emojis` (4.7M recent), `tough-cookie` ->
  `cookie_store` (91M recent), `strip-ansi` -> `strip-ansi-escapes` (59M recent),
  `iconv-lite` -> `encoding_rs`, `yaml` -> `serde_yaml`/`yaml-rust2`,
  `ms` -> `humantime`, `commander` -> `clap`.
- The rest are **build tooling for JavaScript itself**: `eslint`, `jest`,
  `postcss`, `webpack`, `browserslist`, `istanbul-lib-instrument`, `parse5`,
  `acorn`, `es-abstract`, `rxjs`. Rust cannot serve these and has no reason to
  try; where it does, Oxc, SWC, Biome and Rspack already own the space.

Verdict: the npm top-400 is fully served. Do not scan it again.

## What the niche sweep found

### Gap 1, the real one: EBICS

| | |
|---|---|
| Demand | PyPI `sepaxml` **311,525/month**; npm `ebics-client` 3,536/week. EBICS is the B2B banking channel used across DE, FR, CH, AT, IT, NL, CZ, HU and more. |
| Best Rust crate | **none.** `crates.io?q=EBICS` returns **total 0**. The exact names `ebics`, `ebics-client` and `ebics-rs` are all **FREE**. |
| The field | Empty. Not thin, empty. |

This is the strongest result of the round. Every European corporate treasurer
talks to their bank over EBICS, and the libraries are Java (the reference
implementation, `org.kapott.hbci`/ebics-java), Python (`ebics-client`) and
TypeScript (`node-ebics-client`). There is no Rust one.

Scope that fits this workspace: the EBICS 3.0 XML schema (H000, H004, H005
envelopes), the three order types that matter (`INI`, `HIA`, `HPB`, plus
`HKD`/`HAC`/`HPD` for statements), the **E002 key letter** for signature
classes, and the cryptography. The crypto is `rsa` + `sha2` + `aes` in ECB mode
with PKCS#7 padding, all of which are std-adjacent and available.

Why it is not a certain win: EBICS is a *banking connection*, so a crate is only
useful if a bank will accept it, and banks certify clients. A pure Rust library
that a service embeds to *parse* and *build* EBICS messages is the honest scope;
being a certified client is not. That is the same shape as `soap-client`, which
this workspace already ships.

### Gap 2: MICR E-13B and PCL are both literally empty

| | |
|---|---|
| MICR E-13B (cheque line) | `crates.io?q=MICR` total **0**; exact names `micr` and `micr-e13b` **FREE**. PyPI has `micr` with no published stats. |
| PCL6 / PCL5 printer language | `crates.io?q=PCL` total **0**; exact name `pcl6` **FREE** (`pcl` is taken by a dead 2024 crate). |
| FatturaPA (Italian e-invoice) | `crates.io?q=FatturaPA` total **0**; exact name `fattura-elettronica` **FREE**. |

These are genuinely empty, but the honest reading is that they are empty because
the demand is small, not because nobody thought of them. Recorded so they are
not re-discovered as "gaps" a third time. FatturaPA is the most interesting of
the three: Italy mandates it for every invoice, but the Rust-side audience is a
handful of integrators, and `factur-x` already covers the EU hybrid-invoice shape.

### Near-misses, measured, and why they are not gaps

| Capability | Best Rust crate | recent | revdeps | Verdict |
|---|---|---|---|---|
| SWIFT MT103 | `swift-mt-message` | 3,928 | ? | Served, and `mx20022` is adding MT<->MX translation |
| ISO 20022 payments | `sepa` | 1,594 | 1 | Thin but alive, updated 2026-10-13 |
| CAMT.053 statements | `camt053` | 220 | 0 | New (2026-07), name taken, one author |
| FacturaE (Spain) | `factura`/`sunat_cpe` | 7 / 32 | 0 | Effectively empty; `facturae` name FREE |
| NF-e (Brazil) | `fiscal` | 217 | 0 | One crate, updated 2026-07 |
| KSeF (Poland) | `ksef` | 24 | 0 | Three crates, all under 1,000 recent |
| Verifactu (Spain) | `verifactu` | 47 | 0 | One crate, API client only |
| OFX / QIF | `ofx-rs`/`ofxy`/`qif` | 2,902 / 1,269 / 69 | 1 / 0 / 0 | Served |
| AdES (CAdES/PAdES/XAdES) | `ades-rs`/`xades` | 79 / 89 | 0 / 2 | Young; `pades` name is FREE |
| EN 16931 / Factur-X / Peppol | `en16931`/`zugferd`/`core-invoice` | 5,661 / 159 / 237 | 0-4 | **Served and active**; `facturx`, `peppol`, `ubl`, `cii` names FREE |
| MRZ (ICAO 9303) | `mrz` | 6,402 | 0 | Served |
| HL7 v2 | `hl7` | 172 | 0 | Served, updated 2026-10-16 |
| SMPP | `rusmppc` | 2,802 | ? | Served; PyPI `smpplib` 55K/month |
| SS7 / SIGTRAN | none | 0 | 0 | Empty; the name `ss7` is FREE. Telephony signalling, tiny audience |
| PROFINET | none usable | 0 | 0 | Empty; `profinet` FREE. Needs a real-time stack, not a crate |
| CANopen / J1939 / IEC 60870 | `canopen`, `j1939`, `iec60870-5` | 2,049 / 31,729 / 7,118 | 0-9 | Served |
| GS1 / GTIN | `gs1`, `gtin` | 270 / 5 | 3 / 0 | `gs1` is stale (2024) but present |
| EMVCo QR | `emvco`, `pix-br`, `promptparse` | 5 / 0 / 1,061 | 0 | Fragmented and tiny |
| PDF/A, PDF/UA | already in this workspace (`pdfa`) | - | - | Shipped |

## Recommended next crate

**EBICS.** It is the only entry in this round that is empty *and* has measurable
demand from a large, regulated industry that is currently on Java and Python.
The name is free, the scope is a schema plus crypto (the workspace's shape), and
the oracle for differential testing is `ebics-client` on PyPI, which is exactly
the method used for `stdnum-rs`, `hijridate` and `adhan`.

If EBICS is judged too bank-specific, the next is **FatturaPA**, which is empty
with a free name, and where `factur-x` in this workspace already provides the
XML-and-signature scaffolding to copy.

## Rejected, with the number that rejected them

- **The npm top-400.** 211 name misses, zero real gaps. Every miss is either
  served under another name or is JavaScript build tooling.
- **PCL6, MICR E-13B.** Empty, free names, no measurable demand.
- **SS7, PROFINET.** Empty, but they need real-time or hardware, not a pure crate.
- **ISO 20022 / Peppol / EN 16931.** Already served by active crates
  (`en16931` 5,661 recent, `core-invoice`, `asx-rs` 1,549 for AS4). A new crate
  here competes with a moving incumbent, which is the case round 5 warned about.
