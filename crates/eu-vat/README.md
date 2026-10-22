# eu-vat

[![crates.io](https://img.shields.io/crates/v/eu-vat.svg)](https://crates.io/crates/eu-vat)
[![docs.rs](https://docs.rs/eu-vat/badge.svg)](https://docs.rs/eu-vat)

Validation of European Union VAT identification numbers, for all twenty-seven
member states plus Northern Ireland. No dependencies.

```rust
use eu_vat::{Country, Vat};

let vat = Vat::parse("NL 0044.95445.B01").unwrap();
assert_eq!(vat.country(), Country::Netherlands);
assert_eq!(vat.to_string(), "NL004495445B01");

assert!(Vat::parse("NL004495446B01").is_err());
```

Each member state has its own number shape and its own check digit, so a single
regular expression is not enough: Belgium checks the number modulo 97, Germany
uses ISO 7064 Mod 11, 10, Italy validates the province of residence, and Latvia
reads a birth date out of its personal codes. This crate implements the check
that each national authority applies.

## What it does not do

This is a format and check-digit filter, not a registry lookup. A number that
passes may still be deregistered. Confirming that a number is *live* requires
the European Commission's VIES service, which this crate deliberately does not
contact; there is no network access anywhere in it.

## Greece, Northern Ireland and the United Kingdom

Greece uses `EL` as its VAT prefix even though its ISO 3166 code is `GR`. Both
spellings are accepted and `EL` is the canonical one.

`XI` is Northern Ireland, which keeps using the United Kingdom numbering scheme
after Brexit. The United Kingdom itself is no longer inside the EU VAT area, so
`GB` is rejected as an unknown prefix; a `GB` number is a United Kingdom tax
matter rather than an EU one.

## Data provenance

The country rules are derived from the published national VAT specifications,
and were cross-checked against
[`python-stdnum`](https://arthurdejong.org/python-stdnum/) by Arthur de Jong and
contributors. python-stdnum is distributed under the GNU Lesser General Public
License version 2.1 or later; the rules it encodes are the national
specifications, not its own text, and this crate is an independent
reimplementation that shares no code with it. The test fixtures were generated
from python-stdnum, so the tests compare against an independent implementation
rather than against this crate's own output. See
[`NOTICE`](https://github.com/cipansor/crateworks/blob/main/crates/eu-vat/NOTICE).

A VAT number is a two-letter country prefix followed by a national number
whose shape and check digit differ per member state. This crate knows all
twenty-seven of them plus Northern Ireland (`XI`), and it performs the same
country-specific check that the national authority applies.

## Example

```
use eu_vat::{Country, Vat};

// Spaces and dots are separators, not part of the number.
let vat = Vat::parse("NL 0044.95445.B01").unwrap();
assert_eq!(vat.country(), Country::Netherlands);
assert_eq!(vat.number(), "004495445B01");
assert_eq!(vat.to_string(), "NL004495445B01");

// A wrong BSN check digit is rejected.
assert!(Vat::parse("NL004495446B01").is_err());
```

## Scope

This is a format and check-digit filter, not a registry lookup. A number
that passes may still be deregistered, and a number that fails is certainly
not a VAT number. Confirming that a number is *live* requires the European
Commission's VIES service, which this crate deliberately does not contact.

## Greece and Northern Ireland

Greece uses `EL` as its VAT prefix even though its ISO 3166 code is `GR`;
both spellings are accepted here and `EL` is what the canonical form uses.
`XI` is Northern Ireland, which applies the same numbering scheme as the
United Kingdom. The United Kingdom itself is no longer inside the EU VAT
area, so `GB` is not accepted; a `GB` number is a UK tax matter.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
