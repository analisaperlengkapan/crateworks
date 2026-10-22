# pdfa

[![crates.io](https://img.shields.io/crates/v/pdfa.svg)](https://crates.io/crates/pdfa)
[![docs.rs](https://docs.rs/pdfa/badge.svg)](https://docs.rs/pdfa)

PDF/A conformance, XMP metadata, and the Factur-X container for e-invoices.

`factur-x` in this workspace writes the EN 16931 invoice as XML. That XML is not
the invoice a tax authority accepts: it has to travel inside a PDF/A-3 document
as an attached file, with metadata that says so. This crate is that missing half.

```rust
use pdfa::{DocumentInfo, EmbeddedFile, Level, WriteOptions, Xmp, embed};

// Any ordinary PDF, produced by whatever the caller already uses.
let original = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\n\
xref\n0 3\n0000000000 65535 f \n0000000009 00000 n \n0000000063 00000 n \n\
trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n133\n%%EOF\n";

let xml = b"<rsm:CrossIndustryInvoice/>";
let info = DocumentInfo {
    title: "Invoice INV-2026-0001".into(),
    author: "Seller GmbH".into(),
    subject: "Invoice INV-2026-0001".into(),
    producer: "crateworks".into(),
    creator_tool: "crateworks".into(),
    timestamp: "2026-10-21T14:47:53+00:00".into(),
};
let options = WriteOptions {
    xmp: Xmp::factur_x(Level::En16931, info),
    timestamp: "2026-10-21T14:47:53+00:00".into(),
    compress: false,
};

let container = embed(original, &EmbeddedFile::factur_x(xml.to_vec()), &options).unwrap();

// And check what was built.
let report = pdfa::check_factur_x(&container, Some("factur-x.xml"));
assert!(report.is_valid(), "{:?}", report.findings);
assert_eq!(pdfa::embedded_file(&container).unwrap(), xml);
```

## What the checker proves, and what it does not

`check_factur_x` verifies the container contract:

- the metadata claims PDF/A-3,
- the catalog points at it with `/Metadata`,
- the attachment is present under the agreed name,
- it declares an `/AFRelationship`, as PDF/A-3 requires,
- its checksum matches its bytes.

That last one is the failure that actually happens: the XML is regenerated and
the checksum in the container is left at its old value. A validator rejects it
and a reader may not even notice.

It does not apply the full ISO 19005 rules. Device-independent colour, every font
embedded, no encryption, transparency restrictions: that is veraPDF's job, and a
small crate claiming it would be lying. The report says what was checked, so a
caller who reads "valid" knows exactly what that means.

## Why it rewrites the file instead of appending to it

The usual way to add something to a PDF is an incremental update, keeping every
original byte and appending. That is what a signing workflow needs, because a
signature covers the bytes it was made over.

This crate rewrites instead, because the new objects have to be renumbered either
way to be safe. An appended object number can collide with one the original
defined further down, and a PDF whose objects are numbered twice is corrupt in a
way that only shows up in whichever reader picks the other definition. Rewriting
gives one numbering and one cross-reference table. Page content, fonts and images
are copied through byte for byte, so the visual output cannot change.

## What it reads

A PDF may lay its objects out in three ways, and all three are handled. The
oldest is a plain `xref` table with a `trailer` dictionary. PDF 1.5 added the
cross-reference **stream** and the object **stream**, and newer tools use both by
default, so a reader that understands only the table cannot open a file from a
modern word processor at all. Both are decoded here, predictors included.

`FlateDecode` is decompressed in full. Real deflate uses fixed and dynamic
Huffman codes, and the attachment and metadata of a real invoice are compressed
that way, so the decompressor covers all three block types and the tests check it
against streams Python's `zlib` produced. Writing stays simple: the attachment
this crate adds is a zlib stream with *stored* blocks, which is valid, which any
reader inflates, and which costs only a few bytes per sixty-five thousand.

## What it does not do

It is not a renderer and not a general PDF library. It does not decrypt, and it
does not resolve an indirect `/Length`, which needs the cross-reference table to
be walked first. Page content, fonts and images are copied through byte for byte
and never interpreted. Incremental updates are followed through `/Prev`, so an
object redefined by a later revision is the one that wins.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
