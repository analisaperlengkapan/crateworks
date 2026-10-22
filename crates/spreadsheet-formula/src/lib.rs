#![doc = include_str!("../README.md")]
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
    /// `IFS` was given no condition that held, and no fallback to fall back on.
    NoMatchingCondition,
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
            Self::NoMatchingCondition => {
                f.write_str("IFS found no condition that holds and has no fallback")
            }
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
