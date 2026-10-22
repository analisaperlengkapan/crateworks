//! Build an invoice-shaped PDF and write it to `invoice.pdf`.
//!
//! Run with `cargo run --example invoice`.

use pdfmake::{
    Align, Cell, Column, ColumnWidth, Content, Document, List, ListItem, Marker, PageOptions,
    Paragraph, Row, Rule, StandardFont, Table, TableBorders, TextStyle,
};

fn main() -> std::io::Result<()> {
    let heading = TextStyle::new(StandardFont::HelveticaBold, 18.0);
    let small = TextStyle::new(StandardFont::Helvetica, 9.0);
    let mono = TextStyle::new(StandardFont::Courier, 9.0);

    let items = [
        ("Widget, stainless", 3, 12_500i64),
        ("Bolt, 8mm", 40, 750),
        ("Cable, 2m", 5, 24_000),
        ("Café-grade filter paper", 2, 18_900),
    ];

    let rows: Vec<Row> = items
        .iter()
        .map(|(name, qty, price)| {
            Row::new(vec![
                Cell::text(*name),
                Cell::text(qty.to_string()).align(Align::Right),
                Cell::text(format!("{price}")).align(Align::Right),
                Cell::text(format!("{}", qty * price)).align(Align::Right),
            ])
        })
        .collect();

    let total: i64 = items.iter().map(|(_, q, p)| q * p).sum();

    let table = Table::new(
        vec![
            Column::new(ColumnWidth::Fraction(4.0)),
            Column::new(ColumnWidth::Fraction(1.0)).align(Align::Right),
            Column::new(ColumnWidth::Fraction(1.5)).align(Align::Right),
            Column::new(ColumnWidth::Fraction(1.5)).align(Align::Right),
        ],
        rows,
    )
    .header(Row::new(vec![
        Cell::text("Description"),
        Cell::text("Qty").align(Align::Right),
        Cell::text("Unit").align(Align::Right),
        Cell::text("Amount").align(Align::Right),
    ]))
    .borders(TableBorders::horizontal_rules())
    .cell_padding(5.0)
    .spacing_after(10.0);

    let totals = Table::new(
        vec![
            Column::new(ColumnWidth::Fraction(3.0)),
            Column::new(ColumnWidth::Fraction(1.0)).align(Align::Right),
        ],
        vec![
            Row::new(vec![Cell::text("Subtotal"), Cell::text(total.to_string())]),
            Row::new(vec![
                Cell::text("VAT 11%"),
                Cell::text((total * 11 / 100).to_string()),
            ])
            .background(pdfmake::Color::rgb(0xf2, 0xf2, 0xf2)),
            Row::new(vec![
                Cell::text("Total due").style(TextStyle::new(StandardFont::HelveticaBold, 11.0)),
                Cell::text((total + total * 11 / 100).to_string())
                    .style(TextStyle::new(StandardFont::HelveticaBold, 11.0)),
            ]),
        ],
    )
    .borders(TableBorders::horizontal_rules())
    .cell_padding(5.0);

    let notes = List::new(vec![
        ListItem::new(Content::Text(Paragraph::new(
            "Payment is due within 30 days.",
        ))),
        ListItem::new(Content::Text(Paragraph::new(
            "Quote the invoice number on the transfer.",
        ))),
    ])
    .marker(Marker::Decimal);

    let pdf = Document::new()
        .options(
            PageOptions::default()
                .margin(56.0)
                .header(
                    Paragraph::new("Cipansor Trading")
                        .style(TextStyle::new(StandardFont::HelveticaBold, 12.0)),
                )
                .footer(
                    Paragraph::new("Page {{page}} of {{pages}}")
                        .style(TextStyle::new(StandardFont::Helvetica, 8.0)),
                ),
        )
        .push(Paragraph::new("Invoice 2026-0042").style(heading))
        .push(Paragraph::new("Issued 22 October 2026").style(small))
        .push(Rule::new(pdfmake::Color::GREY, 0.5).spacing_after(10.0))
        .push(table)
        .push(totals)
        .push(Paragraph::new("Notes").style(TextStyle::new(StandardFont::HelveticaBold, 11.0)))
        .push(notes)
        .push(Paragraph::new("Account 1234-5678 · Cipansor, Indonesia").style(mono))
        .to_pdf();

    std::fs::write("invoice.pdf", &pdf)?;
    println!("wrote invoice.pdf, {} bytes", pdf.len());
    Ok(())
}
