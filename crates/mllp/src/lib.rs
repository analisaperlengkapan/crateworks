#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod decoder;
mod frame;

pub use decoder::Decoder;
pub use frame::{CARRIAGE_RETURN, END_BLOCK, START_BLOCK, frame, unframe};

/// Why a byte sequence is not a valid MLLP frame.
///
/// The streaming [`Decoder`] reports the same errors, except that it can also
/// wait for more input, which it signals with `None` instead of an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The input was empty.
    Empty,
    /// The input does not begin with the start block, `0x0B`. The [`Decoder`]
    /// skips ahead to the next start block, so this reports a desync that it
    /// has already recovered from.
    MissingStartBlock,
    /// The input has no end block followed by a carriage return.
    MissingEndBlock,
    /// Bytes followed the closing carriage return, so this is more than one
    /// frame. Use [`Decoder`] to read a stream.
    TrailingBytes(usize),
    /// A frame grew past the decoder's ceiling without ever terminating, which
    /// means the terminator is missing rather than that the message is large.
    FrameTooLarge {
        /// The ceiling that was exceeded.
        max: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("input is empty"),
            Self::MissingStartBlock => {
                write!(
                    f,
                    "input does not begin with the start block {START_BLOCK:#04X}"
                )
            }
            Self::MissingEndBlock => {
                write!(
                    f,
                    "no end block ({END_BLOCK:#04X}) followed by a carriage return"
                )
            }
            Self::TrailingBytes(n) => {
                write!(
                    f,
                    "{n} bytes after the frame; this looks like more than one frame"
                )
            }
            Self::FrameTooLarge { max } => {
                write!(f, "frame exceeds {max} bytes without terminating")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
