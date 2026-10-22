# spreadsheet-formula

[![crates.io](https://img.shields.io/crates/v/spreadsheet-formula.svg)](https://crates.io/crates/spreadsheet-formula)
[![docs.rs](https://docs.rs/spreadsheet-formula/badge.svg)](https://docs.rs/spreadsheet-formula)

A spreadsheet formula parser and evaluator, independent of any file format.

```rust
use spreadsheet_formula::{CellRef, Evaluator, Sheet, Value};

struct Grid;
impl Sheet for Grid {
    fn value(&self, cell: CellRef) -> Value {
        match (cell.column, cell.row) {
            (0, 0) => Value::Number(2.0),
            (0, 1) => Value::Number(3.0),
            _ => Value::Empty,
        }
    }
}

assert_eq!(Evaluator::new(&Grid).eval("=SUM(A1:A2)*2").unwrap(), Value::Number(10.0));
```

38 functions plus `IF` and `IFERROR`, which are evaluated lazily. Cell and range
references, absolute references, sheet qualifiers, text concatenation, and
comparison operators.

The crate is deliberately independent of any file format. It takes a formula
string and a way to resolve cell references, so it can be used with `xlsx`,
`ods`, or a database of cells without pulling in a spreadsheet reader.

## Scope

The evaluator covers arithmetic, comparisons, text concatenation, cell and
range references, and a growing set of functions. It does not implement
array formulas, volatile functions or iterative recalculation; a cyclic
reference is reported as an error rather than resolved.

`FUNCTION_NAMES` is the list of every function the crate implements, and a caller
can validate a formula against it. The conditional ones are `IF` and `IFERROR`,
which the evaluator handles itself so that they do not evaluate every argument,
plus `IFS`, `COUNTIF`, `SUMIF`, `AVERAGEIF` and `COUNTBLANK`. `COUNTIF`, `SUMIF`
and `AVERAGEIF` take the spreadsheet criteria of a bare value or a comparison
such as `">3"`, and match text case-insensitively. `SUMIF` and `AVERAGEIF` take
the optional third argument as the range to add up, aligned by position with the
range being tested.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
