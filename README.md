# crateworks

A workspace of small, single-purpose Rust crates that fill gaps where the
JavaScript and Python ecosystems have mature libraries and Rust does not.

Every crate here follows the same rules:

- No dependencies beyond the standard library unless a dependency is genuinely
  necessary, so a crate can be dropped into any project.
- Checksum and structural validation only. Nothing contacts a network service
  or a registry.
- Rules over tables. A calendar or a validator is expressed as rules that hold
  for any year or any input, not as a data table that goes stale.
- Errors are typed enums with a `Display` implementation, never a string.
- Public API documented, with a doctest on every crate root.

## Crates

Seventeen crates, published independently. Most have no dependencies at all; the
rest depend only on `xmlkit` from this same workspace.

### XML and the document stacks built on it

| Crate | What it does | Why it exists |
| --- | --- | --- |
| [`xmlkit`](crates/xmlkit) | Small XML parser, tree and serialiser | Foundation for the SOAP and ISO 20022 crates; no `quick-xml` dependency needed |
| [`iso20022-rs`](crates/iso20022-rs) | `camt`, `pain` and `pacs` payment messages | The one crate on crates.io is from 2020 and dead |
| [`soap-client`](crates/soap-client) | SOAP 1.1/1.2 envelopes and WSDL reading | Every SOAP crate on crates.io has zero reverse dependencies |
| [`wssec`](crates/wssec) | WS-Security headers: UsernameToken, timestamp, password digests | The missing half of `soap-client`: every bank and government SOAP endpoint requires it, and the only attempt on crates.io has eleven downloads |
| [`factur-x`](crates/factur-x) | Factur-X and ZUGFeRD e-invoices, EN 16931 in CII and UBL | PyPI `factur-x` is downloaded hundreds of thousands of times a month; the Rust crate in the area has a few thousand downloads |
| [`pdfa`](crates/pdfa) | PDF/A conformance, XMP metadata and the Factur-X container | The other missing half: the EN 16931 XML has to travel inside a PDF/A-3 document, and the Rust PDF writers have no notion of conformance |

### Validation

| Crate | What it does | Why it exists |
| --- | --- | --- |
| [`stdnum-rs`](crates/stdnum-rs) | IBAN, ISBN, EAN/UPC, Luhn and national ID validation | Rust has only fragmented per-country crates and no shared entry point |
| [`eu-vat`](crates/eu-vat) | EU VAT number validation, 27 member states plus Northern Ireland | The `vat` crate stopped in 2016 and nothing replaced it |

### Dates, calendars and text

| Crate | What it does | Why it exists |
| --- | --- | --- |
| [`holidays-rs`](crates/holidays-rs) | Public holiday calendars and working-day arithmetic | `holidays` on crates.io has a single 2023 release and an abandoned repo |
| [`hijridate`](crates/hijridate) | Hijri/Gregorian conversion on the Umm al-Qura calendar | `hijri-converter` on PyPI has millions of monthly downloads; the only Rust Hijri crates are a CLI and a 2018 stub |
| [`adhan`](crates/adhan) | Prayer times from a date and coordinates | The same domain as `hijridate`; the Rust space is split across an 848-download crate and a 2023 stub |
| [`arabic`](crates/arabic) | Arabic shaping: contextual forms, ligatures, harakat, normalisation | The Rust reshaping crates have under ten thousand downloads between them |

### Data formats

| Crate | What it does | Why it exists |
| --- | --- | --- |
| [`spreadsheet-formula`](crates/spreadsheet-formula) | Spreadsheet formula parser and evaluator | The existing engines are unmaintained and tied to a file format |
| [`iso8583-msg`](crates/iso8583-msg) | ISO 8583 card transaction messages | PyPI `pyiso8583` is downloaded 400,000 times a month; the Rust crates are all abandoned |
| [`javaproperties`](crates/javaproperties) | Java `.properties`: escapes, continuations, ISO-8859-1 default | npm `properties` is downloaded 300,000 times a week; the Rust `java-properties` has not moved since 2023 |
| [`object-hash`](crates/object-hash) | A stable hash of a structured value, order-independent | npm `object-hash` is downloaded over 40 million times a week; the name is free and the only Rust equivalent has 58 recent downloads |
| [`mllp`](crates/mllp) | MLLP framing for HL7 v2 messages, with a streaming decoder | PyPI `hl7apy` is downloaded 758,000 times a month; the Rust HL7 crates are leaderless and none validates |

## Building

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

The minimum supported Rust version is 1.99 and the crates use edition 2024.
`rust-version` is a floor, not the toolchain used to build: CI checks the
workspace both on current stable and on 1.99, so the floor cannot drift away
from the code the way it did when it read 1.85 while the crates used let-chains
that need 1.88.

## Using a single crate

Each crate is published independently. A project that only needs holiday
arithmetic depends on `holidays-rs` alone:

```toml
[dependencies]
holidays-rs = "0.1"
```

## How the crates are checked

Where a crate reproduces something another ecosystem already defines, it is
checked against that implementation rather than against itself: `adhan` against
the JavaScript `adhan` library, `object-hash` against the npm package,
`javaproperties` against Python's `javaproperties`, `factur-x` against the
official package. The oracle is installed for the test run and is never a
dependency of a crate.

Several crates also carry a random corpus generated by asking that reference for
the answer to thousands of inputs no one would think to write down. It is what
found the boundaries the hand-picked cases passed straight over. The generators
are committed under [`scripts/fuzz`](scripts/fuzz), so a corpus can be rebuilt
rather than guessed at.

## Background

The research behind this workspace, including the measurements that identified
these gaps, is in [`research/`](research). The scripts that produced the numbers
are in [`research/tools`](research/tools), and [`PLAN.md`](PLAN.md) records the
order the crates were built in and what each one deliberately leaves out.

## Contributing

Contributions are welcome, including a single country calendar or one more
validator. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the rules every crate
follows and the checks to run before opening a pull request.

Security problems should not be reported in a public issue. See
[`SECURITY.md`](SECURITY.md) for how to report one privately and for an honest
account of what these crates do and do not protect against.

## License

Licensed under the Apache License, Version 2.0. See
[`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).

Copyright 2026 Yayasan Pesantren Cipansor and contributors.
