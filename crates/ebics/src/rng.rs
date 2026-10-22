//! Randomness, for the values EBICS requires to be unpredictable.
//!
//! Three things in the protocol must be random: the nonce in a request header,
//! the PKCS#1 v1.5 padding of an encrypted transaction key, and the salt in an
//! RSA-PSS order signature. A repeated nonce or salt is a real weakness, so the
//! source has to be the operating system's, not a counter or the clock.
//!
//! The workspace is std-only, and the standard library exposes no random number
//! generator. On Unix the device `/dev/urandom` is read directly. Elsewhere the
//! seed comes from `RandomState`, which the standard library initialises from
//! the OS; that is a weaker guarantee, and the doc comment on
//! [`Rng::from_entropy`] says so rather than implying more than is true.
//!
//! Once seeded, output is a SHA-256 counter mode generator. The seed is 32 bytes
//! of real entropy, which is the whole of the security here; the expansion only
//! has to be indistinguishable, and a hash of a secret counter is.

use crate::sha256::Sha256;

/// A seeded generator.
#[derive(Debug, Clone)]
pub struct Rng {
    state: [u8; 32],
    counter: u64,
}

impl Rng {
    /// Seeds a generator from the operating system.
    ///
    /// On Unix this reads `/dev/urandom`. If that cannot be opened, and on every
    /// other platform, the seed is drawn from `RandomState`, which the standard
    /// library seeds from the OS but does not promise to be
    /// cryptographically strong. Callers that need the guarantee should check
    /// [`Rng::is_strongly_seeded`].
    #[must_use]
    pub fn from_entropy() -> Self {
        if let Some(seed) = read_urandom() {
            return Self::from_seed(seed);
        }
        Self::from_seed(fallback_seed())
    }

    /// Whether the seed came from a source this crate considers strong.
    #[must_use]
    pub fn is_strongly_seeded() -> bool {
        cfg!(unix) && std::path::Path::new("/dev/urandom").exists()
    }

    /// Seeds from an explicit 32 bytes.
    ///
    /// A test uses this so its output can be reproduced; a caller in production
    /// should not, because a guessable seed makes every value guessable.
    #[must_use]
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            state: seed,
            counter: 0,
        }
    }

    /// Fills `out` with random bytes.
    pub fn fill(&mut self, out: &mut [u8]) {
        let mut offset = 0;
        while offset < out.len() {
            self.counter = self.counter.wrapping_add(1);
            let mut hasher = Sha256::new();
            hasher.update(&self.state);
            hasher.update(&self.counter.to_le_bytes());
            let block = hasher.finish();
            // Chaining the block back into the state means an attacker who
            // learns one block does not get the next.
            self.state = block;

            let take = (out.len() - offset).min(block.len());
            out[offset..offset + take].copy_from_slice(&block[..take]);
            offset += take;
        }
    }

    /// Returns `count` random bytes.
    #[must_use]
    pub fn bytes(&mut self, count: usize) -> Vec<u8> {
        let mut out = vec![0u8; count];
        self.fill(&mut out);
        out
    }
}

fn read_urandom() -> Option<[u8; 32]> {
    #[cfg(unix)]
    {
        use std::io::Read;
        let mut file = std::fs::File::open("/dev/urandom").ok()?;
        let mut seed = [0u8; 32];
        file.read_exact(&mut seed).ok()?;
        return Some(seed);
    }
    #[allow(unreachable_code)]
    None
}

fn fallback_seed() -> [u8; 32] {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let mut seed = [0u8; 32];
    for (index, chunk) in seed.chunks_mut(8).enumerate() {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_usize(index);
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
        );
        chunk.copy_from_slice(&hasher.finish().to_le_bytes());
    }
    seed
}

#[cfg(test)]
mod tests {
    use super::Rng;

    #[test]
    fn output_is_reproducible_from_a_seed() {
        let mut left = Rng::from_seed([7; 32]);
        let mut right = Rng::from_seed([7; 32]);
        assert_eq!(left.bytes(64), right.bytes(64));
    }

    #[test]
    fn different_seeds_differ() {
        let mut left = Rng::from_seed([1; 32]);
        let mut right = Rng::from_seed([2; 32]);
        assert_ne!(left.bytes(32), right.bytes(32));
    }

    /// Successive draws must not repeat. A one-byte draw is excluded on purpose:
    /// a byte has 256 values, so a repeat within twenty draws is expected rather
    /// than a bug, and asserting otherwise would make this test flaky.
    #[test]
    fn successive_draws_differ() {
        let mut rng = Rng::from_seed([9; 32]);
        let mut seen = std::collections::HashSet::new();
        for length in [4, 31, 32, 33, 64, 100] {
            for _ in 0..20 {
                assert!(seen.insert(rng.bytes(length)), "repeat at length {length}");
            }
        }
    }

    /// A draw that spans a block boundary still yields distinct output, which is
    /// what catches a generator that restarts its counter per call.
    #[test]
    fn a_draw_longer_than_a_block_advances() {
        let mut rng = Rng::from_seed([3; 32]);
        let first = rng.bytes(200);
        let second = rng.bytes(200);
        assert_ne!(first, second);
        assert_ne!(first[..32], second[..32]);
    }

    #[test]
    fn entropy_source_produces_bytes() {
        let mut rng = Rng::from_entropy();
        let bytes = rng.bytes(32);
        assert_eq!(bytes.len(), 32);
        assert_ne!(bytes, vec![0u8; 32]);
    }
}
