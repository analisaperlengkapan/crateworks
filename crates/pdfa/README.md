# pdfa

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

## What it refuses

A PDF whose objects live in object streams, or that uses a cross-reference stream
rather than a table, is refused with `Error::UnsupportedPdf` rather than mangled.
Both are legal and both are common in output from newer tools. Handling them
means parsing compressed object containers; rewriting a file the parser does not
fully understand would risk corrupting it silently, which is worse than saying no.

The same honesty applies to `FlateDecode`. This crate writes a zlib stream with
*stored* blocks, which is valid and which any reader inflates, rather than
shipping a whole deflate implementation. When the checker meets a stream
compressed with real deflate it reports that the checksum could not be checked,
rather than reporting a mismatch it cannot prove.

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
