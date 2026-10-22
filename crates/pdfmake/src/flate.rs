//! A DEFLATE compressor, used to compress PDF streams.
//!
//! PDF content streams are mostly repeated drawing operators, so they compress
//! well, and a smaller file is the whole reason `/FlateDecode` exists. Only the
//! fixed-Huffman encoding of RFC 1951 is implemented: it needs no per-file code
//! table, which keeps the output self-describing and the code small. Dynamic
//! Huffman would compress a little better on long streams and is not worth the
//! complexity here.

/// Length codes 257..=285: the base length and how many extra bits follow.
const LENGTH_TABLE: [(u16, u8); 29] = [
    (3, 0),
    (4, 0),
    (5, 0),
    (6, 0),
    (7, 0),
    (8, 0),
    (9, 0),
    (10, 0),
    (11, 1),
    (13, 1),
    (15, 1),
    (17, 1),
    (19, 2),
    (23, 2),
    (27, 2),
    (31, 2),
    (35, 3),
    (43, 3),
    (51, 3),
    (59, 3),
    (67, 4),
    (83, 4),
    (99, 4),
    (115, 4),
    (131, 5),
    (163, 5),
    (195, 5),
    (227, 5),
    (258, 0),
];

/// Distance codes 0..=29: the base distance and how many extra bits follow.
const DISTANCE_TABLE: [(u16, u8); 30] = [
    (1, 0),
    (2, 0),
    (3, 0),
    (4, 0),
    (5, 1),
    (7, 1),
    (9, 2),
    (13, 2),
    (17, 3),
    (25, 3),
    (33, 4),
    (49, 4),
    (65, 5),
    (97, 5),
    (129, 6),
    (193, 6),
    (257, 7),
    (385, 7),
    (513, 8),
    (769, 8),
    (1025, 9),
    (1537, 9),
    (2049, 10),
    (3073, 10),
    (4097, 11),
    (6145, 11),
    (8193, 12),
    (12289, 12),
    (16385, 13),
    (24577, 13),
];

const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const WINDOW: usize = 32768;
/// Bits of the rolling hash used to find candidate matches.
const HASH_BITS: usize = 15;
const HASH_SIZE: usize = 1 << HASH_BITS;

/// Writes bits least-significant first, as DEFLATE requires.
struct BitWriter {
    out: Vec<u8>,
    buffer: u32,
    bits: u32,
}

impl BitWriter {
    fn new() -> BitWriter {
        BitWriter {
            out: Vec::new(),
            buffer: 0,
            bits: 0,
        }
    }

    /// Write the low `count` bits of `value`, least-significant bit first.
    fn write(&mut self, value: u32, count: u32) {
        self.buffer |= value << self.bits;
        self.bits += count;
        while self.bits >= 8 {
            self.out.push((self.buffer & 0xff) as u8);
            self.buffer >>= 8;
            self.bits -= 8;
        }
    }

    /// Write a Huffman code, which DEFLATE packs most-significant bit first.
    fn write_code(&mut self, code: u32, count: u32) {
        self.write(reverse_bits(code, count), count);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            self.out.push((self.buffer & 0xff) as u8);
        }
        self.out
    }
}

/// Reverse the low `count` bits of `value`.
fn reverse_bits(value: u32, count: u32) -> u32 {
    let mut result = 0;
    for i in 0..count {
        result |= ((value >> i) & 1) << (count - 1 - i);
    }
    result
}

/// The fixed literal/length Huffman code for `symbol`, and its bit length.
fn fixed_literal_code(symbol: u16) -> (u32, u32) {
    match symbol {
        0..=143 => (0x30 + u32::from(symbol), 8),
        144..=255 => (0x190 + u32::from(symbol) - 144, 9),
        256..=279 => (u32::from(symbol) - 256, 7),
        _ => (0xc0 + u32::from(symbol) - 280, 8),
    }
}

/// The length code and extra bits for a match of `length` bytes.
fn length_code(length: usize) -> (u16, u16, u8) {
    // 258 is reachable both as code 285 with no extra bits and as code 284 with
    // extra bits set to 31. The specification's table lists 285 as the code for
    // 258, so take that one: it is shorter and unambiguous.
    if length >= MAX_MATCH {
        return (285, 0, 0);
    }
    for (index, &(base, extra)) in LENGTH_TABLE.iter().enumerate() {
        let span = 1usize << extra;
        if length >= usize::from(base) && length < usize::from(base) + span {
            return (
                257 + index as u16,
                (length - usize::from(base)) as u16,
                extra,
            );
        }
    }
    unreachable!("length {length} is outside the DEFLATE table")
}

/// The distance code and extra bits for a match at `distance`.
fn distance_code(distance: usize) -> (u16, u16, u8) {
    for (index, &(base, extra)) in DISTANCE_TABLE.iter().enumerate() {
        let span = 1usize << extra;
        if distance >= usize::from(base) && distance < usize::from(base) + span {
            return (index as u16, (distance - usize::from(base)) as u16, extra);
        }
    }
    unreachable!("distance {distance} is outside the DEFLATE window")
}

/// Compress `input` into a zlib stream (RFC 1950 wrapping RFC 1951).
pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    // zlib header: deflate, 32 KiB window, no preset dictionary. 0x9C is the
    // check value that makes 0x78 0x9C a multiple of 31.
    out.push(0x78);
    out.push(0x9c);
    out.extend_from_slice(&deflate(input));
    out.extend_from_slice(&adler32(input).to_be_bytes());
    out
}

/// The raw DEFLATE block stream, without the zlib wrapper.
fn deflate(input: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::new();
    // One final block, fixed Huffman. The three header bits are `1` (final) and
    // `01` (fixed), which the writer places least-significant bit first.
    writer.write(1, 1);
    writer.write(1, 2);

    let mut chains: Vec<i32> = vec![-1; HASH_SIZE];
    let mut heads: Vec<i32> = vec![-1; HASH_SIZE];
    let mut position = 0usize;

    while position < input.len() {
        let mut best_length = 0usize;
        let mut best_distance = 0usize;

        if position + MIN_MATCH <= input.len() {
            let hash = hash_at(input, position);
            let mut candidate = heads[hash];
            let limit = position.saturating_sub(WINDOW);
            let mut steps = 0;
            while candidate >= 0 && (candidate as usize) >= limit && steps < 64 {
                let at = candidate as usize;
                let length = match_length(input, at, position);
                if length > best_length {
                    best_length = length;
                    best_distance = position - at;
                    if best_length >= MAX_MATCH {
                        break;
                    }
                }
                candidate = chains[at];
                steps += 1;
            }
            // Record this position so later ones can match against it.
            chains[position] = heads[hash];
            heads[hash] = position as i32;
        }

        if best_length >= MIN_MATCH {
            let (symbol, extra, extra_bits) = length_code(best_length);
            let (code, bits) = fixed_literal_code(symbol);
            writer.write_code(code, bits);
            if extra_bits > 0 {
                writer.write(u32::from(extra), u32::from(extra_bits));
            }
            let (distance, extra, extra_bits) = distance_code(best_distance);
            writer.write_code(u32::from(distance), 5);
            if extra_bits > 0 {
                writer.write(u32::from(extra), u32::from(extra_bits));
            }
            // Insert the positions the match skipped over.
            for offset in 1..best_length {
                let at = position + offset;
                if at + MIN_MATCH <= input.len() {
                    let hash = hash_at(input, at);
                    chains[at] = heads[hash];
                    heads[hash] = at as i32;
                }
            }
            position += best_length;
        } else {
            let (code, bits) = fixed_literal_code(u16::from(input[position]));
            writer.write_code(code, bits);
            position += 1;
        }
    }

    // End-of-block symbol 256, which is seven zero bits in the fixed code.
    let (code, bits) = fixed_literal_code(256);
    writer.write_code(code, bits);
    writer.finish()
}

/// A rolling hash of the three bytes at `position`.
fn hash_at(input: &[u8], position: usize) -> usize {
    let a = u32::from(input[position]);
    let b = u32::from(input[position + 1]);
    let c = u32::from(input[position + 2]);
    (((a << 10) ^ (b << 5) ^ c) as usize) & (HASH_SIZE - 1)
}

/// How many bytes match starting at `left` and `right`, up to the maximum.
fn match_length(input: &[u8], left: usize, right: usize) -> usize {
    let mut length = 0;
    while length < MAX_MATCH
        && right + length < input.len()
        && input[left + length] == input[right + length]
    {
        length += 1;
    }
    length
}

/// The Adler-32 checksum zlib requires.
fn adler32(input: &[u8]) -> u32 {
    const MODULUS: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in input {
        a = (a + u32::from(byte)) % MODULUS;
        b = (b + a) % MODULUS;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_bits_reverses_the_low_bits() {
        assert_eq!(reverse_bits(0b1100, 4), 0b0011);
        assert_eq!(reverse_bits(0b1, 3), 0b100);
        assert_eq!(reverse_bits(0, 5), 0);
    }

    #[test]
    fn length_codes_cover_the_table() {
        assert_eq!(length_code(3), (257, 0, 0));
        assert_eq!(length_code(10), (264, 0, 0));
        assert_eq!(length_code(11), (265, 0, 1));
        assert_eq!(length_code(12), (265, 1, 1));
        assert_eq!(length_code(258), (285, 0, 0));
    }

    #[test]
    fn distance_codes_cover_the_table() {
        assert_eq!(distance_code(1), (0, 0, 0));
        assert_eq!(distance_code(4), (3, 0, 0));
        assert_eq!(distance_code(5), (4, 0, 1));
        assert_eq!(distance_code(6), (4, 1, 1));
        assert_eq!(distance_code(32768), (29, 8191, 13));
    }

    #[test]
    fn adler32_matches_the_reference_value() {
        // zlib's own documentation uses this example.
        assert_eq!(adler32(b"Wikipedia"), 0x11E60398);
    }

    #[test]
    fn fixed_codes_are_the_ones_in_the_specification() {
        assert_eq!(fixed_literal_code(0), (0x30, 8));
        assert_eq!(fixed_literal_code(143), (0xbf, 8));
        assert_eq!(fixed_literal_code(144), (0x190, 9));
        assert_eq!(fixed_literal_code(255), (0x1ff, 9));
        assert_eq!(fixed_literal_code(256), (0, 7));
        assert_eq!(fixed_literal_code(279), (0x17, 7));
        assert_eq!(fixed_literal_code(280), (0xc0, 8));
        assert_eq!(fixed_literal_code(287), (0xc7, 8));
    }

    #[test]
    fn the_zlib_header_names_deflate_and_a_32_kib_window() {
        let out = compress(b"hello");
        assert_eq!(&out[..2], &[0x78, 0x9c]);
        // The two header bytes have to be a multiple of 31.
        assert_eq!((u32::from(out[0]) * 256 + u32::from(out[1])) % 31, 0);
    }

    #[test]
    fn empty_input_produces_a_valid_stream() {
        let out = compress(b"");
        assert_eq!(out.len(), 8); // header, one empty block, four checksum bytes
    }

    #[test]
    fn repeated_input_becomes_shorter_than_its_input() {
        // A match must actually be emitted, not one literal per byte.
        let input = b"hello hello hello hello hello hello hello hello";
        assert!(compress(input).len() < input.len());
    }
}
