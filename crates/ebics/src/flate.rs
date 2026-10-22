//! DEFLATE and zlib, both written and read.
//!
//! EBICS compresses order data with zlib before it is encrypted or base64
//! encoded: `OrderData` in an unsecured request and the plaintext under the
//! transaction key in an upload both travel as a zlib stream. The decompressor
//! here is the one from this workspace's `pdfa` crate, where it was written to
//! open PDF `FlateDecode` streams; a zlib stream is a zlib stream whatever the
//! container, so the code moved rather than being written twice.
//!
//! [`compress`] writes stored blocks only. That is valid zlib, every reader
//! accepts it, and it costs a few bytes per sixty-five thousand. The order data
//! it wraps is XML that compresses poorly anyway, and the alternative is a
//! compressor whose output this crate would then have to defend.
//!
//! [`inflate`] reads what other implementations produce: stored, fixed-Huffman
//! and dynamic-Huffman blocks. A bank's response is inflated by it, and a bank
//! compresses with whatever zlib it was built with.

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
