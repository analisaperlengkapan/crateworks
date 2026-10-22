//! A long table that runs onto several pages, to show the header repeating.
//!
//! Run with `cargo run --example multipage`.

use pdfmake::{
    Align, Cell, Column, ColumnWidth, Document, PageOptions, Paragraph, Row, StandardFont, Table,
    TableBorders, TextStyle,
};

fn main() -> std::io::Result<()> {
    let rows: Vec<Row> = (1..=200)
        .map(|i| {
            Row::new(vec![
                Cell::text(format!("Item {i:03}")),
                Cell::text(format!("{}", i * 7 % 100)).align(Align::Right),
                Cell::text(format!("{}", i * 1250)).align(Align::Right),
            ])
        })
        .collect();

    let table = Table::new(
        vec![
            Column::new(ColumnWidth::Fraction(3.0)),
            Column::new(ColumnWidth::Fraction(1.0)).align(Align::Right),
            Column::new(ColumnWidth::Fraction(2.0)).align(Align::Right),
        ],
        rows,
    )
    .header(Row::new(vec![
        Cell::text("Item"),
        Cell::text("Qty").align(Align::Right),
        Cell::text("Price").align(Align::Right),
    ]))
    .borders(TableBorders::grid())
    .cell_padding(4.0);

    let pdf = Document::new()
        .options(
            PageOptions::default()
                .header(
                    Paragraph::new("Price list")
                        .style(TextStyle::new(StandardFont::HelveticaBold, 12.0)),
                )
                .footer(
                    Paragraph::new("Page {{page}} / {{pages}}")
                        .style(TextStyle::new(StandardFont::Helvetica, 8.0)),
                ),
        )
        .push(table)
        .to_pdf();

    std::fs::write("multipage.pdf", &pdf)?;
    println!("wrote multipage.pdf, {} bytes", pdf.len());
    Ok(())
}
