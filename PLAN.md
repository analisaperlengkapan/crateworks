# Plan

The order in which the crates in this workspace were built, what each one
covers today, and what is deliberately left out. The dependency order matters:
`crateworks-xml` is the foundation that `iso20022` and `soap-client` build on,
so it had to be correct first.

## 1. `crateworks-xml` (foundation)

**Why first.** Both the ISO 20022 and SOAP crates need to read XML, and pulling
in a large parser would have made them heavy. This is a small, dependency-free
parser that keeps the tree, so a caller can inspect attributes and namespaces
directly.

**Covers today.** Parsing with attributes, namespaces, comments and CDATA;
text access (`text`, `deep_text`); escaping for text and attribute values;
round-trip serialisation; local-name matching for namespace-prefixed elements.

**Not covered.** Schema validation, XPath, streaming over documents larger than
memory. Those belong to a dedicated parser.

## 2. `holidays-rs`

**Why.** Working-day arithmetic is needed by HR systems, banks computing
settlement dates, and ticket systems computing SLAs. The existing `holidays`
crate has one release from 2023 and an abandoned repository.

**Covers today.** Five countries (Indonesia, United States, United Kingdom,
Germany, France). Fixed-date rules, Easter-relative rules via the Gregorian
computus, and *n*-th-weekday rules including "last Monday of the month".
Weekend substitution per country, including the United States rule (Saturday
back to Friday) and the United Kingdom rule (both days forward to Monday), and
collision resolution so Christmas and Boxing Day never land on the same day.
Queries: `is_holiday`, `holiday_name`, `is_working_day`, `next_working_day`,
`nth_working_day_after`, `nth_working_day_before`, `working_days_between`.

**Not covered.** Islamic and Chinese-derived Indonesian observances (Idul Fitri,
Imlek, Nyepi) depend on a yearly government decree rather than a formula, so
they are added per year with `Calendar::with_extra`. Regional holidays (German
state holidays, United Kingdom local holidays) are out of scope. More countries
are the obvious next step.

## 3. `stdnum-rs`

**Why.** Rust has per-country crates for single identifier types but no shared
entry point, and no crate that covers IBAN alongside barcodes and tax numbers.
The Python `python-stdnum` library covers 200+ types; this is the start of the
Rust equivalent.

**Covers today.** IBAN (78 countries, MOD-97, check-digit generation), ISBN-10
and ISBN-13 with conversion, EAN-8/EAN-13/UPC-A with UPC-to-EAN conversion,
Luhn with check-digit generation, and national identifiers: Indonesian NPWP and
NIK, Brazilian CPF and CNPJ, Spanish DNI and NIE, German tax ID, Polish PESEL,
Indian Aadhaar with Verhoeff.

**Not covered.** The remaining national identifiers, and anything that needs a
registry lookup. A number that passes may still be unissued; this is a form
filter, not proof of existence.

## 4. `spreadsheet-formula`

**Why.** Spreadsheet formula engines in Rust are either abandoned or welded to
a file format. This crate takes a formula string and a cell resolver, so it
works with `xlsx`, `ods`, or a database of cells.

**Covers today.** Tokeniser with absolute references, sheet qualifiers and
ranges; recursive-descent parser with spreadsheet operator precedence;
evaluator with 29 functions plus `IF` and `IFERROR`, which are handled
specially so they do not evaluate both branches. A range size limit bounds the
work a hostile formula can request.

**Not covered.** Array formulas, volatile functions (`NOW`, `RAND`), date and
lookup functions, iterative recalculation. Cyclic references are detected and
reported rather than resolved.

## 5. `iso20022`

**Why.** ISO 20022 is replacing SWIFT MT, and every bank integration needs it.
The only crate on crates.io is from 2020 and unmaintained.

**Covers today.** `camt.053` bank statements: account, balances, entries with
credit/debit sign, counterparty, remittance and value date; closing balance
either as reported or computed from the entries. `pain.001` payment initiation:
read and generate, with the control sum and transaction count computed from the
transfers so the output cannot disagree with itself.

**Not covered.** `pacs` clearing messages, other `camt` variants, schema
validation, and the full generated type set. Amounts use `f64` with the original
decimal text preserved in `Amount::text`.

## 6. `soap-client`

**Why.** SOAP remains the only interface for many banks, government systems and
enterprise products, and every SOAP crate on crates.io has zero reverse
dependencies. The widest gap, and the least proven Rust demand.

**Covers today.** SOAP 1.1 and 1.2 envelopes: build a request, parse a
response, detect and read a fault in either version's shape. WSDL reading:
endpoint address, port types, operations with their input and output messages,
and the transport binding.

**Not covered.** The HTTP call itself, WS-Security, and typed code generation
from a WSDL. Those are deliberate omissions: the HTTP layer belongs to the
caller's client, and WS-Security needs a certificate story that should not be
guessed at.

## Publishing order

`cargo package` resolves a `path` dependency against the registry, so a crate
that depends on another crate in this workspace cannot be packaged until that
dependency is published. Publish bottom-up:

1. `crateworks-xml`
2. `holidays-rs`, `stdnum-rs`, `spreadsheet-formula` (independent of the rest)
3. `iso20022`, `soap-client` (both depend on `crateworks-xml`)

## Verification

```sh
cargo test --workspace --all-targets      # 218 unit tests, 6 README tests, 6 doctests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```
