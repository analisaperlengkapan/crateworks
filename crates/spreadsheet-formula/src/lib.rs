//! Parser and evaluator for spreadsheet formulas.
//!
//! The crate is deliberately independent of any file format. It takes a formula
//! string and a way to resolve cell references, so it can be used with `xlsx`,
//! `ods`, or a database of cells without pulling in a spreadsheet reader.
//!
//! # Example
//!
//! ```
//! use spreadsheet_formula::{CellRef, Evaluator, Sheet, Value};
//!
//! struct Grid;
//! impl Sheet for Grid {
//!     fn value(&self, cell: CellRef) -> Value {
//!         match (cell.column, cell.row) {
//!             (0, 0) => Value::Number(2.0),
//!             (0, 1) => Value::Number(3.0),
//!             (0, 2) => Value::Number(4.0),
//!             _ => Value::Empty,
//!         }
//!     }
//! }
//!
//! let v = Evaluator::new(&Grid).eval("=SUM(A1:A3) * 2").unwrap();
//! assert_eq!(v, Value::Number(18.0));
//! ```
//!
//! # Scope
//!
//! The evaluator covers arithmetic, comparisons, text concatenation, cell and
//! range references, and a growing set of functions. It does not implement
//! array formulas, volatile functions or iterative recalculation; a cyclic
//! reference is reported as an error rather than resolved.

#![forbid(unsafe_code)]

mod eval;
mod functions;
mod lexer;
mod parser;
mod value;

pub use eval::{CellRef, Evaluator, Sheet};
pub use functions::FUNCTION_NAMES;
pub use parser::{Expr, parse_formula};
pub use value::{Error as ValueError, Value};

/// Errors produced while parsing or evaluating a formula.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The formula text could not be tokenised.
    Lex {
        /// Character offset where the problem was found.
        position: usize,
        /// Description of the problem.
        message: String,
    },
    /// The token stream could not be parsed.
    Parse {
        /// Character offset where the problem was found.
        position: usize,
        /// Description of the problem.
        message: String,
    },
    /// A function was called that this crate does not implement.
    UnknownFunction(String),
    /// A function was called with the wrong number of arguments.
    WrongArity {
        /// Function name.
        name: String,
        /// Arguments that were supplied.
        given: usize,
        /// Arguments the function accepts.
        expected: &'static str,
    },
    /// A value could not be coerced to the type an operation required.
    TypeError {
        /// Operation that failed.
        operation: String,
        /// The value that was rejected.
        found: Value,
    },
    /// A formula referenced a cell that is part of its own dependency chain.
    CircularReference(CellRef),
    /// Arithmetic failed, for example division by zero.
    Arithmetic(ValueError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex { position, message } => write!(f, "at {position}: {message}"),
            Self::Parse { position, message } => write!(f, "at {position}: {message}"),
            Self::UnknownFunction(name) => write!(f, "unknown function {name}"),
            Self::WrongArity {
                name,
                given,
                expected,
            } => {
                write!(f, "{name} takes {expected} arguments, got {given}")
            }
            Self::TypeError { operation, found } => {
                write!(f, "{operation} cannot be applied to {found}")
            }
            Self::CircularReference(cell) => write!(f, "circular reference at {cell}"),
            Self::Arithmetic(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ValueError> for Error {
    fn from(e: ValueError) -> Self {
        Self::Arithmetic(e)
    }
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_is_informative() {
        let e = Error::WrongArity {
            name: "SUM".into(),
            given: 0,
            expected: "1 or more",
        };
        assert_eq!(e.to_string(), "SUM takes 1 or more arguments, got 0");
    }

    #[test]
    fn value_errors_convert() {
        let e: Error = ValueError::DivisionByZero.into();
        assert!(matches!(e, Error::Arithmetic(ValueError::DivisionByZero)));
    }
}
