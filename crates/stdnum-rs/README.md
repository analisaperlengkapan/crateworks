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

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under Apache-2.0 or MIT.
