# stdnum-rs

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

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
