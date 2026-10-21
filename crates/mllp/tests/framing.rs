//! Integration tests: the framing rules a real HL7 v2 endpoint relies on.

use mllp::{Decoder, Error, frame, unframe};

/// A realistic ADT^A01 with three segments, each ending in a carriage return.
const ADT: &[u8] = b"MSH|^~\\&|SEND|FAC|RECV|FAC|20260101120000||ADT^A01|MSG1|P|2.5.1\r\
EVN|A01|20260101120000\r\
PID|||MRN1||Doe^Jane||19800101|F\r";

#[test]
fn round_trip_of_a_multi_segment_message() {
    let wire = frame(ADT);
    assert_eq!(unframe(&wire).unwrap(), ADT);
}

#[test]
fn the_segment_carriage_returns_do_not_end_the_frame() {
    // Three segments means three 0x0D bytes before the terminator. A naive
    // scanner that stops at the first 0x0D would return only the MSH segment.
    let wire = frame(ADT);
    let payload = unframe(&wire).unwrap();
    assert_eq!(payload.iter().filter(|b| **b == 0x0D).count(), 3);
    assert!(payload.ends_with(b"\r"));
}

#[test]
fn a_decoder_reassembles_a_whole_conversation() {
    // A request and an acknowledgement, arriving in three ragged reads.
    let request = frame(ADT);
    let ack = frame(b"MSH|^~\\&|RECV|FAC|SEND|FAC|20260101120000||ACK|MSG2|P|2.5.1\rMSA|AA|MSG1\r");
    let stream: Vec<u8> = request.iter().chain(ack.iter()).copied().collect();

    let mut decoder = Decoder::new();
    let mut messages = Vec::new();
    for chunk in stream.chunks(7) {
        decoder.push(chunk);
        while let Some(frame) = decoder.next_frame() {
            messages.push(frame.unwrap());
        }
    }

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0], ADT);
    assert!(messages[1].starts_with(b"MSH|"));
    assert!(messages[1].windows(7).any(|w| w == b"MSA|AA|"));
}

#[test]
fn a_payload_may_contain_any_byte_except_the_terminator() {
    // Every byte value except the two that make up the terminator, so the frame
    // must survive content that looks like framing.
    let payload: Vec<u8> = (0u8..=255).filter(|b| *b != 0x1C && *b != 0x0B).collect();
    let wire = frame(&payload);
    assert_eq!(unframe(&wire).unwrap(), payload);
}

#[test]
fn a_binary_payload_containing_the_end_block_survives() {
    // 0x1C not followed by 0x0D is data, so this frame is unambiguous.
    let payload = b"\x1c\x1c\x0b\x00\xff";
    assert_eq!(unframe(&frame(payload)).unwrap(), payload);

    let mut decoder = Decoder::new();
    decoder.push(&frame(payload));
    assert_eq!(decoder.next_frame().unwrap().unwrap(), payload);
}

#[test]
fn trailing_bytes_are_reported_rather_than_silently_ignored() {
    let mut wire = frame(ADT);
    wire.push(b'x');
    assert_eq!(unframe(&wire), Err(Error::TrailingBytes(1)));
}

#[test]
fn a_truncated_frame_is_not_an_error_to_the_streaming_decoder() {
    let wire = frame(ADT);
    for cut in 1..wire.len() {
        let mut decoder = Decoder::new();
        decoder.push(&wire[..cut]);
        assert_eq!(decoder.next_frame(), None, "cut at {cut}");
    }
}

#[test]
fn a_frame_larger_than_the_ceiling_is_refused_and_reported_once() {
    let mut decoder = Decoder::with_max_frame_size(4);
    decoder.push(b"\x0b");
    decoder.push(b"way too long without a terminator");
    assert_eq!(
        decoder.next_frame(),
        Some(Err(Error::FrameTooLarge { max: 4 }))
    );
    assert_eq!(decoder.next_frame(), None);
    assert!(decoder.is_empty());
}
