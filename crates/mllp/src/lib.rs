//! MLLP framing for HL7 v2 messages.
//!
//! MLLP, the Minimal Lower Layer Protocol, is how almost every HL7 v2 message
//! travels over TCP. It is a session layer and nothing more: each message is
//! wrapped in a start block, the payload, and an end block, so a receiver
//! reading a byte stream can tell where one message stops and the next begins.
//!
//! The framing is three bytes:
//!
//! | Role | Name | Byte |
//! |---|---|---|
//! | Start block | `SB`, vertical tab | `0x0B` |
//! | End block | `EB`, file separator | `0x1C` |
//! | Trailing | `CR`, carriage return | `0x0D` |
//!
//! A frame on the wire is `0x0B` + payload + `0x1C 0x0D`.
//!
//! # Example
//!
//! ```
//! use mllp::{frame, unframe};
//!
//! // An HL7 v2 message ends with a carriage return of its own, which is the
//! // reason the terminator is two bytes and not one: a lone `0x0D` inside the
//! // payload must not be mistaken for the end of the block.
//! let message = b"MSH|^~\\&|SEND|FAC|RECV|FAC|20260101||ADT^A01|1|P|2.5.1\r";
//!
//! let wire = frame(message);
//! assert_eq!(wire[0], 0x0B);
//! assert_eq!(&wire[wire.len() - 2..], &[0x1C, 0x0D]);
//!
//! let payload = unframe(&wire).unwrap();
//! assert_eq!(payload, message);
//! ```
//!
//! # Reading a stream
//!
//! A TCP read returns whatever has arrived, which is rarely a whole frame. A
//! [`Decoder`] holds the leftover bytes between reads, so a frame split at any
//! position still comes out whole, and a single read carrying two frames yields
//! two messages:
//!
//! ```
//! use mllp::Decoder;
//!
//! let mut decoder = Decoder::new();
//!
//! // The frame arrives in two pieces, split in the middle of the payload.
//! decoder.push(b"\x0bMSH|^~\\&|A|B|2026");
//! assert!(decoder.next_frame().is_none());
//! decoder.push(b"0101||ADT^A01|1|P|2.5.1\r\x1c\x0d");
//!
//! let message = decoder.next_frame().unwrap().unwrap();
//! assert!(message.starts_with(b"MSH|"));
//! assert_eq!(decoder.next_frame(), None);
//! ```
//!
//! # Scope
//!
//! This crate frames bytes. It does not parse HL7 v2 itself: the payload is
//! opaque, and the delimiters that make up a message (`|`, `^`, `~`, `&`) are
//! the caller's business. It also does not open sockets, retry, or decide how
//! long to wait for a reply. The one piece of HL7 knowledge here is that a
//! payload may contain a carriage return, which is why the terminator is the
//! pair `0x1C 0x0D`.

#![forbid(unsafe_code)]

mod decoder;
mod frame;

pub use decoder::Decoder;
pub use frame::{CARRIAGE_RETURN, END_BLOCK, START_BLOCK, frame, unframe};

/// The README, compiled as a doctest so its example cannot drift from the API.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

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
