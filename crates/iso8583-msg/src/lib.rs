#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod bitmap;
mod codec;
mod field;
mod message;
mod spec;

pub use codec::{DataEncoding, LengthEncoding, Padding};
pub use field::{FieldSpec, FieldType};
pub use message::Message;
pub use spec::{Spec, default_ascii, default_bcd, default_ebcdic};

/// Why a message could not be encoded or decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The message type indicator is not four characters.
    BadMti(String),
    /// The specification has no entry for this field number.
    UnknownField(u8),
    /// The specification itself is inconsistent.
    BadSpec(String),
    /// A field number outside the range the format covers.
    FieldOutOfRange(u8),
    /// A field's value is longer than its maximum.
    TooLong {
        /// Field number.
        field: u8,
        /// Maximum the specification allows.
        max: usize,
        /// Length that was supplied.
        found: usize,
    },
    /// A fixed-length field's value is not exactly the required length.
    WrongLength {
        /// Field number.
        field: u8,
        /// Length the specification requires.
        expected: usize,
        /// Length that was supplied.
        found: usize,
    },
    /// A field's value is not valid for its encoding, such as a non-digit in a
    /// binary-coded decimal field.
    InvalidValue {
        /// Field number.
        field: u8,
        /// What was wrong.
        reason: &'static str,
    },
    /// The byte stream ended in the middle of a field.
    Truncated {
        /// Field number that was being read.
        field: u8,
        /// Bytes the field needed.
        needed: usize,
        /// Bytes that were available.
        available: usize,
    },
    /// Bytes left over after the last field, which means the bitmap and the
    /// field data disagree.
    TrailingBytes(usize),
    /// The input was empty.
    Empty,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadMti(mti) => {
                write!(f, "message type indicator {mti:?} is not four characters")
            }
            Self::UnknownField(n) => write!(f, "specification has no field {n}"),
            Self::BadSpec(reason) => write!(f, "specification is inconsistent: {reason}"),
            Self::FieldOutOfRange(n) => write!(f, "field {n} is outside the range 1-192"),
            Self::TooLong { field, max, found } => {
                write!(f, "field {field} is {found} characters, maximum is {max}")
            }
            Self::WrongLength {
                field,
                expected,
                found,
            } => write!(f, "field {field} needs {expected} characters, got {found}"),
            Self::InvalidValue { field, reason } => write!(f, "field {field}: {reason}"),
            Self::Truncated {
                field,
                needed,
                available,
            } => write!(
                f,
                "field {field} needs {needed} bytes but only {available} remain"
            ),
            Self::TrailingBytes(n) => write!(f, "{n} bytes left over after the last field"),
            Self::Empty => f.write_str("message is empty"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// The number of fields the format covers: 64 in the primary bitmap, 64 in the
/// secondary, and 64 more in the optional tertiary.
pub const MAX_FIELDS: u8 = 192;
