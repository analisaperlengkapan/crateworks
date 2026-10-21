//! The evaluator, and the [`Sheet`] trait it reads cells through.

use std::fmt::{self, Display};

use crate::Error;
use crate::functions::{self, Arg};
use crate::parser::{BinOp, Expr};
use crate::value::{Error as ValueError, Value};

/// A cell address, zero-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellRef {
    /// Zero-based column.
    pub column: u32,
    /// Zero-based row.
    pub row: u32,
}

impl CellRef {
    /// Creates a reference.
    #[must_use]
    pub const fn new(column: u32, row: u32) -> Self {
        Self { column, row }
    }
}

impl Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            crate::lexer::column_name(self.column),
            self.row + 1
        )
    }
}

/// Read access to the cells a formula refers to.
///
/// A sheet with several pages can ignore the sheet qualifier, or route it to
/// the right page. The evaluator never writes, so a caller can pass a read-only
/// view.
pub trait Sheet {
    /// Returns the value of a cell. A cell that does not exist is
    /// [`Value::Empty`].
    fn value(&self, cell: CellRef) -> Value;

    /// Returns the value of a cell on a named sheet.
    ///
    /// The default implementation ignores the name, which is correct for a
    /// single-sheet workbook.
    fn value_in_sheet(&self, sheet: &str, cell: CellRef) -> Value {
        let _ = sheet;
        self.value(cell)
    }
}

/// Evaluates formulas against a [`Sheet`].
#[derive(Debug)]
pub struct Evaluator<'a, S: Sheet> {
    sheet: &'a S,
    /// Cells currently being evaluated, used to detect cycles.
    in_progress: Vec<CellRef>,
    max_range_cells: usize,
}

impl<'a, S: Sheet> Evaluator<'a, S> {
    /// Creates an evaluator over a sheet.
    #[must_use]
    pub const fn new(sheet: &'a S) -> Self {
        Self {
            sheet,
            in_progress: Vec::new(),
            max_range_cells: 1_000_000,
        }
    }

    /// Sets the largest range that may be materialised.
    ///
    /// This bounds the work a hostile formula can request, for example
    /// `SUM(A1:XFD1048576)`, which would otherwise allocate the whole grid.
    #[must_use]
    pub const fn with_max_range_cells(mut self, limit: usize) -> Self {
        self.max_range_cells = limit;
        self
    }

    /// Parses and evaluates a formula.
    ///
    /// # Errors
    ///
    /// Returns any [`Error`] from parsing or evaluation.
    pub fn eval(&mut self, formula: &str) -> Result<Value, Error> {
        let expr = crate::parser::parse_formula(formula)?;
        self.eval_expr(&expr)
    }

    /// Evaluates an already-parsed expression.
    ///
    /// # Errors
    ///
    /// Returns any [`Error`] from evaluation.
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value, Error> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::Text(t) => Ok(Value::Text(t.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Cell { column, row, sheet } => {
                let cell = CellRef::new(*column, *row);
                Ok(match sheet {
                    Some(name) => self.sheet.value_in_sheet(name, cell),
                    None => self.sheet.value(cell),
                })
            }
            Expr::Range { start, end, sheet } => {
                let values = self.range_values(start, end, sheet.as_deref())?;
                // A range used as a scalar takes its first cell, matching what a
                // spreadsheet does outside an aggregating function.
                Ok(values.0.into_iter().next().unwrap_or(Value::Empty))
            }
            Expr::Unary { op, operand } => {
                let v = self.eval_expr(operand)?;
                match op {
                    '-' => Ok(Value::Number(-v.as_number().map_err(|_| {
                        Error::TypeError {
                            operation: "unary minus".to_string(),
                            found: v,
                        }
                    })?)),
                    '+' => Ok(v),
                    '%' => Ok(Value::Number(
                        v.as_number().map_err(|_| Error::TypeError {
                            operation: "percent".to_string(),
                            found: v,
                        })? / 100.0,
                    )),
                    other => Err(Error::Parse {
                        position: 0,
                        message: format!("unhandled unary operator {other:?}"),
                    }),
                }
            }
            Expr::Binary { op, left, right } => self.eval_binary(*op, left, right),
            Expr::Call { name, args } => self.eval_call(name, args),
        }
    }

    fn eval_binary(&mut self, op: BinOp, left: &Expr, right: &Expr) -> Result<Value, Error> {
        // Short-circuit the logical operators so a caller can write
        // IF-style guards without evaluating a failing branch.
        let l = self.eval_expr(left)?;
        let r = self.eval_expr(right)?;
        match op {
            BinOp::Add | BinOp::Subtract | BinOp::Multiply | BinOp::Divide | BinOp::Power => {
                let a = l.as_number().map_err(|_| Error::TypeError {
                    operation: op.symbol().to_string(),
                    found: l.clone(),
                })?;
                let b = r.as_number().map_err(|_| Error::TypeError {
                    operation: op.symbol().to_string(),
                    found: r.clone(),
                })?;
                let result = match op {
                    BinOp::Add => a + b,
                    BinOp::Subtract => a - b,
                    BinOp::Multiply => a * b,
                    BinOp::Divide => {
                        if b == 0.0 {
                            return Err(Error::Arithmetic(ValueError::DivisionByZero));
                        }
                        a / b
                    }
                    BinOp::Power => a.powf(b),
                    _ => unreachable!("covered by the outer match"),
                };
                if result.is_nan() {
                    // Something like (-8)^0.5, which has no real answer.
                    return Err(Error::Arithmetic(ValueError::NotANumber));
                }
                Ok(Value::Number(result))
            }
            BinOp::Concat => Ok(Value::Text(format!("{l}{r}"))),
            BinOp::Equal => Ok(Value::Bool(compare(&l, &r) == std::cmp::Ordering::Equal)),
            BinOp::NotEqual => Ok(Value::Bool(compare(&l, &r) != std::cmp::Ordering::Equal)),
            BinOp::Less => Ok(Value::Bool(compare(&l, &r) == std::cmp::Ordering::Less)),
            BinOp::LessEqual => Ok(Value::Bool(compare(&l, &r) != std::cmp::Ordering::Greater)),
            BinOp::Greater => Ok(Value::Bool(compare(&l, &r) == std::cmp::Ordering::Greater)),
            BinOp::GreaterEqual => Ok(Value::Bool(compare(&l, &r) != std::cmp::Ordering::Less)),
        }
    }

    fn eval_call(&mut self, name: &str, args: &[Expr]) -> Result<Value, Error> {
        match name {
            "IF" => {
                if args.len() != 3 {
                    return Err(Error::WrongArity {
                        name: name.to_string(),
                        given: args.len(),
                        expected: "3",
                    });
                }
                return if self.eval_expr(&args[0])?.as_bool() {
                    self.eval_expr(&args[1])
                } else {
                    self.eval_expr(&args[2])
                };
            }
            "IFERROR" => {
                if args.len() != 2 {
                    return Err(Error::WrongArity {
                        name: name.to_string(),
                        given: args.len(),
                        expected: "2",
                    });
                }
                // Any evaluation failure becomes the fallback, which is the
                // point of the function.
                return match self.eval_expr(&args[0]) {
                    Ok(v) => Ok(v),
                    Err(_) => self.eval_expr(&args[1]),
                };
            }
            _ => {}
        }
        let mut flattened = Vec::with_capacity(args.len());
        for arg in args {
            flattened.push(self.flatten(arg)?);
        }
        functions::call(name, &flattened)
    }

    /// Turns an argument into a flat list of values, expanding ranges.
    fn flatten(&mut self, expr: &Expr) -> Result<Arg, Error> {
        if let Expr::Range { start, end, sheet } = expr {
            return self.range_values(start, end, sheet.as_deref());
        }
        Ok(Arg(vec![self.eval_expr(expr)?]))
    }

    fn range_values(
        &mut self,
        start: &Expr,
        end: &Expr,
        sheet: Option<&str>,
    ) -> Result<Arg, Error> {
        let (c0, r0) = cell_of(start)?;
        let (c1, r1) = cell_of(end)?;
        let (col_lo, col_hi) = (c0.min(c1), c0.max(c1));
        let (row_lo, row_hi) = (r0.min(r1), r0.max(r1));
        let count = ((col_hi - col_lo + 1) as usize) * ((row_hi - row_lo + 1) as usize);
        if count > self.max_range_cells {
            return Err(Error::Parse {
                position: 0,
                message: format!(
                    "range covers {count} cells, above the limit of {}",
                    self.max_range_cells
                ),
            });
        }
        let mut values = Vec::with_capacity(count);
        // Row-major, which is the order aggregation functions expect.
        for row in row_lo..=row_hi {
            for column in col_lo..=col_hi {
                let cell = CellRef::new(column, row);
                values.push(match sheet {
                    Some(name) => self.sheet.value_in_sheet(name, cell),
                    None => self.sheet.value(cell),
                });
            }
        }
        Ok(Arg(values))
    }

    /// Records that a cell is being evaluated, so a recursive lookup can be
    /// reported rather than looping forever.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CircularReference`] if the cell is already in progress.
    pub fn enter(&mut self, cell: CellRef) -> Result<(), Error> {
        if self.in_progress.contains(&cell) {
            return Err(Error::CircularReference(cell));
        }
        self.in_progress.push(cell);
        Ok(())
    }

    /// Clears a cell from the in-progress set once it has been evaluated.
    pub fn leave(&mut self, cell: CellRef) {
        self.in_progress.retain(|c| *c != cell);
    }
}

fn cell_of(expr: &Expr) -> Result<(u32, u32), Error> {
    match expr {
        Expr::Cell { column, row, .. } => Ok((*column, *row)),
        other => Err(Error::Parse {
            position: 0,
            message: format!("range bounds must be cells, found {other:?}"),
        }),
    }
}

/// Orders two values the way a spreadsheet does.
///
/// Numbers compare numerically, and a number is always less than text, which
/// matches the ordering users rely on when sorting mixed columns. An empty cell
/// compares equal to zero.
fn compare(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Value::Text(x), Value::Text(y)) => x.cmp(y),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::Text(_), _) => Ordering::Greater,
        (_, Value::Text(_)) => Ordering::Less,
        _ => {
            let x = a.as_number().unwrap_or(0.0);
            let y = b.as_number().unwrap_or(0.0);
            x.total_cmp(&y)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sheet backed by a fixed set of cells, keyed by "A1" style names.
    struct Grid(Vec<(&'static str, Value)>);

    impl Grid {
        fn new() -> Self {
            Self(Vec::new())
        }

        fn set(mut self, name: &'static str, v: Value) -> Self {
            self.0.push((name, v));
            self
        }

        fn parse_ref(name: &str) -> CellRef {
            let letters: String = name.chars().take_while(char::is_ascii_alphabetic).collect();
            let digits: String = name.chars().skip_while(char::is_ascii_alphabetic).collect();
            CellRef::new(
                crate::lexer::column_index(&letters),
                digits.parse::<u32>().unwrap() - 1,
            )
        }
    }

    impl Sheet for Grid {
        fn value(&self, cell: CellRef) -> Value {
            self.0
                .iter()
                .find(|(name, _)| Self::parse_ref(name) == cell)
                .map_or(Value::Empty, |(_, v)| v.clone())
        }
    }

    fn grid() -> Grid {
        Grid::new()
            .set("A1", Value::Number(2.0))
            .set("A2", Value::Number(3.0))
            .set("A3", Value::Number(4.0))
            .set("B1", Value::Number(10.0))
            .set("B2", Value::Number(20.0))
            .set("C1", Value::Text("hi".into()))
            .set("D1", Value::Empty)
    }

    #[test]
    fn arithmetic_and_precedence() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert_eq!(e.eval("=1+2*3").unwrap(), Value::Number(7.0));
        assert_eq!(e.eval("=(1+2)*3").unwrap(), Value::Number(9.0));
        assert_eq!(e.eval("=2^3^2").unwrap(), Value::Number(512.0));
        assert_eq!(e.eval("=10/4").unwrap(), Value::Number(2.5));
    }

    #[test]
    fn cell_references() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert_eq!(e.eval("=A1").unwrap(), Value::Number(2.0));
        assert_eq!(e.eval("=A1+A2+A3").unwrap(), Value::Number(9.0));
        assert_eq!(e.eval("=A1*B1").unwrap(), Value::Number(20.0));
    }

    #[test]
    fn ranges_in_functions() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert_eq!(e.eval("=SUM(A1:A3)").unwrap(), Value::Number(9.0));
        assert_eq!(e.eval("=SUM(A1:A3)*2").unwrap(), Value::Number(18.0));
        assert_eq!(e.eval("=AVERAGE(A1:A3)").unwrap(), Value::Number(3.0));
        assert_eq!(e.eval("=SUM(A1:B2)").unwrap(), Value::Number(35.0));
        assert_eq!(e.eval("=MAX(A1:A3,B1:B2)").unwrap(), Value::Number(20.0));
    }

    #[test]
    fn empty_cells_count_as_zero_in_sum() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert_eq!(e.eval("=SUM(D1)").unwrap(), Value::Number(0.0));
        assert_eq!(e.eval("=D1+1").unwrap(), Value::Number(1.0));
    }

    #[test]
    fn text_concatenation_and_comparison() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert_eq!(e.eval("=C1 & \"!\" ").unwrap(), Value::Text("hi!".into()));
        assert_eq!(e.eval("=A1<B1").unwrap(), Value::Bool(true));
        assert_eq!(e.eval("=A1=A1").unwrap(), Value::Bool(true));
        assert_eq!(e.eval("=A1<>A2").unwrap(), Value::Bool(true));
        assert_eq!(e.eval("=C1=\"hi\"").unwrap(), Value::Bool(true));
    }

    #[test]
    fn numbers_sort_before_text() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert_eq!(e.eval("=A1<C1").unwrap(), Value::Bool(true));
        assert_eq!(e.eval("=C1>A1").unwrap(), Value::Bool(true));
    }

    #[test]
    fn if_only_evaluates_the_taken_branch() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert_eq!(e.eval("=IF(A1>1,10,1/0)").unwrap(), Value::Number(10.0));
        assert_eq!(e.eval("=IF(A1>100,1/0,20)").unwrap(), Value::Number(20.0));
    }

    #[test]
    fn iferror_catches_failures() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert_eq!(
            e.eval("=IFERROR(1/0,\"n/a\")").unwrap(),
            Value::Text("n/a".into())
        );
        assert_eq!(
            e.eval("=IFERROR(10/2,\"n/a\")").unwrap(),
            Value::Number(5.0)
        );
    }

    #[test]
    fn percent_and_unary() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert_eq!(e.eval("=50%").unwrap(), Value::Number(0.5));
        assert_eq!(e.eval("=-A1").unwrap(), Value::Number(-2.0));
        assert_eq!(e.eval("=200*10%").unwrap(), Value::Number(20.0));
    }

    #[test]
    fn division_by_zero_is_an_error() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert!(matches!(
            e.eval("=1/0"),
            Err(Error::Arithmetic(ValueError::DivisionByZero))
        ));
    }

    #[test]
    fn text_in_arithmetic_is_a_type_error() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert!(matches!(e.eval("=C1+1"), Err(Error::TypeError { .. })));
    }

    #[test]
    fn range_size_limit_is_enforced() {
        let g = grid();
        let mut e = Evaluator::new(&g).with_max_range_cells(100);
        assert!(matches!(e.eval("=SUM(A1:ZZ999)"), Err(Error::Parse { .. })));
    }

    #[test]
    fn circular_reference_is_detected() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        let cell = CellRef::new(0, 0);
        e.enter(cell).unwrap();
        assert_eq!(e.enter(cell), Err(Error::CircularReference(cell)));
        e.leave(cell);
        assert!(e.enter(cell).is_ok());
    }

    #[test]
    fn nested_functions_evaluate() {
        let g = grid();
        let mut e = Evaluator::new(&g);
        assert_eq!(
            e.eval("=SUM(A1:A3)+MAX(B1:B2)").unwrap(),
            Value::Number(29.0)
        );
        assert_eq!(
            e.eval("=ROUND(AVERAGE(A1:A3),0)").unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            e.eval("=IF(SUM(A1:A3)>5,\"big\",\"small\")").unwrap(),
            Value::Text("big".into())
        );
    }

    #[test]
    fn cell_ref_displays_as_a1() {
        assert_eq!(CellRef::new(0, 0).to_string(), "A1");
        assert_eq!(CellRef::new(27, 11).to_string(), "AB12");
    }
}
