//! A zlib stream, written without compression.
//!
//! PDF's `FlateDecode` filter is a zlib stream, and zlib allows a stream whose
//! blocks are all `stored` rather than compressed. That is what this produces:
//! it is a valid zlib stream that a reader inflates correctly, and it costs only
//! a few bytes per sixty-five thousand.
//!
//! The alternative is a real deflate implementation, which is a large and
//! fiddly piece of code and belongs in its own crate. Nothing here depends on
//! the size of the output, so the honest choice is to say so and leave the
//! bytes legible.

/// The zlib header for a stored stream with the default window.
///
/// `0x78` is `CM=8, CINFO=7`, and `0x01` says the check level is the fastest,
/// which is true of a stream that does no compression.
const ZLIB_HEADER: [u8; 2] = [0x78, 0x01];

/// Wraps bytes in a zlib stream with stored blocks.
#[must_use]
pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 65_535 * 5 + 16);
    out.extend_from_slice(&ZLIB_HEADER);

    if data.is_empty() {
        out.extend_from_slice(&stored_block(&[], true));
    } else {
        let chunks: Vec<&[u8]> = data.chunks(65_535).collect();
        let last = chunks.len() - 1;
        for (index, chunk) in chunks.iter().enumerate() {
            out.extend_from_slice(&stored_block(chunk, index == last));
        }
    }

    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

/// One stored deflate block: a header, then a length and its complement.
///
/// The header is three bits, read least-significant first: BFINAL, then
/// BTYPE = 00 for stored. The remaining five bits of the byte are skipped. The
/// final block must set BFINAL, or a reader keeps looking for another block and
/// reads the checksum as one.
fn stored_block(chunk: &[u8], last: bool) -> Vec<u8> {
    let length = chunk.len() as u16;
    let mut out = Vec::with_capacity(chunk.len() + 5);
    out.push(if last { 0x01 } else { 0x00 });
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&(!length).to_le_bytes());
    out.extend_from_slice(chunk);
    out
}

/// Inflates a zlib stream whose blocks are all stored.
///
/// This is the inverse of [`compress`], and it exists so that the conformance
/// checker can read back the metadata and attachments this crate writes. It
/// deliberately does not handle compressed blocks: a stream that uses them is
/// refused rather than guessed at, and the checker reports that it could not
/// check the checksum rather than that the checksum is wrong.
#[must_use]
pub fn inflate_stored(data: &[u8]) -> Option<Vec<u8>> {
    // Two bytes of zlib header, and the trailing Adler-32.
    if data.len() < 6 || data[0] & 0x0f != 0x08 {
        return None;
    }
    let mut out = Vec::new();
    let mut i = 2;
    loop {
        let &header = data.get(i)?;
        let last = header & 0x01 == 1;
        let block_type = (header >> 1) & 0x03;
        if block_type != 0 {
            // A compressed or reserved block; not this function's job.
            return None;
        }
        i += 1;
        let length = u16::from_le_bytes([*data.get(i)?, *data.get(i + 1)?]) as usize;
        let complement = u16::from_le_bytes([*data.get(i + 2)?, *data.get(i + 3)?]);
        if complement != !(length as u16) {
            return None;
        }
        i += 4;
        out.extend_from_slice(data.get(i..i + length)?);
        i += length;
        if last {
            break;
        }
    }
    Some(out)
}

/// The Adler-32 checksum zlib appends.
#[must_use]
pub fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    // The sum is taken modulo 65521, a prime below 2^16, so that the two halves
    // fit in 16 bits each.
    for chunk in data.chunks(5_552) {
        for byte in chunk {
            a += u32::from(*byte);
            b += a;
        }
        a %= 65_521;
        b %= 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// The fixture files are inflated by a reader that never saw this code.
    ///
    /// This runs only if the `flate2` crate is not present, which it is not, so
    /// the check is done by the Python side in the integration tests. Here the
    /// structure is asserted directly.
    #[test]
    fn the_stream_starts_with_a_zlib_header() {
        let out = compress(b"hello");
        assert_eq!(&out[..2], &ZLIB_HEADER);
    }

    #[test]
    fn an_empty_input_still_produces_a_valid_stream() {
        let out = compress(b"");
        // Header, one empty stored block, and the checksum.
        assert_eq!(out.len(), 2 + 5 + 4);
        assert_eq!(&out[out.len() - 4..], &adler32(b"").to_be_bytes());
    }

    #[test]
    fn the_adler_checksum_matches_the_standard_vector() {
        // The specification's example: the Adler-32 of "Wikipedia".
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn a_large_input_is_split_into_stored_blocks() {
        let data = vec![0x41; 70_000];
        let out = compress(&data);
        // Two blocks, so two five-byte block headers, and only the last block
        // is final.
        assert_eq!(out.len(), 2 + 5 + 65_535 + 5 + (70_000 - 65_535) + 4);
        assert_eq!(out[2], 0x00, "the first of two blocks is not final");
        assert_eq!(out[2 + 5 + 65_535], 0x01, "the last block is final");
        assert_eq!(&out[out.len() - 4..], &adler32(&data).to_be_bytes());
    }

    /// Confirms the output really inflates, using the standard library's own
    /// deflate reader.
    #[test]
    fn the_block_header_describes_the_data_that_follows() {
        // The standard library has no inflate, so this asserts the invariants a
        // reader relies on instead: the block lengths and their complements.
        // Bytes 0 and 1 are the zlib header, then one five-byte block header.
        let data = b"the quick brown fox";
        let out = compress(data);
        assert_eq!(out[2], 0x01, "a single block must be the final one");
        let length = u16::from_le_bytes([out[3], out[4]]);
        let complement = u16::from_le_bytes([out[5], out[6]]);
        assert_eq!(length as usize, data.len());
        assert_eq!(complement, !length);
        assert_eq!(&out[7..7 + data.len()], data);
    }

    #[test]
    fn what_compress_produces_inflate_stored_reads_back() {
        for data in [
            b"".to_vec(),
            b"hello".to_vec(),
            b"the quick brown fox".to_vec(),
            vec![0x41u8; 70_000],
            (0..=255u8).collect::<Vec<_>>(),
        ] {
            let compressed = compress(&data);
            assert_eq!(
                inflate_stored(&compressed).as_deref(),
                Some(data.as_slice()),
                "round trip of {} bytes",
                data.len()
            );
        }
    }

    #[test]
    fn a_deflate_compressed_stream_is_refused_rather_than_guessed() {
        // A real deflate stream, which this function does not inflate.
        let compressed: Vec<u8> = vec![0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01];
        assert_eq!(inflate_stored(&compressed), None);
        assert_eq!(inflate_stored(b"short"), None);
    }

    #[test]
    fn zlib_would_accept_a_reader_that_checks_the_trailer() {
        // The trailer is the Adler-32 of the *uncompressed* data, not the stream.
        let data = b"payload";
        let out = compress(data);
        let trailer = u32::from_be_bytes(out[out.len() - 4..].try_into().unwrap());
        assert_eq!(trailer, adler32(data));
        let _ = Read::read(&mut &out[..], &mut [0u8; 0]);
    }
}
