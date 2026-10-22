//! The core fonts, and the WinAnsi encoding they are written in.
//!
//! The 14 standard PDF fonts need no embedding, which is what lets this crate
//! produce a PDF with no font files and no dependencies. The price is that text
//! is limited to what WinAnsi can represent, and that the width of every glyph
//! has to be known before a line can be wrapped.

mod metrics;

pub(crate) use metrics::{Metrics, WINANSI_NON_ASCII};

/// One of the 14 fonts every PDF reader must provide.
///
/// Only the twelve text fonts are offered. Symbol and ZapfDingbats are symbol
/// fonts with their own encodings; they are not useful for documents and are
/// deliberately absent.
///
/// # Examples
///
/// ```
/// use pdfmake::StandardFont;
///
/// assert_eq!(StandardFont::Helvetica.as_str(), "Helvetica");
/// assert_eq!(StandardFont::default(), StandardFont::Helvetica);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StandardFont {
    /// Helvetica, the default. A sans-serif face.
    #[default]
    Helvetica,
    /// Helvetica Bold.
    HelveticaBold,
    /// Helvetica Oblique.
    HelveticaOblique,
    /// Helvetica Bold Oblique.
    HelveticaBoldOblique,
    /// Times Roman, a serif face.
    TimesRoman,
    /// Times Bold.
    TimesBold,
    /// Times Italic.
    TimesItalic,
    /// Times Bold Italic.
    TimesBoldItalic,
    /// Courier, a monospaced face.
    Courier,
    /// Courier Bold.
    CourierBold,
    /// Courier Oblique.
    CourierOblique,
    /// Courier Bold Oblique.
    CourierBoldOblique,
}

impl StandardFont {
    /// Every text font, in a stable order.
    pub const ALL: [StandardFont; 12] = [
        StandardFont::Helvetica,
        StandardFont::HelveticaBold,
        StandardFont::HelveticaOblique,
        StandardFont::HelveticaBoldOblique,
        StandardFont::TimesRoman,
        StandardFont::TimesBold,
        StandardFont::TimesItalic,
        StandardFont::TimesBoldItalic,
        StandardFont::Courier,
        StandardFont::CourierBold,
        StandardFont::CourierOblique,
        StandardFont::CourierBoldOblique,
    ];

    /// The name this font is written under in a PDF.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::StandardFont;
    ///
    /// assert_eq!(StandardFont::TimesBoldItalic.as_str(), "Times-BoldItalic");
    /// ```
    pub const fn as_str(self) -> &'static str {
        match self {
            StandardFont::Helvetica => "Helvetica",
            StandardFont::HelveticaBold => "Helvetica-Bold",
            StandardFont::HelveticaOblique => "Helvetica-Oblique",
            StandardFont::HelveticaBoldOblique => "Helvetica-BoldOblique",
            StandardFont::TimesRoman => "Times-Roman",
            StandardFont::TimesBold => "Times-Bold",
            StandardFont::TimesItalic => "Times-Italic",
            StandardFont::TimesBoldItalic => "Times-BoldItalic",
            StandardFont::Courier => "Courier",
            StandardFont::CourierBold => "Courier-Bold",
            StandardFont::CourierOblique => "Courier-Oblique",
            StandardFont::CourierBoldOblique => "Courier-BoldOblique",
        }
    }

    /// The bold face of the same family.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::StandardFont;
    ///
    /// assert_eq!(StandardFont::TimesRoman.bold(), StandardFont::TimesBold);
    /// assert_eq!(StandardFont::CourierBoldOblique.bold(), StandardFont::CourierBoldOblique);
    /// ```
    pub const fn bold(self) -> StandardFont {
        match self {
            StandardFont::Helvetica
            | StandardFont::HelveticaBold
            | StandardFont::HelveticaOblique
            | StandardFont::HelveticaBoldOblique => match self {
                StandardFont::Helvetica | StandardFont::HelveticaBold => {
                    StandardFont::HelveticaBold
                }
                _ => StandardFont::HelveticaBoldOblique,
            },
            StandardFont::TimesRoman | StandardFont::TimesBold => StandardFont::TimesBold,
            StandardFont::TimesItalic | StandardFont::TimesBoldItalic => {
                StandardFont::TimesBoldItalic
            }
            StandardFont::Courier | StandardFont::CourierBold => StandardFont::CourierBold,
            _ => StandardFont::CourierBoldOblique,
        }
    }

    /// Metrics for this font: vertical extents and glyph widths.
    pub(crate) fn metrics(self) -> Metrics {
        metrics::metrics(self)
    }

    /// Width of `text` at `size` points, in points.
    ///
    /// Characters WinAnsi cannot represent are measured at zero width rather
    /// than reported as an error, so that a layout can still be computed for
    /// text that will be replaced by a fallback at write time.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdfmake::StandardFont;
    ///
    /// // Courier is monospaced, so ten characters are 10 * 600/1000 * 10pt.
    /// let width = StandardFont::Courier.width_of("0123456789", 10.0);
    /// assert!((width - 60.0).abs() < 0.001);
    /// ```
    pub fn width_of(self, text: &str, size: f32) -> f32 {
        let widths = self.metrics().widths;
        let units: u32 = text
            .chars()
            .map(|c| match encode_char(c) {
                Some(code) => u32::from(widths[usize::from(code)]),
                None => 0,
            })
            .sum();
        units as f32 * size / 1000.0
    }

    /// Distance from the baseline to the top of the tallest glyph, at `size`.
    pub(crate) fn ascent(self, size: f32) -> f32 {
        f32::from(self.metrics().ascent) * size / 1000.0
    }

    /// The y offset of the lowest descender relative to the baseline, at
    /// `size`. Negative, because a descender sits below the baseline.
    pub(crate) fn descent(self, size: f32) -> f32 {
        -f32::from(self.metrics().descent) * size / 1000.0
    }
}

/// The WinAnsi code for `c`, or `None` when WinAnsi cannot represent it.
///
/// WinAnsi is cp1252 with a handful of differences, and it is the encoding a
/// core font is assumed to use when a PDF does not say otherwise. ASCII maps to
/// itself; everything else is a table lookup.
pub(crate) fn encode_char(c: char) -> Option<u8> {
    match c {
        ' '..='~' => Some(c as u8),
        _ => WINANSI_NON_ASCII
            .binary_search_by_key(&c, |&(ch, _)| ch)
            .ok()
            .map(|i| WINANSI_NON_ASCII[i].1),
    }
}

/// Encode `text` as WinAnsi bytes, replacing anything unrepresentable.
///
/// Returns the bytes and the number of characters that had to be replaced.
pub(crate) fn encode_text(text: &str) -> (Vec<u8>, usize) {
    let mut out = Vec::with_capacity(text.len());
    let mut replaced = 0;
    for c in text.chars() {
        match encode_char(c) {
            Some(code) => out.push(code),
            None => {
                out.push(b'?');
                replaced += 1;
            }
        }
    }
    (out, replaced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_round_trips() {
        for c in ' '..='~' {
            assert_eq!(encode_char(c), Some(c as u8), "failed for {c:?}");
        }
    }

    #[test]
    fn latin1_letters_are_at_their_own_code_point() {
        // WinAnsi agrees with Latin-1 from U+00A0 upwards, except where cp1252
        // puts punctuation. These are the letters a European invoice needs.
        for (c, code) in [
            ('é', 233),
            ('ü', 252),
            ('ß', 223),
            ('ñ', 241),
            ('ø', 248),
            ('å', 229),
            ('æ', 230),
        ] {
            assert_eq!(encode_char(c), Some(code), "failed for {c:?}");
        }
    }

    #[test]
    fn cp1252_punctuation_uses_the_high_codes() {
        // These sit in the 0x80..0x9F range that cp1252 repurposed, which is
        // where a naive "cast the code point" implementation goes wrong.
        for (c, code) in [
            ('\u{20ac}', 128), // euro
            ('\u{2018}', 145), // left single quote
            ('\u{2019}', 146), // right single quote
            ('\u{201c}', 147), // left double quote
            ('\u{201d}', 148), // right double quote
            ('\u{2013}', 150), // en dash
            ('\u{2014}', 151), // em dash
            ('\u{2026}', 133), // ellipsis
            ('\u{2122}', 153), // trade mark
        ] {
            assert_eq!(encode_char(c), Some(code), "failed for {c:?}");
        }
    }

    #[test]
    fn bullet_is_at_149_not_127() {
        // Annex D of the PDF specification leaves 0x7F unused and puts the
        // bullet at 0x95. A well-known font library disagrees; this pins the
        // specification's answer so the table cannot silently regress.
        assert_eq!(encode_char('\u{2022}'), Some(149));
    }

    #[test]
    fn unrepresentable_characters_are_absent() {
        for c in ['\u{4e2d}', '\u{0430}', '\u{05d0}', '\u{1f600}'] {
            assert_eq!(encode_char(c), None, "{c:?} should not encode");
        }
    }

    #[test]
    fn encoding_replaces_unrepresentable_text() {
        let (bytes, replaced) = encode_text("café 中文");
        assert_eq!(bytes, b"caf\xe9 ??");
        assert_eq!(replaced, 2);
    }

    #[test]
    fn widths_match_known_afm_values() {
        // Taken from Adobe's Helvetica.afm. These are the numbers every
        // implementation of the standard fonts has to agree on.
        let h = StandardFont::Helvetica.metrics().widths;
        assert_eq!(h[usize::from(b' ')], 278);
        assert_eq!(h[usize::from(b'A')], 667);
        assert_eq!(h[usize::from(b'i')], 222);
        assert_eq!(h[usize::from(b'W')], 944);
        assert_eq!(h[128], 556); // euro
    }

    #[test]
    fn courier_is_monospaced() {
        let widths = StandardFont::Courier.metrics().widths;
        for code in 32u8..=126 {
            assert_eq!(widths[usize::from(code)], 600, "code {code} differs");
        }
    }

    #[test]
    fn width_of_a_known_string() {
        // Adobe's Helvetica: H 722, e 556, l 222, l 222, o 556 = 2278.
        let width = StandardFont::Helvetica.width_of("Hello", 1000.0);
        assert!((width - 2278.0).abs() < 0.001, "got {width}");
    }

    #[test]
    fn vertical_metrics_are_the_afm_values() {
        let h = StandardFont::Helvetica.metrics();
        assert_eq!(h.ascent, 718);
        // The metrics table stores the descent as a magnitude; `descent()` is
        // what turns it into the negative offset the baseline needs.
        assert_eq!(h.descent, 207);
        assert_eq!(StandardFont::Helvetica.descent(1000.0), -207.0);
    }
}
