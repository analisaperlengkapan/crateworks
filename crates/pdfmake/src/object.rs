//! Assembling the bytes of a PDF file.
//!
//! A PDF is a sequence of numbered objects, a cross-reference table saying
//! where each one starts, and a trailer pointing at the table. Nothing here
//! knows about pages or text; the layout engine builds objects and this module
//! writes them out.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// An object number. Object 0 is reserved for the free-list head.
pub(crate) type ObjectId = u32;

/// A PDF object.
#[derive(Debug, Clone)]
pub(crate) enum Object {
    /// A dictionary, written as `<< /Key Value ... >>`.
    Dict(Dict),
    /// A stream: a dictionary, the keyword `stream`, the bytes, `endstream`.
    Stream {
        /// The stream dictionary. `/Length` is added automatically.
        dict: Dict,
        /// The stream bytes.
        data: Vec<u8>,
    },
}

impl Object {
    /// Render this object as PDF syntax.
    pub(crate) fn write(&self, out: &mut Vec<u8>) {
        match self {
            Object::Dict(dict) => {
                let mut s = String::new();
                dict.write(&mut s);
                out.extend_from_slice(s.as_bytes());
            }
            Object::Stream { dict, data } => {
                let mut dict = dict.clone();
                dict.set("Length", Value::Int(data.len() as i64));
                let mut s = String::new();
                dict.write(&mut s);
                out.extend_from_slice(s.as_bytes());
                out.extend_from_slice(b"\nstream\n");
                // The stream bytes go in raw. They are usually compressed, so
                // they are not text and must not pass through a `String`.
                out.extend_from_slice(data);
                out.extend_from_slice(b"\nendstream");
            }
        }
    }
}

/// A value inside a dictionary or array.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    /// A whole number.
    Int(i64),
    /// A decimal number, written without an exponent.
    Real(f32),
    /// A name, written as `/Name`.
    Name(String),
    /// A reference to another object, written as `12 0 R`.
    Ref(ObjectId),
    /// A string literal.
    Text(Vec<u8>),
    /// An array of values.
    Array(Vec<Value>),
    /// A nested dictionary.
    Dict(Dict),
}

impl Value {
    /// Render this value as PDF syntax.
    pub(crate) fn write(&self, out: &mut String) {
        match self {
            Value::Int(n) => {
                let _ = write!(out, "{n}");
            }
            Value::Real(x) => write_real(*x, out),
            Value::Name(name) => {
                out.push('/');
                for c in name.chars() {
                    // A name cannot contain whitespace or a delimiter, so those
                    // bytes are written as `#xx`.
                    if c.is_ascii_alphanumeric() || "-_+.".contains(c) {
                        out.push(c);
                    } else {
                        let _ = write!(out, "#{:02X}", c as u32);
                    }
                }
            }
            Value::Ref(id) => {
                let _ = write!(out, "{id} 0 R");
            }
            Value::Text(bytes) => write_text_string(bytes, out),
            Value::Array(values) => {
                out.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(' ');
                    }
                    value.write(out);
                }
                out.push(']');
            }
            Value::Dict(dict) => dict.write(out),
        }
    }
}

/// Write a real number the way PDF wants it: no exponent, no trailing zeros.
fn write_real(value: f32, out: &mut String) {
    let mut s = format!("{value:.4}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    // `-0` is legal but pointless, and makes output comparisons noisy.
    if s == "-0" {
        s = "0".to_string();
    }
    out.push_str(&s);
}

/// Write a byte string in parentheses, escaping the characters PDF reserves.
fn write_text_string(bytes: &[u8], out: &mut String) {
    out.push('(');
    for &byte in bytes {
        match byte {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(byte as char);
            }
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            // Below 0x20 and above 0x7E are written as octal so the output is
            // plain ASCII whatever the text contains.
            0x20..=0x7e => out.push(byte as char),
            _ => {
                let _ = write!(out, "\\{byte:03o}");
            }
        }
    }
    out.push(')');
}

/// A dictionary, which keeps its keys in a fixed order so output is stable.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Dict {
    entries: BTreeMap<&'static str, Value>,
}

impl Dict {
    /// An empty dictionary.
    pub(crate) fn new() -> Dict {
        Dict {
            entries: BTreeMap::new(),
        }
    }

    /// Set a key, replacing any previous value.
    pub(crate) fn set(&mut self, key: &'static str, value: Value) -> &mut Dict {
        self.entries.insert(key, value);
        self
    }

    /// Render as `<< /Key Value ... >>`.
    pub(crate) fn write(&self, out: &mut String) {
        out.push_str("<<");
        for (key, value) in &self.entries {
            out.push_str(" /");
            out.push_str(key);
            out.push(' ');
            value.write(out);
        }
        out.push_str(" >>");
    }
}

/// A PDF being assembled.
///
/// Objects are added in any order and numbered as they are added. Object 0 is
/// the head of the free list and is created by the writer, so the first object
/// a caller adds is number 1.
#[derive(Debug)]
pub(crate) struct Document {
    objects: Vec<Object>,
    info: Dict,
}

impl Document {
    /// A document with no objects yet.
    pub(crate) fn new() -> Document {
        Document {
            objects: Vec::new(),
            info: Dict::new(),
        }
    }

    /// Add an object and return its id.
    pub(crate) fn add(&mut self, object: Object) -> ObjectId {
        self.objects.push(object);
        self.objects.len() as ObjectId
    }

    /// Reserve an id to be filled in later, for a cycle.
    ///
    /// A page tree needs a reference to its parent, and the parent needs a
    /// reference to the page, so one of the two has to be placed before the
    /// other exists.
    pub(crate) fn reserve(&mut self) -> ObjectId {
        self.objects.push(Object::Dict(Dict::new()));
        self.objects.len() as ObjectId
    }

    /// Replace an object previously reserved.
    pub(crate) fn replace(&mut self, id: ObjectId, object: Object) {
        self.objects[(id - 1) as usize] = object;
    }

    /// Set a key in the document information dictionary.
    pub(crate) fn set_info(&mut self, key: &'static str, value: Value) {
        self.info.set(key, value);
    }

    /// Serialise the whole file.
    ///
    /// The header declares PDF 1.7. The binary comment on the second line is
    /// what tells a transfer program the file is not text, which the
    /// specification requires.
    pub(crate) fn finish(self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        // The second line is a comment holding four bytes above 0x7F, which is
        // how the specification marks a file as binary.
        out.extend_from_slice(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n");
        let mut offsets = Vec::with_capacity(self.objects.len() + 1);
        let mut scratch = String::new();

        for (index, object) in self.objects.iter().enumerate() {
            offsets.push(out.len());
            scratch.clear();
            let _ = writeln!(scratch, "{} 0 obj", index + 1);
            out.extend_from_slice(scratch.as_bytes());
            object.write(&mut out);
            out.extend_from_slice(b"\nendobj\n");
        }

        let info_id = if self.info.entries.is_empty() {
            None
        } else {
            offsets.push(out.len());
            scratch.clear();
            let _ = writeln!(scratch, "{} 0 obj", self.objects.len() + 1);
            out.extend_from_slice(scratch.as_bytes());
            let mut s = String::new();
            self.info.write(&mut s);
            out.extend_from_slice(s.as_bytes());
            out.extend_from_slice(b"\nendobj\n");
            Some(self.objects.len() as ObjectId + 1)
        };

        let xref_offset = out.len();
        let count = offsets.len() + 1;
        scratch.clear();
        let _ = writeln!(scratch, "xref\n0 {count}");
        out.extend_from_slice(scratch.as_bytes());
        // The free-list head: object 0, generation 65535, in use never.
        out.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            scratch.clear();
            let _ = writeln!(scratch, "{offset:010} 00000 n ");
            out.extend_from_slice(scratch.as_bytes());
        }

        scratch.clear();
        let _ = write!(scratch, "trailer\n<< /Size {count} /Root 1 0 R");
        if let Some(id) = info_id {
            let _ = write!(scratch, " /Info {id} 0 R");
        }
        let _ = writeln!(scratch, " >>\nstartxref\n{xref_offset}\n%%EOF");
        out.extend_from_slice(scratch.as_bytes());

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(object: &Object) -> String {
        let mut bytes = Vec::new();
        object.write(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Index of `needle` in `haystack`.
    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    fn render_value(value: Value) -> String {
        let mut s = String::new();
        value.write(&mut s);
        s
    }

    #[test]
    fn integers_and_reals_lose_trailing_zeros() {
        let mut s = String::new();
        Value::Real(1.5).write(&mut s);
        assert_eq!(s, "1.5");

        let mut s = String::new();
        Value::Real(10.0).write(&mut s);
        assert_eq!(s, "10");

        let mut s = String::new();
        Value::Real(-0.0).write(&mut s);
        assert_eq!(s, "0");

        let mut s = String::new();
        Value::Int(-7).write(&mut s);
        assert_eq!(s, "-7");
    }

    #[test]
    fn names_escape_delimiters() {
        let mut s = String::new();
        Value::Name("Helvetica-Bold".into()).write(&mut s);
        assert_eq!(s, "/Helvetica-Bold");

        let mut s = String::new();
        Value::Name("a b".into()).write(&mut s);
        assert_eq!(s, "/a#20b");
    }

    #[test]
    fn references_are_written_with_a_generation() {
        let mut s = String::new();
        Value::Ref(12).write(&mut s);
        assert_eq!(s, "12 0 R");
    }

    #[test]
    fn strings_escape_the_reserved_characters() {
        assert_eq!(
            render_value(Value::Text(b"a(b)c\\d".to_vec())),
            "(a\\(b\\)c\\\\d)"
        );
        assert_eq!(render_value(Value::Text(b"line\n".to_vec())), "(line\\n)");
        // A byte outside printable ASCII becomes octal, so the file stays ASCII.
        assert_eq!(render_value(Value::Text(vec![0xe9])), "(\\351)");
    }

    #[test]
    fn dictionaries_are_written_in_a_stable_order() {
        let mut dict = Dict::new();
        dict.set("Type", Value::Name("Page".into()));
        dict.set("Count", Value::Int(1));
        assert_eq!(render(&Object::Dict(dict)), "<< /Count 1 /Type /Page >>");
    }

    #[test]
    fn a_stream_gains_its_length() {
        let mut dict = Dict::new();
        dict.set("Filter", Value::Name("FlateDecode".into()));
        let object = Object::Stream {
            dict,
            data: b"abc".to_vec(),
        };
        assert_eq!(
            render(&object),
            "<< /Filter /FlateDecode /Length 3 >>\nstream\nabc\nendstream"
        );
    }

    #[test]
    fn the_trailer_points_at_the_cross_reference_table() {
        let mut doc = Document::new();
        let pages = doc.add(Object::Dict(Dict::new()));
        assert_eq!(pages, 1);
        doc.replace(pages, {
            let mut dict = Dict::new();
            dict.set("Type", Value::Name("Catalog".into()));
            Object::Dict(dict)
        });
        let bytes = doc.finish();
        // The header's binary comment holds bytes above 0x7F, so the file is
        // searched as bytes; decoding it as UTF-8 would move every offset.
        assert!(bytes.starts_with(b"%PDF-1.7\n"));
        assert!(find(&bytes, b"xref\n0 2\n").is_some());
        assert!(find(&bytes, b"/Root 1 0 R").is_some());
        assert!(bytes.ends_with(b"%%EOF\n"));
        // The offset in `startxref` must be where `xref` really begins.
        let xref_at = find(&bytes, b"\nxref\n").unwrap() + 1;
        let declared: usize =
            std::str::from_utf8(&bytes[find(&bytes, b"startxref\n").unwrap() + 10..])
                .unwrap()
                .lines()
                .next()
                .unwrap()
                .trim()
                .parse()
                .unwrap();
        assert_eq!(declared, xref_at);
    }
}
