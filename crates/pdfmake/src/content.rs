//! The document model: the data a caller passes in, with no page geometry in it.
//!
//! Nothing here knows how tall a page is. The layout engine measures these
//! values against the page it is given, which is what lets the same content
//! reflow onto a different page size without changing the content.

use crate::style::{Align, Border, Color, TextStyle};

/// A block of content.
///
/// # Examples
///
/// ```
/// use pdfmake::{Content, Paragraph};
///
/// let content = Content::Text(Paragraph::new("Invoice"));
/// assert_eq!(content.plain_text(), "Invoice");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    /// A paragraph of wrapped text.
    Text(Paragraph),
    /// Side-by-side columns, laid out left to right.
    Columns(Columns),
    /// A bulleted or numbered list.
    List(List),
    /// A table, which may repeat its header across pages.
    Table(Table),
    /// Vertical space, in points.
    Spacer(f32),
    /// A horizontal rule.
    Rule(Rule),
    /// Start a new page here, unless already at the top of one.
    PageBreak,
}

impl Content {
    /// The text this block contains, with blocks separated by newlines.
    ///
    /// Useful in tests and in a search index. Structural values such as a
    /// spacer contribute nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Content, Paragraph};
    ///
    /// assert_eq!(Content::Spacer(12.0).plain_text(), "");
    /// assert_eq!(
    ///     Content::Text(Paragraph::new("a")).plain_text(),
    ///     "a"
    /// );
    /// ```
    pub fn plain_text(&self) -> String {
        match self {
            Content::Text(paragraph) => paragraph.text.clone(),
            Content::Columns(columns) => columns
                .columns
                .iter()
                .map(Content::plain_text)
                .collect::<Vec<_>>()
                .join("\n"),
            Content::List(list) => list
                .items
                .iter()
                .map(|item| item.content.plain_text())
                .collect::<Vec<_>>()
                .join("\n"),
            Content::Table(table) => {
                let mut lines = Vec::new();
                if let Some(header) = &table.header {
                    lines.push(header.plain_text());
                }
                for row in &table.rows {
                    lines.push(row.plain_text());
                }
                lines.join("\n")
            }
            Content::Spacer(_) | Content::Rule(_) | Content::PageBreak => String::new(),
        }
    }
}

/// A paragraph of text that wraps to the width it is given.
///
/// # Examples
///
/// ```
/// use pdfmake::{Align, Paragraph, StandardFont, TextStyle};
///
/// let paragraph = Paragraph::new("Total due")
///     .style(TextStyle::new(StandardFont::HelveticaBold, 12.0))
///     .align(Align::Right)
///     .spacing_after(6.0);
///
/// assert_eq!(paragraph.align, Align::Right);
/// assert_eq!(paragraph.text, "Total due");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph {
    /// The text, wrapped at whitespace when it does not fit.
    pub text: String,
    /// Font, size and colour.
    pub style: TextStyle,
    /// How the lines sit in their column.
    pub align: Align,
    /// Space to leave below the paragraph, in points.
    pub spacing_after: f32,
}

impl Paragraph {
    /// A left-aligned paragraph in the default style.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Paragraph;
    ///
    /// let paragraph = Paragraph::new("hello");
    /// assert_eq!(paragraph.text, "hello");
    /// ```
    pub fn new(text: impl Into<String>) -> Paragraph {
        Paragraph {
            text: text.into(),
            style: TextStyle::default(),
            align: Align::Left,
            spacing_after: 0.0,
        }
    }

    /// Use a particular font, size and colour.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Paragraph, StandardFont, TextStyle};
    ///
    /// let p = Paragraph::new("x").style(TextStyle::new(StandardFont::Courier, 8.0));
    /// assert_eq!(p.style.size, 8.0);
    /// ```
    pub fn style(mut self, style: TextStyle) -> Paragraph {
        self.style = style;
        self
    }

    /// Align the lines within the column.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Align, Paragraph};
    ///
    /// assert_eq!(Paragraph::new("x").align(Align::Center).align, Align::Center);
    /// ```
    pub fn align(mut self, align: Align) -> Paragraph {
        self.align = align;
        self
    }

    /// Leave space below the paragraph.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Paragraph;
    ///
    /// assert_eq!(Paragraph::new("x").spacing_after(4.0).spacing_after, 4.0);
    /// ```
    pub fn spacing_after(mut self, points: f32) -> Paragraph {
        self.spacing_after = points;
        self
    }
}

impl From<&str> for Content {
    fn from(text: &str) -> Content {
        Content::Text(Paragraph::new(text))
    }
}

impl From<String> for Content {
    fn from(text: String) -> Content {
        Content::Text(Paragraph::new(text))
    }
}

impl From<Paragraph> for Content {
    fn from(paragraph: Paragraph) -> Content {
        Content::Text(paragraph)
    }
}

impl From<Columns> for Content {
    fn from(columns: Columns) -> Content {
        Content::Columns(columns)
    }
}

impl From<List> for Content {
    fn from(list: List) -> Content {
        Content::List(list)
    }
}

impl From<Table> for Content {
    fn from(table: Table) -> Content {
        Content::Table(table)
    }
}

impl From<Rule> for Content {
    fn from(rule: Rule) -> Content {
        Content::Rule(rule)
    }
}

/// Side-by-side columns.
///
/// Each column is laid out independently at its own width. The block occupies
/// the height of its tallest column, so the columns are aligned at the top.
///
/// # Examples
///
/// ```
/// use pdfmake::{Columns, Content, Paragraph};
///
/// let row = Columns::new(vec![
///     Content::Text(Paragraph::new("left")),
///     Content::Text(Paragraph::new("right")),
/// ]);
/// assert_eq!(row.columns.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Columns {
    /// The content of each column, left to right.
    pub columns: Vec<Content>,
    /// Width of each column as a fraction of the available width. Empty means
    /// equal shares. Values are normalised, so `[1.0, 3.0]` is a quarter and
    /// three quarters.
    pub fractions: Vec<f32>,
    /// Space between columns, in points.
    pub gap: f32,
}

impl Columns {
    /// Equal-width columns with a 12 point gap.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Columns, Content, Paragraph};
    ///
    /// let row = Columns::new(vec![Paragraph::new("a").into(), Paragraph::new("b").into()]);
    /// assert_eq!(row.gap, 12.0);
    /// ```
    pub fn new(columns: Vec<Content>) -> Columns {
        Columns {
            columns,
            fractions: Vec::new(),
            gap: 12.0,
        }
    }

    /// Give the columns relative widths.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Columns, Content, Paragraph};
    ///
    /// let row = Columns::new(vec![Paragraph::new("a").into(), Paragraph::new("b").into()])
    ///     .fractions(vec![1.0, 3.0]);
    /// assert_eq!(row.fractions, vec![1.0, 3.0]);
    /// ```
    pub fn fractions(mut self, fractions: Vec<f32>) -> Columns {
        self.fractions = fractions;
        self
    }

    /// Set the space between columns.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Columns, Content, Paragraph};
    ///
    /// let row = Columns::new(vec![Paragraph::new("a").into()]).gap(0.0);
    /// assert_eq!(row.gap, 0.0);
    /// ```
    pub fn gap(mut self, gap: f32) -> Columns {
        self.gap = gap;
        self
    }
}

/// The marker drawn before each list item.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Marker {
    /// A round bullet.
    #[default]
    Bullet,
    /// `1.`, `2.`, `3.` and so on.
    Decimal,
    /// A fixed string, for example `-` or `*`.
    Custom(String),
}

impl Marker {
    /// The text drawn for item `index`, counting from zero.
    pub(crate) fn text(&self, index: usize) -> String {
        match self {
            Marker::Bullet => "\u{2022}".to_string(),
            Marker::Decimal => format!("{}.", index + 1),
            Marker::Custom(s) => s.clone(),
        }
    }
}

/// A list of items, each marked and indented.
///
/// # Examples
///
/// ```
/// use pdfmake::{Content, List, ListItem, Marker, Paragraph};
///
/// let list = List::new(vec![
///     ListItem::new(Content::Text(Paragraph::new("first"))),
///     ListItem::new(Content::Text(Paragraph::new("second"))),
/// ])
/// .marker(Marker::Decimal);
///
/// assert_eq!(list.items.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct List {
    /// The items, in order.
    pub items: Vec<ListItem>,
    /// What to draw before each item.
    pub marker: Marker,
    /// Space between the marker and the item text, in points.
    pub gap: f32,
    /// Space between items, in points.
    pub item_gap: f32,
}

impl List {
    /// A bulleted list.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Content, List, ListItem, Marker, Paragraph};
    ///
    /// let list = List::new(vec![ListItem::new(Content::Text(Paragraph::new("x")))]);
    /// assert_eq!(list.marker, Marker::Bullet);
    /// ```
    pub fn new(items: Vec<ListItem>) -> List {
        List {
            items,
            marker: Marker::Bullet,
            gap: 8.0,
            item_gap: 4.0,
        }
    }

    /// Choose the marker.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Content, List, ListItem, Marker, Paragraph};
    ///
    /// let list = List::new(vec![ListItem::new(Content::Text(Paragraph::new("x")))])
    ///     .marker(Marker::Custom("-".into()));
    /// assert_eq!(list.marker, Marker::Custom("-".into()));
    /// ```
    pub fn marker(mut self, marker: Marker) -> List {
        self.marker = marker;
        self
    }

    /// Set the space between the marker and the item text.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Content, List, ListItem, Paragraph};
    ///
    /// let list = List::new(vec![ListItem::new(Content::Text(Paragraph::new("x")))])
    ///     .gap(10.0);
    /// assert_eq!(list.gap, 10.0);
    /// ```
    pub fn gap(mut self, gap: f32) -> List {
        self.gap = gap;
        self
    }

    /// Set the space between items.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Content, List, ListItem, Paragraph};
    ///
    /// let list = List::new(vec![ListItem::new(Content::Text(Paragraph::new("x")))])
    ///     .item_gap(2.0);
    /// assert_eq!(list.item_gap, 2.0);
    /// ```
    pub fn item_gap(mut self, gap: f32) -> List {
        self.item_gap = gap;
        self
    }
}

/// One entry in a [`List`].
///
/// # Examples
///
/// ```
/// use pdfmake::{Content, ListItem, Paragraph};
///
/// let item = ListItem::new(Content::Text(Paragraph::new("an entry")));
/// assert_eq!(item.content.plain_text(), "an entry");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// The item's content.
    pub content: Content,
    /// Extra indent for this item, in points. Useful for nesting.
    pub indent: f32,
}

impl ListItem {
    /// An item at the list's own indent.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Content, ListItem, Paragraph};
    ///
    /// assert_eq!(ListItem::new(Paragraph::new("x").into()).indent, 0.0);
    /// ```
    pub fn new(content: Content) -> ListItem {
        ListItem {
            content,
            indent: 0.0,
        }
    }

    /// Indent this item further.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Content, ListItem, Paragraph};
    ///
    /// let item = ListItem::new(Paragraph::new("x").into()).indent(16.0);
    /// assert_eq!(item.indent, 16.0);
    /// ```
    pub fn indent(mut self, points: f32) -> ListItem {
        self.indent = points;
        self
    }
}

/// A horizontal rule.
///
/// # Examples
///
/// ```
/// use pdfmake::{Color, Rule};
///
/// let rule = Rule::new(Color::GREY, 0.5);
/// assert_eq!(rule.thickness, 0.5);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// Colour of the line.
    pub color: Color,
    /// Thickness in points.
    pub thickness: f32,
    /// Space to leave below, in points.
    pub spacing_after: f32,
}

impl Rule {
    /// A rule with no space below it.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Color, Rule};
    ///
    /// assert_eq!(Rule::new(Color::BLACK, 1.0).spacing_after, 0.0);
    /// ```
    pub fn new(color: Color, thickness: f32) -> Rule {
        Rule {
            color,
            thickness,
            spacing_after: 0.0,
        }
    }

    /// Leave space below the rule.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Color, Rule};
    ///
    /// let rule = Rule::new(Color::BLACK, 1.0).spacing_after(8.0);
    /// assert_eq!(rule.spacing_after, 8.0);
    /// ```
    pub fn spacing_after(mut self, points: f32) -> Rule {
        self.spacing_after = points;
        self
    }
}

/// How wide a table column is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnWidth {
    /// A fixed width in points.
    Fixed(f32),
    /// A share of the width left after the fixed columns are subtracted.
    Fraction(f32),
}

impl ColumnWidth {
    /// Resolve the column widths against the width a table has been given.
    ///
    /// Fixed columns take their width first. Whatever is left is shared out by
    /// fraction, so a table of all-fraction columns fills its width exactly.
    /// When the fixed columns alone exceed the width, the fractions get the
    /// smallest positive share rather than a negative one.
    pub(crate) fn resolve(widths: &[ColumnWidth], available: f32) -> Vec<f32> {
        let fixed: f32 = widths
            .iter()
            .map(|w| match w {
                ColumnWidth::Fixed(points) => *points,
                ColumnWidth::Fraction(_) => 0.0,
            })
            .sum();
        let total_fraction: f32 = widths
            .iter()
            .map(|w| match w {
                ColumnWidth::Fraction(share) => *share,
                ColumnWidth::Fixed(_) => 0.0,
            })
            .sum();
        let leftover = (available - fixed).max(0.0);

        widths
            .iter()
            .map(|w| match w {
                ColumnWidth::Fixed(points) => *points,
                ColumnWidth::Fraction(share) => {
                    if total_fraction <= 0.0 {
                        0.0
                    } else {
                        leftover * share / total_fraction
                    }
                }
            })
            .collect()
    }
}

/// One cell of a table.
///
/// # Examples
///
/// ```
/// use pdfmake::{Cell, Paragraph};
///
/// let cell = Cell::text("Quantity");
/// assert_eq!(cell.blocks.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The cell's content, stacked vertically.
    pub blocks: Vec<Content>,
    /// Override the text style for this cell.
    pub style: Option<TextStyle>,
    /// Override the horizontal alignment for this cell.
    pub align: Option<Align>,
    /// How many columns this cell spans. At least 1.
    pub colspan: usize,
}

impl Cell {
    /// An empty cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Cell;
    ///
    /// assert!(Cell::empty().blocks.is_empty());
    /// ```
    pub fn empty() -> Cell {
        Cell {
            blocks: Vec::new(),
            style: None,
            align: None,
            colspan: 1,
        }
    }

    /// A cell holding a single paragraph.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Cell;
    ///
    /// let cell = Cell::text("Rp 1.250.000");
    /// assert_eq!(cell.blocks[0].plain_text(), "Rp 1.250.000");
    /// ```
    pub fn text(text: impl Into<String>) -> Cell {
        Cell {
            blocks: vec![Content::Text(Paragraph::new(text))],
            style: None,
            align: None,
            colspan: 1,
        }
    }

    /// A cell holding arbitrary blocks.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Cell, Content, Paragraph};
    ///
    /// let cell = Cell::blocks(vec![
    ///     Paragraph::new("Total").into(),
    ///     Paragraph::new("Rp 1.250.000").into(),
    /// ]);
    /// assert_eq!(cell.blocks.len(), 2);
    /// ```
    pub fn blocks(blocks: Vec<Content>) -> Cell {
        Cell {
            blocks,
            style: None,
            align: None,
            colspan: 1,
        }
    }

    /// Set the text style for this cell's paragraphs.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Cell, StandardFont, TextStyle};
    ///
    /// let cell = Cell::text("x").style(TextStyle::new(StandardFont::Courier, 8.0));
    /// assert_eq!(cell.style.unwrap().size, 8.0);
    /// ```
    pub fn style(mut self, style: TextStyle) -> Cell {
        self.style = Some(style);
        self
    }

    /// Set the alignment for this cell's paragraphs.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Align, Cell};
    ///
    /// assert_eq!(Cell::text("x").align(Align::Right).align, Some(Align::Right));
    /// ```
    pub fn align(mut self, align: Align) -> Cell {
        self.align = Some(align);
        self
    }

    /// Span several columns.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Cell;
    ///
    /// assert_eq!(Cell::text("x").colspan(3).colspan, 3);
    /// ```
    pub fn colspan(mut self, columns: usize) -> Cell {
        self.colspan = columns.max(1);
        self
    }
}

/// One row of a table.
///
/// # Examples
///
/// ```
/// use pdfmake::{Cell, Row};
///
/// let row = Row::new(vec![Cell::text("a"), Cell::text("b")]);
/// assert_eq!(row.cells.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The cells, left to right.
    pub cells: Vec<Cell>,
    /// Fill colour behind the row.
    pub background: Option<Color>,
}

impl Row {
    /// A row with no background.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Cell, Row};
    ///
    /// assert!(Row::new(vec![Cell::text("a")]).background.is_none());
    /// ```
    pub fn new(cells: Vec<Cell>) -> Row {
        Row {
            cells,
            background: None,
        }
    }

    /// Fill the row with a colour.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Cell, Color, Row};
    ///
    /// let row = Row::new(vec![Cell::text("a")]).background(Color::rgb(0xf0, 0xf0, 0xf0));
    /// assert!(row.background.is_some());
    /// ```
    pub fn background(mut self, color: Color) -> Row {
        self.background = Some(color);
        self
    }

    /// The row's text, with cells separated by tabs.
    pub(crate) fn plain_text(&self) -> String {
        self.cells
            .iter()
            .map(|cell| {
                cell.blocks
                    .iter()
                    .map(Content::plain_text)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\t")
    }
}

/// The lines drawn around and between table cells.
///
/// # Examples
///
/// ```
/// use pdfmake::TableBorders;
///
/// let borders = TableBorders::none();
/// assert!(!borders.horizontal.is_visible());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TableBorders {
    /// Lines between rows.
    pub horizontal: Border,
    /// Lines between columns.
    pub vertical: Border,
    /// The line around the outside of the table.
    pub outline: Border,
    /// The line under the header row, which is often darker than the others.
    pub header_rule: Border,
}

impl TableBorders {
    /// No lines at all.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::TableBorders;
    ///
    /// assert!(!TableBorders::none().outline.is_visible());
    /// ```
    pub const fn none() -> TableBorders {
        TableBorders {
            horizontal: Border::none(),
            vertical: Border::none(),
            outline: Border::none(),
            header_rule: Border::none(),
        }
    }

    /// A light grid between all cells.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::TableBorders;
    ///
    /// assert!(TableBorders::grid().horizontal.is_visible());
    /// ```
    pub const fn grid() -> TableBorders {
        TableBorders {
            horizontal: Border::new(Color::GREY, 0.5),
            vertical: Border::new(Color::GREY, 0.5),
            outline: Border::new(Color::GREY, 0.5),
            header_rule: Border::new(Color::BLACK, 0.75),
        }
    }

    /// A horizontal rule under the header and below the last row.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::TableBorders;
    ///
    /// let borders = TableBorders::horizontal_rules();
    /// assert!(!borders.vertical.is_visible());
    /// assert!(borders.header_rule.is_visible());
    /// ```
    pub const fn horizontal_rules() -> TableBorders {
        TableBorders {
            horizontal: Border::none(),
            vertical: Border::none(),
            outline: Border::none(),
            header_rule: Border::new(Color::BLACK, 0.75),
        }
    }
}

/// A table.
///
/// # Examples
///
/// ```
/// use pdfmake::{Cell, Column, ColumnWidth, Row, Table, TableBorders};
///
/// let table = Table::new(
///     vec![
///         Column::new(ColumnWidth::Fraction(3.0)),
///         Column::new(ColumnWidth::Fraction(1.0)),
///     ],
///     vec![Row::new(vec![Cell::text("Item"), Cell::text("Qty")])],
/// )
/// .header(Row::new(vec![Cell::text("Item"), Cell::text("Qty")]))
/// .borders(TableBorders::horizontal_rules());
///
/// assert!(table.header.is_some());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// The columns, which fix the widths of every row.
    pub columns: Vec<Column>,
    /// Rows drawn on every page the table continues onto.
    pub header: Option<Row>,
    /// The body rows.
    pub rows: Vec<Row>,
    /// Which lines to draw.
    pub borders: TableBorders,
    /// Space inside each cell, in points.
    pub cell_padding: f32,
    /// Space to leave below the table, in points.
    pub spacing_after: f32,
}

impl Table {
    /// A table with 4 points of cell padding and no borders.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Cell, Column, ColumnWidth, Row, Table};
    ///
    /// let table = Table::new(
    ///     vec![Column::new(ColumnWidth::Fraction(1.0))],
    ///     vec![Row::new(vec![Cell::text("x")])],
    /// );
    /// assert_eq!(table.cell_padding, 4.0);
    /// ```
    pub fn new(columns: Vec<Column>, rows: Vec<Row>) -> Table {
        Table {
            columns,
            header: None,
            rows,
            borders: TableBorders::none(),
            cell_padding: 4.0,
            spacing_after: 0.0,
        }
    }

    /// Draw `header` at the top of every page the table reaches.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Cell, Column, ColumnWidth, Row, Table};
    ///
    /// let header = Row::new(vec![Cell::text("Item")]);
    /// let table = Table::new(vec![Column::new(ColumnWidth::Fraction(1.0))], Vec::new())
    ///     .header(header);
    /// assert!(table.header.is_some());
    /// ```
    pub fn header(mut self, header: Row) -> Table {
        self.header = Some(header);
        self
    }

    /// Choose which lines to draw.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Cell, Column, ColumnWidth, Row, Table, TableBorders};
    ///
    /// let table = Table::new(
    ///     vec![Column::new(ColumnWidth::Fraction(1.0))],
    ///     vec![Row::new(vec![Cell::text("x")])],
    /// )
    /// .borders(TableBorders::grid());
    /// assert!(table.borders.vertical.is_visible());
    /// ```
    pub fn borders(mut self, borders: TableBorders) -> Table {
        self.borders = borders;
        self
    }

    /// Set the space inside each cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Column, ColumnWidth, Table};
    ///
    /// let table = Table::new(vec![Column::new(ColumnWidth::Fraction(1.0))], Vec::new())
    ///     .cell_padding(2.0);
    /// assert_eq!(table.cell_padding, 2.0);
    /// ```
    pub fn cell_padding(mut self, points: f32) -> Table {
        self.cell_padding = points;
        self
    }

    /// Leave space below the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Column, ColumnWidth, Table};
    ///
    /// let table = Table::new(vec![Column::new(ColumnWidth::Fraction(1.0))], Vec::new())
    ///     .spacing_after(10.0);
    /// assert_eq!(table.spacing_after, 10.0);
    /// ```
    pub fn spacing_after(mut self, points: f32) -> Table {
        self.spacing_after = points;
        self
    }
}

/// One column of a [`Table`].
///
/// # Examples
///
/// ```
/// use pdfmake::{Align, Column, ColumnWidth};
///
/// let column = Column::new(ColumnWidth::Fixed(60.0)).align(Align::Right);
/// assert_eq!(column.align, Align::Right);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Column {
    /// How wide the column is.
    pub width: ColumnWidth,
    /// Default alignment for cells in this column.
    pub align: Align,
}

impl Column {
    /// A left-aligned column.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Align, Column, ColumnWidth};
    ///
    /// assert_eq!(Column::new(ColumnWidth::Fixed(50.0)).align, Align::Left);
    /// ```
    pub fn new(width: ColumnWidth) -> Column {
        Column {
            width,
            align: Align::Left,
        }
    }

    /// Set the default alignment for this column.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Align, Column, ColumnWidth};
    ///
    /// let column = Column::new(ColumnWidth::Fraction(1.0)).align(Align::Right);
    /// assert_eq!(column.align, Align::Right);
    /// ```
    pub fn align(mut self, align: Align) -> Column {
        self.align = align;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractions_are_shared_out_after_fixed_columns() {
        let widths = ColumnWidth::resolve(
            &[
                ColumnWidth::Fixed(100.0),
                ColumnWidth::Fraction(1.0),
                ColumnWidth::Fraction(3.0),
            ],
            500.0,
        );
        assert_eq!(widths, vec![100.0, 100.0, 300.0]);
    }

    #[test]
    fn fraction_columns_fill_the_width_exactly() {
        let widths = ColumnWidth::resolve(
            &[ColumnWidth::Fraction(1.0), ColumnWidth::Fraction(1.0)],
            200.0,
        );
        assert!((widths.iter().sum::<f32>() - 200.0).abs() < 0.001);
    }

    #[test]
    fn oversized_fixed_columns_do_not_produce_negative_widths() {
        let widths = ColumnWidth::resolve(
            &[ColumnWidth::Fixed(400.0), ColumnWidth::Fraction(1.0)],
            100.0,
        );
        assert_eq!(widths[1], 0.0);
    }

    #[test]
    fn markers_render_by_position() {
        assert_eq!(Marker::Bullet.text(0), "\u{2022}");
        assert_eq!(Marker::Decimal.text(0), "1.");
        assert_eq!(Marker::Decimal.text(9), "10.");
        assert_eq!(Marker::Custom("-".into()).text(3), "-");
    }

    #[test]
    fn plain_text_walks_the_whole_tree() {
        let content = Content::Columns(Columns::new(vec![
            Paragraph::new("left").into(),
            Content::List(List::new(vec![
                ListItem::new(Paragraph::new("one").into()),
                ListItem::new(Paragraph::new("two").into()),
            ])),
        ]));
        assert_eq!(content.plain_text(), "left\none\ntwo");
    }

    #[test]
    fn plain_text_of_a_table_includes_header_and_rows() {
        let table = Table::new(
            vec![Column::new(ColumnWidth::Fraction(1.0))],
            vec![Row::new(vec![Cell::text("body")])],
        )
        .header(Row::new(vec![Cell::text("head")]));
        assert_eq!(Content::Table(table).plain_text(), "head\nbody");
    }

    #[test]
    fn a_str_becomes_a_paragraph() {
        let content: Content = "hello".into();
        assert_eq!(content.plain_text(), "hello");
    }
}
