//! Runtime values and the arithmetic that operates on them.

use std::fmt::{self, Display};

/// A failure of a numeric or textual operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Division by zero.
    DivisionByZero,
    /// The value is not a number.
    NotANumber,
    /// The value is not text.
    NotText,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero => f.write_str("division by zero"),
            Self::NotANumber => f.write_str("value is not a number"),
            Self::NotText => f.write_str("value is not text"),
        }
    }
}

impl std::error::Error for Error {}

/// The value of a cell or of a computed expression.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    /// A number. Spreadsheets have a single numeric type.
    Number(f64),
    /// A text string.
    Text(String),
    /// A logical value.
    Bool(bool),
    /// A cell that has never been written to.
    #[default]
    Empty,
}

impl Value {
    /// Coerces to a number the way a spreadsheet does.
    ///
    /// An empty cell is zero, a boolean is one or zero, and text is parsed
    /// leniently, so `"3.5"` becomes `3.5`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotANumber`] when the value cannot be read as a number.
    pub fn as_number(&self) -> Result<f64, Error> {
        match self {
            Self::Number(n) => Ok(*n),
            Self::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Self::Empty => Ok(0.0),
            Self::Text(t) => t.trim().parse::<f64>().map_err(|_| Error::NotANumber),
        }
    }

    /// Interprets the value as a condition.
    ///
    /// Numbers are true when non-zero, text is parsed as a boolean word, and an
    /// empty cell is false.
    #[must_use]
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Number(n) => *n != 0.0,
            Self::Empty => false,
            Self::Text(t) => matches!(t.trim().to_ascii_uppercase().as_str(), "TRUE" | "YES" | "1"),
        }
    }

    /// Returns the text of the value, or an error when it is not text.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotText`] for non-text values.
    pub fn as_text(&self) -> Result<&str, Error> {
        match self {
            Self::Text(t) => Ok(t),
            _ => Err(Error::NotText),
        }
    }

    /// Whether the value is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => {
                // Print integers without a trailing ".0" so results read the way
                // a spreadsheet shows them.
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            Self::Text(t) => f.write_str(t),
            Self::Bool(b) => f.write_str(if *b { "TRUE" } else { "FALSE" }),
            Self::Empty => f.write_str(""),
        }
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Self::Number(f64::from(n))
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_coercion_follows_spreadsheet_rules() {
        assert_eq!(Value::Number(2.5).as_number(), Ok(2.5));
        assert_eq!(Value::Bool(true).as_number(), Ok(1.0));
        assert_eq!(Value::Bool(false).as_number(), Ok(0.0));
        assert_eq!(Value::Empty.as_number(), Ok(0.0));
        assert_eq!(Value::Text(" 3.5 ".into()).as_number(), Ok(3.5));
        assert_eq!(
            Value::Text("abc".into()).as_number(),
            Err(Error::NotANumber)
        );
    }

    #[test]
    fn boolean_coercion() {
        assert!(Value::Number(-1.0).as_bool());
        assert!(!Value::Number(0.0).as_bool());
        assert!(Value::Text("true".into()).as_bool());
        assert!(!Value::Empty.as_bool());
    }

    #[test]
    fn display_of_integers_has_no_decimal_point() {
        assert_eq!(Value::Number(18.0).to_string(), "18");
        assert_eq!(Value::Number(2.5).to_string(), "2.5");
        assert_eq!(Value::Bool(true).to_string(), "TRUE");
        assert_eq!(Value::Empty.to_string(), "");
    }

    #[test]
    fn default_is_empty() {
        assert!(Value::default().is_empty());
    }
}
