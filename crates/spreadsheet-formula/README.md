# spreadsheet-formula

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

29 functions plus `IF` and `IFERROR`, which are evaluated lazily. Cell and range
references, absolute references, sheet qualifiers, text concatenation, and
comparison operators.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
