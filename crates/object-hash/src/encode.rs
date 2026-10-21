//! The canonical form of a value.
//!
//! A structured value is written out as a single byte string in a way that
//! depends only on what the value *is*, never on how it was built. Two values
//! that a caller would call equal produce the same bytes, so hashing the bytes
//! gives a stable name for the value.
//!
//! The encoding is the one the `object-hash` npm package uses, so the bytes here
//! are byte-for-byte the same and a digest computed here matches one computed
//! there. That is deliberate: it lets a Rust service and a Node service agree on
//! a cache key or an ETag without either one knowing about the other.
//!
//! # What the value is, not how it was made
//!
//! - A map is written in key order, so two maps with the same entries in
//!   different insertion order are the same value.
//! - An array keeps its order; a set does not.
//! - Integers and floats of equal value are the same number. There is no
//!   separate integer case, because the reference has none.
//! - Every string carries its length, so no two different structures can write
//!   the same bytes by accident.

use std::fmt::Write as _;

use crate::Value;

/// Writes the canonical form of a value into a string.
///
/// The result is text because the encoding is text: it is what the reference
/// writes into its hash stream. The bytes are its UTF-8 encoding.
pub(crate) fn write(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("Null"),
        Value::Bool(true) => out.push_str("bool:true"),
        Value::Bool(false) => out.push_str("bool:false"),
        Value::Number(number) => write_number(*number, out),
        Value::String(text) => write_string(text, out),
        Value::Array(items) => write_array(items, out),
        Value::Set(items) => {
            out.push_str("set:");
            write_set(items, out);
        }
        Value::Map(entries) => {
            out.push_str("map:");
            write_map(entries, out);
        }
    }
}

/// Numbers, written the way JavaScript writes them.
///
/// That means an integral float loses its point (`100`, not `100.0`), the two
/// infinities and not-a-number have names, and negative zero is `0` because
/// `String(-0)` is `"0"`.
fn write_number(number: f64, out: &mut String) {
    out.push_str("number:");
    if number.is_nan() {
        out.push_str("NaN");
    } else if number == f64::INFINITY {
        out.push_str("Infinity");
    } else if number == f64::NEG_INFINITY {
        out.push_str("-Infinity");
    } else if number == 0.0 {
        // Covers -0.0 as well: the reference prints it as "0".
        out.push('0');
    } else if number.fract() == 0.0 && number.abs() < 1e21 {
        // An integral value in the range JavaScript prints without an exponent.
        let _ = write!(out, "{}", number as i128);
    } else {
        let _ = write!(out, "{number}");
    }
}

/// Strings carry their length, counted in UTF-16 code units.
///
/// JavaScript measures strings that way, so a character above the basic plane
/// counts as two. Counting `char`s instead would give a different length for any
/// emoji and break compatibility.
fn write_string(text: &str, out: &mut String) {
    let length: usize = text.chars().map(|c| c.len_utf16()).sum();
    let _ = write!(out, "string:{length}:");
    out.push_str(text);
}

/// Arrays keep their order and carry their length.
fn write_array(items: &[Value], out: &mut String) {
    let _ = write!(out, "array:{}:", items.len());
    for item in items {
        write(item, out);
    }
}

/// Sets and maps are unordered, so their entries are sorted.
///
/// This is where the encoding stops being obvious. The reference sorts them by
/// writing each entry out first, sorting those strings, and then writing the
/// sorted strings *as strings*, length prefix and all. So a two-entry map is
/// `array:2:` followed by `array:2:` again and then each entry quoted as a
/// string. With one entry or none there is nothing to sort and the entries are
/// written straight out, which is why a one-entry map looks different from a
/// two-entry one.
fn write_unordered(entries: Vec<String>, out: &mut String) {
    let count = entries.len();
    let _ = write!(out, "array:{count}:");
    if count <= 1 {
        for entry in entries {
            out.push_str(&entry);
        }
        return;
    }
    let mut sorted = entries;
    sorted.sort();
    let _ = write!(out, "array:{count}:");
    for entry in sorted {
        write_string(&entry, out);
    }
}

/// A set ignores the order of its entries.
fn write_set(items: &[Value], out: &mut String) {
    write_unordered(items.iter().map(canonical).collect(), out);
}

/// A map is written in key order, whatever order it was built in.
///
/// Each entry is written as a two-element array of the key and the value, and
/// the entries are sorted by that written form. Sorting by the written form
/// rather than by the key matters: `z` sorts before `a10`, because the length
/// prefix is part of the string, and sorting by the key would put them the other
/// way round.
fn write_map(entries: &[(String, Value)], out: &mut String) {
    let serialized = entries
        .iter()
        .map(|(key, value)| {
            let mut entry = String::from("array:2:");
            write_string(key, &mut entry);
            write(value, &mut entry);
            entry
        })
        .collect();
    write_unordered(serialized, out);
}

/// The canonical form of one value, on its own.
pub(crate) fn canonical(value: &Value) -> String {
    let mut out = String::new();
    write(value, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_null_is_the_word() {
        assert_eq!(canonical(&Value::Null), "Null");
    }

    #[test]
    fn booleans_are_labelled() {
        assert_eq!(canonical(&Value::Bool(true)), "bool:true");
        assert_eq!(canonical(&Value::Bool(false)), "bool:false");
    }

    #[test]
    fn integers_and_equal_floats_are_the_same() {
        assert_eq!(canonical(&Value::Number(1.0)), "number:1");
        assert_eq!(canonical(&Value::Number(100.0)), "number:100");
        assert_eq!(canonical(&Value::Number(-1.0)), "number:-1");
    }

    #[test]
    fn negative_zero_is_written_as_zero() {
        assert_eq!(canonical(&Value::Number(-0.0)), "number:0");
    }

    #[test]
    fn the_special_numbers_have_names() {
        assert_eq!(canonical(&Value::Number(f64::NAN)), "number:NaN");
        assert_eq!(canonical(&Value::Number(f64::INFINITY)), "number:Infinity");
        assert_eq!(
            canonical(&Value::Number(f64::NEG_INFINITY)),
            "number:-Infinity"
        );
    }

    #[test]
    fn a_string_carries_its_utf16_length() {
        assert_eq!(canonical(&Value::String(String::new())), "string:0:");
        assert_eq!(canonical(&Value::String("hello".into())), "string:5:hello");
        assert_eq!(canonical(&Value::String("café".into())), "string:4:café");
        assert_eq!(canonical(&Value::String("中文".into())), "string:2:中文");
        // An emoji is one character but two UTF-16 code units.
        assert_eq!(canonical(&Value::String("😀".into())), "string:2:😀");
    }

    #[test]
    fn an_array_keeps_its_order() {
        assert_eq!(canonical(&Value::Array(vec![])), "array:0:");
        assert_eq!(
            canonical(&Value::Array(vec![Value::Number(1.0), Value::Number(2.0)])),
            "array:2:number:1number:2"
        );
        assert_eq!(
            canonical(&Value::Array(vec![Value::Number(2.0), Value::Number(1.0)])),
            "array:2:number:2number:1"
        );
    }

    #[test]
    fn a_map_is_written_in_key_order() {
        let forward = Value::Map(vec![
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::Number(2.0)),
        ]);
        let backward = Value::Map(vec![
            ("b".into(), Value::Number(2.0)),
            ("a".into(), Value::Number(1.0)),
        ]);
        assert_eq!(canonical(&forward), canonical(&backward));
    }

    #[test]
    fn a_set_ignores_order_but_an_array_does_not() {
        let one = Value::Number(1.0);
        let two = Value::Number(2.0);
        let set_forward = Value::Set(vec![one.clone(), two.clone()]);
        let set_backward = Value::Set(vec![two.clone(), one.clone()]);
        assert_eq!(canonical(&set_forward), canonical(&set_backward));

        let array_forward = Value::Array(vec![one.clone(), two.clone()]);
        let array_backward = Value::Array(vec![two, one]);
        assert_ne!(canonical(&array_forward), canonical(&array_backward));
    }

    #[test]
    fn a_map_key_cannot_be_confused_with_a_value() {
        // The length prefix is what stops `{"ab": 1}` and `{"a": "b1"}` from
        // writing the same bytes.
        let first = Value::Map(vec![("ab".into(), Value::Number(1.0))]);
        let second = Value::Map(vec![("a".into(), Value::String("b1".into()))]);
        assert_ne!(canonical(&first), canonical(&second));
    }
}
