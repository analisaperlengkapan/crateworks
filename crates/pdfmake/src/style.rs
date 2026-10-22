//! Colours, text styles and the other small values a document is built from.

use crate::fonts::StandardFont;

/// A colour in the RGB space, each channel from 0 to 255.
///
/// # Examples
///
/// ```
/// use pdfmake::Color;
///
/// assert_eq!(Color::rgb(0, 0, 0).to_pdf_operator(), "0 0 0 rg");
/// assert_eq!(Color::WHITE.to_pdf_operator(), "1 1 1 rg");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red channel, 0 to 255.
    pub red: u8,
    /// Green channel, 0 to 255.
    pub green: u8,
    /// Blue channel, 0 to 255.
    pub blue: u8,
}

impl Color {
    /// Black, the default text colour.
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    /// White, the default page colour.
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    /// A mid grey, useful for rules and secondary text.
    pub const GREY: Color = Color::rgb(128, 128, 128);

    /// A colour from its red, green and blue channels.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Color;
    ///
    /// let navy = Color::rgb(0x1f, 0x3a, 0x5f);
    /// assert_eq!((navy.red, navy.green, navy.blue), (0x1f, 0x3a, 0x5f));
    /// ```
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Color {
        Color { red, green, blue }
    }

    /// The colour as a PDF fill operator, for example `0 0 0 rg`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Color;
    ///
    /// assert_eq!(Color::rgb(255, 0, 0).to_pdf_operator(), "1 0 0 rg");
    /// ```
    pub fn to_pdf_operator(self) -> String {
        let (r, g, b) = self.to_unit();
        format!("{r} {g} {b} rg")
    }

    /// The colour as a PDF stroke operator, for example `0 0 0 RG`.
    pub(crate) fn to_stroke_operator(self) -> String {
        let (r, g, b) = self.to_unit();
        format!("{r} {g} {b} RG")
    }

    fn to_unit(self) -> (String, String, String) {
        (unit(self.red), unit(self.green), unit(self.blue))
    }
}

/// Render one 0..=255 channel as a PDF colour component in 0..=1.
fn unit(value: u8) -> String {
    if value == 0 {
        return "0".to_string();
    }
    if value == 255 {
        return "1".to_string();
    }
    let scaled = f32::from(value) / 255.0;
    // Four decimals is finer than an 8-bit channel can express, and short
    // enough that the numbers stay readable in a content stream.
    let mut s = format!("{scaled:.4}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

/// A font, a size and the colour to draw text in.
///
/// # Examples
///
/// ```
/// use pdfmake::{Color, StandardFont, TextStyle};
///
/// let heading = TextStyle::new(StandardFont::HelveticaBold, 18.0);
/// assert_eq!(heading.font, StandardFont::HelveticaBold);
///
/// let mut warning = heading.clone();
/// warning.color = Color::rgb(0xb0, 0x00, 0x20);
/// assert_ne!(warning.color, heading.color);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// The face to draw with.
    pub font: StandardFont,
    /// The size in points.
    pub size: f32,
    /// The colour of the glyphs.
    pub color: Color,
}

impl TextStyle {
    /// A style in black at the given size.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Color, StandardFont, TextStyle};
    ///
    /// let style = TextStyle::new(StandardFont::Courier, 9.0);
    /// assert_eq!(style.color, Color::BLACK);
    /// assert_eq!(style.size, 9.0);
    /// ```
    pub fn new(font: StandardFont, size: f32) -> TextStyle {
        TextStyle {
            font,
            size,
            color: Color::BLACK,
        }
    }

    /// The same style with a different colour.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Color, StandardFont, TextStyle};
    ///
    /// let grey = TextStyle::new(StandardFont::Helvetica, 10.0).with_color(Color::GREY);
    /// assert_eq!(grey.color, Color::GREY);
    /// ```
    pub fn with_color(mut self, color: Color) -> TextStyle {
        self.color = color;
        self
    }

    /// The same style at a different size.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{StandardFont, TextStyle};
    ///
    /// let big = TextStyle::new(StandardFont::Helvetica, 10.0).with_size(24.0);
    /// assert_eq!(big.size, 24.0);
    /// ```
    pub fn with_size(mut self, size: f32) -> TextStyle {
        self.size = size;
        self
    }

    /// The same style in a different face.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{StandardFont, TextStyle};
    ///
    /// let bold = TextStyle::new(StandardFont::Helvetica, 10.0)
    ///     .with_font(StandardFont::HelveticaBold);
    /// assert_eq!(bold.font, StandardFont::HelveticaBold);
    /// ```
    pub fn with_font(mut self, font: StandardFont) -> TextStyle {
        self.font = font;
        self
    }

    /// Height of one line in this style, in points.
    ///
    /// This is the distance between baselines: the font's ascent plus the
    /// magnitude of its descent.
    pub(crate) fn line_height(&self) -> f32 {
        self.font.ascent(self.size) + self.font.descent(self.size).abs()
    }
}

impl Default for TextStyle {
    fn default() -> TextStyle {
        TextStyle::new(StandardFont::Helvetica, 11.0)
    }
}

/// Alignment of a line of text within its column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Against the left edge. The default.
    #[default]
    Left,
    /// Centred in the column.
    Center,
    /// Against the right edge.
    Right,
}

impl Align {
    /// The `x` offset at which a run of `width` should start inside a column of
    /// `available` points.
    pub(crate) fn offset(self, available: f32, width: f32) -> f32 {
        match self {
            Align::Left => 0.0,
            Align::Center => (available - width) / 2.0,
            Align::Right => available - width,
        }
    }
}

/// A line above, below or around an element.
///
/// # Examples
///
/// ```
/// use pdfmake::{Color, Border};
///
/// let rule = Border::new(Color::GREY, 0.5);
/// assert_eq!(rule.width, 0.5);
/// assert_eq!(Border::none().width, 0.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    /// The colour of the line.
    pub color: Color,
    /// The line width in points. Zero means no line.
    pub width: f32,
}

impl Border {
    /// A border of the given colour and width.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Color, Border};
    ///
    /// let b = Border::new(Color::BLACK, 1.0);
    /// assert_eq!(b.color, Color::BLACK);
    /// ```
    pub const fn new(color: Color, width: f32) -> Border {
        Border { color, width }
    }

    /// No border.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::Border;
    ///
    /// assert_eq!(Border::none().width, 0.0);
    /// ```
    pub const fn none() -> Border {
        Border::new(Color::BLACK, 0.0)
    }

    /// Whether this border would draw anything.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::{Border, Color};
    ///
    /// assert!(!Border::none().is_visible());
    /// assert!(Border::new(Color::BLACK, 0.25).is_visible());
    /// ```
    pub fn is_visible(self) -> bool {
        self.width > 0.0
    }
}

impl Default for Border {
    fn default() -> Border {
        Border::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_components_are_scaled_to_the_unit_interval() {
        assert_eq!(Color::rgb(0, 0, 0).to_pdf_operator(), "0 0 0 rg");
        assert_eq!(Color::rgb(255, 255, 255).to_pdf_operator(), "1 1 1 rg");
        assert_eq!(Color::rgb(255, 0, 0).to_pdf_operator(), "1 0 0 rg");
        // 128/255 = 0.50196..., which rounds to 0.502.
        assert_eq!(Color::rgb(128, 0, 0).to_pdf_operator(), "0.502 0 0 rg");
    }

    #[test]
    fn stroke_operator_uses_capital_rg() {
        assert_eq!(Color::BLACK.to_stroke_operator(), "0 0 0 RG");
    }

    #[test]
    fn alignment_offsets_place_the_run() {
        assert_eq!(Align::Left.offset(100.0, 40.0), 0.0);
        assert_eq!(Align::Center.offset(100.0, 40.0), 30.0);
        assert_eq!(Align::Right.offset(100.0, 40.0), 60.0);
    }

    #[test]
    fn line_height_is_ascent_plus_descent() {
        let style = TextStyle::new(StandardFont::Helvetica, 1000.0);
        assert!((style.line_height() - 925.0).abs() < 0.001);
    }

    #[test]
    fn borders_know_whether_they_draw() {
        assert!(!Border::none().is_visible());
        assert!(Border::new(Color::BLACK, 0.25).is_visible());
    }
}
