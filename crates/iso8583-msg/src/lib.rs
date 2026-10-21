//! ISO 8583 messages: the format card switches use for ATM, EDC and POS
//! transactions.
//!
//! An ISO 8583 message is a four-character message type indicator, one or two
//! bitmaps saying which fields are present, and then those fields in ascending
//! order. Each field has its own type, length and encoding, so the format only
//! makes sense together with a specification that says how each of the 128 (or
//! 192) fields is laid out.
//!
//! # Example
//!
//! ```
//! use iso8583_msg::{Message, default_ascii};
//!
//! let spec = default_ascii();
//!
//! let mut msg = Message::new("0200").unwrap();
//! msg.set(2, "4111111111111111").unwrap();
//! msg.set(3, "000000").unwrap();
//! msg.set(4, "000000010000").unwrap();
//!
//! let bytes = msg.encode(&spec).unwrap();
//! assert_eq!(&bytes[..4], b"0200");
//!
//! let back = Message::decode(&bytes, &spec).unwrap();
//! assert_eq!(back.get(2), Some("4111111111111111"));
//! assert_eq!(back.get(4), Some("000000010000"));
//! assert_eq!(back, msg);
//! ```
//!
//! # Specifications
//!
//! ISO 8583 does not standardise field lengths. Every switch publishes its own
//! dialect, so a [`Spec`] is data rather than code: build one with [`Spec::new`]
//! and describe each field you use. [`default_ascii`], [`default_bcd`] and
//! [`default_ebcdic`] are the usual starting points, and [`FieldSpec`] carries
//! the same information as a field dictionary in a switch's interface
//! specification.
//!
//! # Scope
//!
//! Only the framing is handled: message type indicator, bitmaps, field
//! encoding and length prefixes. Field *contents* are treated as opaque
//! strings, because what a processing code or a response code means is defined
//! per switch. No business-rule validation is attempted.

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
