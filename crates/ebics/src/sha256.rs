//! SHA-256.
//!
//! EBICS uses SHA-256 in three places, and all three are load-bearing:
//!
//! * The digest of a public key in the static header, `BankPubKeyDigests` and
//!   `EncryptionPubKeyDigest`. That digest is how a client tells the bank which
//!   of its keys it already has, so a wrong one is rejected.
//! * The digest of the authenticated part of a request, `ds:DigestValue`.
//! * The digest signed by the authentication key, RSA-SHA256, and by the
//!   signature key, RSA-PSS with SHA-256 as both the hash and the MGF1 hash.
//!
//! The workspace has no digest crate and the policy is std-only, so the
//! compression function lives here. It follows FIPS 180-4 directly and is
//! checked against the standard's own test vectors, and against every digest
//! Python's `hashlib` produces for the fixtures in this crate.

/// The round constants: the first 32 bits of the fractional parts of the cube
/// roots of the first sixty-four primes.
const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// The initial state: the first 32 bits of the fractional parts of the square
/// roots of the first eight primes.
const H0: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// A SHA-256 digest, updated incrementally.
///
/// EBICS hashes pieces that are assembled as text is built, so a streaming
/// interface is the natural one. [`Sha256::digest`] is the one-shot form for the
/// common case.
#[derive(Debug, Clone)]
pub struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffered: usize,
    length: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// A fresh hasher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: H0,
            buffer: [0; 64],
            buffered: 0,
            length: 0,
        }
    }

    /// The digest of `data`, in one call.
    #[must_use]
    pub fn digest(data: &[u8]) -> [u8; 32] {
        let mut hasher = Self::new();
        hasher.update(data);
        hasher.finish()
    }

    /// Adds `data` to the message.
    pub fn update(&mut self, data: &[u8]) {
        self.length = self.length.wrapping_add(data.len() as u64);
        let mut rest = data;

        if self.buffered > 0 {
            let want = 64 - self.buffered;
            let take = want.min(rest.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&rest[..take]);
            self.buffered += take;
            rest = &rest[take..];
            if self.buffered < 64 {
                // The buffer did not fill, so it still holds this message's
                // tail. Overwriting it with the empty remainder below would
                // silently drop those bytes and reset the count to zero.
                return;
            }
            let block = self.buffer;
            self.compress(&block);
            self.buffered = 0;
        }

        let (blocks, remainder) = rest.as_chunks::<64>();
        for block in blocks {
            self.compress(block);
        }

        self.buffer[..remainder.len()].copy_from_slice(remainder);
        self.buffered = remainder.len();
    }

    /// Finishes the message and returns the digest.
    ///
    /// The padding is the message, a `1` bit, zero bits up to a length that is
    /// 56 modulo 64, and the message length in bits as a big-endian `u64`.
    #[must_use]
    pub fn finish(mut self) -> [u8; 32] {
        let bit_length = self.length.wrapping_mul(8);
        self.update(&[0x80]);

        while self.buffered != 56 {
            self.update(&[0]);
        }

        // `update` counts these bytes into `length`, but the length field was
        // already captured above, so the trailing write is what matters.
        self.buffer[56..64].copy_from_slice(&bit_length.to_be_bytes());
        let block = self.buffer;
        self.compress(&block);

        let mut out = [0u8; 32];
        for (chunk, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (index, chunk) in block.as_chunks::<4>().0.iter().enumerate() {
            w[index] = u32::from_be_bytes(*chunk);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Sha256;

    /// The one-shot and streaming forms must agree, and a message longer than
    /// one block exercises the buffer boundary.
    #[test]
    fn streaming_matches_one_shot() {
        let message: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        for cut in [0, 1, 63, 64, 65, 127, 128, 999] {
            let mut hasher = Sha256::new();
            hasher.update(&message[..cut]);
            hasher.update(&message[cut..]);
            assert_eq!(hasher.finish(), Sha256::digest(&message), "cut at {cut}");
        }
    }
}
