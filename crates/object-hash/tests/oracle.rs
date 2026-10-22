//! Comparison against the `object-hash` npm package.
//!
//! The fixtures are the reference's own output: the canonical byte stream it
//! feeds to its hash, and the SHA-1 digest it produces by default. Agreeing on
//! both means a Rust service and a Node service can compute the same cache key,
//! which is the point of matching the reference at all.

use object_hash::{Algorithm, Value, canonical, hash, hash_with};

/// A case from the fixture: a name, the canonical form, and the digest.
struct Case {
    name: String,
    canonical: String,
    sha1: String,
    sha256: String,
    md5: String,
}

/// Reads the fixture, which is tab separated to avoid a JSON dependency.
fn cases() -> Vec<Case> {
    include_str!("fixtures/oracle.tsv")
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            Case {
                name: fields[0].to_string(),
                canonical: fields[1].to_string(),
                sha1: fields[2].to_string(),
                sha256: fields[3].to_string(),
                md5: fields[4].to_string(),
            }
        })
        .collect()
}

/// The values the fixture names, built the same way the generator built them.
fn value_for(name: &str) -> Value {
    match name {
        "null" => Value::Null,
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "zero" => Value::Number(0.0),
        "neg-zero" => Value::Number(-0.0),
        "one" => Value::Number(1.0),
        "neg-one" => Value::Number(-1.0),
        "int-42" => Value::Number(42.0),
        "big-int" => Value::Number(9_007_199_254_740_991.0),
        "float-0.1" => Value::Number(0.1),
        "float-1.5" => Value::Number(1.5),
        "float-neg-2.25" => Value::Number(-2.25),
        "float-100" => Value::Number(100.0),
        "nan" => Value::Number(f64::NAN),
        "infinity" => Value::Number(f64::INFINITY),
        "neg-infinity" => Value::Number(f64::NEG_INFINITY),
        "empty-string" => Value::String(String::new()),
        "string" => Value::String("hello".into()),
        "string-space" => Value::String("a b".into()),
        "string-unicode" => Value::String("café".into()),
        "string-cjk" => Value::String("中文".into()),
        "string-emoji" => Value::String("😀".into()),
        "string-colon" => Value::String("a:b".into()),
        "string-number-like" => Value::String("42".into()),
        "empty-array" => Value::Array(vec![]),
        "array-one" => Value::Array(vec![Value::Number(1.0)]),
        "array-ints" => Value::Array(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]),
        "array-order" => Value::Array(vec![
            Value::Number(3.0),
            Value::Number(1.0),
            Value::Number(2.0),
        ]),
        "array-mixed" => Value::Array(vec![
            Value::Number(1.0),
            Value::String("two".into()),
            Value::Bool(true),
            Value::Null,
        ]),
        "array-nested" => Value::Array(vec![
            Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]),
            Value::Array(vec![Value::Number(3.0)]),
        ]),
        "empty-map" => Value::Map(vec![]),
        "map-one" => Value::Map(vec![("a".into(), Value::Number(1.0))]),
        "map-two" => Value::Map(vec![
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::Number(2.0)),
        ]),
        "map-reversed" => Value::Map(vec![
            ("b".into(), Value::Number(2.0)),
            ("a".into(), Value::Number(1.0)),
        ]),
        "map-nested" => Value::Map(vec![(
            "a".into(),
            Value::Map(vec![("b".into(), Value::Number(1.0))]),
        )]),
        "map-in-array" => Value::Array(vec![
            Value::Map(vec![("a".into(), Value::Number(1.0))]),
            Value::Map(vec![("b".into(), Value::Number(2.0))]),
        ]),
        "map-array-value" => Value::Map(vec![(
            "list".into(),
            Value::Array(vec![
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
            ]),
        )]),
        "map-null-value" => Value::Map(vec![("a".into(), Value::Null)]),
        "map-empty-string-key" => Value::Map(vec![(String::new(), Value::Number(1.0))]),
        "map-unicode-key" => Value::Map(vec![("café".into(), Value::String("x".into()))]),
        "deep" => Value::Map(vec![(
            "a".into(),
            Value::Map(vec![(
                "b".into(),
                Value::Map(vec![(
                    "c".into(),
                    Value::Array(vec![
                        Value::Number(1.0),
                        Value::Array(vec![
                            Value::Number(2.0),
                            Value::Array(vec![Value::Number(3.0)]),
                        ]),
                    ]),
                )]),
            )]),
        )]),
        "map-many" => Value::Map(vec![
            ("k1".into(), Value::Number(1.0)),
            ("k2".into(), Value::Number(2.0)),
            ("k3".into(), Value::Number(3.0)),
            ("k4".into(), Value::Number(4.0)),
            ("k5".into(), Value::Number(5.0)),
        ]),
        "array-of-strings" => Value::Array(vec![
            Value::String("b".into()),
            Value::String("a".into()),
            Value::String("c".into()),
        ]),
        "map-bool-values" => Value::Map(vec![
            ("t".into(), Value::Bool(true)),
            ("f".into(), Value::Bool(false)),
        ]),
        "empty-set" => Value::Set(vec![]),
        "set-one" => Value::Set(vec![Value::Number(1.0)]),
        "set-ints" => Value::Set(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]),
        "set-order" => Value::Set(vec![
            Value::Number(3.0),
            Value::Number(1.0),
            Value::Number(2.0),
        ]),
        "set-strings" => Value::Set(vec![
            Value::String("b".into()),
            Value::String("a".into()),
            Value::String("c".into()),
        ]),
        "set-nested" => Value::Set(vec![
            Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]),
            Value::Array(vec![Value::Number(3.0)]),
        ]),
        "set-scalar-dup" => Value::Set(vec![Value::Number(1.0), Value::Number(1.0)]),
        "set-nested-dup" => Value::Set(vec![
            Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]),
            Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]),
        ]),
        "set-nan-dup" => Value::Set(vec![Value::Number(f64::NAN), Value::Number(f64::NAN)]),
        "map-dup-key" => Value::Map(vec![
            ("a".into(), Value::Number(1.0)),
            ("a".into(), Value::Number(2.0)),
        ]),
        "map-dup-key3" => Value::Map(vec![
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::Number(9.0)),
            ("a".into(), Value::Number(2.0)),
        ]),
        "mixed-nest" => Value::Map(vec![(
            "list".into(),
            Value::Array(vec![
                Value::Map(vec![("x".into(), Value::Number(1.0))]),
                Value::Number(2.0),
                Value::String("three".into()),
            ]),
        )]),
        other => panic!("no value built for fixture case {other}"),
    }
}

#[test]
fn every_fixture_case_has_the_same_canonical_form_and_digest() {
    let cases = cases();
    assert!(cases.len() > 55, "only {} cases", cases.len());

    let mut failures = Vec::new();
    for case in &cases {
        let value = value_for(&case.name);
        let got_canonical = canonical(&value);
        if got_canonical != case.canonical {
            failures.push(format!(
                "{}: canonical form\n  expected {:?}\n  got      {:?}",
                case.name, case.canonical, got_canonical
            ));
        }
        let got_digest = hash(&value);
        if got_digest != case.sha1 {
            failures.push(format!(
                "{}: sha1 digest\n  expected {}\n  got      {}",
                case.name, case.sha1, got_digest
            ));
        }
        for (algorithm, expected) in [
            (Algorithm::Sha256, &case.sha256),
            (Algorithm::Md5, &case.md5),
        ] {
            let got = hash_with(&value, algorithm);
            if &got != expected {
                failures.push(format!(
                    "{}: {algorithm:?} digest\n  expected {expected}\n  got      {got}",
                    case.name
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn a_map_is_the_same_whatever_order_it_was_built_in() {
    // The property the whole crate exists for, checked on the values rather than
    // on the fixture names.
    let forward = Value::Map(vec![
        ("a".into(), Value::Number(1.0)),
        ("b".into(), Value::Number(2.0)),
        ("c".into(), Value::Number(3.0)),
    ]);
    let mut entries = vec![
        ("c".into(), Value::Number(3.0)),
        ("a".into(), Value::Number(1.0)),
        ("b".into(), Value::Number(2.0)),
    ];
    let shuffled = Value::Map(std::mem::take(&mut entries));
    assert_eq!(hash(&forward), hash(&shuffled));
}

#[test]
fn the_digest_is_stable_across_processes() {
    // This is the property `RandomState` does not have: a fixed value has to give
    // a fixed digest, or it cannot be written down. The expected value is
    // recorded rather than computed, so a change to the encoding is caught.
    let value = Value::Map(vec![
        ("name".into(), Value::String("cipansor".into())),
        ("year".into(), Value::Number(2026.0)),
    ]);
    assert_eq!(hash(&value), "3f7b8dd64e1323a0f8b49746c64fcd803ff63b0e");
}

#[test]
fn a_nested_change_changes_the_digest() {
    let base = Value::Map(vec![(
        "list".into(),
        Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]),
    )]);
    let changed = Value::Map(vec![(
        "list".into(),
        Value::Array(vec![Value::Number(1.0), Value::Number(3.0)]),
    )]);
    assert_ne!(hash(&base), hash(&changed));
}
