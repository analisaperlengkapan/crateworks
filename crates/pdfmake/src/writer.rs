//! Turning laid-out pages into PDF bytes.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::flate;
use crate::fonts::StandardFont;
use crate::layout::{Mark, Page, PageOptions};
use crate::object::{Dict, Document, Object, ObjectId, Value};

/// The resource name each font is referred to by inside a content stream.
///
/// PDF names may not contain a hyphen, so `Helvetica-Bold` cannot be used as a
/// resource name. A fixed mapping keeps the name used in the stream and the name
/// registered in `/Resources` from drifting apart.
fn font_name(font: StandardFont) -> &'static str {
    match font {
        StandardFont::Helvetica => "F1",
        StandardFont::HelveticaBold => "F2",
        StandardFont::HelveticaOblique => "F3",
        StandardFont::HelveticaBoldOblique => "F4",
        StandardFont::TimesRoman => "F5",
        StandardFont::TimesBold => "F6",
        StandardFont::TimesItalic => "F7",
        StandardFont::TimesBoldItalic => "F8",
        StandardFont::Courier => "F9",
        StandardFont::CourierBold => "F10",
        StandardFont::CourierOblique => "F11",
        StandardFont::CourierBoldOblique => "F12",
    }
}

/// Write the finished document.
pub(crate) fn write(pages: &[Page], options: &PageOptions, replacements: usize) -> Vec<u8> {
    let mut doc = Document::new();

    // The catalog and the page tree are reserved first: a page names the tree as
    // its parent and the tree names the page as its kid, so one of the two has to
    // be placed before the other is known.
    let catalog_id = doc.reserve();
    let pages_id = doc.reserve();

    let mut font_ids: BTreeMap<&'static str, ObjectId> = BTreeMap::new();
    for font in StandardFont::ALL {
        let mut dict = Dict::new();
        dict.set("Type", Value::Name("Font".into()));
        dict.set("Subtype", Value::Name("Type1".into()));
        dict.set("BaseFont", Value::Name(font.as_str().into()));
        dict.set("Encoding", Value::Name("WinAnsiEncoding".into()));
        font_ids.insert(font_name(font), doc.add(Object::Dict(dict)));
    }

    let mut resources = Dict::new();
    {
        let mut fonts = Dict::new();
        for (name, id) in &font_ids {
            fonts.set(name, Value::Ref(*id));
        }
        resources.set("Font", Value::Dict(fonts));
    }
    let resources_id = doc.add(Object::Dict(resources));

    let mut page_ids = Vec::with_capacity(pages.len());
    for page in pages {
        let mut stream_dict = Dict::new();
        stream_dict.set("Filter", Value::Name("FlateDecode".into()));
        let content_id = doc.add(Object::Stream {
            dict: stream_dict,
            data: content_stream(page, options),
        });

        let mut dict = Dict::new();
        dict.set("Type", Value::Name("Page".into()));
        dict.set("Parent", Value::Ref(pages_id));
        dict.set(
            "MediaBox",
            Value::Array(vec![
                Value::Int(0),
                Value::Int(0),
                Value::Real(options.page_size.width()),
                Value::Real(options.page_size.height()),
            ]),
        );
        dict.set("Resources", Value::Ref(resources_id));
        dict.set("Contents", Value::Ref(content_id));
        page_ids.push(doc.add(Object::Dict(dict)));
    }

    let mut pages_dict = Dict::new();
    pages_dict.set("Type", Value::Name("Pages".into()));
    pages_dict.set("Count", Value::Int(page_ids.len() as i64));
    pages_dict.set(
        "Kids",
        Value::Array(page_ids.iter().map(|id| Value::Ref(*id)).collect()),
    );
    doc.replace(pages_id, Object::Dict(pages_dict));

    let mut catalog = Dict::new();
    catalog.set("Type", Value::Name("Catalog".into()));
    catalog.set("Pages", Value::Ref(pages_id));
    doc.replace(catalog_id, Object::Dict(catalog));

    if replacements > 0 {
        // Worth recording in the file: a reader that sees this knows the text is
        // not a faithful copy of the source.
        doc.set_info(
            "Subject",
            Value::Text(
                format!("{replacements} characters replaced (not in WinAnsi)").into_bytes(),
            ),
        );
    }

    doc.finish()
}

/// The content stream for one page: the drawing operators, deflated.
///
/// Coordinates arrive measured from the top-left, which is how the layout engine
/// thinks. PDF's origin is the bottom-left, so every `y` is mirrored here and
/// nowhere else.
fn content_stream(page: &Page, options: &PageOptions) -> Vec<u8> {
    let height = options.page_size.height();
    let mut out = String::new();

    for mark in &page.marks {
        match mark {
            Mark::Fill {
                x,
                top,
                width,
                height: h,
                color,
            } => {
                out.push_str(&color.to_pdf_operator());
                out.push('\n');
                let y = height - top - h;
                let _ = writeln!(out, "{x:.4} {y:.4} {width:.4} {h:.4} re f");
            }
            Mark::Line {
                x1,
                y1,
                x2,
                y2,
                color,
                width,
            } => {
                out.push_str(&color.to_stroke_operator());
                out.push('\n');
                let _ = writeln!(out, "{width:.4} w");
                let _ = writeln!(
                    out,
                    "{x1:.4} {:.4} m {x2:.4} {:.4} l S",
                    height - y1,
                    height - y2
                );
            }
            Mark::Text {
                x,
                baseline,
                bytes,
                font,
                size,
                color,
            } => {
                out.push_str(&color.to_pdf_operator());
                out.push('\n');
                out.push_str("BT\n");
                let _ = writeln!(out, "/{} {size:.4} Tf", font_name(*font));
                let _ = writeln!(out, "1 0 0 1 {x:.4} {:.4} Tm", height - baseline);
                out.push('(');
                // The bytes are already WinAnsi, so they are escaped rather than
                // encoded again.
                for &byte in bytes {
                    match byte {
                        b'(' | b')' | b'\\' => {
                            out.push('\\');
                            out.push(byte as char);
                        }
                        0x20..=0x7e => out.push(byte as char),
                        _ => {
                            let _ = write!(out, "\\{byte:03o}");
                        }
                    }
                }
                out.push_str(") Tj\n");
                out.push_str("ET\n");
            }
        }
    }

    flate::compress(out.as_bytes())
}
