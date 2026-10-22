# wssec

WS-Security headers for SOAP services: user name tokens, timestamps, and the
password digest that lets a password travel without being sent.

```rust
use wssec::{Security, Timestamp, UsernameToken};

// The nonce and the timestamp are inputs, not something generated here, so the
// caller can record them and a test can reproduce the digest.
let nonce = b"0123456789abcdef".to_vec();
let created = "2026-10-21T12:00:00Z";

let token = UsernameToken::digest("NNK", "secret", nonce, created);
let header = Security::new()
    .with_timestamp(&Timestamp::new(created, "2026-10-21T12:05:00Z"))
    .with_username_token(&token)
    .to_xml();

assert!(header.contains("wsse:UsernameToken"));
assert!(header.contains("PasswordDigest"));
assert!(header.contains("wsu:Timestamp"));
// The password itself is nowhere in the document.
assert!(!header.contains("secret"));
```

`soap-client` in this workspace builds and reads SOAP envelopes, and says plainly
that it does not implement WS-Security. This crate is that missing half. Almost
every bank, government and ERP endpoint requires a `Security` header, and without
one the request is rejected before the service looks at it. The header this crate
produces is a fragment: hand it to `soap_client::Envelope::with_header_xml` and
the two compose into a request an enterprise service will accept.

## Reading a header back

```rust
use wssec::{ParsedToken, parse_security};

let xml = r#"<wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
  <wsse:UsernameToken>
    <wsse:Username>scott</wsse:Username>
    <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">g01uA646Xg1x0SE/Xfa3CM/TLMI=</wsse:Password>
    <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">AAECAwQFBgcICQoLDA0ODw==</wsse:Nonce>
    <wsu:Created xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">2026-10-21T12:00:00+00:00</wsu:Created>
  </wsse:UsernameToken>
</wsse:Security>"#;

let tokens = parse_security(xml).unwrap();
let ParsedToken::Username(token) = &tokens[0] else { unreachable!() };
assert_eq!(token.username, "scott");
// Check a password against the digest without ever holding the password.
assert!(token.matches_digest("secret"));
assert!(!token.matches_digest("wrong"));
```

## The nonce is bytes, not the base64 text

The specification defines the nonce as a random byte sequence, base64 encoded for
transport only. The digest is taken over the raw bytes:

```text
digest = Base64( SHA1( nonce + created + password ) )
```

It is worth saying plainly because the most widely used Python client, `zeep`,
does not do this. It hands the base64 *text* to the digest, so its digest covers
24 bytes of ASCII instead of the 16 random bytes, and it base64 encodes the
result a second time in the element. A server written against the specification
rejects that. `digest_with_text_nonce` reproduces the other behaviour for the
rare server written to match it.

The same care applies to `PasswordText`, which the profile sends in clear text.
`zeep` writes it plainly and so does this crate; base64 would look plausible and
fail against every real server.

## What this crate does not do

It does not sign messages, does not carry X.509 certificates, and does not talk
to a hardware token. `Signature` and `EncryptedData` are the other half of
WS-Security and need a certificate story this crate deliberately leaves alone.
Nor does it generate a nonce or read a clock: a library that did would be
untestable, and would hide the timestamp the caller has to record anyway.

The digest helpers use SHA-1 and MD5 because the standards name them, and both
are broken against collisions. That does not matter here, where the digest
authenticates a shared secret to a server that already knows it and the input
includes a nonce the attacker cannot choose. It would matter for a server that
accepts a replayed token, which is a protocol concern: check the timestamp.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
