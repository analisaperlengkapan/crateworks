# mllp

MLLP framing for HL7 v2 messages. No dependencies.

MLLP, the Minimal Lower Layer Protocol, is how almost every HL7 v2 message
travels over TCP. It wraps each message in a start block, the payload, and an
end block, so a receiver reading a byte stream knows where one message stops and
the next begins.

```rust
use mllp::{frame, unframe, Decoder};

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
assert!(decoder.next_frame().is_none()); // half a frame, wait for more
decoder.push(b"|FAC|20260101||ADT^A01|1|P|2.5.1\r\x1c\x0d");
assert_eq!(decoder.next_frame().unwrap().unwrap(), message);
```

## The framing bytes

| Role | Name | Byte |
|---|---|---|
| Start block | `SB`, vertical tab | `0x0B` |
| End block | `EB`, file separator | `0x1C` |
| Trailing | `CR`, carriage return | `0x0D` |

A frame is `0x0B` + payload + `0x1C 0x0D`.

The terminator is two bytes, and that matters. An HL7 v2 message ends each
segment with a carriage return, so the payload normally contains `0x0D`. If the
frame ended at the first `0x0D`, every message would be cut at its first segment.
This crate searches for the pair `0x1C 0x0D`, so a carriage return inside the
payload is data. A `0x1C` that is not followed by `0x0D` is data too, because a
binary payload may contain any byte.

## Reading a stream

`Decoder` holds the bytes that do not yet form a whole frame. Push each socket
read in, drain `next_frame` until it returns `None`. A frame split at any byte
comes out whole, and one read carrying several frames yields several messages;
both cases are covered by tests.

`next_frame` returns three things: `Some(Ok(payload))` for a whole frame,
`None` when more bytes are needed, and `Some(Err(..))` for input that cannot be
part of a frame. Bytes before the next start block are discarded and reported
once, so a decoder resynchronises after a desync instead of stalling.

A frame that never terminates would otherwise grow the buffer without bound.
`Decoder` stops at 16 MiB by default and returns `Error::FrameTooLarge`;
`Decoder::with_max_frame_size` changes the ceiling. That is a guard against a
missing terminator, not a limit on real messages.

## What it does not do

This crate frames bytes. It does not parse HL7 v2: the payload is opaque, and the
delimiters that make up a message (`|`, `^`, `~`, `&`) are the caller's business.
It does not open sockets, retry, or decide how long to wait for a reply, and it
does not implement TLS or the newer, block-based MLLP (MLLP over HTTPS). It also
does not build HL7 acknowledgement messages; that needs a message parser.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
