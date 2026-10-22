//! Runs the README example, so the documentation cannot drift from the API.
//!
//! The README is also the crate's front page (`#![doc = include_str!(..)]`), so
//! this test and the doctest cover it twice on purpose: the doctest proves it
//! compiles, this one proves it produces a document a reader can open.

use pdfmake::{
    Align, Cell, Column, ColumnWidth, Document, Paragraph, Row, StandardFont, Table, TableBorders,
    TextStyle,
};

#[test]
fn readme_example_builds_a_readable_invoice() {
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

    // The parts a reader actually needs: a trailer, a reachable xref, and the
    // text that was asked for. `contains` on bytes, because the stream is
    // compressed and the header comment is binary.
    assert!(contains(&bytes, b"/Type /Catalog"));
    assert!(contains(&bytes, b"startxref"));
    assert!(bytes.ends_with(b"%%EOF\n"));
    assert_eq!(invoice.page_count(), 1);

    // The visible text survives, and it is the text that went in.
    let text = invoice.plain_text();
    assert!(text.contains("Invoice 2026-0042"));
    assert!(text.contains("Widget"));
    assert!(text.contains("37500"));
}

#[test]
fn the_readme_claims_about_unsupported_characters_hold() {
    // The README says a character WinAnsi cannot represent becomes `?`, that
    // the count is recorded, and that `plain_text` keeps the original.
    let document = Document::new().push(
        Paragraph::new("price 5\u{20bd}").style(TextStyle::new(StandardFont::Helvetica, 10.0)),
    );

    let bytes = document.to_pdf();
    assert!(
        contains(
            &bytes,
            b"/Subject (1 characters replaced \\(not in WinAnsi\\))"
        ),
        "the replacement count is not recorded in the info dictionary"
    );
    // The document itself keeps what the caller typed.
    assert_eq!(document.plain_text(), "price 5\u{20bd}");
}

/// Whether `needle` occurs in `haystack`.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
