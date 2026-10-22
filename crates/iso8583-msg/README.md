# iso8583-msg

[![crates.io](https://img.shields.io/crates/v/iso8583-msg.svg)](https://crates.io/crates/iso8583-msg)
[![docs.rs](https://docs.rs/iso8583-msg/badge.svg)](https://docs.rs/iso8583-msg)

ISO 8583 messages: the format card switches use for ATM, EDC and POS
transactions. No dependencies.

```rust
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
```

An ISO 8583 message is a four-character message type indicator, one or two
bitmaps saying which fields are present, and then those fields in ascending
order. The crate handles all of that: the bitmap chain, fixed and
variable-length fields, ASCII, binary-coded decimal, binary and EBCDIC code
page 500 encodings, and length prefixes in ASCII, binary-coded decimal or
binary.

## Specifications

ISO 8583 does not standardise field lengths; every switch publishes its own
dialect. So a `Spec` is data rather than code:

```rust
use iso8583_msg::{DataEncoding, FieldSpec, LengthEncoding, Spec};

let mut spec = Spec::new();
spec.set(0, FieldSpec::fixed(DataEncoding::Ascii, LengthEncoding::Ascii, 4)).unwrap();
spec.set(1, FieldSpec::fixed(DataEncoding::Ascii, LengthEncoding::Ascii, 16)).unwrap();
spec.set(2, FieldSpec::variable(DataEncoding::Ascii, LengthEncoding::Ascii, 2, 19)).unwrap();
```

`default_ascii`, `default_bcd` and `default_ebcdic` are ready-made starting
points. `default_ascii` matches the widely used
[`pyiso8583`](https://github.com/knovichikhin/pyiso8583) specification, and the
test fixtures in this crate are generated from that library, so the bytes this
crate produces are byte-for-byte what it produces.

## What it does not do

Field contents are opaque strings. What a processing code or a response code
*means* is defined per switch, so no business-rule validation is attempted.
There is also no transport, no MAC verification and no PIN block decryption:
this crate frames messages and nothing more.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
