# stdnum-rs

[![crates.io](https://img.shields.io/crates/v/stdnum-rs.svg)](https://crates.io/crates/stdnum-rs)
[![docs.rs](https://docs.rs/stdnum-rs/badge.svg)](https://docs.rs/stdnum-rs)

Validation for standard numbers: IBAN, ISBN, EAN/UPC, Luhn and national
identifiers.

```rust
use stdnum_rs::{Iban, Luhn};

assert!(Iban::parse("GB82 WEST 1234 5698 7654 32").is_ok());
assert!(Luhn::parse("4539148803436467").is_ok());
```

Checksum and structure only; nothing here contacts a registry. National
identifiers covered: Indonesian NPWP and NIK, Brazilian CPF and CNPJ, Spanish
DNI and NIE, German tax ID, Polish PESEL, Indian Aadhaar.

## Data provenance

The rules were cross-checked against
[`python-stdnum`](https://arthurdejong.org/python-stdnum/) by Arthur de Jong and
contributors, which in turn follows each authority's published specification.
This is an independent reimplementation and shares no code with it; see
[`NOTICE`](https://github.com/cipansor/crateworks/blob/main/crates/stdnum-rs/NOTICE).

Each number type lives in its own module so that a program only pays for
what it uses, and `validate` offers a uniform entry point when the type
is only known at runtime.

## Example

```
use stdnum_rs::{Iban, validate, NumberType};

// IBAN: country code, two check digits, then the national part.
assert!(Iban::parse("GB82 WEST 1234 5698 7654 32").is_ok());
assert!(Iban::parse("GB82 WEST 1234 5698 7654 33").is_err());

assert!(validate(NumberType::Iban, "DE89370400440532013000").is_ok());
```

## Scope

Only checksum and structural validation is performed. Nothing here contacts
a registry, so a number that passes may still be unissued. Treat this as a
first filter in a form, never as proof that a number exists.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
