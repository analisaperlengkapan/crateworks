//! A stable hash of a structured value.
//!
//! Given a value — a map, an array, a string, a number, nested in any
//! combination — this produces a short digest that is the same every time for
//! values that are equal. Not merely equal within one run of a program: equal
//! across runs, across processes, and across machines.
//!
//! # Why `Hash` is not enough
//!
//! `std::hash::Hash` looks like the right tool and is not. The default hasher,
//! `RandomState`, is seeded per process, so the same value hashes differently
//! every time the program starts. That is exactly right for a `HashMap`, where
//! the seed is what stops an attacker choosing collisions, and exactly wrong for
//! anything that has to be written down or compared across processes. The other
//! half of the problem is that a `HashMap` iterates in an unspecified order, so
//! even with a fixed hasher two equal maps can hash differently.
//!
//! So the value has to be normalised first: written into a canonical form that
//! depends only on what the value is. Then the canonical bytes are hashed. This
//! crate does both.
//!
//! # What it is for
//!
//! - **Cache keys.** A key built from the request parameters has to hit the same
//!   entry for the same request.
//! - **ETags.** An entity tag should change when the content changes and not
//!   when a field is reordered.
//! - **Idempotency keys.** Recognise a repeated request as the same action.
//! - **Change detection and deduplication.** Compare two digests instead of two
//!   documents.
//!
//! # Example
//!
//! ```
//! use object_hash::{Value, hash};
//!
//! let one = Value::Map(vec![
//!     ("name".into(), Value::String("cipansor".into())),
//!     ("year".into(), Value::Number(2026.0)),
//! ]);
//! // The same entries, inserted the other way round.
//! let two = Value::Map(vec![
//!     ("year".into(), Value::Number(2026.0)),
//!     ("name".into(), Value::String("cipansor".into())),
//! ]);
//!
//! assert_eq!(hash(&one), hash(&two), "key order does not matter for a map");
//!
//! let three = Value::Map(vec![
//!     ("name".into(), Value::String("cipansor".into())),
//!     ("year".into(), Value::Number(2027.0)),
//! ]);
//! assert_ne!(hash(&one), hash(&three), "a changed value changes the digest");
//! ```
//!
//! # Compatibility
//!
//! The canonical form is the one the `object-hash` npm package produces, byte
//! for byte, and the digest is SHA-1 in lowercase hexadecimal, which is its
//! default. A digest computed here equals one computed there for the same value,
//! so a Rust service and a Node service can agree on a cache key without either
//! knowing about the other. That is also why a map is written as an array of
//! pairs and an integral float has no point: the reference writes them that way,
//! and agreeing with it is worth more than tidier output.
//!
//! The one thing that does not carry over is the reference's handling of
//! JavaScript objects. An object there is a thing with a prototype and a
//! constructor, and it mixes those into the hash; a `Value::Map` here is just
//! its entries. That is a deliberate difference: a Rust caller has no prototype
//! to respect, and mixing one in would make the digest depend on the library
//! rather than on the value.
//!
//! # What it is not
//!
//! SHA-1 is not used here as a security primitive. The digest names a value; it
//! does not protect it. Do not treat a digest as a signature or as proof that a
//! value came from somewhere. If a stronger digest is wanted, hash the canonical
//! bytes — [`canonical`] returns them — with any other function.

mod encode;
mod sha1;

use std::fmt;

/// A value that can be hashed.
///
/// This is the set of things the reference can hash, minus the ones that only
/// exist in JavaScript: there is no case for a function, a symbol, or a `Date`,
/// because a Rust caller would not pass one. A `Map` here stands for the
/// reference's plain object and its `Map`, which it treats the same way.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// The absence of a value. Written `Null`, not the empty string.
    Null,
    /// A boolean.
    Bool(bool),
    /// A number. An integer is written as a float that happens to be integral,
    /// because the reference has no separate integer case.
    Number(f64),
    /// A string.
    String(String),
    /// A sequence whose order matters.
    Array(Vec<Value>),
    /// A collection whose order does not matter.
    Set(Vec<Value>),
    /// A mapping from strings to values, written in key order.
    Map(Vec<(String, Value)>),
}

impl Value {
    /// A string value from anything string-like.
    #[must_use]
    pub fn string(text: impl Into<String>) -> Self {
        Self::String(text.into())
    }

    /// A number value.
    #[must_use]
    pub fn number(number: impl Into<f64>) -> Self {
        Self::Number(number.into())
    }

    /// The digest of this value, as lowercase hexadecimal SHA-1.
    #[must_use]
    pub fn hash(&self) -> String {
        hash(self)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&canonical(self))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Number(value as f64)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

/// The canonical form of a value, as text.
///
/// Two values that are equal have the same canonical form. It is returned rather
/// than hashed so a caller can use a different digest, or none.
#[must_use]
pub fn canonical(value: &Value) -> String {
    encode::canonical(value)
}

/// The digest of a value, as lowercase hexadecimal SHA-1.
///
/// The digest is stable across runs and machines for equal values. See the
/// crate documentation for what equal means here.
#[must_use]
pub fn hash(value: &Value) -> String {
    sha1::hex(canonical(value).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_digest_is_forty_hexadecimal_characters() {
        let digest = hash(&Value::Null);
        assert_eq!(digest.len(), 40);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn equal_values_hash_the_same_and_different_values_do_not() {
        assert_eq!(hash(&Value::Number(1.0)), hash(&Value::Number(1.0)));
        assert_ne!(hash(&Value::Number(1.0)), hash(&Value::Number(2.0)));
        assert_ne!(hash(&Value::Number(1.0)), hash(&Value::String("1".into())));
        assert_ne!(hash(&Value::Null), hash(&Value::String("Null".into())));
    }

    #[test]
    fn the_conversions_build_the_expected_values() {
        assert_eq!(Value::from("a"), Value::String("a".into()));
        assert_eq!(Value::from(1_i32), Value::Number(1.0));
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(
            Value::from(vec![1_i32, 2]),
            Value::Array(vec![Value::Number(1.0), Value::Number(2.0)])
        );
    }

    #[test]
    fn the_display_form_is_the_canonical_form() {
        assert_eq!(Value::Bool(true).to_string(), "bool:true");
        assert_eq!(Value::Null.to_string(), "Null");
    }
}
