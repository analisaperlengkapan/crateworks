//! One frame in, one frame out.

use crate::{Error, Result};

/// Start block, `SB`, ASCII vertical tab.
pub const START_BLOCK: u8 = 0x0B;

/// End block, `EB`, ASCII file separator.
pub const END_BLOCK: u8 = 0x1C;

/// Carriage return, `CR`, which closes the frame after the end block.
pub const CARRIAGE_RETURN: u8 = 0x0D;

/// Wraps a payload in an MLLP frame.
///
/// The payload is copied unchanged. A carriage return inside it is left alone,
/// because the terminator is the pair `0x1C 0x0D` and cannot be confused with a
/// lone `0x0D`.
#[must_use]
pub fn frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 3);
    out.push(START_BLOCK);
    out.extend_from_slice(payload);
    out.push(END_BLOCK);
    out.push(CARRIAGE_RETURN);
    out
}

/// Reads the payload out of exactly one frame.
///
/// # Errors
///
/// Returns [`Error::Empty`] for empty input, [`Error::MissingStartBlock`] when
/// the frame does not open with `0x0B`, [`Error::MissingEndBlock`] when no
/// terminator is present, and [`Error::TrailingBytes`] when anything follows
/// the frame.
///
/// The trailing case is the one to watch: a socket read often carries more than
/// one frame. Use [`Decoder`](crate::Decoder) for a stream; this function is for
/// a single frame already in hand.
pub fn unframe(input: &[u8]) -> Result<&[u8]> {
    if input.is_empty() {
        return Err(Error::Empty);
    }
    if input[0] != START_BLOCK {
        return Err(Error::MissingStartBlock);
    }

    if let Some(end) = find_terminator(input, 1) {
        let rest = end + 2;
        if rest < input.len() {
            return Err(Error::TrailingBytes(input.len() - rest));
        }
        return Ok(&input[1..end]);
    }

    Err(Error::MissingEndBlock)
}

/// The index of the `0x1C 0x0D` pair that ends the frame, at or after `from`.
///
/// The search is for the pair, not for the end block alone. A `0x1C` that is
/// not followed by `0x0D` is payload data and the search goes on: the payload of
/// a binary message can contain any byte, and only the pair is unambiguous.
pub(crate) fn find_terminator(input: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 1 < input.len() {
        if input[i] == END_BLOCK && input[i + 1] == CARRIAGE_RETURN {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let payload = b"MSH|^~\\&|A|B|20260101||ADT^A01|1|P|2.5.1\r";
        assert_eq!(unframe(&frame(payload)).unwrap(), payload);
    }

    #[test]
    fn frame_has_the_three_framing_bytes() {
        let wire = frame(b"x");
        assert_eq!(wire, vec![0x0B, b'x', 0x1C, 0x0D]);
    }

    #[test]
    fn empty_payload_is_a_valid_frame() {
        assert_eq!(frame(b""), vec![0x0B, 0x1C, 0x0D]);
        assert_eq!(unframe(&[0x0B, 0x1C, 0x0D]).unwrap(), b"");
    }

    #[test]
    fn carriage_returns_inside_the_payload_are_kept() {
        // Two segments, each ending with its own CR, as HL7 v2 writes them.
        let payload = b"MSH|a\rEVN|b\r";
        assert_eq!(unframe(&frame(payload)).unwrap(), payload);
    }

    #[test]
    fn end_block_not_followed_by_cr_is_payload_data() {
        let payload = b"a\x1cb";
        assert_eq!(unframe(&frame(payload)).unwrap(), payload);
    }

    #[test]
    fn errors() {
        assert_eq!(unframe(b""), Err(Error::Empty));
        assert_eq!(unframe(b"MSH|"), Err(Error::MissingStartBlock));
        assert_eq!(unframe(&[0x0B]), Err(Error::MissingEndBlock));
        assert_eq!(unframe(&[0x0B, b'a']), Err(Error::MissingEndBlock));
        assert_eq!(unframe(&[0x0B, 0x1C]), Err(Error::MissingEndBlock));
        assert_eq!(
            unframe(&[0x0B, b'a', 0x1C, 0x0D, b'b']),
            Err(Error::TrailingBytes(1))
        );
    }
}
