# factur-x

[![crates.io](https://img.shields.io/crates/v/factur-x.svg)](https://crates.io/crates/factur-x)
[![docs.rs](https://docs.rs/factur-x/badge.svg)](https://docs.rs/factur-x)

Factur-X and ZUGFeRD hybrid e-invoices: read, write and check the EN 16931 XML
in both the CII and the UBL syntax.

```rust
use factur_x::{Syntax, generate, parse, validate};

let invoice = factur_x::test_support::sample_invoice();

assert!(validate(&invoice).is_valid());

let xml = generate(&invoice, Syntax::Cii);
assert!(xml.starts_with("<?xml"));

let back = parse(&xml, Syntax::Cii).unwrap();
assert_eq!(back, invoice);
```

Factur-X, called ZUGFeRD in Germany, is the European standard for a PDF that
carries its invoice data as XML inside itself. The XML is EN 16931, and it exists
in two syntaxes: CII, used by Factur-X and ZUGFeRD, and UBL, used by Peppol and
by most national mandates outside France and Germany.

This crate models the invoice once and writes or reads either syntax, so the same
value can be sent to a German receiver and a Belgian one without a mapping layer
in between. Fields are documented with the standard's own numbers, `BT-1` for the
invoice number and `BG-25` for the group of invoice lines, because those numbers
are the only way to talk about the data across two syntaxes that use completely
different element names for the same thing.

## Checking an invoice

`validate` implements the arithmetic rules of the standard, which are the ones
that catch a genuinely wrong invoice:

```rust
use factur_x::{validate, Amount};

let mut invoice = factur_x::test_support::sample_invoice();
invoice.tax_breakdown[0].tax_amount = Amount::parse("180.00").unwrap();

let report = validate(&invoice);
assert!(!report.is_valid());
assert!(report.rules().contains(&"BR-CO-17"));
```

The rules checked are `BR-01` to `BR-06`, `BR-09`, `BR-16`, and the `BR-CO-10`
to `BR-CO-18` and `BR-S-08` arithmetic. Code list validation and the country
profiles that extend EN 16931, such as XRechnung, are out of scope.

## Scope

The crate works on the XML. Embedding it in a PDF and reading it back out needs a
PDF library; `generate` gives you the XML to embed. `parse` reads both syntaxes,
and `detect_syntax` tells you which one a document is.

Factur-X, called ZUGFeRD in Germany, is the European standard for a PDF that
carries its invoice data as XML inside itself. The XML is EN 16931, the
European semantic standard, and it exists in two syntaxes:

* CII, the UN/CEFACT Cross Industry Invoice, used by Factur-X and ZUGFeRD.
* UBL, the OASIS Universal Business Language, used by Peppol and by most
  national mandates outside France and Germany.

This crate models the invoice once and writes or reads either syntax, so the
same value can be sent to a German receiver and a Belgian one without a
mapping layer in between.

## Example

```
use factur_x::{Syntax, generate, parse, validate};

let invoice = factur_x::test_support::sample_invoice();

// Check it before sending it anywhere.
assert!(validate(&invoice).is_valid());

// Write the syntax the receiver expects.
let xml = generate(&invoice, Syntax::Cii);
assert!(xml.starts_with("<?xml"));

// And read either syntax back.
let back = parse(&xml, Syntax::Cii).unwrap();
assert_eq!(back, invoice);
```

## What it does not do

This crate works on the XML. Embedding it in a PDF, and reading it back out,
is a separate job that needs a PDF library; `generate` gives you the XML to
embed. Code list validation and the country profiles that extend EN 16931,
such as XRechnung and the French CTC profile, are also out of scope. See
`validate::validate` for exactly which rules are checked.

## Field numbers

EN 16931 refers to every field by a number, `BT-1` for the invoice number and
`BG-25` for the group of invoice lines. The two syntaxes use entirely
different element names for the same field, so the model in `model` carries
those numbers in its documentation and the code stays readable across both.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
