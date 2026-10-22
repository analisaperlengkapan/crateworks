//! Arbitrary-precision arithmetic, enough for RSA.
//!
//! EBICS puts the client's three RSA keys on the wire and signs with two of
//! them, so a Rust implementation of the protocol has to do modular
//! exponentiation on 2048-bit numbers. The workspace has no bignum crate and the
//! policy is std-only, so this module carries the little that RSA needs: reading
//! a big-endian integer, multiplying, and raising to a power modulo a fixed
//! modulus.
//!
//! # Why Montgomery
//!
//! The obvious way to reduce a product modulo `n` is long division, and writing
//! Knuth's algorithm D correctly is a page of fiddly code. Montgomery
//! multiplication gets the same result with shifts and adds only, at the cost of
//! carrying values in a transformed domain. Since every multiplication here is
//! modulo the *same* modulus, the transform is computed once per exponentiation
//! and the hot loop needs no division at all.
//!
//! Numbers are little-endian arrays of 32-bit limbs, which is the shape the
//! multiplications want.

/// A big-endian unsigned integer, little-endian limbs, no trailing zero limbs.
///
/// An empty limb vector is zero, which keeps every operation total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigUint {
    limbs: Vec<u32>,
}

impl BigUint {
    /// Reads a big-endian byte string, as it appears base64-encoded in a
    /// message.
    #[must_use]
    pub fn from_be_bytes(bytes: &[u8]) -> Self {
        let mut limbs = Vec::with_capacity(bytes.len().div_ceil(4));
        // The final chunk is the most significant, so it is pushed last.
        let mut chunks = bytes.rchunks(4);
        for chunk in &mut chunks {
            let mut word = 0u32;
            for byte in chunk {
                word = (word << 8) | u32::from(*byte);
            }
            limbs.push(word);
        }
        Self::from_limbs(limbs)
    }

    /// Writes the value as `length` big-endian bytes.
    ///
    /// RSA needs a fixed-width output: the result of a modular exponentiation is
    /// padded to the modulus length with leading zeros, and a signature that is
    /// one byte short is rejected. Returns `None` if the value does not fit.
    #[must_use]
    pub fn to_be_bytes(&self, length: usize) -> Option<Vec<u8>> {
        let mut out = vec![0u8; length];
        for (index, limb) in self.limbs.iter().enumerate() {
            let bytes = limb.to_be_bytes();
            for (offset, byte) in bytes.iter().enumerate() {
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

    /// Whether the value is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    /// Whether the value is odd, which every RSA modulus is.
    #[must_use]
    pub fn is_odd(&self) -> bool {
        self.limbs.first().is_some_and(|limb| limb & 1 == 1)
    }

    /// The number of significant bits.
    #[must_use]
    pub fn bit_len(&self) -> usize {
        match self.limbs.last() {
            None => 0,
            Some(top) => (self.limbs.len() - 1) * 32 + (32 - top.leading_zeros() as usize),
        }
    }

    /// The bit at `index`, counting from the least significant.
    #[must_use]
    pub fn bit(&self, index: usize) -> bool {
        let limb = index / 32;
        self.limbs
            .get(limb)
            .is_some_and(|word| word >> (index % 32) & 1 == 1)
    }

    /// The value one.
    #[must_use]
    pub fn one() -> Self {
        Self { limbs: vec![1] }
    }

    fn from_limbs(mut limbs: Vec<u32>) -> Self {
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        Self { limbs }
    }

    /// `self` shifted left one bit, without reduction. Used only while building
    /// the Montgomery constants, where the value is kept below the modulus.
    fn shl1(&mut self) {
        let mut carry = 0u32;
        for limb in &mut self.limbs {
            let next = *limb >> 31;
            *limb = (*limb << 1) | carry;
            carry = next;
        }
        if carry != 0 {
            self.limbs.push(carry);
        }
    }

    /// Compares against `other`, treating both as the same width.
    fn cmp_padded(&self, other: &Self) -> std::cmp::Ordering {
        let width = self.limbs.len().max(other.limbs.len());
        for index in (0..width).rev() {
            let left = self.limbs.get(index).copied().unwrap_or(0);
            let right = other.limbs.get(index).copied().unwrap_or(0);
            match left.cmp(&right) {
                std::cmp::Ordering::Equal => {}
                other => return other,
            }
        }
        std::cmp::Ordering::Equal
    }

    /// `self - other`, which must not go negative.
    fn sub(&self, other: &Self) -> Self {
        let mut out = Vec::with_capacity(self.limbs.len());
        let mut borrow = 0i64;
        for index in 0..self.limbs.len() {
            let left = i64::from(self.limbs[index]);
            let right = i64::from(other.limbs.get(index).copied().unwrap_or(0));
            let mut value = left - right - borrow;
            if value < 0 {
                value += 1 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            out.push(value as u32);
        }
        Self::from_limbs(out)
    }

    /// The schoolbook product, wide enough that nothing is lost.
    fn mul_wide(&self, other: &Self) -> Vec<u32> {
        let mut out = vec![0u32; self.limbs.len() + other.limbs.len() + 1];
        for (i, left) in self.limbs.iter().enumerate() {
            let mut carry = 0u64;
            for (j, right) in other.limbs.iter().enumerate() {
                let sum = u64::from(out[i + j]) + u64::from(*left) * u64::from(*right) + carry;
                out[i + j] = sum as u32;
                carry = sum >> 32;
            }
            let mut index = i + other.limbs.len();
            while carry != 0 {
                let sum = u64::from(out[index]) + carry;
                out[index] = sum as u32;
                carry = sum >> 32;
                index += 1;
            }
        }
        out
    }
}

/// A fixed modulus with the Montgomery constants precomputed.
///
/// Every exponentiation in EBICS is modulo the same key, so the constants are
/// built once and reused across squarings.
#[derive(Debug, Clone)]
pub struct Modulus {
    n: BigUint,
    /// `-n^-1 mod 2^32`, the factor that clears one limb per reduction step.
    n0inv: u32,
    limbs: usize,
    /// `R mod n`, which is one in the Montgomery domain.
    r: BigUint,
    /// `R^2 mod n`, which converts into the Montgomery domain.
    r2: BigUint,
}

impl Modulus {
    /// Builds a modulus, or `None` if it is zero or even.
    ///
    /// Montgomery multiplication requires an odd modulus. Every RSA modulus is
    /// the product of two odd primes, so this only rejects a malformed key.
    #[must_use]
    pub fn new(n: BigUint) -> Option<Self> {
        if n.is_zero() || !n.is_odd() {
            return None;
        }
        let limbs = n.limbs.len();

        // Newton's iteration doubles the number of correct bits each step, and
        // five steps take one bit to thirty-two.
        let mut inverse = 1u32;
        for _ in 0..5 {
            inverse = inverse.wrapping_mul(2u32.wrapping_sub(n.limbs[0].wrapping_mul(inverse)));
        }
        let n0inv = inverse.wrapping_neg();

        // R = 2^(32 * limbs). Build R mod n by doubling one, reducing each time,
        // and R^2 mod n by continuing from there. Both are one-off costs.
        let mut r = BigUint::one();
        for _ in 0..32 * limbs {
            r.shl1();
            if r.cmp_padded(&n) != std::cmp::Ordering::Less {
                r = r.sub(&n);
            }
        }
        let mut r2 = r.clone();
        for _ in 0..32 * limbs {
            r2.shl1();
            if r2.cmp_padded(&n) != std::cmp::Ordering::Less {
                r2 = r2.sub(&n);
            }
        }

        Some(Self {
            n,
            n0inv,
            limbs,
            r,
            r2,
        })
    }

    /// The modulus in bytes, which is the width every RSA result is padded to.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.n.limbs.len() * 4
    }

    /// One Montgomery step: `a * b * R^-1 mod n`.
    fn mont_mul(&self, a: &BigUint, b: &BigUint) -> BigUint {
        let mut accumulator = a.mul_wide(b);
        accumulator.resize(self.limbs * 2 + 2, 0);

        for i in 0..self.limbs {
            let factor = accumulator[i].wrapping_mul(self.n0inv);
            let mut carry = 0u64;
            for (j, limb) in self.n.limbs.iter().enumerate() {
                let sum =
                    u64::from(accumulator[i + j]) + u64::from(factor) * u64::from(*limb) + carry;
                accumulator[i + j] = sum as u32;
                carry = sum >> 32;
            }
            let mut index = i + self.limbs;
            while carry != 0 {
                let sum = u64::from(accumulator[index]) + carry;
                accumulator[index] = sum as u32;
                carry = sum >> 32;
                index += 1;
            }
        }

        // The low `limbs` words are now zero, and the rest is below 2n, so at
        // most one subtraction brings it under n.
        let mut reduced = BigUint::from_limbs(accumulator[self.limbs..].to_vec());
        while reduced.cmp_padded(&self.n) != std::cmp::Ordering::Less {
            reduced = reduced.sub(&self.n);
        }
        reduced
    }

    /// Converts into the Montgomery domain: `a * R mod n`.
    fn mont_form(&self, a: &BigUint) -> BigUint {
        self.mont_mul(a, &self.r2)
    }

    /// Converts out of the Montgomery domain: `a * R^-1 mod n`.
    fn natural_form(&self, a: &BigUint) -> BigUint {
        self.mont_mul(a, &BigUint::one())
    }

    /// `base^exponent mod n`, by square-and-multiply.
    ///
    /// The exponent is a big-endian byte string, which is how an RSA exponent
    /// and a private exponent arrive.
    #[must_use]
    pub fn pow(&self, base: &BigUint, exponent: &[u8]) -> BigUint {
        let exponent = BigUint::from_be_bytes(exponent);
        let base = self.mont_form(base);
        let mut result = self.r.clone();

        for index in (0..exponent.bit_len()).rev() {
            result = self.mont_mul(&result, &result);
            if exponent.bit(index) {
                result = self.mont_mul(&result, &base);
            }
        }

        self.natural_form(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::{BigUint, Modulus};

    #[test]
    fn bytes_round_trip() {
        let cases: [&[u8]; 6] = [
            &[],
            &[0],
            &[0, 0, 1],
            &[1],
            &[0xff, 0xff, 0xff, 0xff],
            &[0x01, 0x00, 0x00, 0x00, 0x00],
        ];
        for case in cases {
            let value = BigUint::from_be_bytes(case);
            let padded = value.to_be_bytes(case.len().max(1)).unwrap();
            let expected: Vec<u8> = {
                let mut trimmed = case.to_vec();
                while trimmed.len() > 1 && trimmed[0] == 0 {
                    trimmed.remove(0);
                }
                let mut out = vec![0u8; case.len().max(1) - trimmed.len()];
                out.extend_from_slice(&trimmed);
                out
            };
            assert_eq!(padded, expected, "case {case:?}");
        }
    }

    #[test]
    fn bit_length() {
        assert_eq!(BigUint::from_be_bytes(&[]).bit_len(), 0);
        assert_eq!(BigUint::from_be_bytes(&[1]).bit_len(), 1);
        assert_eq!(BigUint::from_be_bytes(&[0xff]).bit_len(), 8);
        assert_eq!(BigUint::from_be_bytes(&[1, 0]).bit_len(), 9);
    }

    #[test]
    fn rejects_a_usable_modulus() {
        assert!(Modulus::new(BigUint::from_be_bytes(&[])).is_none());
        assert!(Modulus::new(BigUint::from_be_bytes(&[2])).is_none());
        assert!(Modulus::new(BigUint::from_be_bytes(&[9])).is_some());
    }

    /// `2^10 mod 1009` is 15, small enough to check by hand. The modulus must be
    /// odd: 1000 would be rejected, which is correct, and is why this uses the
    /// prime above it.
    #[test]
    fn small_power() {
        let modulus = Modulus::new(BigUint::from_be_bytes(&1009u32.to_be_bytes())).unwrap();
        let base = BigUint::from_be_bytes(&2u32.to_be_bytes());
        let result = modulus.pow(&base, &10u32.to_be_bytes());
        assert_eq!(result.to_be_bytes(2).unwrap(), vec![0, 15]);
    }

    /// An even modulus is not a usable Montgomery modulus, so it is refused
    /// rather than producing a wrong answer later.
    #[test]
    fn rejects_an_even_modulus() {
        assert!(Modulus::new(BigUint::from_be_bytes(&1000u32.to_be_bytes())).is_none());
    }
}
