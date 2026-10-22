# pades

[![crates.io](https://img.shields.io/crates/v/pades.svg)](https://crates.io/crates/pades)
[![docs.rs](https://docs.rs/pades/badge.svg)](https://docs.rs/pades)

Read a PAdES-signed PDF, check what each signature covers, and report how far its
long-term validation reaches.

`pdf_signer`, `underskrift` and `oxidize-pdf` all pull in RustCrypto and between
them carry 23, 35 and 45 dependencies, and none of them has a reverse dependency
on crates.io. Producing a signature is solved. Saying whether a document someone
else signed is still intact, and what exactly was verified, is not.

This crate is the verifier. It reads the document, and it is explicit about the
difference between "checked and correct" and "did not check".

```rust
// A PDF signed with pyHanko, read back to see what the signature covers.
let document = std::fs::read("tests/fixtures/signed_lt.pdf").expect("a signed PDF");
let report = pades::verify(&document)?;
let signature = &report.signatures[0];
assert_eq!(signature.integrity, pades::Integrity::Intact);
// The validation data was appended after signing, so the range stops at the
// end of the signature's own revision rather than at the end of the file.
assert_eq!(signature.coverage, pades::Coverage::EntireRevision);
assert_eq!(report.level, Some(pades::Level::B_LT));
# Ok::<(), pades::Error>(())
```

## What it proves, and what it does not

For each signature it reports:

- the field name and the `/SubFilter`, so a caller knows which profile was claimed,
- the digest algorithm, read from the CMS rather than from the dictionary,
- the `/ByteRange` and what that range covers,
- whether the covered bytes still hash to what the CMS says,
- the signer certificate's subject and validity window,
- whether an RFC 3161 timestamp is embedded, and what it says,
- how much revocation material the DSS carries,
- the highest baseline level reached, and every finding with a stable code.

It does not build a certification path. Full RFC 5280 validation, revocation
fetching and policy processing is a large subject on its own, and this crate is
network-free by design. It reports the chain and the material present, and the
caller decides trust by passing anchors.

## Why a verifier and not a signer

Producing a signature needs a private key, a keystore format, a timestamp client
and a certificate chain, and three crates already do it. Consuming one needs the
opposite set of things: a reader that is careful about what it did not check.
That is a smaller, more useful, and much less crowded problem.

## Coverage is not the same as integrity

A long-term-valid document is signed over a *revision*, not over the whole file,
and the validation data is appended afterwards on purpose. So a document where
bytes follow the signature is normal, and a verifier that demands the signature
cover the entire file rejects every B-LT document in existence. The report
distinguishes the three cases.

## Relationship to `pdfa`

`pdfa` reads the PDF itself: objects, the cross-reference chain, object streams.
`pades` depends on it for that and adds the signature half. The CMS inside
`/Contents` is DER, so this crate carries an ASN.1 reader, not an XML one.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
