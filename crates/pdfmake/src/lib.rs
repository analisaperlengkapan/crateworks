#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod content;
pub mod fonts;
pub mod style;
pub mod wrap;

mod flate;
mod layout;
mod object;
mod writer;

pub use content::{
    Cell, Column, ColumnWidth, Columns, Content, List, ListItem, Marker, Paragraph, Row, Rule,
    Table, TableBorders,
};
pub use fonts::StandardFont;
pub use layout::{PageOptions, PageSize};
pub use style::{Align, Border, Color, TextStyle};

/// A document: the content, the page geometry, and the running header and footer.
///
/// # Examples
///
/// ```
/// use pdfmake::{Document, Paragraph};
///
/// let pdf = Document::new()
///     .push(Paragraph::new("Hello, world."))
///     .to_pdf();
///
/// assert!(pdf.starts_with(b"%PDF-1.7"));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    content: Vec<Content>,
    options: PageOptions,
}

impl Document {
    /// An empty A4 document with 48 point margins.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Document;
    ///
    /// let document = Document::new();
    /// assert!(document.is_empty());
    /// ```
    pub fn new() -> Document {
        Document {
            content: Vec::new(),
            options: PageOptions::default(),
        }
    }

    /// Add a block at the end of the document.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Document, Paragraph};
    ///
    /// let document = Document::new().push(Paragraph::new("one"));
    /// assert_eq!(document.len(), 1);
    /// ```
    pub fn push(mut self, content: impl Into<Content>) -> Document {
        self.content.push(content.into());
        self
    }

    /// Add a block that is already a [`Content`].
    ///
    /// This is the same as [`Document::push`]; it exists so the intent reads
    /// clearly when the argument is a table or a list rather than a paragraph.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Content, Document, Paragraph};
    ///
    /// let document = Document::new().with(Content::Text(Paragraph::new("x")));
    /// assert_eq!(document.len(), 1);
    /// ```
    pub fn with(self, content: Content) -> Document {
        self.push(content)
    }

    /// Add several blocks at once.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Document, Paragraph};
    ///
    /// let document = Document::new().extend([
    ///     Paragraph::new("one").into(),
    ///     Paragraph::new("two").into(),
    /// ]);
    /// assert_eq!(document.len(), 2);
    /// ```
    pub fn extend(mut self, blocks: impl IntoIterator<Item = Content>) -> Document {
        self.content.extend(blocks);
        self
    }

    /// Replace the page geometry, header and footer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Document, PageOptions, PageSize};
    ///
    /// let document = Document::new().options(PageOptions::default().size(PageSize::Letter));
    /// assert_eq!(document.page_options().page_size, PageSize::Letter);
    /// ```
    pub fn options(mut self, options: PageOptions) -> Document {
        self.options = options;
        self
    }

    /// The page geometry in use.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Document, PageSize};
    ///
    /// assert_eq!(Document::new().page_options().page_size, PageSize::A4);
    /// ```
    pub fn page_options(&self) -> &PageOptions {
        &self.options
    }

    /// The blocks added so far.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Document, Paragraph};
    ///
    /// let document = Document::new().push(Paragraph::new("x"));
    /// assert_eq!(document.blocks()[0].plain_text(), "x");
    /// ```
    pub fn blocks(&self) -> &[Content] {
        &self.content
    }

    /// How many blocks the document holds.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Document;
    ///
    /// assert_eq!(Document::new().len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Whether the document has no blocks.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Document;
    ///
    /// assert!(Document::new().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// The plain text of every block, in order.
    ///
    /// Useful for tests and for building a search index alongside the PDF.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Document, Paragraph};
    ///
    /// let document = Document::new().push(Paragraph::new("hello"));
    /// assert_eq!(document.plain_text(), "hello");
    /// ```
    pub fn plain_text(&self) -> String {
        self.content
            .iter()
            .map(Content::plain_text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Render the document to PDF bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Document, Paragraph};
    ///
    /// let pdf = Document::new().push(Paragraph::new("hello")).to_pdf();
    /// assert!(pdf.starts_with(b"%PDF-1.7"));
    /// assert!(pdf.ends_with(b"%%EOF\n"));
    /// ```
    pub fn to_pdf(&self) -> Vec<u8> {
        let (pages, replacements) = layout::render(&self.content, &self.options);
        writer::write(&pages, &self.options, replacements)
    }

    /// Render to a file.
    ///
    /// # Errors
    ///
    /// Returns whatever `std::fs::write` returns.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pdfmake::{Document, Paragraph};
    ///
    /// Document::new()
    ///     .push(Paragraph::new("hello"))
    ///     .save("out.pdf")
    ///     .unwrap();
    /// ```
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::write(path, self.to_pdf())
    }

    /// How many pages the document will occupy.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Document, Paragraph};
    ///
    /// assert_eq!(Document::new().push(Paragraph::new("x")).page_count(), 1);
    /// ```
    pub fn page_count(&self) -> usize {
        layout::render(&self.content, &self.options).0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_document_is_a_valid_pdf() {
        let pdf = Document::new().to_pdf();
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF-1.7\n"));
        assert!(text.ends_with("%%EOF\n"));
        assert!(text.contains("/Type /Catalog"));
        assert!(text.contains("/Type /Pages"));
        assert!(text.contains("/Type /Page "));
    }

    #[test]
    fn the_page_count_matches_the_number_of_page_objects() {
        let document = Document::new().push(Paragraph::new("word ".repeat(6000)));
        let pdf = document.to_pdf();
        let text = String::from_utf8_lossy(&pdf);
        let objects = text.matches("/Type /Page ").count();
        assert_eq!(objects, document.page_count());
    }

    #[test]
    fn every_core_font_is_registered() {
        let pdf = Document::new().push(Paragraph::new("x")).to_pdf();
        let text = String::from_utf8_lossy(&pdf);
        for font in StandardFont::ALL {
            assert!(
                text.contains(&format!("/BaseFont /{}", font.as_str())),
                "{} is missing",
                font.as_str()
            );
        }
        assert!(text.contains("/Encoding /WinAnsiEncoding"));
    }

    #[test]
    fn the_cross_reference_offset_points_at_the_table() {
        let pdf = Document::new().push(Paragraph::new("hello")).to_pdf();
        // The header comment holds bytes above 0x7F, so offsets are byte
        // offsets and the file is searched as bytes.
        let at = pdf
            .windows(10)
            .position(|w| w == b"startxref\n")
            .expect("no startxref");
        let declared: usize = std::str::from_utf8(&pdf[at + 10..])
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(&pdf[declared..declared + 4], b"xref");
    }

    #[test]
    fn save_writes_the_same_bytes() {
        let document = Document::new().push(Paragraph::new("hello"));
        let dir = std::env::temp_dir().join("pdfmake-save-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.pdf");
        document.save(&path).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), document.to_pdf());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
