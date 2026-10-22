# pdfmake

[![crates.io](https://img.shields.io/crates/v/pdfmake.svg)](https://crates.io/crates/pdfmake)
[![docs.rs](https://docs.rs/pdfmake/badge.svg)](https://docs.rs/pdfmake)

Build paginated PDFs from a plain Rust data structure. Text wraps, tables repeat
their header on every page, and page numbers resolve after the last page is laid
out. No font files, no dependencies.

```rust
use pdfmake::{Align, Cell, Column, ColumnWidth, Document, Paragraph, Row, Table, TableBorders};

let invoice = Document::new()
    .push(Paragraph::new("Invoice 2026-0042"))
    .push(
        Table::new(
            vec![
                Column::new(ColumnWidth::Fraction(3.0)),
                Column::new(ColumnWidth::Fraction(1.0)).align(Align::Right),
            ],
            vec![Row::new(vec![Cell::text("Widget"), Cell::text("37500")])],
        )
        .header(Row::new(vec![Cell::text("Item"), Cell::text("Amount")]))
        .borders(TableBorders::horizontal_rules()),
    );

let bytes = invoice.to_pdf();
assert!(bytes.starts_with(b"%PDF-1.7"));
```

## What it is for

A generated invoice, a receipt, a report: a document whose shape is known at
compile time and whose numbers are known at run time. That document is data, so
building it should not mean concatenating PDF operators by hand, and it should
not mean pulling in a layout engine either.

The crate is the middle of that range. You describe the document as a value and
get bytes back.

## The standard fonts, and what they cost

The 14 fonts every PDF reader must provide are used without embedding them. That
is what makes a dependency-free PDF possible, and it is also the crate's main
limitation: text is written in WinAnsi, which covers the Latin script and the
punctuation around it, and nothing else.

A character WinAnsi cannot represent is replaced by `?`. The count of
replacements is recorded in the PDF's `/Info` dictionary, so a document that
lost characters says so, and [`Document::plain_text`] always returns the
original text.

If you need Arabic, Devanagari, or Chinese, this crate is the wrong tool: those
need an embedded font, which means a font parser, which is a different crate.

## What it does not do

It does not embed fonts, encrypt, sign, or draw images. It does not do
justified text, hyphenation, or floating figures. Each of those is a real
feature and none of them is hidden here; the API has no way to ask for them.

It also does not read PDFs. [`pdfa`](https://crates.io/crates/pdfa) in this
workspace does that, for the narrower job of wrapping an invoice in a PDF/A-3
container.

## Layout rules worth knowing

A table row is never split across pages, and the header row is redrawn at the
top of each page the table reaches. A paragraph, by contrast, breaks between
lines, because a paragraph that jumps to the next page leaves a hole.

A row of [`Columns`] is kept together, since splitting one column away from its
neighbours reads worse than starting the row lower down. A block taller than a
page is drawn anyway and overflows rather than looping forever.

## Compression

Content streams are compressed with a DEFLATE encoder written for this crate
(fixed Huffman, RFC 1951). It is not the smallest possible output; it is small,
and it is one less dependency.

Part of the [crateworks](https://github.com/cipansor/crateworks) workspace.
Licensed under the Apache License, Version 2.0, copyright Yayasan Pesantren
Cipansor and contributors.
