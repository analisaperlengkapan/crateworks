//! A streaming decoder that reassembles frames from arbitrary reads.

use crate::Error;
use crate::frame::{START_BLOCK, find_terminator};

/// The default ceiling on one frame, in bytes.
///
/// A frame that never terminates would otherwise grow the buffer without bound,
/// so a reader can be fed garbage and run the process out of memory. 16 MiB is
/// far above any real HL7 v2 message and small enough to be safe.
pub const DEFAULT_MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Reassembles MLLP frames from a byte stream.
///
/// A TCP read returns whatever has arrived, which is rarely a whole frame: it
/// can be half a frame, or three frames and part of a fourth. Push each read in
/// with [`Decoder::push`] and take whole messages out with
/// [`Decoder::next_frame`], which returns `None` when it needs more bytes.
///
/// ```
/// use mllp::Decoder;
///
/// let mut decoder = Decoder::new();
///
/// // Two whole frames in one read.
/// decoder.push(b"\x0bMSH|first\r\x1c\x0d\x0bMSH|second\r\x1c\x0d");
///
/// assert_eq!(decoder.next_frame().unwrap().unwrap(), b"MSH|first\r");
/// assert_eq!(decoder.next_frame().unwrap().unwrap(), b"MSH|second\r");
/// assert_eq!(decoder.next_frame(), None);
/// ```
#[derive(Debug)]
pub struct Decoder {
    buffer: Vec<u8>,
    max_frame_size: usize,
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder {
    /// A decoder with the default frame-size ceiling.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
        }
    }

    /// A decoder that rejects a frame longer than `max_frame_size` bytes.
    ///
    /// The limit covers the frame from its start block to the byte before its
    /// terminator, so the three framing bytes are not counted.
    #[must_use]
    pub fn with_max_frame_size(max_frame_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_frame_size,
        }
    }

    /// Adds bytes from the stream.
    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Takes the next complete frame, if one has arrived.
    ///
    /// Returns `Ok(payload)` for a whole frame, `Err` for input that cannot be
    /// part of a frame, and `None` when the buffer holds only part of a frame so
    /// far. Call it in a loop after each [`Decoder::push`] to drain everything
    /// the read delivered.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingStartBlock`] for bytes that are not the start of
    /// a frame; the bytes up to the next start block are discarded, so the
    /// decoder recovers. Returns [`Error::FrameTooLarge`] when a frame grows
    /// past the ceiling, which is the sign of a missing terminator rather than a
    /// real message.
    pub fn next_frame(&mut self) -> Option<Result<Vec<u8>, Error>> {
        if self.buffer.is_empty() {
            return None;
        }

        // Anything before the next start block is not part of a frame. Drop it
        // and report once, so a caller can log the desync and carry on.
        if self.buffer[0] != START_BLOCK {
            match self.buffer.iter().position(|b| *b == START_BLOCK) {
                Some(i) => {
                    self.buffer.drain(..i);
                }
                None => self.buffer.clear(),
            }
            return Some(Err(Error::MissingStartBlock));
        }

        if let Some(end) = find_terminator(&self.buffer, 1) {
            let payload = self.buffer[1..end].to_vec();
            self.buffer.drain(..end + 2);
            return Some(Ok(payload));
        }

        // No terminator yet. Either the rest is still in flight, or the stream
        // is broken and this will never end.
        if self.buffer.len() > self.max_frame_size {
            self.buffer.clear();
            return Some(Err(Error::FrameTooLarge {
                max: self.max_frame_size,
            }));
        }
        None
    }

    /// The bytes held back because they are not yet a whole frame.
    #[must_use]
    pub fn buffered(&self) -> &[u8] {
        &self.buffer
    }

    /// Whether no partial frame is held back.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Drops any partial frame, for a connection that is being reset.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// The frame-size ceiling this decoder enforces.
    #[must_use]
    pub fn max_frame_size(&self) -> usize {
        self.max_frame_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_frames_in_one_read() {
        let mut d = Decoder::new();
        d.push(b"\x0bMSH|a\r\x1c\x0d\x0bMSH|b\r\x1c\x0d");
        assert_eq!(d.next_frame().unwrap().unwrap(), b"MSH|a\r");
        assert_eq!(d.next_frame().unwrap().unwrap(), b"MSH|b\r");
        assert_eq!(d.next_frame(), None);
        assert!(d.is_empty());
    }

    #[test]
    fn a_frame_split_at_every_position() {
        let payload = b"MSH|^~\\&|A|B|20260101||ADT^A01|1|P|2.5.1\r";
        let wire = crate::frame(payload);

        // Every split point, including before the start block and after the
        // terminator, has to reassemble to the same payload.
        for split in 0..=wire.len() {
            let mut d = Decoder::new();
            d.push(&wire[..split]);
            let first = d.next_frame();
            d.push(&wire[split..]);

            let mut got = Vec::new();
            while let Some(frame) = d.next_frame() {
                got.push(frame.unwrap());
            }
            if let Some(first) = first {
                got.insert(0, first.unwrap());
            }
            assert_eq!(got, vec![payload.to_vec()], "split at {split}");
        }
    }

    #[test]
    fn partial_frame_is_held_back() {
        let mut d = Decoder::new();
        d.push(b"\x0bMSH|a");
        assert_eq!(d.next_frame(), None);
        assert_eq!(d.buffered(), b"\x0bMSH|a");
        d.push(b"\r\x1c\x0d");
        assert_eq!(d.next_frame().unwrap().unwrap(), b"MSH|a\r");
    }

    #[test]
    fn a_lone_end_block_waits_for_the_carriage_return() {
        let mut d = Decoder::new();
        d.push(b"\x0bMSH|a\r\x1c");
        assert_eq!(d.next_frame(), None);
        d.push(b"\x0d");
        assert_eq!(d.next_frame().unwrap().unwrap(), b"MSH|a\r");
    }

    #[test]
    fn recovers_from_bytes_before_the_start_block() {
        let mut d = Decoder::new();
        d.push(b"garbage\x0bMSH|a\r\x1c\x0d");
        assert_eq!(d.next_frame(), Some(Err(Error::MissingStartBlock)));
        assert_eq!(d.next_frame().unwrap().unwrap(), b"MSH|a\r");
    }

    #[test]
    fn reports_and_drops_pure_garbage() {
        let mut d = Decoder::new();
        d.push(b"not a frame");
        assert_eq!(d.next_frame(), Some(Err(Error::MissingStartBlock)));
        assert_eq!(d.next_frame(), None);
        assert!(d.is_empty());
    }

    #[test]
    fn refuses_a_frame_that_never_terminates() {
        let mut d = Decoder::with_max_frame_size(8);
        d.push(b"\x0b0123456789");
        assert_eq!(d.next_frame(), Some(Err(Error::FrameTooLarge { max: 8 })));
        assert!(d.is_empty());
    }

    #[test]
    fn clear_drops_a_partial_frame() {
        let mut d = Decoder::new();
        d.push(b"\x0bMSH|a");
        d.clear();
        assert!(d.is_empty());
        assert_eq!(d.next_frame(), None);
    }
}
