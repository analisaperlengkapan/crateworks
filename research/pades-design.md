# `pades`: design for a PAdES verification crate

Status: **implemented** as `crates/pades`, verification only, RSA only. The crate
follows this document, and where the implementation had to choose, the choice is
noted here rather than left in the code. The three open questions at the end are
still open; the answers taken for now are stated there.

Written after probing real signed documents rather than from the specification
alone. Every structure quoted below was produced by `pyhanko` and read back with
`pdfa`.

## What was measured before designing

Two signed PDFs were produced with `pyhanko` (the oracle proposed in
`rust-crate-gaps-round8.md`) from a `reportlab` page, one plain PAdES B-B and one
B-LT with embedded validation data, and both were read with this workspace's
`pdfa`.

**`pdfa::container::Pdf::parse` reads a signed PDF correctly.** It returns the
signature object, `/ByteRange`, `/SubFilter` and the revision-3 catalog with
`/DSS`. That is the finding the whole design rests on: the PDF-side work is
already done and does not need to be written again.

**A real bug was found in `pdfa::xref::read`.** It fails on the B-LT document,
which is the primary input to this crate.

| File | revisions | `Pdf::parse` | `xref::read` |
|---|---|---|---|
| blank | 1 | OK | `Some(7)` |
| signed B-B | 2 | OK | `Some(10)` |
| signed B-LT | 3 | OK, via fallback scan | **`None`** |

Cause, confirmed by bisecting the file: `read_table` in `crates/pdfa/src/xref.rs`
does `std::str::from_utf8(bytes.get(offset..)?)` on **everything from the section
to the end of the file**. When a later revision appends a binary stream — the
`/Certs` stream a DSS carries — the bytes after the section offset are no longer
valid UTF-8 and the whole cross-reference read returns `None`.

Measured: on `signed_lt.pdf`, `bytes[6385..]` is **not** valid UTF-8, while
`bytes[963..]` and `bytes[8137..]` are. The section at 6385 is the middle
revision, so the `/Prev` walk dies there and never reaches the newest section.

The failure is currently masked because `Pdf::parse` falls back to scanning for
`N 0 obj` headers. That fallback is worse than it looks:

- Objects that live inside an object stream have no `N 0 obj` header at all, so
  the scan cannot see them and they are silently lost.
- The scan takes the last match for a number, so a byte sequence that happens to
  read `7 0 obj` inside a compressed stream can win over the real object 7.
- The trailer falls back to `rfind(b"trailer")`, which is wrong for a file whose
  newest section is a cross-reference *stream*.

This must be fixed in `pdfa` before `pades` is built, because reading a B-LT
document is the crate's main job. It is worth fixing on its own merits: any
modern producer that appends a binary stream breaks xref resolution today.

## What the oracle's labels turned out to say

The corpus generator (`research/tools/gen_pades_corpus.py`) records what `pyhanko`
reports, and one label changed the design. Measured:

| Case | `intact` | `coverage` |
|---|---|---|
| B-B, one signature, nothing after it | true | `ENTIRE_FILE` |
| B-LT, DSS appended after the signature | true | `ENTIRE_REVISION` |
| B-LT, a byte flipped inside a content stream | **false** | `ENTIRE_FILE` |
| a byte appended after the last revision | true | `ENTIRE_REVISION` |

Two things follow, and both are in the API below:

- **`Coverage::EntireFile` alone is not the test.** A B-LT document is signed
  over a revision, not over the whole file, and the DSS is *supposed* to be
  appended afterwards. A verifier that demands `EntireFile` rejects every
  long-term-valid document. The correct statement is "the range covers
  everything up to the end of the revision it signed, and what follows is only
  additions".
- **An appended byte does not break integrity**, and that is correct: the
  appended byte is outside the signed range. So `append_byte` is not a tampering
  test at all. The tampering case has to flip a byte *inside* the covered range,
  which is why `flip_byte` targets a content stream of the first revision.

Also worth recording: flipping a byte near the end of the revision lands in the
cross-reference table and makes the file unreadable (`PdfStrictReadError`), which
tests nothing about integrity. The byte has to go inside a stream payload.

## The structural decision: a new crate, not an extension

**Not `pdfa`.** Two reasons, and the second is decisive.

1. `pdfa::container::embed` **rewrites the whole document**. It starts a fresh
   buffer with `%PDF-1.7`, copies every object to a new offset and writes a new
   cross-reference table. PAdES requires the signed byte range to stay
   byte-identical, so signing must be an *incremental update*: append only, keep
   the original bytes verbatim, add a new cross-reference section. `pdfa` is a
   rewriter; PAdES needs an appender. They are opposite writers and cannot share
   one document model without making `pdfa` carry both.
2. `pdfa` already owns the PDF/A and Factur-X containers. Adding PAdES makes it
   "PDF/A + Factur-X + PAdES", which breaks the single-purpose rule this
   workspace is built on.

**Not `pdfmake`.** No parser and no crypto; it is a writer only.

**Not `ebics`.** It holds the crypto primitives this crate wants, but a crate
named `ebics` cannot credibly own PAdES.

## Correction to the round 8 note

Round 8 proposed `xmlkit` as a dependency. Probing shows that was **wrong**, and
the correction matters for the scope.

PAdES baseline (ETSI EN 319 142-1) contains no XML. The `/Contents` blob in the
document produced above is DER, and it starts:

```
30 82 05 08 06 09 2A 86 48 86 F7 0D 01 07 02 ...
SEQUENCE    OID 1.2.840.113549.1.7.2  (signedData)
```

The DSS that carries long-term validation material is a **PDF dictionary**
(`/Certs`, `/OCSPs`, `/CRLs`, `/VRI`), not XML. The one XML artefact in the
space, XMP, is already handled by `pdfa`.

So `pades` needs an **ASN.1 DER** reader, not an XML one. `xmlkit` is not a
dependency.

## Proposed dependency shape

```
pades  ->  pdfa          (reading objects, the xref, and object streams)
       ->  (nothing else)
```

One dependency, and it is inside this workspace. That satisfies the `AGENTS.md`
rule ("`xmlkit` is the one crate other crates may depend on") only if `pdfa` is
promoted to the same footing. That promotion is the real cost of this design and
is called out below as the decision it is.

### What the crypto costs

The incumbent crates pull RustCrypto and still have no adopters:

| Crate | dependencies |
|---|---|
| `pdf_signer` | 23 (`cms`, `der`, `rsa`, `p256`, `p384`, `ed25519-dalek`, `x509-cert`, `x509-ocsp`, `p12-keystore`, …) |
| `underskrift` | 35 |
| `oxidize-pdf` | 45 |

`ebics` in this workspace already hand-rolls `sha256.rs` (259 lines), `rsa.rs`
(658 lines, PKCS#1 v1.5 **and** PSS, 5 tests), `aes.rs`, `bignum.rs`, `rng.rs`
and `c14n.rs`, with no dependency at all. So the zero-dependency path is proven
here, not hypothetical.

Verification-only needs, and where each comes from:

| Need | Status |
|---|---|
| SHA-256 | in `ebics` |
| RSA PKCS#1 v1.5 and PSS verify | in `ebics` |
| DER reader | new, ~400 lines |
| CMS `SignedData` / `SignerInfo` | new, ~500 lines |
| X.509: subject, issuer, serial, validity, SPKI | new, ~300 lines |
| SHA-384 / SHA-512 | new, small |
| ECDSA P-256 / P-384 verify | **new, and the risk** |
| OCSP / CRL parsing | new, and deferred |

### Decision 1: RSA only in v0.1

Recommendation: **v0.1 verifies RSA signatures and reports ECDSA ones as
recognised but unverified**, with a `Finding` code that says so. Rationale: the
RSA half is already proven in `ebics`, so v0.1 carries no new cryptographic risk,
and a verifier that reports honestly on what it did not check is more useful than
one that fails silently. ECDSA is a follow-up, either hand-rolled or by allowing
`p256`/`p384` for that path alone.

### Decision 2: no path validation in v0.1

Full RFC 5280 path building, revocation and policy is a large project on its own
and is where `pyhanko_certvalidator` is heavy. Recommendation: v0.1 reports the
certificate chain, the validity window and the revocation material present, and
verifies that the signer certificate's key matches the signature. The caller
decides trust, passing anchors through `Options`. This keeps the crate
network-free, which the workspace requires.

## Proposed public API

```rust
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

pub fn verify(pdf: &[u8]) -> Result<Report>;
pub fn verify_with(pdf: &[u8], options: &Options) -> Result<Report>;

/// Knobs for [`verify_with`]. [`verify`] uses [`Options::default`].
pub struct Options {
    /// Anchors to test the signer chain against. Empty means report, decide nothing.
    pub anchors: Vec<Certificate>,
    /// Report a signature with no RFC 3161 timestamp as a finding.
    pub require_timestamp: bool,
}

pub struct Report {
    /// Every signature in the document, in the order they appear.
    pub signatures: Vec<Signature>,
    /// The lowest level every signature reaches, which is the document's level.
    pub level: Level,
}

pub struct Signature {
    /// The field name, `/T`.
    pub field: String,
    /// `/SubFilter`.
    pub kind: Kind,
    /// The digest algorithm the CMS names.
    pub digest: Digest,
    /// `/ByteRange`, as pairs of `start` and `len`.
    pub byte_range: Vec<Span>,
    /// What the range covers.
    pub coverage: Coverage,
    /// Whether the covered bytes still hash to what the CMS says.
    pub integrity: Integrity,
    /// The signer's certificate.
    pub signer: Option<Certificate>,
    /// The time the CMS claims the signature was made.
    pub claimed_time: Option<Instant>,
    /// The RFC 3161 timestamp, when one is embedded.
    pub timestamp: Option<Timestamp>,
    /// Long-term validation material found in the DSS.
    pub revocation: Revocation,
    /// The highest level this signature reaches.
    pub level: Level,
    /// Everything wrong or worth knowing, each with a stable code.
    pub findings: Vec<Finding>,
}

/// A half-open byte range, `[start, start + len)`.
pub struct Span { pub start: usize, pub len: usize }

pub enum Kind { CadesDetached, Pades, AdbePkcs7Detached, AdbePkcs7Sha1, Other(String) }
pub enum Digest { Sha1, Sha256, Sha384, Sha512, Other(String) }

pub enum Coverage {
    /// The range reaches the end of the file. Nothing follows the signature.
    EntireFile,
    /// The range ends at a revision boundary and bytes follow it. This is what a
    /// long-term-valid document looks like: the DSS is appended after signing,
    /// so this is normal, not a finding. `revision_end` is the offset the signed
    /// revision ends at, which is where the signature's own `%%EOF` sits.
    EntireRevision { revision_end: usize },
    /// The range stops before the end of the revision it signed, so part of the
    /// revision is not covered. This is a finding.
    Partial { uncovered: usize },
}

pub enum Integrity {
    /// The digest over the byte range matches the CMS.
    Intact,
    /// It does not, so the document changed after signing.
    Broken,
    /// The algorithm is one this version does not compute.
    NotChecked(&'static str),
}

pub enum Level { B_B, B_T, B_LT, B_LTA }

pub struct Revocation { pub certs: usize, pub ocsps: usize, pub crls: usize }

pub struct Timestamp { pub time: Instant, pub serial: Vec<u8>, pub authority: Option<String> }

/// A date and time in UTC, as PAdES writes it.
pub struct Instant { pub year: i32, pub month: u8, pub day: u8,
                     pub hour: u8, pub minute: u8, pub second: u8 }

/// One problem or note. The code is stable so a caller can match on it.
pub struct Finding { pub code: Code, pub message: String }

pub enum Code {
    /// `/ByteRange` is malformed: not four numbers, or overlapping, or out of bounds.
    ByteRangeMalformed,
    /// The document changed after this signature was made.
    ModifiedAfterSigning,
    /// The CMS could not be parsed.
    CmsMalformed,
    /// The signer certificate is outside its validity window at the claimed time.
    CertificateExpired,
    /// The signature algorithm is recognised but not verified by this version.
    AlgorithmNotVerified,
    /// There is no RFC 3161 timestamp.
    NoTimestamp,
    /// The DSS carries no revocation material for the chain.
    NoRevocationInfo,
}
```

Why the shape is what it is:

- **`Code` is an enum, not a string.** The differential harness must assert on
  *what* was found, not on English. This is the same reason the crates in this
  workspace carry typed errors.
- **`Integrity::NotChecked` is a first-class variant.** A verifier that cannot
  tell "verified" from "did not look" is dangerous; making the distinction a type
  is the whole point.
- **`Coverage` has three cases, not two.** An appended DSS is normal; a range
  that stops short of the end of its own revision is not. Folding both into
  "bytes follow" would report every long-term-valid document as suspicious.
- **No `chrono`.** `Instant` is a plain struct, as in `ebics::time`. The workspace
  forbids the dependency and dates here are parsed, not computed with.

## Level assignment

```
B_B   CMS parses, integrity is Intact, the signer certificate is present.
B_T   B_B, plus an RFC 3161 timestamp whose message imprint matches.
B_LT  B_T, plus revocation material for the chain in the DSS.
B_LTA B_LT, plus a document timestamp over the whole file.
```

Each step is a check the differential harness can drive from a `pyhanko`-produced
document, because `pyhanko` can produce all four levels.

## Verification plan

The method this workspace already uses, with `pyhanko` as the oracle:

1. **Corpus generation.** `research/tools/gen_pades_corpus.py` builds documents
   with `reportlab` + `pyhanko` at B-B, B-T, B-LT and B-LTA, plus the negative
   cases: a byte flipped inside the signed range, a byte appended after it, a
   truncated `/Contents`, a `/ByteRange` with the wrong length, and a
   self-signed-only chain.
2. **Labels from the oracle.** For each document, record what `pyhanko` reports:
   `intact`, `valid`, `trusted`, coverage, and the level. Commit the labels as a
   fixture, exactly as `stdnum_vectors.tsv` does for `stdnum-rs`.
3. **Assert on codes, not prose.** The harness compares `Code` sets and the level,
   so a wording change never fails a test and a real behaviour change always does.
4. **Round trip both ways.** Verify a document `pyhanko` signed, and have
   `pyhanko` verify a document this crate signs once signing lands.

## Work order

1. **Fix `pdfa::xref::read`.** The `from_utf8` on the tail is a bug on its own,
   it blocks this crate, and it is reproducible from the two fixtures above.
   Bound the decode to the section, not to the end of the file.
2. **DER reader and CMS `SignedData`.** The largest new piece.
3. **X.509 subset**, then integrity and `Code` reporting.
4. **Corpus and differential harness**, then `pyhanko` round trip.
5. Only then consider signing, ECDSA, and path validation.

## Open questions for the owner

1. **Is promoting `pdfa` to a foundation crate acceptable**, as `xmlkit` is? The
   alternative is extracting the object/xref reader into a `pdfcore` crate that
   both `pdfa` and `pades` depend on. That is cleaner layering and two crates to
   republish instead of one.
2. **RSA only in v0.1, with ECDSA reported as unverified?** Or allow `p256` and
   `p384` for the ECDSA path alone and accept two dependencies?
3. **Is verification-only the right first release**, with signing a later one?
   Round 8 argued yes because `pdf_signer` and `underskrift` already produce
   signatures and `pdfluent-sign` (235 downloads) is the only pure verifier.

## What the implementation chose, pending those answers

1. **No change to `pdfa`.** `pades` depends on it directly, and its public API
   was enough. Extracting `pdfcore` is still the cleaner layering and can be done
   later without changing `pades`'s own API.
2. **RSA only.** An ECDSA or EdDSA key is reported as `key-unsupported` with the
   key's own identifier, so a caller sees which key was not checked rather than a
   silent pass. Adding ECDSA is a new module behind the same report.
3. **Verification only.** Nothing in the crate signs, and the README says so.

Two things the implementation added that this document did not foresee:

- **`Integrity::NotChecked`** became load-bearing. A signature with no
  `messageDigest` attribute, or one whose CMS cannot be read, reports
  `NotChecked` and a finding, rather than being absent from the list. A caller
  that treats "no error" as "verified" is the failure mode this exists to
  prevent.
- **The DSS count is a count of DER values, not of streams.** `/Certs` is an
  array of streams, and each stream holds one or more certificates concatenated
  with no separator. Counting streams undercounts a real DSS.

A third thing was found by the tests rather than by design: pyHanko writes **no**
`signingTime` for a PAdES signature, because ETSI expects the time to come from
an RFC 3161 timestamp. A test that demanded the attribute would have been
asserting something the profile forbids, so the report carries it as optional and
the level comes from the timestamp instead.
