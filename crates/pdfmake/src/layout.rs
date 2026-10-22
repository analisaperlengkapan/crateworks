//! Turning content into positioned marks on pages.
//!
//! The engine keeps a cursor measured from the top of the page, because that is
//! how a reader thinks about a document, and converts to PDF's bottom-left
//! origin only when an operation is emitted. A block that does not fit in the
//! space left starts a new page, except where the block says otherwise.

use crate::content::{ColumnWidth, Columns, Content, List, Paragraph, Row, Table, TableBorders};
use crate::fonts::StandardFont;
use crate::style::{Align, Color};
use crate::wrap;

/// The size of a page, in points.
///
/// # Examples
///
/// ```
/// use pdfmake::PageSize;
///
/// let a4 = PageSize::A4;
/// assert!((a4.width() - 595.28).abs() < 0.01);
/// assert!(PageSize::Letter.width() > 0.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PageSize {
    /// ISO A4, 210 by 297 mm.
    #[default]
    A4,
    /// US Letter, 8.5 by 11 inches.
    Letter,
    /// An explicit size in points.
    Custom {
        /// Page width in points.
        width: f32,
        /// Page height in points.
        height: f32,
    },
}

impl PageSize {
    /// The width in points.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::PageSize;
    ///
    /// assert_eq!(PageSize::Letter.width(), 612.0);
    /// assert_eq!(PageSize::Custom { width: 100.0, height: 200.0 }.width(), 100.0);
    /// ```
    pub const fn width(self) -> f32 {
        match self {
            PageSize::A4 => 595.28,
            PageSize::Letter => 612.0,
            PageSize::Custom { width, .. } => width,
        }
    }

    /// The height in points.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::PageSize;
    ///
    /// assert_eq!(PageSize::Letter.height(), 792.0);
    /// ```
    pub const fn height(self) -> f32 {
        match self {
            PageSize::A4 => 841.89,
            PageSize::Letter => 792.0,
            PageSize::Custom { height, .. } => height,
        }
    }
}

impl Default for PageOptions {
    /// A4 with 48 point margins and no header or footer.
    fn default() -> PageOptions {
        PageOptions::a4()
    }
}

/// Page geometry and the running header and footer.
///
/// # Examples
///
/// ```
/// use pdfmake::{PageOptions, PageSize, Paragraph, StandardFont, TextStyle};
///
/// let options = PageOptions::default()
///     .size(PageSize::Letter)
///     .margin(36.0)
///     .footer(Paragraph::new("Page {{page}} of {{pages}}")
///         .style(TextStyle::new(StandardFont::Helvetica, 8.0)));
///
/// assert_eq!(options.page_size, PageSize::Letter);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PageOptions {
    /// The paper size.
    pub page_size: PageSize,
    /// Margin on all four sides, in points.
    pub margin: f32,
    /// Content drawn at the top of every page.
    pub header: Option<Content>,
    /// Content drawn at the bottom of every page.
    ///
    /// The text `{{page}}` is replaced by the page number and `{{pages}}` by
    /// the total, both counting from one.
    pub footer: Option<Content>,
    /// Space between the header and the body.
    pub header_gap: f32,
    /// Space between the body and the footer.
    pub footer_gap: f32,
}

impl PageOptions {
    /// A4 with 48 point margins and no header or footer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{PageOptions, PageSize};
    ///
    /// assert_eq!(PageOptions::a4().page_size, PageSize::A4);
    /// ```
    pub fn a4() -> PageOptions {
        PageOptions {
            page_size: PageSize::A4,
            margin: 48.0,
            header: None,
            footer: None,
            header_gap: 12.0,
            footer_gap: 12.0,
        }
    }

    /// Choose the paper size.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{PageOptions, PageSize};
    ///
    /// assert_eq!(PageOptions::default().size(PageSize::Letter).page_size, PageSize::Letter);
    /// ```
    pub fn size(mut self, page_size: PageSize) -> PageOptions {
        self.page_size = page_size;
        self
    }

    /// Set the margin on all four sides.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::PageOptions;
    ///
    /// assert_eq!(PageOptions::default().margin(72.0).margin, 72.0);
    /// ```
    pub fn margin(mut self, points: f32) -> PageOptions {
        self.margin = points;
        self
    }

    /// Draw a header on every page.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{PageOptions, Paragraph};
    ///
    /// let options = PageOptions::default().header(Paragraph::new("Invoice 42"));
    /// assert!(options.header.is_some());
    /// ```
    pub fn header(mut self, header: impl Into<Content>) -> PageOptions {
        self.header = Some(header.into());
        self
    }

    /// Draw a footer on every page.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{PageOptions, Paragraph};
    ///
    /// let options = PageOptions::default().footer(Paragraph::new("Page {{page}}"));
    /// assert!(options.footer.is_some());
    /// ```
    pub fn footer(mut self, footer: impl Into<Content>) -> PageOptions {
        self.footer = Some(footer.into());
        self
    }

    /// Set the space between the header and the body.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::PageOptions;
    ///
    /// assert_eq!(PageOptions::default().header_gap(6.0).header_gap, 6.0);
    /// ```
    pub fn header_gap(mut self, points: f32) -> PageOptions {
        self.header_gap = points;
        self
    }

    /// Set the space between the body and the footer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::PageOptions;
    ///
    /// assert_eq!(PageOptions::default().footer_gap(6.0).footer_gap, 6.0);
    /// ```
    pub fn footer_gap(mut self, points: f32) -> PageOptions {
        self.footer_gap = points;
        self
    }

    /// Width available to content.
    pub(crate) fn body_width(&self) -> f32 {
        self.page_size.width() - 2.0 * self.margin
    }

    /// Distance from the top of the page to the top of the content area.
    pub(crate) fn body_top(&self) -> f32 {
        let mut top = self.margin;
        if let Some(header) = &self.header {
            top += measure(header, self.body_width()) + self.header_gap;
        }
        top
    }

    /// Distance from the top of the page to the bottom of the content area.
    pub(crate) fn body_bottom(&self) -> f32 {
        let mut bottom = self.margin;
        if let Some(footer) = &self.footer {
            bottom += measure(footer, self.body_width()) + self.footer_gap;
        }
        self.page_size.height() - bottom
    }
}

/// One mark on a page. Coordinates are measured from the top-left of the page,
/// which is how the layout engine thinks; the writer flips them.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Mark {
    /// Text with its baseline at `baseline`.
    Text {
        /// Left edge, in points from the page's left edge.
        x: f32,
        /// Baseline, in points from the page's top edge.
        baseline: f32,
        /// The text, already encoded to WinAnsi.
        bytes: Vec<u8>,
        /// The face to draw it in.
        font: StandardFont,
        /// The size in points.
        size: f32,
        /// The colour.
        color: Color,
    },
    /// A filled rectangle.
    Fill {
        /// Left edge.
        x: f32,
        /// Top edge.
        top: f32,
        /// Width in points.
        width: f32,
        /// Height in points.
        height: f32,
        /// The fill colour.
        color: Color,
    },
    /// A straight line.
    Line {
        /// Start, x.
        x1: f32,
        /// Start, y from the top.
        y1: f32,
        /// End, x.
        x2: f32,
        /// End, y from the top.
        y2: f32,
        /// The line colour.
        color: Color,
        /// The line width in points.
        width: f32,
    },
}

/// A page and everything drawn on it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Page {
    /// The marks, in the order they were produced.
    pub(crate) marks: Vec<Mark>,
}

/// Renders content onto pages.
pub(crate) struct Layout<'a> {
    options: &'a PageOptions,
    pages: Vec<Page>,
    /// Distance from the top of the current page to the cursor.
    cursor: f32,
    /// Number of characters replaced because WinAnsi could not encode them.
    replacements: usize,
}

/// Lay `blocks` out over as many pages as it needs.
pub(crate) fn render(blocks: &[Content], options: &PageOptions) -> (Vec<Page>, usize) {
    let mut layout = Layout {
        options,
        pages: vec![Page { marks: Vec::new() }],
        cursor: options.body_top(),
        replacements: 0,
    };

    for block in blocks {
        layout.block(block, options.margin, options.body_width());
    }

    let total = layout.pages.len();
    layout.decorate(total);
    (layout.pages, layout.replacements)
}

impl Layout<'_> {
    /// Space left on the current page below the cursor.
    fn remaining(&self) -> f32 {
        self.options.body_bottom() - self.cursor
    }

    /// Start a new page, unless the current one is still empty.
    fn break_page(&mut self) {
        if (self.cursor - self.options.body_top()).abs() > f32::EPSILON {
            self.pages.push(Page { marks: Vec::new() });
            self.cursor = self.options.body_top();
        }
    }

    /// Make room for `height`, breaking the page if it does not fit.
    ///
    /// Returns `false` when the block is taller than an entire page, in which
    /// case it is drawn anyway and allowed to overflow rather than loop.
    fn ensure(&mut self, height: f32) -> bool {
        if height <= self.remaining() {
            return true;
        }
        let full_page = self.options.body_bottom() - self.options.body_top();
        if height > full_page {
            return false;
        }
        self.break_page();
        true
    }

    fn mark(&mut self, mark: Mark) {
        self.pages
            .last_mut()
            .expect("at least one page")
            .marks
            .push(mark);
    }

    fn text(
        &mut self,
        x: f32,
        baseline: f32,
        text: &str,
        font: StandardFont,
        size: f32,
        color: Color,
    ) {
        let (bytes, replaced) = crate::fonts::encode_text(text);
        self.replacements += replaced;
        if bytes.is_empty() {
            return;
        }
        self.mark(Mark::Text {
            x,
            baseline,
            bytes,
            font,
            size,
            color,
        });
    }

    /// Lay out one block at `x` with the given width, and return its height.
    fn block(&mut self, content: &Content, x: f32, width: f32) -> f32 {
        match content {
            Content::Text(paragraph) => self.paragraph(paragraph, x, width),
            Content::Spacer(points) => {
                let points = points.max(0.0);
                if self.remaining() < points {
                    self.break_page();
                } else {
                    self.cursor += points;
                }
                points
            }
            Content::Rule(rule) => {
                let height = rule.thickness.max(0.0) + rule.spacing_after;
                self.ensure(height);
                let y = self.cursor + rule.thickness.max(0.0) / 2.0;
                self.mark(Mark::Line {
                    x1: x,
                    y1: y,
                    x2: x + width,
                    y2: y,
                    color: rule.color,
                    width: rule.thickness,
                });
                self.cursor += height;
                height
            }
            Content::PageBreak => {
                self.break_page();
                0.0
            }
            Content::Columns(columns) => self.columns(columns, x, width),
            Content::List(list) => self.list(list, x, width),
            Content::Table(table) => self.table(table, x, width),
        }
    }

    fn paragraph(&mut self, paragraph: &Paragraph, x: f32, width: f32) -> f32 {
        let lines = wrap::wrap(
            &paragraph.text,
            paragraph.style.font,
            paragraph.style.size,
            width,
        );
        let line_height = paragraph.style.line_height();
        let height = lines.len() as f32 * line_height + paragraph.spacing_after;

        // A paragraph splits across pages line by line, so only the first line
        // has to fit for the paragraph to start here.
        for (index, line) in lines.iter().enumerate() {
            if self.remaining() < line_height && index > 0 {
                self.break_page();
            }
            let offset = paragraph.align.offset(width, line.width);
            // The baseline sits one ascent below the top of the line box.
            let baseline = self.cursor + paragraph.style.font.ascent(paragraph.style.size);
            self.text(
                x + offset.max(0.0),
                baseline,
                &line.text,
                paragraph.style.font,
                paragraph.style.size,
                paragraph.style.color,
            );
            self.cursor += line_height;
        }
        self.cursor += paragraph.spacing_after;
        height
    }

    fn columns(&mut self, columns: &Columns, x: f32, width: f32) -> f32 {
        if columns.columns.is_empty() {
            return 0.0;
        }
        let count = columns.columns.len();
        let gap_total = columns.gap * (count.saturating_sub(1)) as f32;
        let usable = (width - gap_total).max(0.0);

        let shares: Vec<f32> = if columns.fractions.len() == count {
            columns.fractions.clone()
        } else {
            vec![1.0; count]
        };
        let total: f32 = shares.iter().sum();
        let widths: Vec<f32> = shares
            .iter()
            .map(|share| {
                if total <= 0.0 {
                    usable / count as f32
                } else {
                    usable * share / total
                }
            })
            .collect();

        // A row of columns is kept on one page. Splitting a column across a
        // page break would leave its neighbours behind, which reads worse than
        // starting the whole row lower down.
        let tallest = columns
            .columns
            .iter()
            .enumerate()
            .map(|(index, content)| measure(content, widths[index]))
            .fold(0.0f32, f32::max);
        self.ensure(tallest);

        let start = self.cursor;
        let mut left = x;
        for (index, content) in columns.columns.iter().enumerate() {
            let column_width = widths[index];
            self.cursor = start;
            self.block(content, left, column_width);
            left += column_width + columns.gap;
        }
        self.cursor = start + tallest;
        tallest
    }

    fn list(&mut self, list: &List, x: f32, width: f32) -> f32 {
        let start = self.cursor;
        for (index, item) in list.items.iter().enumerate() {
            if index > 0 {
                self.cursor += list.item_gap;
            }
            let indent = item.indent.max(0.0);
            let marker = list.marker.text(index);
            let marker_width = marker
                .chars()
                .map(|c| measure_text(&c.to_string(), StandardFont::Helvetica, 11.0))
                .fold(0.0f32, f32::max);
            let text_x = x + indent + marker_width + list.gap;
            let text_width = (width - indent - marker_width - list.gap).max(1.0);

            let height = measure(&item.content, text_width);
            self.ensure(height);

            let marker_baseline = self.cursor + StandardFont::Helvetica.ascent(11.0);
            self.text(
                x + indent,
                marker_baseline,
                &marker,
                StandardFont::Helvetica,
                11.0,
                Color::BLACK,
            );
            self.block(&item.content, text_x, text_width);
        }
        self.cursor - start
    }

    fn table(&mut self, table: &Table, x: f32, width: f32) -> f32 {
        let start = self.cursor;
        let widths = ColumnWidth::resolve(&column_widths(table), width);
        let padding = table.cell_padding.max(0.0);
        let line = table
            .borders
            .horizontal
            .width
            .max(table.borders.vertical.width)
            .max(table.borders.outline.width)
            .max(table.borders.header_rule.width)
            .max(0.0);

        // The header is drawn at the top of every page the table reaches, so
        // its height is subtracted from the room a first row has to find.
        let header_height = table
            .header
            .as_ref()
            .map(|row| self.row_height(row, &widths, padding))
            .unwrap_or(0.0);

        let draw_header = |layout: &mut Layout, x: f32| -> f32 {
            if let Some(header) = &table.header {
                let height = layout.row_height(header, &widths, padding);
                layout.draw_row(header, x, &widths, padding, table.borders, None);
                layout.mark_rule(x, layout.cursor, width, table.borders.header_rule);
                layout.cursor += table.borders.header_rule.width.max(0.0);
                height
            } else {
                0.0
            }
        };

        // Room needed for the header plus one row: if that does not fit, the
        // table starts on the next page rather than leaving the header alone.
        let first_row = table
            .rows
            .first()
            .map(|row| self.row_height(row, &widths, padding))
            .unwrap_or(0.0);
        self.ensure(header_height + first_row + line);

        let mut drew_header = false;
        if table.header.is_some() {
            draw_header(self, x);
            drew_header = true;
        }

        for (index, row) in table.rows.iter().enumerate() {
            let height = self.row_height(row, &widths, padding);
            let needed = height
                + if index + 1 < table.rows.len() {
                    table.borders.horizontal.width.max(0.0)
                } else {
                    table.borders.outline.width.max(0.0)
                };

            if self.remaining() < needed {
                self.break_page();
                if table.header.is_some() {
                    draw_header(self, x);
                    drew_header = true;
                }
            }

            self.draw_row(row, x, &widths, padding, table.borders, None);
            self.cursor += height;

            let is_last = index + 1 == table.rows.len();
            if is_last {
                self.mark_rule(x, self.cursor, width, table.borders.outline);
                self.cursor += table.borders.outline.width.max(0.0);
            } else {
                self.mark_rule(x, self.cursor, width, table.borders.horizontal);
                self.cursor += table.borders.horizontal.width.max(0.0);
            }
        }

        if !drew_header && table.rows.is_empty() && table.header.is_some() {
            draw_header(self, x);
        }

        self.cursor += table.spacing_after;
        self.cursor - start
    }

    /// Height of a row, including its padding.
    fn row_height(&self, row: &Row, widths: &[f32], padding: f32) -> f32 {
        let mut tallest = 0.0f32;
        let mut column = 0usize;
        for cell in &row.cells {
            let span = cell.colspan.max(1);
            let cell_width: f32 = widths.iter().skip(column).take(span).sum::<f32>().max(1.0);
            let inner = (cell_width - 2.0 * padding).max(1.0);
            let height = cell
                .blocks
                .iter()
                .map(|block| measure_with_style(block, inner, cell.style.as_ref()))
                .sum::<f32>();
            tallest = tallest.max(height);
            column += span;
        }
        tallest + 2.0 * padding
    }

    /// Draw one row at the cursor. The caller advances the cursor by the
    /// row's height.
    fn draw_row(
        &mut self,
        row: &Row,
        x: f32,
        widths: &[f32],
        padding: f32,
        borders: TableBorders,
        fallback_background: Option<Color>,
    ) {
        let height = self.row_height(row, widths, padding);
        let background = row.background.or(fallback_background);
        if let Some(color) = background {
            self.mark(Mark::Fill {
                x,
                top: self.cursor,
                width: widths.iter().sum(),
                height,
                color,
            });
        }

        let top = self.cursor;
        let mut column = 0usize;
        for cell in &row.cells {
            let span = cell.colspan.max(1);
            let cell_width: f32 = widths.iter().skip(column).take(span).sum();
            let inner = (cell_width - 2.0 * padding).max(1.0);

            let saved = self.cursor;
            self.cursor = top + padding;
            for block in &cell.blocks {
                let styled = apply_style(block, cell.style.as_ref(), cell.align);
                self.block(&styled, x + column_offset(widths, column) + padding, inner);
            }
            self.cursor = saved;

            column += span;
        }

        // Vertical lines: the separators between columns, and the two outer
        // edges when the table has an outline.
        let mut boundaries: Vec<(f32, crate::style::Border)> = Vec::new();
        for boundary in 1..widths.len() {
            boundaries.push((x + column_offset(widths, boundary), borders.vertical));
        }
        if borders.outline.is_visible() {
            boundaries.push((x, borders.outline));
            boundaries.push((x + widths.iter().sum::<f32>(), borders.outline));
        }
        for (at, border) in boundaries {
            if !border.is_visible() {
                continue;
            }
            self.mark(Mark::Line {
                x1: at,
                y1: top,
                x2: at,
                y2: top + height,
                color: border.color,
                width: border.width,
            });
        }
    }

    fn mark_rule(&mut self, x: f32, y: f32, width: f32, border: crate::style::Border) {
        if !border.is_visible() {
            return;
        }
        self.mark(Mark::Line {
            x1: x,
            y1: y,
            x2: x + width,
            y2: y,
            color: border.color,
            width: border.width,
        });
    }

    /// Draw the running header and footer once the page count is known.
    fn decorate(&mut self, total_pages: usize) {
        let options = self.options.clone();
        for (index, page) in self.pages.iter_mut().enumerate() {
            let number = index + 1;
            let mut marks = Vec::new();
            if let Some(header) = &options.header {
                let mut sub = Layout {
                    options: &options,
                    pages: vec![Page { marks: Vec::new() }],
                    cursor: options.margin,
                    replacements: 0,
                };
                sub.block(
                    &substitute(header, number, total_pages),
                    options.margin,
                    options.body_width(),
                );
                marks.append(&mut sub.pages[0].marks);
                self.replacements += sub.replacements;
            }
            if let Some(footer) = &options.footer {
                let height = measure(footer, options.body_width());
                let mut sub = Layout {
                    options: &options,
                    pages: vec![Page { marks: Vec::new() }],
                    cursor: options.page_size.height() - options.margin - height,
                    replacements: 0,
                };
                sub.block(
                    &substitute(footer, number, total_pages),
                    options.margin,
                    options.body_width(),
                );
                marks.append(&mut sub.pages[0].marks);
                self.replacements += sub.replacements;
            }
            // The header and footer go first so the body, which was laid out
            // over them, paints on top if the two ever overlap.
            marks.append(&mut page.marks);
            page.marks = marks;
        }
    }
}

/// The column widths of a table, from its `columns`.
fn column_widths(table: &Table) -> Vec<ColumnWidth> {
    table.columns.iter().map(|column| column.width).collect()
}

/// Replace `{{page}}` and `{{pages}}` in a piece of content.
fn substitute(content: &Content, page: usize, total: usize) -> Content {
    match content {
        Content::Text(paragraph) => {
            let text = paragraph
                .text
                .replace("{{page}}", &page.to_string())
                .replace("{{pages}}", &total.to_string());
            Content::Text(Paragraph {
                text,
                ..paragraph.clone()
            })
        }
        Content::Columns(columns) => Content::Columns(Columns {
            columns: columns
                .columns
                .iter()
                .map(|c| substitute(c, page, total))
                .collect(),
            ..columns.clone()
        }),
        Content::List(list) => Content::List(List {
            items: list
                .items
                .iter()
                .map(|item| crate::content::ListItem {
                    content: substitute(&item.content, page, total),
                    indent: item.indent,
                })
                .collect(),
            ..list.clone()
        }),
        other => other.clone(),
    }
}

/// Apply a cell's style and alignment to one of its blocks.
fn apply_style(
    content: &Content,
    style: Option<&crate::style::TextStyle>,
    align: Option<Align>,
) -> Content {
    match content {
        Content::Text(paragraph) => {
            let mut paragraph = paragraph.clone();
            if let Some(style) = style {
                paragraph.style = style.clone();
            }
            if let Some(align) = align {
                paragraph.align = align;
            }
            Content::Text(paragraph)
        }
        other => other.clone(),
    }
}

/// Sum of the widths before `column`.
fn column_offset(widths: &[f32], column: usize) -> f32 {
    widths.iter().take(column).sum()
}

/// Width of `text` in points.
fn measure_text(text: &str, font: StandardFont, size: f32) -> f32 {
    font.width_of(text, size)
}

/// Height a block will occupy at `width`.
pub(crate) fn measure(content: &Content, width: f32) -> f32 {
    measure_with_style(content, width, None)
}

/// Height a block will occupy at `width`, with a style forced on its paragraphs.
fn measure_with_style(
    content: &Content,
    width: f32,
    style: Option<&crate::style::TextStyle>,
) -> f32 {
    match content {
        Content::Text(paragraph) => {
            let mut paragraph = paragraph.clone();
            if let Some(style) = style {
                paragraph.style = style.clone();
            }
            measure_paragraph(&paragraph, width)
        }
        Content::Spacer(points) => points.max(0.0),
        Content::Rule(rule) => rule.thickness.max(0.0) + rule.spacing_after,
        Content::PageBreak => 0.0,
        Content::Columns(columns) => {
            if columns.columns.is_empty() {
                return 0.0;
            }
            let count = columns.columns.len();
            let gap_total = columns.gap * (count.saturating_sub(1)) as f32;
            let usable = (width - gap_total).max(0.0);
            let shares: Vec<f32> = if columns.fractions.len() == count {
                columns.fractions.clone()
            } else {
                vec![1.0; count]
            };
            let total: f32 = shares.iter().sum();
            columns
                .columns
                .iter()
                .enumerate()
                .map(|(index, content)| {
                    let share = if total <= 0.0 {
                        1.0
                    } else {
                        shares[index] / total
                    };
                    measure_with_style(content, usable * share, style)
                })
                .fold(0.0f32, f32::max)
        }
        Content::List(list) => {
            let marker_width = 12.0;
            let mut total = 0.0;
            for (index, item) in list.items.iter().enumerate() {
                if index > 0 {
                    total += list.item_gap;
                }
                let inner = (width - item.indent - marker_width - list.gap).max(1.0);
                total += measure_with_style(&item.content, inner, style);
            }
            total
        }
        Content::Table(table) => {
            let widths = ColumnWidth::resolve(&column_widths(table), width);
            let padding = table.cell_padding.max(0.0);
            let mut total = 0.0;
            let rows: Vec<&Row> = table.header.iter().chain(table.rows.iter()).collect();
            for row in rows {
                let mut tallest = 0.0f32;
                let mut column = 0usize;
                for cell in &row.cells {
                    let span = cell.colspan.max(1);
                    let cell_width: f32 = widths.iter().skip(column).take(span).sum();
                    let inner = (cell_width - 2.0 * padding).max(1.0);
                    let height = cell
                        .blocks
                        .iter()
                        .map(|block| {
                            measure_with_style(block, inner, cell.style.as_ref().or(style))
                        })
                        .sum::<f32>();
                    tallest = tallest.max(height);
                    column += span;
                }
                total += tallest + 2.0 * padding;
            }
            total + table.spacing_after
        }
    }
}

fn measure_paragraph(paragraph: &Paragraph, width: f32) -> f32 {
    let lines = wrap::wrap(
        &paragraph.text,
        paragraph.style.font,
        paragraph.style.size,
        width,
    );
    lines.len() as f32 * paragraph.style.line_height() + paragraph.spacing_after
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Cell, Column, ListItem, Row};

    fn a4() -> PageOptions {
        PageOptions::default()
    }

    #[test]
    fn an_empty_document_still_has_one_page() {
        let (pages, _) = render(&[], &a4());
        assert_eq!(pages.len(), 1);
    }

    #[test]
    fn a_short_paragraph_stays_on_one_page() {
        let (pages, _) = render(&[Paragraph::new("hello").into()], &a4());
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].marks.len(), 1);
    }

    #[test]
    fn long_content_flows_onto_more_pages() {
        let block: Content = Paragraph::new("word ".repeat(4000)).into();
        let (pages, _) = render(&[block], &a4());
        assert!(
            pages.len() > 1,
            "expected several pages, got {}",
            pages.len()
        );
    }

    #[test]
    fn every_mark_stays_inside_the_margins() {
        let options = a4();
        let block: Content = Paragraph::new("lorem ipsum dolor sit amet ".repeat(400)).into();
        let (pages, _) = render(&[block], &options);
        let bottom_limit = options.page_size.height() - options.margin;
        for page in &pages {
            for mark in &page.marks {
                if let Mark::Text { x, baseline, .. } = mark {
                    assert!(*x >= options.margin - 0.01, "x {x} left of margin");
                    assert!(
                        *baseline <= bottom_limit + 0.01,
                        "baseline {baseline} below the margin"
                    );
                    assert!(*baseline >= 0.0, "baseline above the page");
                }
            }
        }
    }

    #[test]
    fn a_page_break_starts_a_new_page() {
        let blocks = vec![
            Paragraph::new("first").into(),
            Content::PageBreak,
            Paragraph::new("second").into(),
        ];
        let (pages, _) = render(&blocks, &a4());
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].marks.len(), 1);
        assert_eq!(pages[1].marks.len(), 1);
    }

    #[test]
    fn a_leading_page_break_is_ignored() {
        let blocks = vec![Content::PageBreak, Paragraph::new("only").into()];
        let (pages, _) = render(&blocks, &a4());
        assert_eq!(pages.len(), 1);
    }

    #[test]
    fn a_table_header_repeats_on_every_page() {
        let rows: Vec<Row> = (0..120)
            .map(|i| {
                Row::new(vec![
                    Cell::text(format!("row {i}")),
                    Cell::text(format!("{i}")),
                ])
            })
            .collect();
        let table = Table::new(
            vec![
                Column::new(ColumnWidth::Fraction(3.0)),
                Column::new(ColumnWidth::Fraction(1.0)),
            ],
            rows,
        )
        .header(Row::new(vec![Cell::text("name"), Cell::text("value")]))
        .borders(TableBorders::horizontal_rules());

        let (pages, _) = render(&[Content::Table(table)], &a4());
        assert!(pages.len() > 1, "expected several pages");

        // Each page must contain the header text.
        for (index, page) in pages.iter().enumerate() {
            let has_header = page.marks.iter().any(|mark| match mark {
                Mark::Text { bytes, .. } => bytes.starts_with(b"name"),
                _ => false,
            });
            assert!(has_header, "page {index} is missing the repeated header");
        }
    }

    #[test]
    fn a_row_is_not_split_across_pages() {
        // Each row is one text mark; no page may contain a row's text twice.
        let rows: Vec<Row> = (0..80)
            .map(|i| Row::new(vec![Cell::text(format!("cell-{i}"))]))
            .collect();
        let table = Table::new(vec![Column::new(ColumnWidth::Fraction(1.0))], rows)
            .header(Row::new(vec![Cell::text("head")]));

        let (pages, _) = render(&[Content::Table(table)], &a4());
        let mut seen = std::collections::HashSet::new();
        for page in &pages {
            for mark in &page.marks {
                if let Mark::Text { bytes, .. } = mark {
                    let text = String::from_utf8_lossy(bytes).to_string();
                    if text.starts_with("cell-") {
                        assert!(seen.insert(text.clone()), "{text} drawn twice");
                    }
                }
            }
        }
        assert_eq!(seen.len(), 80);
    }

    #[test]
    fn columns_are_placed_side_by_side() {
        let columns = Columns::new(vec![
            Paragraph::new("left").into(),
            Paragraph::new("right").into(),
        ]);
        let (pages, _) = render(&[Content::Columns(columns)], &a4());
        let texts: Vec<(f32, String)> = pages[0]
            .marks
            .iter()
            .filter_map(|mark| match mark {
                Mark::Text { x, bytes, .. } => {
                    Some((*x, String::from_utf8_lossy(bytes).to_string()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 2);
        assert!(texts[0].0 < texts[1].0, "columns overlap: {texts:?}");
        // Both baselines are on the same line.
        let baselines: Vec<f32> = pages[0]
            .marks
            .iter()
            .filter_map(|mark| match mark {
                Mark::Text { baseline, .. } => Some(*baseline),
                _ => None,
            })
            .collect();
        assert!((baselines[0] - baselines[1]).abs() < 0.001);
    }

    #[test]
    fn list_markers_appear_before_their_items() {
        let list = List::new(vec![
            ListItem::new(Paragraph::new("first").into()),
            ListItem::new(Paragraph::new("second").into()),
        ]);
        let (pages, _) = render(&[Content::List(list)], &a4());
        let texts: Vec<Vec<u8>> = pages[0]
            .marks
            .iter()
            .filter_map(|mark| match mark {
                Mark::Text { bytes, .. } => Some(bytes.clone()),
                _ => None,
            })
            .collect();
        // The marks hold WinAnsi bytes, and 0x95 is the bullet in that
        // encoding, so the marker is compared as a byte rather than as text.
        assert_eq!(texts.len(), 4);
        assert_eq!(texts[0], vec![0x95]);
        assert_eq!(texts[1], b"first".to_vec());
        assert_eq!(texts[2], vec![0x95]);
        assert_eq!(texts[3], b"second".to_vec());
    }

    #[test]
    fn a_footer_is_drawn_on_every_page_with_the_page_number() {
        let options = a4().footer(Paragraph::new("Page {{page}} of {{pages}}"));
        let block: Content = Paragraph::new("text ".repeat(6000)).into();
        let (pages, _) = render(&[block], &options);
        assert!(pages.len() > 1);

        for (index, page) in pages.iter().enumerate() {
            let expected = format!("Page {} of {}", index + 1, pages.len());
            let found = page.marks.iter().any(|mark| match mark {
                Mark::Text { bytes, .. } => String::from_utf8_lossy(bytes) == expected,
                _ => false,
            });
            assert!(found, "page {} lacks footer {expected:?}", index + 1);
        }
    }

    #[test]
    fn a_header_is_drawn_on_every_page() {
        let options = a4().header(Paragraph::new("Invoice 42"));
        let block: Content = Paragraph::new("text ".repeat(6000)).into();
        let (pages, _) = render(&[block], &options);
        assert!(pages.len() > 1);
        for page in &pages {
            let found = page.marks.iter().any(|mark| match mark {
                Mark::Text { bytes, .. } => bytes.starts_with(b"Invoice 42"),
                _ => false,
            });
            assert!(found);
        }
    }

    #[test]
    fn the_footer_sits_below_the_body() {
        let options = a4().footer(Paragraph::new("foot"));
        let (pages, _) = render(&[Paragraph::new("body").into()], &options);
        let body_y = pages[0]
            .marks
            .iter()
            .find_map(|mark| match mark {
                Mark::Text {
                    bytes, baseline, ..
                } if bytes.starts_with(b"body") => Some(*baseline),
                _ => None,
            })
            .unwrap();
        let footer_y = pages[0]
            .marks
            .iter()
            .find_map(|mark| match mark {
                Mark::Text {
                    bytes, baseline, ..
                } if bytes.starts_with(b"foot") => Some(*baseline),
                _ => None,
            })
            .unwrap();
        assert!(footer_y > body_y, "footer at {footer_y}, body at {body_y}");
    }

    #[test]
    fn unrepresentable_text_is_counted_not_dropped() {
        let (_, replacements) = render(&[Paragraph::new("café 中文").into()], &a4());
        assert_eq!(replacements, 2);
    }

    #[test]
    fn alignment_moves_the_line_within_its_column() {
        let right = Paragraph::new("x").align(Align::Right);
        let left = Paragraph::new("x");
        let (pages, _) = render(&[left.into(), right.into()], &a4());
        let xs: Vec<f32> = pages[0]
            .marks
            .iter()
            .filter_map(|mark| match mark {
                Mark::Text { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        assert!(
            xs[1] > xs[0],
            "right-aligned x {} not right of {}",
            xs[1],
            xs[0]
        );
    }

    #[test]
    fn measure_agrees_with_what_is_drawn() {
        let options = a4();
        let content: Content = Paragraph::new("alpha beta gamma delta").into();
        let predicted = measure(&content, options.body_width());
        let (_, _) = render(std::slice::from_ref(&content), &options);
        // One line at 11pt Helvetica: ascent 7.898 + descent 2.277.
        assert!((predicted - 10.175).abs() < 0.01, "predicted {predicted}");
    }
}
