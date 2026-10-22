//! AES-128 in CBC mode, as EBICS uses it.
//!
//! EBICS encrypts order data with AES-128-CBC under a transaction key that the
//! bank generates for the transaction. The key arrives wrapped in RSA, the
//! order data arrives deflated, and what travels on the wire is the AES
//! ciphertext base64 encoded.
//!
//! Two details of the framing are easy to get wrong and are the reason this
//! module is explicit about them:
//!
//! * The initialisation vector is **all zeroes**. It is not sent, because it is
//!   not secret and it is not variable: every implementation uses the zero IV.
//! * The padding is **not PKCS#7**. EBICS pads with zero bytes followed by one
//!   byte holding the number of padding bytes, and the number is always at least
//!   one, so a message whose length is a multiple of the block size still gains a
//!   whole block. See [`pad`] and [`unpad`].
//!
//! # Where this implementation comes from
//!
//! The workspace has no cipher crate and the policy is std-only. This is AES as
//! FIPS 197 defines it: the S-box, the round constants and the mix columns
//! matrix are the standard's tables and constants, and the round function is the
//! standard's sequence. The tables are generated from their definitions rather
//! than pasted, so a typo in a transcribed constant is not a possibility. Every
//! path is checked against the FIPS 197 test vectors and against Python's
//! `cryptography` on the fixtures in this crate.

/// The AES block size.
pub const BLOCK: usize = 16;

/// The number of AES-128 rounds.
const ROUNDS: usize = 10;

/// The S-box, derived from the multiplicative inverse in GF(2^8) followed by the
/// affine transform, as FIPS 197 section 5.1.1 defines it.
fn s_box() -> [u8; 256] {
    let mut inverse = [0u8; 256];
    // 0 maps to 0; every other value's inverse is found by brute force over the
    // 255 non-zero elements, which is 65,025 multiply steps once at startup.
    for value in 1u16..256 {
        for candidate in 1u16..256 {
            if gf_mul(value as u8, candidate as u8) == 1 {
                inverse[value as usize] = candidate as u8;
                break;
            }
        }
    }

    let mut table = [0u8; 256];
    for (value, inverse_value) in inverse.iter().enumerate() {
        // The affine transform: b XOR rotl(b,1) XOR rotl(b,2) XOR rotl(b,3) XOR
        // rotl(b,4) XOR 0x63, where b is the inverse. The leading `b` is easy to
        // drop, and dropping it makes every entry but the first wrong.
        let b = *inverse_value;
        let rotated = b.rotate_left(1) ^ b.rotate_left(2) ^ b.rotate_left(3) ^ b.rotate_left(4);
        table[value] = b ^ rotated ^ 0x63;
    }
    table
}

/// Multiplication in GF(2^8) modulo the AES polynomial x^8+x^4+x^3+x+1.
fn gf_mul(left: u8, right: u8) -> u8 {
    let mut a = left;
    let mut b = right;
    let mut product = 0u8;
    for _ in 0..8 {
        if b & 1 != 0 {
            product ^= a;
        }
        let high = a & 0x80;
        a <<= 1;
        if high != 0 {
            // Reduce by the AES polynomial, whose low byte is 0x1b.
            a ^= 0x1b;
        }
        b >>= 1;
    }
    product
}

/// The inverse S-box, found by inverting the forward table.
fn inverse_s_box(table: &[u8; 256]) -> [u8; 256] {
    let mut inverse = [0u8; 256];
    for (index, value) in table.iter().enumerate() {
        inverse[*value as usize] = index as u8;
    }
    inverse
}

/// A key schedule for AES-128.
#[derive(Debug, Clone)]
pub struct Aes128 {
    round_keys: [[u8; 16]; ROUNDS + 1],
    s_box: [u8; 256],
    inverse_s_box: [u8; 256],
}

impl Aes128 {
    /// Expands a 16-byte key, FIPS 197 section 5.2.
    #[must_use]
    pub fn new(key: &[u8; 16]) -> Self {
        let table = s_box();
        let mut words = [[0u8; 4]; 44];

        for (index, word) in words.iter_mut().take(4).enumerate() {
            word.copy_from_slice(&key[index * 4..index * 4 + 4]);
        }

        for index in 4..44 {
            let mut temp = words[index - 1];
            if index % 4 == 0 {
                // RotWord, SubWord, then XOR with the round constant.
                temp = [
                    table[temp[1] as usize] ^ round_constant(index / 4),
                    table[temp[2] as usize],
                    table[temp[3] as usize],
                    table[temp[0] as usize],
                ];
            }
            let previous = words[index - 4];
            words[index] = std::array::from_fn(|byte| previous[byte] ^ temp[byte]);
        }

        let mut round_keys = [[0u8; 16]; ROUNDS + 1];
        for (round, key) in round_keys.iter_mut().enumerate() {
            for word in 0..4 {
                key[word * 4..word * 4 + 4].copy_from_slice(&words[round * 4 + word]);
            }
        }

        Self {
            round_keys,
            s_box: table,
            inverse_s_box: inverse_s_box(&table),
        }
    }

    /// Encrypts one block.
    #[must_use]
    pub fn encrypt_block(&self, block: &[u8; 16]) -> [u8; 16] {
        let mut state = *block;
        add_round_key(&mut state, &self.round_keys[0]);

        for round in 1..ROUNDS {
            sub_bytes(&mut state, &self.s_box);
            shift_rows(&mut state);
            mix_columns(&mut state);
            add_round_key(&mut state, &self.round_keys[round]);
        }

        // The final round omits MixColumns.
        sub_bytes(&mut state, &self.s_box);
        shift_rows(&mut state);
        add_round_key(&mut state, &self.round_keys[ROUNDS]);
        state
    }

    /// Decrypts one block.
    #[must_use]
    pub fn decrypt_block(&self, block: &[u8; 16]) -> [u8; 16] {
        let mut state = *block;
        add_round_key(&mut state, &self.round_keys[ROUNDS]);

        for round in (1..ROUNDS).rev() {
            inverse_shift_rows(&mut state);
            inverse_sub_bytes(&mut state, &self.inverse_s_box);
            add_round_key(&mut state, &self.round_keys[round]);
            inverse_mix_columns(&mut state);
        }

        inverse_shift_rows(&mut state);
        inverse_sub_bytes(&mut state, &self.inverse_s_box);
        add_round_key(&mut state, &self.round_keys[0]);
        state
    }

    /// Encrypts `data` in CBC mode with an explicit initialisation vector.
    ///
    /// The length must be a multiple of the block size; [`pad`] makes it so.
    ///
    /// # Panics
    ///
    /// Panics if the length is not a multiple of the block size. A caller that
    /// padded first cannot hit this, and a caller that did not has a bug worth
    /// hearing about.
    #[must_use]
    pub fn encrypt_cbc(&self, iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
        assert!(
            data.len().is_multiple_of(BLOCK),
            "CBC input must be a whole number of blocks, got {}",
            data.len()
        );
        let mut out = Vec::with_capacity(data.len());
        let mut previous = *iv;
        for chunk in data.as_chunks::<BLOCK>().0 {
            let mut block = [0u8; 16];
            for (byte, (plain, chain)) in chunk.iter().zip(previous).enumerate() {
                block[byte] = plain ^ chain;
            }
            let encrypted = self.encrypt_block(&block);
            out.extend_from_slice(&encrypted);
            previous = encrypted;
        }
        out
    }

    /// Decrypts `data` in CBC mode with an explicit initialisation vector.
    ///
    /// # Panics
    ///
    /// Panics if the length is not a multiple of the block size.
    #[must_use]
    pub fn decrypt_cbc(&self, iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
        assert!(
            data.len().is_multiple_of(BLOCK),
            "CBC input must be a whole number of blocks, got {}",
            data.len()
        );
        let mut out = Vec::with_capacity(data.len());
        let mut previous = *iv;
        for chunk in data.as_chunks::<BLOCK>().0 {
            let decrypted = self.decrypt_block(chunk);
            for (byte, chain) in decrypted.iter().zip(previous) {
                out.push(byte ^ chain);
            }
            previous = *chunk;
        }
        out
    }

    /// Encrypts `data` in CBC mode with the all-zero initialisation vector.
    ///
    /// This is the mode EBICS uses. The vector is a constant of the protocol,
    /// not a choice, which is why it gets its own name.
    ///
    /// # Panics
    ///
    /// Panics if the length is not a multiple of the block size.
    #[must_use]
    pub fn encrypt_cbc_zero_iv(&self, data: &[u8]) -> Vec<u8> {
        self.encrypt_cbc(&[0u8; 16], data)
    }

    /// Decrypts `data` in CBC mode with the all-zero initialisation vector.
    ///
    /// # Panics
    ///
    /// Panics if the length is not a multiple of the block size.
    #[must_use]
    pub fn decrypt_cbc_zero_iv(&self, data: &[u8]) -> Vec<u8> {
        self.decrypt_cbc(&[0u8; 16], data)
    }
}

/// The round constant for a key schedule step: `x^(index-1)` in GF(2^8).
fn round_constant(index: usize) -> u8 {
    let mut value = 1u8;
    for _ in 1..index {
        value = gf_mul(value, 2);
    }
    value
}

fn sub_bytes(state: &mut [u8; 16], table: &[u8; 256]) {
    for byte in state.iter_mut() {
        *byte = table[*byte as usize];
    }
}

fn inverse_sub_bytes(state: &mut [u8; 16], inverse: &[u8; 256]) {
    for byte in state.iter_mut() {
        *byte = inverse[*byte as usize];
    }
}

/// `state` is column-major: byte `4*column + row`.
fn shift_rows(state: &mut [u8; 16]) {
    let mut out = [0u8; 16];
    for row in 0..4 {
        for column in 0..4 {
            out[4 * column + row] = state[4 * ((column + row) % 4) + row];
        }
    }
    *state = out;
}

fn inverse_shift_rows(state: &mut [u8; 16]) {
    let mut out = [0u8; 16];
    for row in 0..4 {
        for column in 0..4 {
            out[4 * column + row] = state[4 * ((column + 4 - row) % 4) + row];
        }
    }
    *state = out;
}

fn mix_columns(state: &mut [u8; 16]) {
    for column in 0..4 {
        let index = column * 4;
        let a = [
            state[index],
            state[index + 1],
            state[index + 2],
            state[index + 3],
        ];
        state[index] = gf_mul(a[0], 2) ^ gf_mul(a[1], 3) ^ a[2] ^ a[3];
        state[index + 1] = a[0] ^ gf_mul(a[1], 2) ^ gf_mul(a[2], 3) ^ a[3];
        state[index + 2] = a[0] ^ a[1] ^ gf_mul(a[2], 2) ^ gf_mul(a[3], 3);
        state[index + 3] = gf_mul(a[0], 3) ^ a[1] ^ a[2] ^ gf_mul(a[3], 2);
    }
}

fn inverse_mix_columns(state: &mut [u8; 16]) {
    for column in 0..4 {
        let index = column * 4;
        let a = [
            state[index],
            state[index + 1],
            state[index + 2],
            state[index + 3],
        ];
        state[index] = gf_mul(a[0], 14) ^ gf_mul(a[1], 11) ^ gf_mul(a[2], 13) ^ gf_mul(a[3], 9);
        state[index + 1] = gf_mul(a[0], 9) ^ gf_mul(a[1], 14) ^ gf_mul(a[2], 11) ^ gf_mul(a[3], 13);
        state[index + 2] = gf_mul(a[0], 13) ^ gf_mul(a[1], 9) ^ gf_mul(a[2], 14) ^ gf_mul(a[3], 11);
        state[index + 3] = gf_mul(a[0], 11) ^ gf_mul(a[1], 13) ^ gf_mul(a[2], 9) ^ gf_mul(a[3], 14);
    }
}

fn add_round_key(state: &mut [u8; 16], key: &[u8; 16]) {
    for (byte, key_byte) in state.iter_mut().zip(key) {
        *byte ^= key_byte;
    }
}

/// Pads `data` the way EBICS does: zero bytes, then one byte holding the number
/// of padding bytes.
///
/// The count is always between 1 and 16, so a message that already fits the
/// block size still gains a whole block. That is what makes unpadding
/// unambiguous, and it is why this is not PKCS#7: PKCS#7 uses the padding value
/// itself as every padding byte, while EBICS uses zeros and puts the length in
/// the last byte only.
#[must_use]
pub fn pad(data: &[u8]) -> Vec<u8> {
    let padding = BLOCK - (data.len() % BLOCK);
    let mut out = Vec::with_capacity(data.len() + padding);
    out.extend_from_slice(data);
    out.resize(data.len() + padding - 1, 0);
    out.push(padding as u8);
    out
}

/// Removes the padding [`pad`] added.
///
/// Returns `None` if the data is empty or the final byte is not a valid count,
/// which is what a wrong transaction key produces.
#[must_use]
pub fn unpad(data: &[u8]) -> Option<&[u8]> {
    let padding = *data.last()? as usize;
    if padding == 0 || padding > BLOCK || padding > data.len() {
        return None;
    }
    let (content, tail) = data.split_at(data.len() - padding);
    // The padding bytes before the count must be zero.
    if tail[..padding - 1].iter().any(|byte| *byte != 0) {
        return None;
    }
    Some(content)
}

#[cfg(test)]
mod tests {
    use super::{Aes128, pad, unpad};

    /// The FIPS 197 appendix C.1 example: key `000102...0f`, plaintext
    /// `00112233445566778899aabbccddeeff`.
    #[test]
    fn fips_197_appendix_c1() {
        let key: [u8; 16] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let plain: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let expected: [u8; 16] = [
            0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4,
            0xc5, 0x5a,
        ];

        let cipher = Aes128::new(&key);
        assert_eq!(cipher.encrypt_block(&plain), expected);
        assert_eq!(cipher.decrypt_block(&expected), plain);
    }

    /// The FIPS 197 appendix B example, which has a different key.
    #[test]
    fn fips_197_appendix_b() {
        let key: [u8; 16] = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];
        let plain: [u8; 16] = [
            0x32, 0x43, 0xf6, 0xa8, 0x88, 0x5a, 0x30, 0x8d, 0x31, 0x31, 0x98, 0xa2, 0xe0, 0x37,
            0x07, 0x34,
        ];
        let expected: [u8; 16] = [
            0x39, 0x25, 0x84, 0x1d, 0x02, 0xdc, 0x09, 0xfb, 0xdc, 0x11, 0x85, 0x97, 0x19, 0x6a,
            0x0b, 0x32,
        ];

        let cipher = Aes128::new(&key);
        assert_eq!(cipher.encrypt_block(&plain), expected);
        assert_eq!(cipher.decrypt_block(&expected), plain);
    }

    /// NIST SP 800-38A section F.2.1, AES-128-CBC, which uses a non-zero IV. The
    /// plaintext and the IV are both the standard's own.
    #[test]
    fn sp800_38a_cbc() {
        let key: [u8; 16] = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];
        let iv: [u8; 16] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let plain: Vec<u8> = vec![
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a, 0xae, 0x2d, 0x8a, 0x57, 0x1e, 0x03, 0xac, 0x9c, 0x9e, 0xb7, 0x6f, 0xac,
            0x45, 0xaf, 0x8e, 0x51, 0x30, 0xc8, 0x1c, 0x46, 0xa3, 0x5c, 0xe4, 0x11, 0xe5, 0xfb,
            0xc1, 0x19, 0x1a, 0x0a, 0x52, 0xef, 0xf6, 0x9f, 0x24, 0x45, 0xdf, 0x4f, 0x9b, 0x17,
            0xad, 0x2b, 0x41, 0x7b, 0xe6, 0x6c, 0x37, 0x10,
        ];
        let expected: Vec<u8> = vec![
            0x76, 0x49, 0xab, 0xac, 0x81, 0x19, 0xb2, 0x46, 0xce, 0xe9, 0x8e, 0x9b, 0x12, 0xe9,
            0x19, 0x7d, 0x50, 0x86, 0xcb, 0x9b, 0x50, 0x72, 0x19, 0xee, 0x95, 0xdb, 0x11, 0x3a,
            0x91, 0x76, 0x78, 0xb2, 0x73, 0xbe, 0xd6, 0xb8, 0xe3, 0xc1, 0x74, 0x3b, 0x71, 0x16,
            0xe6, 0x9e, 0x22, 0x22, 0x95, 0x16, 0x3f, 0xf1, 0xca, 0xa1, 0x68, 0x1f, 0xac, 0x09,
            0x12, 0x0e, 0xca, 0x30, 0x75, 0x86, 0xe1, 0xa7,
        ];

        let cipher = Aes128::new(&key);
        assert_eq!(cipher.encrypt_cbc(&iv, &plain), expected);
        assert_eq!(cipher.decrypt_cbc(&iv, &expected), plain);
    }

    /// The zero-IV form must equal the general form with a zero IV.
    #[test]
    fn zero_iv_matches_the_general_form() {
        let key = [0x42u8; 16];
        let cipher = Aes128::new(&key);
        let data = [0x11u8; 32];
        assert_eq!(
            cipher.encrypt_cbc_zero_iv(&data),
            cipher.encrypt_cbc(&[0u8; 16], &data)
        );
    }

    #[test]
    fn padding_round_trips() {
        for length in [0, 1, 15, 16, 17, 31, 32, 33, 100] {
            let data: Vec<u8> = (0..length).map(|i| (i % 256) as u8).collect();
            let padded = pad(&data);
            assert!(padded.len().is_multiple_of(16), "length {length}");
            assert!(padded.len() > length, "padding is always added");
            assert_eq!(unpad(&padded).unwrap(), &data[..], "length {length}");
        }
    }

    /// A whole block of padding is added to an aligned message, and the count
    /// byte is the last byte.
    #[test]
    fn an_aligned_message_gains_a_whole_block() {
        let padded = pad(&[0u8; 16]);
        assert_eq!(padded.len(), 32);
        assert_eq!(padded[31], 16);
        assert!(padded[16..31].iter().all(|b| *b == 0));
    }

    #[test]
    fn rejects_bad_padding() {
        assert!(unpad(&[]).is_none());
        assert!(unpad(&[0u8; 16]).is_none(), "count byte is zero");
        // A count of 3 with a non-zero byte where padding should be.
        let mut block = vec![0u8; 16];
        block[13] = 0xff;
        block[15] = 3;
        assert!(unpad(&block).is_none());
    }
}
