//! RSA signature verification.
//!
//! Verification only needs `signature^e mod n`, and `e` is 65537 in every
//! certificate a signature is made with, so the exponentiation is sixteen
//! squarings and one multiply. That is why this is a small module and not a
//! general bignum: no Montgomery ladder, no private-key operation, no
//! constant-time requirement, because the values are public.
//!
//! Two encodings are handled: PKCS#1 v1.5, which is what almost every PDF
//! signature uses, and PSS, which some timestamp authorities use. Signing is not
//! here and is not planned.

use crate::sha2::Algorithm;

/// What can go wrong while verifying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The signature is not the width of the modulus.
    WrongLength,
    /// The public exponent is zero, so no verification is possible.
    BadExponent,
    /// The recovered block does not have the structure the scheme requires.
    BadEncoding,
    /// The block is well-formed but does not carry the expected digest.
    Mismatch,
    /// The scheme or the hash is not one this crate verifies.
    Unsupported,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongLength => write!(f, "the signature is not the width of the modulus"),
            Self::BadExponent => write!(f, "the public exponent is not usable"),
            Self::BadEncoding => write!(f, "the recovered block is malformed"),
            Self::Mismatch => write!(f, "the signature does not match the message"),
            Self::Unsupported => write!(f, "the scheme is not supported"),
        }
    }
}

impl std::error::Error for Error {}

/// A result from this module.
pub type Result<T> = std::result::Result<T, Error>;

/// An RSA public key: a modulus and an exponent, both big-endian.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey {
    modulus: Vec<u8>,
    exponent: Vec<u8>,
}

impl PublicKey {
    /// Builds a key from a DER `RSAPublicKey`: a SEQUENCE of two INTEGERs.
    ///
    /// # Errors
    ///
    /// Whatever the DER reader reports, plus [`Error::BadExponent`] when the
    /// exponent is zero.
    pub fn from_der(bytes: &[u8]) -> std::result::Result<Self, crate::der::Error> {
        let sequence = crate::der::one(bytes)?;
        if sequence.tag != crate::der::SEQUENCE {
            return Err(crate::der::Error::UnexpectedTag {
                found: sequence.tag,
                expected: crate::der::SEQUENCE,
            });
        }
        let mut reader = sequence.reader();
        let modulus = reader.expect(crate::der::INTEGER)?;
        let exponent = reader.expect(crate::der::INTEGER)?;
        reader.finish()?;
        Ok(Self {
            modulus: modulus.unsigned_bytes().to_vec(),
            exponent: exponent.unsigned_bytes().to_vec(),
        })
    }

    /// Builds a key from raw big-endian values.
    #[must_use]
    pub fn new(modulus: Vec<u8>, exponent: Vec<u8>) -> Self {
        Self { modulus, exponent }
    }

    /// The modulus in bytes, which is also the signature width.
    #[must_use]
    pub fn width(&self) -> usize {
        self.modulus.len()
    }

    /// Recovers the encoded message block from a signature.
    fn recover(&self, signature: &[u8]) -> Result<Vec<u8>> {
        if signature.len() != self.width() {
            return Err(Error::WrongLength);
        }
        if self.exponent.iter().all(|&byte| byte == 0) {
            return Err(Error::BadExponent);
        }
        let base = BigUint::from_be_bytes(signature);
        let modulus = BigUint::from_be_bytes(&self.modulus);
        let value = modulus.pow(&base, &self.exponent);
        value.to_be_bytes(self.width()).ok_or(Error::WrongLength)
    }

    /// Verifies a PKCS#1 v1.5 signature over `message` with `algorithm`.
    ///
    /// # Errors
    ///
    /// [`Error::BadEncoding`] when the block is not `00 01 FF..FF 00 DigestInfo`,
    /// and [`Error::Mismatch`] when the digest inside it is not the digest of
    /// `message`.
    pub fn verify_pkcs1v15(
        &self,
        message: &[u8],
        signature: &[u8],
        algorithm: Algorithm,
    ) -> Result<()> {
        let block = self.recover(signature)?;
        // 00 01, at least eight 0xFF bytes, then 00, then the DigestInfo.
        if block.len() < 11 || block[0] != 0x00 || block[1] != 0x01 {
            return Err(Error::BadEncoding);
        }
        let separator = block[2..]
            .iter()
            .position(|&byte| byte != 0xFF)
            .map(|offset| offset + 2)
            .ok_or(Error::BadEncoding)?;
        if separator < 10 || block[separator] != 0x00 {
            return Err(Error::BadEncoding);
        }
        let digest_info = &block[separator + 1..];
        let prefix = digest_info_prefix(algorithm).ok_or(Error::Unsupported)?;
        if !digest_info.starts_with(prefix) {
            return Err(Error::BadEncoding);
        }
        let digest = &digest_info[prefix.len()..];
        if digest != algorithm.digest(message).as_slice() {
            return Err(Error::Mismatch);
        }
        Ok(())
    }

    /// Verifies an RSASSA-PSS signature over `message` with `algorithm`.
    ///
    /// The salt length is taken as the digest length, which is what RFC 3161
    /// timestamp authorities use.
    ///
    /// # Errors
    ///
    /// [`Error::BadEncoding`] when the padding or the trailer is wrong, and
    /// [`Error::Mismatch`] when the recovered hash does not match.
    pub fn verify_pss(&self, message: &[u8], signature: &[u8], algorithm: Algorithm) -> Result<()> {
        let block = self.recover(signature)?;
        let hash_length = algorithm.length();
        let salt_length = hash_length;
        // `emBits` is one less than the modulus bit count, and `emLen` is the
        // octet length that needs. For a 2048-bit modulus they are 2047 and 256.
        let em_bits = self.width() * 8 - 1;
        let em_length = em_bits.div_ceil(8);
        let unused = 8 * em_length - em_bits;
        if block.len() != em_length {
            return Err(Error::BadEncoding);
        }
        if block[em_length - 1] != 0xBC {
            return Err(Error::BadEncoding);
        }
        // EM = maskedDB || H || 0xbc.
        let db_length = em_length - hash_length - 1;
        let masked = &block[..db_length];
        let hash = &block[db_length..em_length - 1];
        if masked.is_empty() || unused >= 8 && masked[0] != 0 {
            return Err(Error::BadEncoding);
        }
        // The unused high bits of the first octet are padding and must be zero.
        if masked[0] >> (8 - unused) != 0 {
            return Err(Error::BadEncoding);
        }
        let mask = mgf1(hash, db_length, algorithm);
        let mut db: Vec<u8> = masked
            .iter()
            .zip(mask.iter())
            .map(|(left, right)| left ^ right)
            .collect();
        db[0] &= 0xFF >> unused;
        // DB = 0x00.. || 0x01 || salt.
        let separator = db_length - salt_length - 1;
        if db[..separator].iter().any(|&byte| byte != 0) {
            return Err(Error::BadEncoding);
        }
        if db[separator] != 0x01 {
            return Err(Error::BadEncoding);
        }
        let salt = &db[separator + 1..];
        let mut padding = vec![0u8; 8];
        padding.extend_from_slice(&algorithm.digest(message));
        padding.extend_from_slice(salt);
        if algorithm.digest(&padding) != hash {
            return Err(Error::Mismatch);
        }
        Ok(())
    }
}

/// The MGF1 mask of `length` bytes.
fn mgf1(seed: &[u8], length: usize, algorithm: Algorithm) -> Vec<u8> {
    let mut out = Vec::with_capacity(length);
    let mut counter: u32 = 0;
    while out.len() < length {
        let mut input = seed.to_vec();
        input.extend_from_slice(&counter.to_be_bytes());
        out.extend_from_slice(&algorithm.digest(&input));
        counter += 1;
    }
    out.truncate(length);
    out
}

/// The DER prefix of a `DigestInfo` for the given algorithm.
fn digest_info_prefix(algorithm: Algorithm) -> Option<&'static [u8]> {
    match algorithm {
        Algorithm::Sha1 => Some(&[
            0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2B, 0x0E, 0x03, 0x02, 0x1A, 0x05, 0x00, 0x04,
            0x14,
        ]),
        Algorithm::Sha256 => Some(&[
            0x30, 0x31, 0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x01, 0x05, 0x00, 0x04, 0x20,
        ]),
        Algorithm::Sha384 => Some(&[
            0x30, 0x41, 0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x02, 0x05, 0x00, 0x04, 0x30,
        ]),
        Algorithm::Sha512 => Some(&[
            0x30, 0x51, 0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x03, 0x05, 0x00, 0x04, 0x40,
        ]),
    }
}

/// An unsigned integer as little-endian 32-bit limbs, with no zero limbs above
/// the top one.
///
/// This exists so a signature can be verified without a general bignum crate.
/// Only the operations the verification needs are here: multiply, remainder by
/// long division, and square-and-multiply.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BigUint {
    limbs: Vec<u32>,
}

impl BigUint {
    fn from_be_bytes(bytes: &[u8]) -> Self {
        let mut limbs = Vec::with_capacity(bytes.len().div_ceil(4));
        let mut chunks = bytes.rchunks(4);
        for chunk in &mut chunks {
            let mut word = 0u32;
            for byte in chunk {
                word = (word << 8) | u32::from(*byte);
            }
            limbs.push(word);
        }
        Self::normalise(limbs)
    }

    fn to_be_bytes(&self, length: usize) -> Option<Vec<u8>> {
        let mut out = vec![0u8; length];
        for (index, limb) in self.limbs.iter().enumerate() {
            for (offset, byte) in limb.to_be_bytes().iter().enumerate() {
                let position = index * 4 + (3 - offset);
                if position >= length {
                    if *byte != 0 {
                        return None;
                    }
                    continue;
                }
                out[length - 1 - position] = *byte;
            }
        }
        Some(out)
    }

    fn normalise(mut limbs: Vec<u32>) -> Self {
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        Self { limbs }
    }

    fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    fn bit(&self, index: usize) -> bool {
        self.limbs
            .get(index / 32)
            .is_some_and(|word| word >> (index % 32) & 1 == 1)
    }

    fn bit_len(&self) -> usize {
        match self.limbs.last() {
            None => 0,
            Some(top) => (self.limbs.len() - 1) * 32 + (32 - top.leading_zeros() as usize),
        }
    }

    /// The product, with no reduction.
    fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self { limbs: Vec::new() };
        }
        let mut product = vec![0u32; self.limbs.len() + other.limbs.len()];
        for (left_index, &left) in self.limbs.iter().enumerate() {
            let mut carry = 0u64;
            for (right_index, &right) in other.limbs.iter().enumerate() {
                let slot = left_index + right_index;
                let total = u64::from(left) * u64::from(right) + u64::from(product[slot]) + carry;
                product[slot] = total as u32;
                carry = total >> 32;
            }
            let mut slot = left_index + other.limbs.len();
            while carry != 0 {
                let total = u64::from(product[slot]) + carry;
                product[slot] = total as u32;
                carry = total >> 32;
                slot += 1;
            }
        }
        Self::normalise(product)
    }

    /// The remainder of `self` divided by `modulus`, by shifting and subtracting.
    ///
    /// A shift-and-subtract division is slower than a limb-wise one, and it does
    /// not matter here: the exponent is 65537, so there are seventeen
    /// multiplications in total and each one is over a 2048-bit modulus.
    fn rem(&self, modulus: &Self) -> Self {
        if modulus.is_zero() {
            return Self { limbs: Vec::new() };
        }
        let mut remainder = Self { limbs: Vec::new() };
        for index in (0..self.bit_len()).rev() {
            remainder = remainder.shl1();
            if self.bit(index) {
                remainder.set_bit(0);
            }
            if remainder.ge(modulus) {
                remainder = remainder.sub(modulus);
            }
        }
        remainder
    }

    fn shl1(&self) -> Self {
        let mut limbs = vec![0u32; self.limbs.len() + 1];
        let mut carry = 0u32;
        for (index, &limb) in self.limbs.iter().enumerate() {
            limbs[index] = (limb << 1) | carry;
            carry = limb >> 31;
        }
        limbs[self.limbs.len()] = carry;
        Self::normalise(limbs)
    }

    fn set_bit(&mut self, index: usize) {
        let limb = index / 32;
        if self.limbs.len() <= limb {
            self.limbs.resize(limb + 1, 0);
        }
        self.limbs[limb] |= 1 << (index % 32);
    }

    /// Whether `self >= other`.
    fn ge(&self, other: &Self) -> bool {
        if self.limbs.len() != other.limbs.len() {
            return self.limbs.len() > other.limbs.len();
        }
        for (left, right) in self.limbs.iter().zip(other.limbs.iter()).rev() {
            if left != right {
                return left > right;
            }
        }
        true
    }

    /// `self - other`, which the caller must have checked is not negative.
    fn sub(&self, other: &Self) -> Self {
        let mut limbs = Vec::with_capacity(self.limbs.len());
        let mut borrow = 0i64;
        for index in 0..self.limbs.len() {
            let left = i64::from(self.limbs[index]);
            let right = i64::from(other.limbs.get(index).copied().unwrap_or(0));
            let mut value = left - right - borrow;
            if value < 0 {
                value += 1i64 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            limbs.push(value as u32);
        }
        Self::normalise(limbs)
    }

    /// `base^exponent mod self`, by square and multiply.
    fn pow(&self, base: &BigUint, exponent: &[u8]) -> BigUint {
        let mut result = BigUint { limbs: vec![1] };
        let reduced = base.rem(self);
        let bits = exponent.len() * 8;
        for index in (0..bits).rev() {
            result = result.mul(&result).rem(self);
            if exponent[exponent.len() - 1 - index / 8] >> (index % 8) & 1 == 1 {
                result = result.mul(&reduced).rem(self);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_uint_multiplies() {
        let left = BigUint::from_be_bytes(&[0x01, 0x00]);
        let right = BigUint::from_be_bytes(&[0x01, 0x00]);
        assert_eq!(
            left.mul(&right).to_be_bytes(3).unwrap(),
            vec![0x01, 0x00, 0x00]
        );
    }

    #[test]
    fn big_uint_takes_a_remainder() {
        // 10 mod 3 is 1.
        let value = BigUint::from_be_bytes(&[10]);
        let modulus = BigUint::from_be_bytes(&[3]);
        assert_eq!(value.rem(&modulus).to_be_bytes(1).unwrap(), vec![1]);
    }

    #[test]
    fn big_uint_raises_to_a_power() {
        // 3^5 mod 7 is 243 mod 7, which is 5.
        let modulus = BigUint::from_be_bytes(&[7]);
        let base = BigUint::from_be_bytes(&[3]);
        assert_eq!(modulus.pow(&base, &[5]).to_be_bytes(1).unwrap(), vec![5]);
    }

    /// The modulus, exponent, and two signatures for the message `abc`, made
    /// with Python's `cryptography` at 1024 bits so the vector fits in a file.
    /// Regenerate with `research/tools/gen_rsa_vectors.py`.
    fn vector() -> (PublicKey, Vec<u8>, Vec<u8>) {
        let modulus = hex(
            "d27331fe8af5ea3155064b18ee04d8000e0be08f53df315ddc841378cec992ed\
             c4d5c2e960fe60ddb5812b8d4485e7845634473c64202ce2286d569310ed938d\
             fd3c00a8b854b9f74e004a30885a9c69e18e8de4c59b00a6c14a92e4aa8083ae\
             5ecf2f5e2024ad172d378b55cd59d1e3c0853eb080046e89fff840a75894303b",
        );
        let pkcs1v15 = hex(
            "31f5dfa894f5e029c84c457b01abd4a48a094d8906fd8a7b79bc5afdfc6374b9\
             efbe6929c448e2a9c9a7e9a9e670155c1502e70c53d397c27fb1d07dffd41b20\
             ea32cfd5db82c155207f01e662d356d87adceff74f4f5e84d95b4e22ba538f76\
             d45f05fe40f57ca6f8bce3d3a8f42ab8d7b912803a4ffa5521f216e348ba8b6e",
        );
        let pss = hex(
            "397fcb8a5ce9bc88fffdacaf053827139ff0f28d9f1f09b47fe265d9ceb7cc89\
             fffd093d99edb7575495cea9abe7aeb4856316f3e240afcdbe1e9dac8ddf9be2\
             8e4d8a1a0aac3671e96d7cce6732a4eec17b9dec160e4a75719c4c534e23ee66\
             5dcc2782ff94fa5392288bb4ce1ae3cc753cbd7cfbbeb361b9c12d137f20b860",
        );
        (
            PublicKey::new(modulus, vec![0x01, 0x00, 0x01]),
            pkcs1v15,
            pss,
        )
    }

    #[test]
    fn verifies_a_pkcs1v15_signature() {
        let (key, signature, _) = vector();
        assert_eq!(key.width(), 128);
        key.verify_pkcs1v15(b"abc", &signature, Algorithm::Sha256)
            .unwrap();
    }

    #[test]
    fn verifies_a_pss_signature() {
        let (key, _, signature) = vector();
        key.verify_pss(b"abc", &signature, Algorithm::Sha256)
            .unwrap();
    }

    #[test]
    fn rejects_a_changed_message() {
        let (key, signature, _) = vector();
        assert_eq!(
            key.verify_pkcs1v15(b"abd", &signature, Algorithm::Sha256),
            Err(Error::Mismatch)
        );
    }

    #[test]
    fn rejects_a_changed_signature() {
        let (key, mut signature, _) = vector();
        signature[0] ^= 0x01;
        assert!(
            key.verify_pkcs1v15(b"abc", &signature, Algorithm::Sha256)
                .is_err()
        );
    }

    #[test]
    fn rejects_a_signature_of_the_wrong_width() {
        let (key, signature, _) = vector();
        assert_eq!(
            key.verify_pkcs1v15(b"abc", &signature[..64], Algorithm::Sha256),
            Err(Error::WrongLength)
        );
    }

    #[test]
    fn rejects_the_wrong_hash() {
        // The signature carries a SHA-256 DigestInfo, so verifying it as SHA-512
        // must not succeed.
        let (key, signature, _) = vector();
        assert_eq!(
            key.verify_pkcs1v15(b"abc", &signature, Algorithm::Sha512),
            Err(Error::BadEncoding)
        );
    }

    fn hex(text: &str) -> Vec<u8> {
        let clean: String = text.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        (0..clean.len() / 2)
            .map(|index| u8::from_str_radix(&clean[index * 2..index * 2 + 2], 16).unwrap())
            .collect()
    }
}
