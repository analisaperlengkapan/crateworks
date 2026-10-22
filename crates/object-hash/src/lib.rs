#![doc = include_str!("../README.md")]
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

    /// Whether this value is a scalar.
    ///
    /// Scalars are the values a JavaScript `Set` deduplicates; arrays and maps
    /// it compares by identity instead. Nothing outside the crate needs this,
    /// but it is the distinction the encoding turns on.
    pub(crate) fn is_scalar(&self) -> bool {
        matches!(
            self,
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::String(_)
        )
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
///
/// # Example
///
/// ```
/// use object_hash::{hash, Value};
///
/// let left = Value::Map(vec![
///     ("b".into(), Value::number(2)),
///     ("a".into(), Value::string("one")),
/// ]);
/// // A map's key order does not change the digest.
/// let right = Value::Map(vec![
///     ("a".into(), Value::string("one")),
///     ("b".into(), Value::number(2)),
/// ]);
/// assert_eq!(hash(&left), hash(&right));
///
/// // An array's order does.
/// assert_ne!(
///     hash(&Value::Array(vec![Value::number(1), Value::number(2)])),
///     hash(&Value::Array(vec![Value::number(2), Value::number(1)])),
/// );
/// ```
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
