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

| Crate | What it does | Why it exists |
| --- | --- | --- |
| [`xmlkit`](crates/xmlkit) | Small XML parser, tree, and serialiser | Foundation for the SOAP and ISO 20022 crates; no `quick-xml` dependency needed |
| [`holidays-rs`](crates/holidays-rs) | Public holiday calendars and working-day arithmetic | `holidays` on crates.io has a single 2023 release and an abandoned repo |
| [`hijridate`](crates/hijridate) | Hijri/Gregorian conversion on the Umm al-Qura calendar | The name is free, the only Rust Hijri crates are a CLI and a 2018 stub, while `hijri-converter` on PyPI has millions of monthly downloads |
| [`stdnum-rs`](crates/stdnum-rs) | IBAN, ISBN, EAN/UPC, Luhn and national ID validation | Rust has only fragmented per-country crates and no shared entry point |
| [`spreadsheet-formula`](crates/spreadsheet-formula) | Spreadsheet formula parser and evaluator | The existing engines are unmaintained and tied to a file format |
| [`iso20022-rs`](crates/iso20022-rs) | `camt`, `pain` and `pacs` payment messages | The one crate on crates.io is from 2020 and dead |
| [`soap-client`](crates/soap-client) | SOAP 1.1/1.2 envelopes and WSDL reading | Every SOAP crate on crates.io has zero reverse dependencies |

## Building

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

The minimum supported Rust version is 1.85 and the crates use edition 2024.

## Using a single crate

Each crate is published independently. A project that only needs holiday
arithmetic depends on `holidays-rs` alone:

```toml
[dependencies]
holidays-rs = "0.1"
```

## Background

The research behind this workspace, including the measurements that identified
these gaps, is in [`research/`](research). The scripts that produced the numbers
are in [`research/tools`](research/tools).

## Contributing

Contributions are welcome, including a single country calendar or one more
validator. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the rules every crate
follows and the checks to run before opening a pull request.

Security problems should not be reported in a public issue. See
[`SECURITY.md`](SECURITY.md) for how to report one privately and for an honest
account of what these crates do and do not protect against.

## License

Licensed under the Apache License, Version 2.0. See
[`LICENSE-APACHE`](LICENSE-APACHE) and [`NOTICE`](NOTICE).

Copyright 2026 Yayasan Pesantren Cipansor and contributors.
