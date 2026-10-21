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

use std::collections::HashSet;
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
/// This is the part of the encoding that is easiest to get subtly wrong. Rust
/// and JavaScript both print the shortest text that reads back as the same
/// number, but they disagree about *where* to put the decimal point. For
/// `1e21` Rust prints twenty-two digits; JavaScript prints `1e+21`. For `1e-7`
/// Rust prints `0.0000001`; JavaScript prints `1e-7`. Getting this wrong makes
/// every large or small number hash differently from the reference, silently.
///
/// So the rules from the ECMAScript specification are followed directly rather
/// than Rust's `Display`. The number is first reduced to its significant digits
/// and an exponent, which is the form both languages agree on, and then the
/// specification's four cases decide the layout.
///
/// The specials: `NaN`, `Infinity` and `-Infinity` are named, and negative zero
/// is `0`, because `String(-0)` is `"0"`.
fn write_number(number: f64, out: &mut String) {
    out.push_str("number:");
    out.push_str(&number_to_string(number));
}

/// The text JavaScript's `String(number)` would produce.
fn number_to_string(number: f64) -> String {
    if number.is_nan() {
        return "NaN".to_string();
    }
    if number == f64::INFINITY {
        return "Infinity".to_string();
    }
    if number == f64::NEG_INFINITY {
        return "-Infinity".to_string();
    }
    if number == 0.0 {
        // Covers -0.0: JavaScript prints it as "0".
        return "0".to_string();
    }

    let sign = if number < 0.0 { "-" } else { "" };
    // `{:e}` gives the shortest digits that read back as the same number, in the
    // form `1.23456e7`, which is the one piece both languages agree on.
    let scientific = format!("{:e}", number.abs());
    let (mantissa, exponent) = scientific.split_once('e').expect("scientific form");
    let exponent: i32 = exponent.parse().expect("exponent");

    let mut digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }

    let digits = digits.as_str();
    let count = digits.len() as i32;
    // Where the decimal point sits, as JavaScript counts it.
    let point = exponent + 1;

    if count <= point && point <= 21 {
        // Whole number: the digits then enough zeros to reach the point.
        let mut text = String::from(digits);
        text.extend(std::iter::repeat_n('0', (point - count) as usize));
        format!("{sign}{text}")
    } else if 0 < point && point <= 21 {
        // The point falls inside the digits.
        let split = point as usize;
        format!("{sign}{}.{}", &digits[..split], &digits[split..])
    } else if -6 < point && point <= 0 {
        // Below one: a leading zero, then the point's zeros, then the digits.
        let mut text = String::from("0.");
        text.extend(std::iter::repeat_n('0', (-point) as usize));
        text.push_str(digits);
        format!("{sign}{text}")
    } else if count == 1 {
        // One digit in exponential form.
        format!(
            "{sign}{digits}e{}{}",
            exponent_sign(point - 1),
            (point - 1).abs()
        )
    } else {
        format!(
            "{sign}{}.{}e{}{}",
            &digits[..1],
            &digits[1..],
            exponent_sign(point - 1),
            (point - 1).abs()
        )
    }
}

/// JavaScript writes a plus sign on a non-negative exponent.
fn exponent_sign(exponent: i32) -> char {
    if exponent < 0 { '-' } else { '+' }
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

/// Compare two strings the way JavaScript's default sort does.
///
/// JavaScript compares strings by UTF-16 code unit. Comparing the UTF-8 bytes
/// instead agrees for everything in the basic plane but not above it: an emoji
/// starts at U+D83D, which is below U+FFFF, while its UTF-8 bytes start at 0xF0,
/// which is above. Sorting a set holding both an emoji and U+FFFF would put them
/// in the wrong order and change the digest. The UTF-16 units have to be built
/// to get this right.
fn utf16_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
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
    sorted.sort_by(|a, b| utf16_cmp(a, b));
    let _ = write!(out, "array:{count}:");
    for entry in sorted {
        write_string(&entry, out);
    }
}

/// A set ignores the order of its entries.
///
/// Scalar entries are deduplicated, because a JavaScript `Set` does that: it
/// keeps one of each distinct value. Arrays and maps are not, because in
/// JavaScript they are compared by identity, and two equal-looking arrays are
/// still two different objects. That asymmetry is the reference's, not a choice
/// made here.
fn write_set(items: &[Value], out: &mut String) {
    let mut seen = HashSet::new();
    let entries = items
        .iter()
        .filter_map(|item| {
            if item.is_scalar() {
                let written = canonical(item);
                if !seen.insert(written.clone()) {
                    return None;
                }
                Some(written)
            } else {
                Some(canonical(item))
            }
        })
        .collect();
    write_unordered(entries, out);
}

/// A map is written in key order, whatever order it was built in.
///
/// A repeated key keeps the last value, because that is what a JavaScript `Map`
/// does, and the entries are sorted by their written form rather than by the key.
/// Sorting by the written form is what makes the order stable for keys whose own
/// order would not settle it, and it is the reference's behaviour.
fn write_map(entries: &[(String, Value)], out: &mut String) {
    let mut kept: Vec<(&String, &Value)> = Vec::with_capacity(entries.len());
    for (key, value) in entries {
        match kept.iter_mut().find(|(existing, _)| *existing == key) {
            Some(existing) => existing.1 = value,
            None => kept.push((key, value)),
        }
    }

    let serialized = kept
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
    fn a_set_drops_repeated_scalars_but_not_repeated_arrays() {
        // A JavaScript Set keeps one of each distinct scalar.
        assert_eq!(
            canonical(&Value::Set(vec![Value::Number(1.0), Value::Number(1.0)])),
            "set:array:1:number:1"
        );
        assert_eq!(
            canonical(&Value::Set(vec![
                Value::Number(f64::NAN),
                Value::Number(f64::NAN)
            ])),
            "set:array:1:number:NaN"
        );
        // Two equal arrays are still two objects, so both are kept.
        let array = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]);
        let two_arrays = Value::Set(vec![array.clone(), array]);
        assert!(canonical(&two_arrays).starts_with("set:array:2:"));
    }

    #[test]
    fn entries_are_ordered_by_utf16_code_unit() {
        // An emoji begins at U+D83D, below U+FFFF, so it sorts first. Its UTF-8
        // bytes begin at 0xF0, above the UTF-8 bytes of U+FFFF, so a byte
        // comparison would put them the other way round.
        let emoji_first = Value::Set(vec![
            Value::String("\u{ffff}z".into()),
            Value::String("\u{1f600}".into()),
        ]);
        let emoji_last = Value::Set(vec![
            Value::String("\u{1f600}".into()),
            Value::String("\u{ffff}z".into()),
        ]);
        assert_eq!(canonical(&emoji_first), canonical(&emoji_last));
        let written = canonical(&emoji_first);
        let emoji_at = written.find('\u{1f600}').expect("emoji present");
        let max_at = written.find('\u{ffff}').expect("U+FFFF present");
        assert!(
            emoji_at < max_at,
            "emoji should sort before U+FFFF: {written}"
        );
    }

    #[test]
    fn a_repeated_map_key_keeps_the_last_value() {
        let repeated = Value::Map(vec![
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::Number(9.0)),
            ("a".into(), Value::Number(2.0)),
        ]);
        let resolved = Value::Map(vec![
            ("a".into(), Value::Number(2.0)),
            ("b".into(), Value::Number(9.0)),
        ]);
        assert_eq!(canonical(&repeated), canonical(&resolved));
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
