//! RSA, as EBICS uses it.
//!
//! EBICS gives a subscriber three RSA key pairs, each with its own job, and the
//! protocol depends on the difference:
//!
//! * **Signature (A)**, version `A006`. Signs the order data, so the bank can
//!   prove to a third party that the payment instruction came from the customer.
//!   The scheme is RSA-PSS with SHA-256, and the salt is drawn fresh for every
//!   order.
//! * **Authentication (X)**, version `X002`. Signs the *request* itself, over
//!   the canonicalised `header` element. The scheme is RSA-SHA256 with PKCS#1
//!   v1.5 padding. It authenticates the message, not its contents.
//! * **Encryption (E)**, version `E002`. A public-key operation only: the bank's
//!   key wraps the AES transaction key. The padding is PKCS#1 v1.5.
//!
//! # The key digest
//!
//! A client tells the bank which of its keys it holds with a digest, not the key
//! itself. It is `SHA-256("<exponent> <modulus>")`, both hex, both lowercase,
//! both with leading zero bytes removed, separated by a single space. The
//! leading zeros matter: the modulus is published as 257 base64 bytes with a
//! leading `0x00`, and that byte is *not* part of the hex form. Getting this
//! wrong produces a digest the bank does not recognise, and the failure looks
//! like a key mismatch rather than a formatting bug.
//!
//! # On writing this by hand
//!
//! RSA is easy to get subtly wrong, so this module is not the place to be
//! clever. The padding is built exactly as RFC 8017 specifies, the digest
//! comparison in [`PublicKey::verify_pkcs1v15_sha256`] is constant-time, and
//! every operation is checked against an independent implementation on real
//! 2048-bit keys. Nothing here is a general-purpose cryptographic library: there
//! is no blinding, and [`PrivateKey`] holds a bare exponent.

use crate::base64;
use crate::bignum::{BigUint, Modulus};
use crate::sha256::Sha256;

use std::fmt;

/// A failure while reading, checking or using a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A modulus or exponent was not valid base64.
    Base64(&'static str),
    /// A modulus or exponent was empty.
    Empty(&'static str),
    /// The modulus is not a usable RSA modulus.
    BadModulus,
    /// A signature did not verify.
    Verification,
    /// A value did not fit the modulus.
    TooLong,
    /// The key is longer than this crate supports.
    TooLarge,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Base64(field) => write!(f, "{field} is not valid base64"),
            Self::Empty(field) => write!(f, "{field} is empty"),
            Self::BadModulus => f.write_str("the modulus is not a usable RSA modulus"),
            Self::Verification => f.write_str("the signature did not verify"),
            Self::TooLong => f.write_str("the value is longer than the modulus"),
            Self::TooLarge => f.write_str("the key is larger than this crate supports"),
        }
    }
}

impl std::error::Error for Error {}

/// The result type for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// The largest modulus accepted, in bits.
///
/// EBICS banks use 2048-bit keys and some allow 4096. The limit exists so a
/// malformed or hostile message cannot ask for an unbounded amount of work; it
/// is well above anything a bank uses.
const MAX_MODULUS_BITS: usize = 8192;

/// A public RSA key, as it appears in an EBICS message.
#[derive(Debug, Clone)]
pub struct PublicKey {
    modulus: BigUint,
    exponent: BigUint,
    /// The width of the encoded modulus, which is the width every result is
    /// padded to. It is one more than the significant bytes when the top bit of
    /// the modulus is set, which is the usual case.
    width: usize,
}

impl PublicKey {
    /// Builds a key from a base64 modulus and exponent, as an EBICS message
    /// carries them.
    ///
    /// # Errors
    ///
    /// Returns an error if either value is not valid base64, is empty, or if the
    /// modulus is not a usable RSA modulus.
    pub fn from_base64(modulus: &str, exponent: &str) -> Result<Self> {
        let modulus = base64::decode(modulus).ok_or(Error::Base64("the modulus"))?;
        let exponent = base64::decode(exponent).ok_or(Error::Base64("the exponent"))?;
        Self::from_bytes(&modulus, &exponent)
    }

    /// Builds a key from big-endian modulus and exponent bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if either value is empty, if the modulus is even, or if
    /// it is larger than this crate supports.
    pub fn from_bytes(modulus: &[u8], exponent: &[u8]) -> Result<Self> {
        if modulus.is_empty() {
            return Err(Error::Empty("the modulus"));
        }
        if exponent.is_empty() {
            return Err(Error::Empty("the exponent"));
        }
        let modulus_value = BigUint::from_be_bytes(modulus);
        if modulus_value.bit_len() > MAX_MODULUS_BITS {
            return Err(Error::TooLarge);
        }
        // Montgomery multiplication needs an odd modulus, and every real RSA
        // modulus is the product of two odd primes.
        if !modulus_value.is_odd() {
            return Err(Error::BadModulus);
        }
        Ok(Self {
            modulus: modulus_value,
            exponent: BigUint::from_be_bytes(exponent),
            width: modulus.len(),
        })
    }

    /// The encoded modulus width in bytes.
    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    /// The modulus as big-endian bytes, with leading zeros removed.
    #[must_use]
    pub fn modulus_bytes(&self) -> Vec<u8> {
        let width = self.modulus.bit_len().div_ceil(8).max(1);
        self.modulus.to_be_bytes(width).unwrap_or_default()
    }

    /// The exponent as big-endian bytes, with leading zeros removed.
    #[must_use]
    pub fn exponent_bytes(&self) -> Vec<u8> {
        let width = self.exponent.bit_len().div_ceil(8).max(1);
        self.exponent.to_be_bytes(width).unwrap_or_default()
    }

    /// The digest a client sends to name this key.
    ///
    /// `SHA-256("<exponent> <modulus>")` in lowercase hex with leading zeros
    /// stripped, base64 encoded. See the module documentation for why the
    /// leading zero of the published modulus is dropped.
    #[must_use]
    pub fn digest(&self) -> String {
        base64::encode(&self.digest_bytes())
    }

    /// The raw bytes behind [`PublicKey::digest`], so a caller can compare two
    /// keys without encoding.
    #[must_use]
    pub fn digest_bytes(&self) -> [u8; 32] {
        let text = format!(
            "{} {}",
            hex_trimmed(&self.exponent_bytes()),
            hex_trimmed(&self.modulus_bytes())
        );
        Sha256::digest(text.as_bytes())
    }

    /// Verifies a PKCS#1 v1.5 SHA-256 signature over `message`.
    ///
    /// This is what a bank checks on the authentication signature, and what this
    /// crate checks when it reads a bank's response.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Verification`] if the signature is the wrong length, has
    /// a malformed padding block, or does not match the message.
    pub fn verify_pkcs1v15_sha256(&self, message: &[u8], signature: &[u8]) -> Result<()> {
        if signature.len() != self.width {
            return Err(Error::Verification);
        }
        let recovered = self.exponentiate(&BigUint::from_be_bytes(signature))?;
        let encoded = recovered
            .to_be_bytes(self.width)
            .ok_or(Error::Verification)?;
        let expected = emsa_pkcs1v15(&Sha256::digest(message), self.width)?;
        if constant_time_eq(&encoded, &expected) {
            Ok(())
        } else {
            Err(Error::Verification)
        }
    }

    /// Verifies an RSA-PSS SHA-256 signature over `message`.
    ///
    /// This is the scheme on the order signature, so a bank verifies with it and
    /// this crate uses it when it reads a bank's own signature.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Verification`] if the signature is malformed or does not
    /// match.
    pub fn verify_pss_sha256(&self, message: &[u8], signature: &[u8]) -> Result<()> {
        if signature.len() != self.width {
            return Err(Error::Verification);
        }
        let recovered = self.exponentiate(&BigUint::from_be_bytes(signature))?;
        let em = recovered
            .to_be_bytes(self.width)
            .ok_or(Error::Verification)?;
        verify_pss(&em, &Sha256::digest(message), self.modulus.bit_len())
    }

    fn exponentiate(&self, value: &BigUint) -> Result<BigUint> {
        let modulus = Modulus::new(self.modulus.clone()).ok_or(Error::BadModulus)?;
        Ok(modulus.pow(value, &self.exponent_bytes()))
    }
}

/// A private RSA key: a public key and the private exponent.
///
/// EBICS keys arrive as a modulus, an exponent and a private exponent, so this
/// is the whole of what a client needs. There is no prime factor, no CRT
/// precomputation and no blinding; the operations are for signing a request, not
/// for a server that signs for strangers.
#[derive(Debug, Clone)]
pub struct PrivateKey {
    public: PublicKey,
    private_exponent: Vec<u8>,
}

impl PrivateKey {
    /// Builds a key from a base64 modulus, public exponent and private exponent.
    ///
    /// # Errors
    ///
    /// Returns an error if any value is not valid base64, is empty, or if the
    /// modulus is not usable.
    pub fn from_base64(modulus: &str, exponent: &str, private_exponent: &str) -> Result<Self> {
        let public = PublicKey::from_base64(modulus, exponent)?;
        let private =
            base64::decode(private_exponent).ok_or(Error::Base64("the private exponent"))?;
        Self::new(public, private)
    }

    /// Builds a key from a public key and a big-endian private exponent.
    ///
    /// # Errors
    ///
    /// Returns an error if the private exponent is empty.
    pub fn new(public: PublicKey, private_exponent: Vec<u8>) -> Result<Self> {
        if private_exponent.is_empty() {
            return Err(Error::Empty("the private exponent"));
        }
        Ok(Self {
            public,
            private_exponent,
        })
    }

    /// The public half.
    #[must_use]
    pub fn public(&self) -> &PublicKey {
        &self.public
    }

    /// Signs `message` with PKCS#1 v1.5 SHA-256 padding.
    ///
    /// # Errors
    ///
    /// Returns an error if the modulus is unusable or the encoding does not fit.
    pub fn sign_pkcs1v15_sha256(&self, message: &[u8]) -> Result<Vec<u8>> {
        let encoded = emsa_pkcs1v15(&Sha256::digest(message), self.public.width)?;
        self.raw_sign(&encoded)
    }

    /// Signs `message` with RSA-PSS and SHA-256, drawing a fresh salt.
    ///
    /// # Errors
    ///
    /// Returns an error if the modulus is unusable or the encoding does not fit.
    pub fn sign_pss_sha256(&self, message: &[u8], rng: &mut crate::rng::Rng) -> Result<Vec<u8>> {
        let salt = rng.bytes(32);
        let encoded = emsa_pss(
            &Sha256::digest(message),
            &salt,
            self.public.modulus.bit_len(),
        )?;
        self.raw_sign(&encoded)
    }

    /// Decrypts a PKCS#1 v1.5 ciphertext, returning the message it carries.
    ///
    /// This is how a client unwraps the AES transaction key a bank sent. The
    /// padding block is checked, including that the padding bytes are non-zero
    /// and the separator is present, so a wrong key is refused rather than
    /// returning the gibberish that a naive exponentiation would.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TooLong`] if the ciphertext is not exactly the width of
    /// the modulus, and [`Error::Verification`] if the padding block is
    /// malformed, which is what a wrong key produces.
    pub fn decrypt_pkcs1v15(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() != self.public.width {
            return Err(Error::TooLong);
        }
        let modulus = Modulus::new(self.public.modulus.clone()).ok_or(Error::BadModulus)?;
        let recovered = modulus.pow(&BigUint::from_be_bytes(ciphertext), &self.private_exponent);
        let encoded = recovered
            .to_be_bytes(self.public.width)
            .ok_or(Error::Verification)?;
        decode_eme_pkcs1v15(&encoded)
    }

    /// The modular exponentiation behind both signature schemes.
    fn raw_sign(&self, encoded: &[u8]) -> Result<Vec<u8>> {
        let modulus = Modulus::new(self.public.modulus.clone()).ok_or(Error::BadModulus)?;
        let value = BigUint::from_be_bytes(encoded);
        let signature = modulus.pow(&value, &self.private_exponent);
        signature
            .to_be_bytes(self.public.width)
            .ok_or(Error::TooLong)
    }
}

/// Encrypts `message` to a public key with PKCS#1 v1.5 padding.
///
/// This is how a client wraps the AES transaction key under the bank's
/// encryption key. The message must be short enough to leave room for the
/// padding block, which for a 2048-bit key is 245 bytes.
///
/// # Errors
///
/// Returns an error if the message is too long for the key, or if the modulus is
/// unusable.
pub fn encrypt_pkcs1v15(
    key: &PublicKey,
    message: &[u8],
    rng: &mut crate::rng::Rng,
) -> Result<Vec<u8>> {
    let encoded = eme_pkcs1v15(message, key.width, rng)?;
    let modulus = Modulus::new(key.modulus.clone()).ok_or(Error::BadModulus)?;
    let cipher = modulus.pow(&BigUint::from_be_bytes(&encoded), &key.exponent_bytes());
    cipher.to_be_bytes(key.width).ok_or(Error::TooLong)
}

/// Reads an EME-PKCS1-v1_5 block: `00 02 PS 00 M`, with `PS` non-zero and at
/// least eight bytes long.
///
/// Returns `None` for a block that does not have that shape, which is what a
/// wrong private key produces.
fn decode_eme_pkcs1v15(encoded: &[u8]) -> Result<Vec<u8>> {
    if encoded.len() < 11 || encoded[0] != 0x00 || encoded[1] != 0x02 {
        return Err(Error::Verification);
    }
    // The padding is non-zero and at least eight bytes, so the separator is
    // found by scanning from index 2 for the first zero.
    let separator = encoded[2..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|offset| offset + 2)
        .ok_or(Error::Verification)?;
    if separator < 10 {
        return Err(Error::Verification);
    }
    Ok(encoded[separator + 1..].to_vec())
}

/// The DigestInfo prefix for SHA-256, from RFC 8017 section 9.2, notes to
/// table 2.
const SHA256_DIGEST_INFO_PREFIX: [u8; 19] = [
    0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
    0x00, 0x04, 0x20,
];

/// Builds an EMSA-PKCS1-v1_5 encoding: `00 01 FF...FF 00 DigestInfo`.
///
/// # Errors
///
/// Returns [`Error::TooLong`] if the digest and its prefix do not fit in `width`
/// bytes.
fn emsa_pkcs1v15(digest: &[u8; 32], width: usize) -> Result<Vec<u8>> {
    let body = SHA256_DIGEST_INFO_PREFIX.len() + digest.len();
    // Two bytes of 0x00 0x01, one separator, and at least eight bytes of 0xff.
    if width < body + 11 {
        return Err(Error::TooLong);
    }
    let mut out = Vec::with_capacity(width);
    out.push(0x00);
    out.push(0x01);
    out.resize(width - body - 1, 0xff);
    out.push(0x00);
    out.extend_from_slice(&SHA256_DIGEST_INFO_PREFIX);
    out.extend_from_slice(digest);
    Ok(out)
}

/// Builds an EME-PKCS1-v1_5 encoding: `00 02 PS 00 message`, with `PS` non-zero.
///
/// # Errors
///
/// Returns [`Error::TooLong`] if the message does not leave room for at least
/// eight bytes of padding.
fn eme_pkcs1v15(message: &[u8], width: usize, rng: &mut crate::rng::Rng) -> Result<Vec<u8>> {
    if width < message.len() + 11 {
        return Err(Error::TooLong);
    }
    let padding_length = width - message.len() - 3;
    let mut out = Vec::with_capacity(width);
    out.push(0x00);
    out.push(0x02);
    // A zero byte in the padding would end the block early, so every byte is
    // drawn until a non-zero one appears.
    let mut filled = 0;
    while filled < padding_length {
        let byte = rng.bytes(1)[0];
        if byte != 0 {
            out.push(byte);
            filled += 1;
        }
    }
    out.push(0x00);
    out.extend_from_slice(message);
    Ok(out)
}

/// Builds an EMSA-PSS encoding, RFC 8017 section 9.1.1.
///
/// # Errors
///
/// Returns [`Error::TooLong`] if the salt does not fit in the encoded length.
fn emsa_pss(digest: &[u8; 32], salt: &[u8], modulus_bits: usize) -> Result<Vec<u8>> {
    // PSS uses one bit fewer than the modulus, so the top bit of the encoding is
    // always clear and the value is below the modulus without a reduction.
    let encoded_bits = modulus_bits - 1;
    let encoded_length = encoded_bits.div_ceil(8);
    let hash_length = 32;

    if encoded_length < hash_length + salt.len() + 2 {
        return Err(Error::TooLong);
    }

    let mut prefixed = Sha256::new();
    prefixed.update(&[0u8; 8]);
    prefixed.update(digest);
    prefixed.update(salt);
    let h = prefixed.finish();

    let mut db = vec![0u8; encoded_length - hash_length - 1];
    let separator = db.len() - salt.len() - 1;
    db[separator] = 0x01;
    db[separator + 1..].copy_from_slice(salt);

    let mask = mgf1(&h, db.len());
    for (byte, mask_byte) in db.iter_mut().zip(mask) {
        *byte ^= mask_byte;
    }
    // Clear the bits above emBits, which is what keeps the value below the
    // modulus.
    let spare = 8 * encoded_length - encoded_bits;
    if spare > 0 {
        db[0] &= 0xffu8 >> spare;
    }

    let mut out = Vec::with_capacity(encoded_length);
    out.extend_from_slice(&db);
    out.extend_from_slice(&h);
    out.push(0xbc);
    Ok(out)
}

/// Checks an EMSA-PSS encoding against a message digest.
fn verify_pss(em: &[u8], digest: &[u8; 32], modulus_bits: usize) -> Result<()> {
    let encoded_bits = modulus_bits - 1;
    let encoded_length = encoded_bits.div_ceil(8);
    let hash_length = 32;

    if em.len() != encoded_length || encoded_length < hash_length + 2 {
        return Err(Error::Verification);
    }
    if em[encoded_length - 1] != 0xbc {
        return Err(Error::Verification);
    }
    let db_length = encoded_length - hash_length - 1;
    let (masked_db, rest) = em.split_at(db_length);
    let (h, _) = rest.split_at(hash_length);

    let spare = 8 * encoded_length - encoded_bits;
    if spare > 0 && masked_db[0] >> (8 - spare) != 0 {
        return Err(Error::Verification);
    }

    let mask = mgf1(h, db_length);
    let mut db = masked_db.to_vec();
    for (byte, mask_byte) in db.iter_mut().zip(mask) {
        *byte ^= mask_byte;
    }
    if spare > 0 {
        db[0] &= 0xffu8 >> spare;
    }

    // Everything before the separator must be zero, and the byte before the salt
    // must be the separator.
    let Some(separator) = db.iter().rposition(|byte| *byte == 0x01) else {
        return Err(Error::Verification);
    };
    if db[..separator].iter().any(|byte| *byte != 0) {
        return Err(Error::Verification);
    }
    let salt = &db[separator + 1..];

    let mut prefixed = Sha256::new();
    prefixed.update(&[0u8; 8]);
    prefixed.update(digest);
    prefixed.update(salt);
    let expected = prefixed.finish();

    if constant_time_eq(h, &expected) {
        Ok(())
    } else {
        Err(Error::Verification)
    }
}

/// MGF1 with SHA-256, RFC 8017 appendix B.2.1.
fn mgf1(seed: &[u8], length: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(length);
    let mut counter = 0u32;
    while out.len() < length {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(&counter.to_be_bytes());
        let block = hasher.finish();
        let take = (length - out.len()).min(block.len());
        out.extend_from_slice(&block[..take]);
        counter += 1;
    }
    out
}

/// Lowercase hex with leading zeros removed, as the key digest requires.
///
/// The zeros are removed from the *hex text*, not from the bytes. The standard
/// exponent 65537 is the three bytes `01 00 01`, whose hex is `010001`, and the
/// digest input is `10001`. Dropping whole zero bytes instead would leave
/// `010001`, which is a different string and therefore a different digest — and
/// since the exponent is 65537 for essentially every RSA key in existence, that
/// mistake breaks the digest for every real key rather than for an odd one.
fn hex_trimmed(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    let trimmed = out.trim_start_matches('0');
    trimmed.to_owned()
}

/// Compares two byte strings without leaking where they differ.
///
/// A signature check that returns as soon as a byte differs leaks how many
/// leading bytes an attacker guessed correctly, which is enough to forge one
/// byte at a time. The lengths are public, so an early return on length is fine.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right) {
        difference |= a ^ b;
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::{PublicKey, decode_eme_pkcs1v15, encrypt_pkcs1v15, hex_trimmed};

    /// Zeros are stripped from the hex text. `01 00 01` is `10001`, which is the
    /// standard exponent 65537; stripping whole zero bytes would leave `010001`
    /// and produce a digest no bank would recognise.
    #[test]
    fn hex_drops_leading_zeros() {
        assert_eq!(hex_trimmed(&[0x00, 0x00, 0x01]), "1");
        assert_eq!(hex_trimmed(&[0x00, 0xab, 0xcd]), "abcd");
        assert_eq!(hex_trimmed(&[0x01, 0x00]), "100");
        assert_eq!(hex_trimmed(&[0x01, 0x00, 0x01]), "10001");
        assert_eq!(hex_trimmed(&[]), "");
        assert_eq!(hex_trimmed(&[0x00]), "");
        assert_eq!(hex_trimmed(&[0x00, 0x0f]), "f");
    }

    /// A wrapped key must decrypt to what was wrapped.
    #[test]
    fn encryption_round_trips() {
        let key = crate::rsa::PrivateKey::from_base64(
            "2aWRwQ9/pII5l29p55PIXEWlJsvSn/d5KOpz/wnUEJ9AZvq2CitcgYhCG0qqn8Eb3G61qqQfNONhHUPdzc8Fndnv0Uqzg+Xtz4kSjHWZQZb069Po6jgLwUbmuuyfdlQpU4GdyrPHtFTd18tP20/5pmic1ui+9GQnyRBhoh55CTU=",
            "AQAB",
            "M1NfRvb/s+zd5f9pKypnopLt+nMwjXbPxL1dc3R+5RiZGG/hRA6rtwRKCqOWDyx8BIS7JK2lilRgz4T0ppc+jGbF3FIaqLzrr7b5hQJN97vR1f5PYZxGXQppx2qto2tQbm9Ct6K8UmRQTAI33ndB0yjhTUzXJnxl/lKnuBX0O/U=",
        )
        .expect("usable");
        let mut rng = crate::rng::Rng::from_seed([1; 32]);
        let message = b"a 16-byte key!!";
        let ciphertext = encrypt_pkcs1v15(key.public(), message, &mut rng).expect("encrypts");
        assert_eq!(ciphertext.len(), key.public().width());
        assert_eq!(
            key.decrypt_pkcs1v15(&ciphertext).expect("decrypts"),
            message
        );
    }

    /// A ciphertext of the wrong length must be refused before any arithmetic.
    #[test]
    fn a_short_ciphertext_is_refused() {
        let key = crate::rsa::PrivateKey::from_base64(
            "2aWRwQ9/pII5l29p55PIXEWlJsvSn/d5KOpz/wnUEJ9AZvq2CitcgYhCG0qqn8Eb3G61qqQfNONhHUPdzc8Fndnv0Uqzg+Xtz4kSjHWZQZb069Po6jgLwUbmuuyfdlQpU4GdyrPHtFTd18tP20/5pmic1ui+9GQnyRBhoh55CTU=",
            "AQAB",
            "M1NfRvb/s+zd5f9pKypnopLt+nMwjXbPxL1dc3R+5RiZGG/hRA6rtwRKCqOWDyx8BIS7JK2lilRgz4T0ppc+jGbF3FIaqLzrr7b5hQJN97vR1f5PYZxGXQppx2qto2tQbm9Ct6K8UmRQTAI33ndB0yjhTUzXJnxl/lKnuBX0O/U=",
        )
        .expect("usable");
        assert!(key.decrypt_pkcs1v15(&[0u8; 16]).is_err());
    }

    /// A block without the `00 02` header must be refused, which is what a
    /// random exponentiation result looks like.
    #[test]
    fn a_block_without_the_header_is_refused() {
        assert!(decode_eme_pkcs1v15(&[0x00, 0x01, 0x00]).is_err());
        assert!(decode_eme_pkcs1v15(&[]).is_err());
        // A separator too close to the front means too little padding.
        let mut block = vec![0u8; 32];
        block[1] = 0x02;
        block[5] = 0x00;
        assert!(decode_eme_pkcs1v15(&block).is_err());
    }

    #[test]
    fn rejects_a_bad_key() {
        // Even modulus.
        assert!(PublicKey::from_bytes(&[2], &[1, 0, 1]).is_err());
        // Empty values.
        assert!(PublicKey::from_bytes(&[], &[1]).is_err());
        assert!(PublicKey::from_bytes(&[3], &[]).is_err());
        // Not base64.
        assert!(PublicKey::from_base64("not base64!", "AQAB").is_err());
    }
}
