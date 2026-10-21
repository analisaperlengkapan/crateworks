//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use iso8583_msg::{Message, default_ascii};

    let spec = default_ascii();

    let mut msg = Message::new("0200").unwrap();
    msg.set(2, "4111111111111111").unwrap();
    msg.set(3, "000000").unwrap();
    msg.set(4, "000000010000").unwrap();

    let bytes = msg.encode(&spec).unwrap();
    assert_eq!(&bytes[..4], b"0200");

    let back = Message::decode(&bytes, &spec).unwrap();
    assert_eq!(back.get(2), Some("4111111111111111"));
}
