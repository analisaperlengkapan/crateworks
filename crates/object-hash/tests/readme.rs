//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use object_hash::{Value, hash};

    let one = Value::Map(vec![
        ("name".into(), Value::String("cipansor".into())),
        ("year".into(), Value::Number(2026.0)),
    ]);
    // The same entries, inserted the other way round.
    let two = Value::Map(vec![
        ("year".into(), Value::Number(2026.0)),
        ("name".into(), Value::String("cipansor".into())),
    ]);

    assert_eq!(hash(&one), hash(&two));
}
