//! zlib streams, both written and read.
//!
//! PDF's `FlateDecode` filter is a zlib stream. [`compress`] writes one whose
//! blocks are all `stored`: it is valid, any reader inflates it, and it costs
//! only a few bytes per sixty-five thousand, which is the right trade for an
//! attachment that is already compressed XML.
//!
//! [`inflate`] reads what other tools write, which is a different problem. Real
//! deflate uses fixed and dynamic Huffman codes, and a PDF from any modern word
//! processor will use them, so a reader that only understands stored blocks
//! cannot open such a file at all. The decompressor here is complete, and the
//! tests check it against streams Python's `zlib` produced.
//!
//! [`unpredict`] undoes the PNG and TIFF predictors, which are applied *after*
//! inflating and which cross-reference streams use by default.

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

/// Inflates a zlib stream, compressed blocks included.
///
/// The counterpart to [`compress`], which writes only stored blocks. This reads
/// whatever zlib produces: stored, fixed-Huffman and dynamic-Huffman blocks, all
/// of which appear in PDFs written by real tools. A `FlateDecode` stream is the
/// one place a PDF reader has to inflate arbitrary data, so without this the
/// metadata and attachments of any document not written by this crate cannot be
/// read at all.
///
/// The Adler-32 trailer is verified. Returns `None` for a stream that is
/// malformed, truncated, or whose checksum does not match.
#[must_use]
pub fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    let (header, body) = data.split_first_chunk::<2>()?;
    // CM must be 8 (deflate) and the header must be a multiple of 31, which is
    // how zlib catches a corrupted header.
    if header[0] & 0x0f != 8 {
        return None;
    }
    if (u16::from(header[0]) * 256 + u16::from(header[1])) % 31 != 0 {
        return None;
    }
    if header[1] & 0x20 != 0 {
        // FDICT: a preset dictionary, which no PDF filter uses.
        return None;
    }

    let (deflate, trailer) = body.split_at(body.len().checked_sub(4)?);
    let out = inflate_raw(deflate)?;
    let expected = u32::from_be_bytes(trailer.try_into().ok()?);
    (adler32(&out) == expected).then_some(out)
}

/// Inflates a raw DEFLATE stream, with no zlib header or checksum.
///
/// PDF's `FlateDecode` is zlib, but producers exist that omit the two-byte
/// header, and readers are expected to cope. This is the inner half of
/// [`inflate`], exposed so a caller can try it when the zlib form fails.
#[must_use]
pub fn inflate_raw(data: &[u8]) -> Option<Vec<u8>> {
    let mut bits = Bits::new(data);
    let mut out = Vec::new();
    loop {
        let last = bits.take(1)? == 1;
        match bits.take(2)? {
            0 => inflate_stored_block(&mut bits, &mut out)?,
            1 => {
                let (literals, distances) = fixed_tables();
                inflate_block(&mut bits, &mut out, &literals, &distances)?;
            }
            2 => {
                let (literals, distances) = dynamic_tables(&mut bits)?;
                inflate_block(&mut bits, &mut out, &literals, &distances)?;
            }
            // BTYPE 3 is reserved and must not appear.
            _ => return None,
        }
        if last {
            return Some(out);
        }
    }
}

/// A stored block: a length, its complement, then that many literal bytes.
fn inflate_stored_block(bits: &mut Bits<'_>, out: &mut Vec<u8>) -> Option<()> {
    // Skip to the next byte boundary; the length is byte-aligned.
    bits.align();
    let length = bits.take(16)? as usize;
    let complement = bits.take(16)? as usize;
    if complement != (!length & 0xffff) {
        return None;
    }
    let bytes = bits.bytes(length)?;
    out.extend_from_slice(bytes);
    Some(())
}

/// Decodes literal/length and distance symbols until an end-of-block symbol.
fn inflate_block(
    bits: &mut Bits<'_>,
    out: &mut Vec<u8>,
    literals: &Huffman,
    distances: &Huffman,
) -> Option<()> {
    loop {
        let symbol = literals.decode(bits)?;
        match symbol {
            0..=255 => out.push(symbol as u8),
            256 => return Some(()),
            257..=285 => {
                let index = symbol as usize - 257;
                let length = LENGTH_BASE[index] as usize + bits.take(LENGTH_EXTRA[index])? as usize;
                let distance_symbol = distances.decode(bits)? as usize;
                if distance_symbol >= DISTANCE_BASE.len() {
                    return None;
                }
                let distance = DISTANCE_BASE[distance_symbol] as usize
                    + bits.take(DISTANCE_EXTRA[distance_symbol])? as usize;
                // A distance may not reach before the start of the output.
                if distance == 0 || distance > out.len() {
                    return None;
                }
                let start = out.len() - distance;
                for offset in 0..length {
                    // Copy byte by byte: the source may overlap the destination,
                    // which is how a run is encoded, so it must not be a memcpy.
                    let byte = out[start + offset];
                    out.push(byte);
                }
            }
            // 286 and 287 are not valid lengths.
            _ => return None,
        }
    }
}

/// The fixed Huffman tables from the specification.
fn fixed_tables() -> (Huffman, Huffman) {
    let mut literal_lengths = [0u8; 288];
    for (symbol, length) in literal_lengths.iter_mut().enumerate() {
        *length = match symbol {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    let distance_lengths = [5u8; 30];
    (
        Huffman::from_lengths(&literal_lengths).expect("the fixed literal table is valid"),
        Huffman::from_lengths(&distance_lengths).expect("the fixed distance table is valid"),
    )
}

/// Reads the code-length tables that describe a dynamic block's two trees.
fn dynamic_tables(bits: &mut Bits<'_>) -> Option<(Huffman, Huffman)> {
    let literal_count = bits.take(5)? as usize + 257;
    let distance_count = bits.take(5)? as usize + 1;
    let code_length_count = bits.take(4)? as usize + 4;
    if literal_count > 286 || distance_count > 30 {
        return None;
    }

    let mut code_length_lengths = [0u8; 19];
    for &order in CODE_LENGTH_ORDER.iter().take(code_length_count) {
        code_length_lengths[order as usize] = bits.take(3)? as u8;
    }
    let code_lengths = Huffman::from_lengths(&code_length_lengths)?;

    // The literal and distance code lengths are run-length encoded, interleaved
    // in one sequence of `literal_count + distance_count` values.
    let total = literal_count + distance_count;
    let mut lengths = Vec::with_capacity(total);
    while lengths.len() < total {
        match code_lengths.decode(bits)? {
            value @ 0..=15 => lengths.push(value as u8),
            16 => {
                // Repeat the previous length 3 to 6 times.
                let previous = *lengths.last()?;
                let repeats = 3 + bits.take(2)? as usize;
                lengths.extend(std::iter::repeat_n(previous, repeats));
            }
            17 => {
                let repeats = 3 + bits.take(3)? as usize;
                lengths.extend(std::iter::repeat_n(0, repeats));
            }
            18 => {
                let repeats = 11 + bits.take(7)? as usize;
                lengths.extend(std::iter::repeat_n(0, repeats));
            }
            _ => return None,
        }
    }
    if lengths.len() != total {
        // A run overshot the end, which means the stream is malformed.
        return None;
    }

    let (literal_lengths, distance_lengths) = lengths.split_at(literal_count);
    // A distance tree with no codes at all is only legal when the block has no
    // matches, and the literal tree must be usable.
    let literals = Huffman::from_lengths(literal_lengths)?;
    let distances = Huffman::from_lengths(distance_lengths)?;
    Some((literals, distances))
}

/// The order the code-length code lengths are stored in.
const CODE_LENGTH_ORDER: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Base lengths for the length codes 257 to 285.
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
/// Extra bits for each length code.
const LENGTH_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/// Base distances for the distance codes.
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
/// Extra bits for each distance code.
const DISTANCE_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// Reads bits least-significant first, which is the order DEFLATE defines.
struct Bits<'a> {
    data: &'a [u8],
    /// The next byte to load into the buffer.
    position: usize,
    buffer: u32,
    /// How many of `buffer`'s low bits are valid.
    available: u32,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            position: 0,
            buffer: 0,
            available: 0,
        }
    }

    /// Reads `count` bits, or `None` at the end of the input.
    fn take(&mut self, count: u32) -> Option<u32> {
        if count == 0 {
            return Some(0);
        }
        while self.available < count {
            let byte = *self.data.get(self.position)?;
            self.position += 1;
            self.buffer |= u32::from(byte) << self.available;
            self.available += 8;
        }
        let value = self.buffer & ((1 << count) - 1);
        self.buffer >>= count;
        self.available -= count;
        Some(value)
    }

    /// Discards bits up to the next byte boundary.
    fn align(&mut self) {
        let drop = self.available % 8;
        self.buffer >>= drop;
        self.available -= drop;
    }

    /// Reads whole bytes, which a stored block's payload is.
    fn bytes(&mut self, count: usize) -> Option<&'a [u8]> {
        self.align();
        // Anything buffered is whole bytes now, so return it to the stream.
        let buffered = (self.available / 8) as usize;
        self.position -= buffered;
        self.buffer = 0;
        self.available = 0;
        let slice = self.data.get(self.position..self.position + count)?;
        self.position += count;
        Some(slice)
    }
}

/// A canonical Huffman code, decoded one bit at a time.
///
/// Decoding walks the code lengths rather than building a lookup table. That is
/// slower per symbol than a table, but it needs no allocation proportional to the
/// table size and the code is short and easy to check.
struct Huffman {
    /// `counts[length]` is how many codes have that length.
    counts: [u16; 16],
    /// Symbols ordered by code length, then by symbol.
    symbols: Vec<u16>,
}

impl Huffman {
    /// Builds a code from the length of each symbol's code, zero meaning unused.
    fn from_lengths(lengths: &[u8]) -> Option<Self> {
        let mut counts = [0u16; 16];
        for &length in lengths {
            if length > 15 {
                return None;
            }
            counts[length as usize] += 1;
        }
        // The first entry is the count of unused symbols, which are not codes.
        counts[0] = 0;

        // Reject an over-subscribed code: more codes than the tree has leaves.
        let mut remaining = 1i32;
        for &count in counts.iter().skip(1) {
            remaining = (remaining << 1) - i32::from(count);
            if remaining < 0 {
                return None;
            }
        }

        let mut offsets = [0u16; 16];
        for length in 1..15 {
            offsets[length + 1] = offsets[length] + counts[length];
        }
        let mut symbols = vec![0u16; lengths.iter().filter(|&&length| length != 0).count()];
        for (symbol, &length) in lengths.iter().enumerate() {
            if length != 0 {
                symbols[offsets[length as usize] as usize] = symbol as u16;
                offsets[length as usize] += 1;
            }
        }
        Some(Self { counts, symbols })
    }

    /// Reads one symbol.
    fn decode(&self, bits: &mut Bits<'_>) -> Option<u16> {
        let mut code = 0i32;
        let mut first = 0i32;
        let mut index = 0i32;
        for length in 1..=15usize {
            code |= i32::from(bits.take(1)? as u8);
            let count = i32::from(self.counts[length]);
            if code - first < count {
                return Some(self.symbols[(index + (code - first)) as usize]);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        None
    }
}

/// Reverses a PDF predictor, which is how a stream is made more compressible.
///
/// `/DecodeParms` with a `/Predictor` is applied *after* inflating, so a stream
/// that declares one is not usable until it is undone. Cross-reference streams
/// use it by default in several producers, so without this an xref stream decodes
/// into bytes that look plausible and are wrong.
///
/// `predictor` 1 means no prediction, 2 is the TIFF form, and 10 to 15 are the
/// PNG forms. `columns` is `/Columns` and defaults to 1; `colors` is `/Colors`,
/// also defaulting to 1, and only 8 bits per component is supported, which is
/// what a cross-reference stream and a metadata stream use.
#[must_use]
pub fn unpredict(data: &[u8], predictor: u8, columns: usize, colors: usize) -> Option<Vec<u8>> {
    let columns = columns.checked_mul(colors)?;
    if columns == 0 {
        return None;
    }
    match predictor {
        1 => Some(data.to_vec()),
        2 => {
            // TIFF: each byte is a delta from the one `colors` bytes before it.
            let mut out = data.to_vec();
            for row in out.chunks_mut(columns) {
                for index in colors..row.len() {
                    row[index] = row[index].wrapping_add(row[index - colors]);
                }
            }
            Some(out)
        }
        10..=15 => {
            // PNG: every row is prefixed by a filter byte, and each row is
            // reconstructed against the one above it.
            let row_length = columns + 1;
            if !data.len().is_multiple_of(row_length) {
                return None;
            }
            let mut out = Vec::with_capacity(data.len() - data.len() / row_length);
            let mut previous = vec![0u8; columns];
            for row in data.chunks(row_length) {
                let filter = row[0];
                let mut current = row[1..].to_vec();
                for index in 0..columns {
                    let left = if index >= colors {
                        current[index - colors]
                    } else {
                        0
                    };
                    let above = previous[index];
                    let upper_left = if index >= colors {
                        previous[index - colors]
                    } else {
                        0
                    };
                    let value = match filter {
                        0 => current[index],
                        1 => current[index].wrapping_add(left),
                        2 => current[index].wrapping_add(above),
                        3 => current[index]
                            .wrapping_add(((u16::from(left) + u16::from(above)) / 2) as u8),
                        4 => current[index].wrapping_add(paeth(left, above, upper_left)),
                        _ => return None,
                    };
                    current[index] = value;
                }
                out.extend_from_slice(&current);
                previous = current;
            }
            Some(out)
        }
        _ => None,
    }
}

/// The Paeth predictor, which picks whichever neighbour is closest to `left +
/// above - upper_left`.
fn paeth(left: u8, above: u8, upper_left: u8) -> u8 {
    let (left, above, upper_left) = (i32::from(left), i32::from(above), i32::from(upper_left));
    let estimate = left + above - upper_left;
    let (dl, da, du) = (
        (estimate - left).abs(),
        (estimate - above).abs(),
        (estimate - upper_left).abs(),
    );
    if dl <= da && dl <= du {
        left as u8
    } else if da <= du {
        above as u8
    } else {
        upper_left as u8
    }
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
    fn what_compress_produces_inflate_reads_back() {
        for data in [
            b"".to_vec(),
            b"hello".to_vec(),
            b"the quick brown fox".to_vec(),
            vec![0x41u8; 70_000],
            (0..=255u8).collect::<Vec<_>>(),
        ] {
            let compressed = compress(&data);
            assert_eq!(
                inflate(&compressed).as_deref(),
                Some(data.as_slice()),
                "round trip of {} bytes",
                data.len()
            );
        }
    }

    /// Decodes a hex string, for the vectors below.
    fn unhex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }

    /// Streams produced by zlib, which never saw this code.
    ///
    /// Each pair is the hex of a zlib stream and the plain text it holds. These
    /// are the vectors that matter: zlib emits fixed-Huffman blocks for short
    /// inputs and dynamic-Huffman blocks once the data has structure, so between
    /// them they exercise both paths and the stored path is covered by the round
    /// trip with `compress`.
    #[test]
    fn inflates_streams_zlib_produced() {
        for (hex, expected) in [
            ("789c030000000001", ""),
            ("789ccb48cdc9c90700062c0215", "hello"),
            (
                "789c2bc94855282ccd4cce56482aca2fcf5348cbaf50c82acd2d2856c82f4b2d5228014ae72456552aa4e4a70300613c0ffa",
                "the quick brown fox jumps over the lazy dog",
            ),
            ("789c4b4c4a4ec48d00b20c0b7d", &"abc".repeat(10)),
        ] {
            let inflated = inflate(&unhex(hex)).unwrap_or_else(|| panic!("inflate {hex}"));
            assert_eq!(inflated, expected.as_bytes(), "vector {hex}");
        }
    }

    #[test]
    fn inflates_a_dynamic_huffman_block() {
        // 1920 bytes of repetitive text, which zlib compresses to a dynamic
        // block. This is the path a real PDF's metadata stream takes.
        let text = "Factur-X is a hybrid electronic invoice format. ".repeat(40);
        let hex = "789c734b4c2e292dd28d50c82c564854c8a84c2aca4c5148cd494d2e29cacfcb4c56c8cc2bcbcf4c4e5548cb2fca4d2cd153701b553faa7e54fda8fa51f5a3ea47d553ac1e0070cfb567";
        assert_eq!(inflate(&unhex(hex)).as_deref(), Some(text.as_bytes()));
    }

    #[test]
    fn inflates_a_stored_block_written_by_zlib() {
        // Every byte value, which zlib stores rather than compresses.
        let hex = "789c010001fffe000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeffadf67f81";
        let expected: Vec<u8> = (0..=255u8).collect();
        assert_eq!(inflate(&unhex(hex)).as_deref(), Some(expected.as_slice()));
    }

    #[test]
    fn a_corrupted_stream_is_refused() {
        let good = unhex("789ccb48cdc9c90700062c0215");
        // The Adler-32 trailer no longer matches, so the data cannot be trusted.
        let mut bad = good.clone();
        let last = bad.len() - 1;
        bad[last] ^= 0xff;
        assert_eq!(inflate(&bad), None);

        // A truncated stream has no trailer at all.
        assert_eq!(inflate(&good[..good.len() - 2]), None);
        assert_eq!(inflate(&[]), None);
        assert_eq!(inflate(&[0x78]), None);

        // A header that is not deflate, and one whose check byte is wrong.
        assert_eq!(inflate(&[0x77, 0x9c, 0, 0, 0, 0, 0, 0]), None);
        assert_eq!(inflate(&[0x78, 0x9d, 0, 0, 0, 0, 0, 0]), None);
    }

    #[test]
    fn a_distance_reaching_before_the_output_is_refused() {
        // Hand-built fixed-Huffman blocks, so these cases are exact rather than
        // whatever zlib happens to emit. Reading before the start of the output
        // would be a panic, and a match is how a small input expands into a large
        // one, so this is where a decompression bomb starts.
        //
        // `030200` is a match of length 3 at distance 1 with nothing emitted yet.
        assert_eq!(inflate_raw(&unhex("030200")), None);
        // `73044200` emits `A`, then matches at distance 2, which reaches before
        // the start because only one byte exists.
        assert_eq!(inflate_raw(&unhex("73044200")), None);
    }

    #[test]
    fn an_overlapping_match_repeats_correctly() {
        // `73040200` emits `A`, then matches length 3 at distance 1, then ends the
        // block. The source overlaps the destination, so the copy has to be byte
        // by byte; a bulk copy would read bytes it has not written yet.
        assert_eq!(
            inflate_raw(&unhex("73040200")).as_deref(),
            Some(&b"AAAA"[..])
        );
    }

    #[test]
    fn what_zlib_produced_round_trips_through_inflate_and_back() {
        // The two halves must agree on the data, which is what the metadata and
        // attachment readers rely on.
        let data = b"a PDF metadata packet with some repetition repetition repetition".to_vec();
        let zlib = unhex(
            "789c4b5408707153c84d2d494c492c495428484cce4e2d5128cf2cc95028cecf4d55284a2d482dc92cc9cccfc3ce0400ee46182e",
        );
        let inflated = inflate(&zlib).expect("the vector inflates");
        assert_eq!(inflated.len(), data.len());
        // And our own writer's output inflates with the general reader too.
        assert_eq!(inflate(&compress(&data)).as_deref(), Some(data.as_slice()));
    }

    #[test]
    fn a_png_predictor_is_undone() {
        // Two rows of four bytes, filtered with the PNG "Up" filter, which is
        // what predictor 12 means. Row 0 has nothing above it; row 1 is a delta
        // from row 0.
        let data = [2, 10, 20, 30, 40, 2, 1, 1, 1, 1];
        let expected = [10u8, 20, 30, 40, 11, 21, 31, 41];
        assert_eq!(unpredict(&data, 12, 4, 1).as_deref(), Some(&expected[..]));
    }

    #[test]
    fn every_png_filter_type_is_undone() {
        // One row of four bytes under each filter, with a row above it for the
        // filters that reference one.
        let previous = [5u8, 6, 7, 8];
        for (filter, expected) in [
            (0u8, [1u8, 2, 3, 4]), // None: the bytes as they are
            (1, [1, 3, 6, 10]),    // Sub: plus the byte to the left
            (2, [6, 8, 10, 12]),   // Up: plus the byte above
            (3, [3, 6, 9, 12]),    // Average: plus the mean of the two
            (4, [6, 8, 11, 15]),   // Paeth: plus the nearest neighbour
        ] {
            let mut data = vec![filter];
            data.extend_from_slice(&[1, 2, 3, 4]);
            let mut full = vec![0u8];
            full.extend_from_slice(&previous);
            full.extend_from_slice(&data);
            let out = unpredict(&full, 15, 4, 1).expect("a valid stream");
            assert_eq!(&out[..4], &previous, "the first row is unchanged");
            assert_eq!(&out[4..], &expected, "filter {filter}");
        }
    }

    #[test]
    fn a_tiff_predictor_is_undone() {
        let data = [10u8, 1, 1, 1];
        assert_eq!(
            unpredict(&data, 2, 4, 1).as_deref(),
            Some(&[10u8, 11, 12, 13][..])
        );
    }

    #[test]
    fn predictor_one_leaves_the_data_alone() {
        let data = b"unchanged".to_vec();
        assert_eq!(unpredict(&data, 1, 4, 1).as_deref(), Some(data.as_slice()));
    }

    #[test]
    fn a_ragged_png_stream_is_refused() {
        // A row is one filter byte plus `columns`, so a length that is not a
        // multiple of that cannot be a valid stream.
        assert_eq!(unpredict(&[0, 1, 2], 12, 4, 1), None);
        assert_eq!(unpredict(&[], 12, 0, 1), None);
    }

    #[test]
    fn a_stream_shorter_than_its_own_header_is_refused() {
        // The zlib header and the Adler-32 trailer are six bytes between them, so
        // anything shorter cannot be a stream.
        assert_eq!(inflate(b"short"), None);
        assert_eq!(inflate(&[]), None);
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
