//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use mllp::{Decoder, frame, unframe};

    let message = b"MSH|^~\\&|SEND|FAC|RECV|FAC|20260101||ADT^A01|1|P|2.5.1\r";

    // Put a message on the wire.
    let wire = frame(message);
    assert_eq!(wire[0], 0x0B);
    assert_eq!(&wire[wire.len() - 2..], &[0x1C, 0x0D]);

    // Take one back off, when you already hold a whole frame.
    assert_eq!(unframe(&wire).unwrap(), message);

    // Or reassemble from a stream, where reads do not line up with frames.
    let mut decoder = Decoder::new();
    decoder.push(b"\x0bMSH|^~\\&|SEND|FAC|RECV");
    assert!(decoder.next_frame().is_none());
    decoder.push(b"|FAC|20260101||ADT^A01|1|P|2.5.1\r\x1c\x0d");
    assert_eq!(decoder.next_frame().unwrap().unwrap(), message);
}
