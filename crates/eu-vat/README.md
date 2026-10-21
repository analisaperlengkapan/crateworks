# eu-vat

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

The country rules are a transcription of the MIT-compatible parts of
[`python-stdnum`](https://arthurdejong.org/python-stdnum/) by Arthur de Jong and
contributors, which in turn follows each authority's published specification.
The test fixtures were generated from python-stdnum, so the tests compare
against an independent implementation rather than against this crate's own
output. See [`NOTICE`](NOTICE).

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
