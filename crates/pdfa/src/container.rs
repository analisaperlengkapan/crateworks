//! Reading a PDF, embedding a file in it, and writing it back.
//!
//! # Why this rewrites the file instead of appending to it
//!
//! The usual way to add something to a PDF is an incremental update: keep every
//! original byte and append the changed objects plus a new cross-reference
//! section pointing back with `/Prev`. It is the gentler approach and it is what
//! a signing workflow needs, because a signature covers the bytes it was made
//! over and an incremental update leaves them untouched.
//!
//! This module rewrites instead, and the reason is that the objects have to be
//! renumbered either way to be safe. A Factur-X file needs new objects, and an
//! appended object number can collide with one the original defined further
//! down; a PDF whose objects are numbered twice is corrupt in a way that only
//! shows up in the reader that happens to pick the other definition. Rewriting
//! gives one numbering and one cross-reference table, which is what the
//! reference implementation produces too.
//!
//! The page content, fonts and images are copied through byte for byte. Nothing
//! here parses a content stream, so the visual output cannot change.
//!
//! # What it refuses
//!
//! A PDF that stores its objects in object streams, or that uses a
//! cross-reference stream rather than a table, is rejected with
//! [`Error::UnsupportedPdf`] rather than mangled. Both are legal and both are
//! common in files produced by newer tools; supporting them means parsing
//! compressed object containers, which is a much larger job than embedding a
//! file and is not worth guessing at.

use std::collections::BTreeMap;

use crate::object::{Object, read_literal_string};
use crate::xmp::Xmp;
use crate::{Error, Result};

/// A file to embed in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedFile {
    /// The name the file appears under, conventionally `factur-x.xml`.
    pub name: String,
    /// The bytes of the file.
    pub data: Vec<u8>,
    /// The MIME subtype, `xml` for a Factur-X attachment.
    pub subtype: String,
    /// The description shown in a reader's attachment panel.
    pub description: String,
    /// The associated-file relationship: `Data`, `Source`, `Alternative`.
    pub relationship: String,
}

impl EmbeddedFile {
    /// An XML attachment named `factur-x.xml`.
    #[must_use]
    pub fn factur_x(xml: impl Into<Vec<u8>>) -> Self {
        Self {
            name: "factur-x.xml".to_string(),
            data: xml.into(),
            subtype: "xml".to_string(),
            description: "Factur-X XML file".to_string(),
            relationship: "Data".to_string(),
        }
    }
}

/// The options for writing a PDF/A-3 document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOptions {
    /// The XMP packet to embed.
    pub xmp: Xmp,
    /// The timestamp written into the file's own dictionary.
    pub timestamp: String,
    /// Whether to compress the metadata stream.
    ///
    /// Left uncompressed by default. PDF/A allows either, but an uncompressed
    /// packet can be read out of the file with nothing more than a search, which
    /// is what a conformance checker and a human both end up doing.
    pub compress: bool,
}

/// A PDF, split into the pieces this crate needs.
///
/// Only the structure is read: the objects themselves are kept as the bytes they
/// arrived as.
#[derive(Debug, Clone)]
pub struct Pdf {
    bytes: Vec<u8>,
    /// Object number to the byte range of `N 0 obj ... endobj`.
    objects: BTreeMap<u32, (usize, usize)>,
    /// The trailer dictionary, as text.
    trailer: String,
    /// The object number of the catalog, from the trailer's `/Root`.
    root: u32,
    /// The object number of the document information dictionary, if any.
    info: Option<u32>,
    /// The declared `/Size`.
    size: u32,
}

impl Pdf {
    /// Reads the structure of a PDF.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedPdf`] for an object stream or a
    /// cross-reference stream, and [`Error::MissingPdfPart`] when the trailer or
    /// the catalog is absent.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if !bytes.starts_with(b"%PDF-") {
            return Err(Error::MissingPdfPart("the %PDF- header"));
        }
        // A cross-reference stream is announced by an /XRef type; an object
        // stream by /ObjStm. Either one means the objects are not laid out the
        // way this parser assumes.
        if bytes.windows(5).any(|w| w == b"/XRef") || bytes.windows(7).any(|w| w == b"/ObjStm") {
            return Err(Error::UnsupportedPdf(
                "object streams and cross-reference streams are not supported",
            ));
        }

        let mut objects = BTreeMap::new();
        let mut cursor = 0;
        while let Some(found) = find_object_header(bytes, cursor) {
            let (number, start) = found;
            let Some(end) = find(bytes, start, b"endobj") else {
                break;
            };
            let end = end + b"endobj".len();
            objects.insert(number, (start, end));
            cursor = end;
        }
        if objects.is_empty() {
            return Err(Error::MissingPdfPart("any object"));
        }

        let trailer_start =
            rfind(bytes, b"trailer").ok_or(Error::MissingPdfPart("the trailer dictionary"))?;
        let trailer_end = find(bytes, trailer_start, b"startxref")
            .ok_or(Error::MissingPdfPart("the startxref keyword"))?;
        let trailer = String::from_utf8_lossy(&bytes[trailer_start..trailer_end]).into_owned();

        let root = number_after(&trailer, "/Root")
            .ok_or(Error::MissingPdfPart("the /Root entry in the trailer"))?;
        let info = number_after(&trailer, "/Info");
        let size = number_after(&trailer, "/Size")
            .unwrap_or_else(|| objects.keys().copied().max().unwrap_or(0) + 1);

        Ok(Self {
            bytes: bytes.to_vec(),
            objects,
            trailer,
            root,
            info,
            size,
        })
    }

    /// The highest object number in the file.
    #[must_use]
    pub fn max_object_number(&self) -> u32 {
        self.objects.keys().copied().max().unwrap_or(0)
    }

    /// The bytes of one object, from `N 0 obj` to `endobj`.
    #[must_use]
    pub fn object_bytes(&self, number: u32) -> Option<&[u8]> {
        self.objects
            .get(&number)
            .map(|(start, end)| &self.bytes[*start..*end])
    }

    /// The catalog object as text.
    #[must_use]
    pub fn catalog(&self) -> Option<String> {
        self.object_bytes(self.root)
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }

    /// The declared `/Size`.
    #[must_use]
    pub fn declared_size(&self) -> u32 {
        self.size
    }

    /// The trailer as text.
    #[must_use]
    pub fn trailer_text(&self) -> &str {
        &self.trailer
    }

    /// The object number of the catalog.
    #[must_use]
    pub fn root_number(&self) -> u32 {
        self.root
    }
}

/// Writes a PDF/A-3 document that carries an embedded file and XMP metadata.
///
/// The original objects are copied through unchanged apart from the catalog,
/// which gains the entries that point at the new pieces.
///
/// # Errors
///
/// Returns an error if the original cannot be read, which happens for an object
/// stream or a cross-reference stream. See [`Pdf::parse`].
pub fn embed(original: &[u8], file: &EmbeddedFile, options: &WriteOptions) -> Result<Vec<u8>> {
    let pdf = Pdf::parse(original)?;

    // New objects, numbered past everything the original defined.
    let base = pdf.max_object_number() + 1;
    let file_stream = base;
    let filespec = base + 1;
    let associated_files = base + 2;
    let metadata = base + 3;
    let names = base + 4;
    let next = base + 5;

    let checksum = crate::digest::md5(&file.data);
    let mut out: Vec<u8> = Vec::with_capacity(original.len() + file.data.len() + 4096);
    out.extend_from_slice(b"%PDF-1.7\n");
    // A binary comment on the second line, which tells a transfer program that
    // the file is not text and must not be translated.
    out.extend_from_slice(b"%\xe2\xe3\xcf\xd3\n");

    let mut offsets: BTreeMap<u32, usize> = BTreeMap::new();
    for (&number, &(start, end)) in &pdf.objects {
        offsets.insert(number, out.len());
        if number == pdf.root {
            out.extend_from_slice(&updated_catalog(&pdf, metadata, associated_files, names)?);
            out.push(b'\n');
        } else {
            out.extend_from_slice(&pdf.bytes[start..end]);
            out.push(b'\n');
        }
    }

    // The embedded file.
    offsets.insert(file_stream, out.len());
    let mut params = BTreeMap::new();
    params.insert("CheckSum".to_string(), Object::HexString(checksum.to_vec()));
    params.insert(
        "ModDate".to_string(),
        Object::LiteralString(options.timestamp.clone()),
    );
    params.insert("Size".to_string(), Object::Integer(file.data.len() as i64));
    let mut stream_dict = BTreeMap::new();
    stream_dict.insert("Type".to_string(), Object::Name("EmbeddedFile".to_string()));
    stream_dict.insert(
        "Subtype".to_string(),
        Object::Name(format!("text/{}", file.subtype)),
    );
    stream_dict.insert("Params".to_string(), Object::Dictionary(params));
    if options.compress {
        stream_dict.insert(
            "Filter".to_string(),
            Object::Name("FlateDecode".to_string()),
        );
    }
    let data = if options.compress {
        crate::flate::compress(&file.data)
    } else {
        file.data.clone()
    };
    stream_dict.insert("Length".to_string(), Object::Integer(data.len() as i64));
    write_object_head(&mut out, file_stream, &Object::Dictionary(stream_dict));
    out.extend_from_slice(b"stream\n");
    out.extend_from_slice(&data);
    out.extend_from_slice(b"\nendstream\nendobj\n");

    // The file specification.
    offsets.insert(filespec, out.len());
    let mut filespec_dict = BTreeMap::new();
    filespec_dict.insert("Type".to_string(), Object::Name("Filespec".to_string()));
    filespec_dict.insert("F".to_string(), Object::LiteralString(file.name.clone()));
    filespec_dict.insert("UF".to_string(), Object::LiteralString(file.name.clone()));
    filespec_dict.insert(
        "Desc".to_string(),
        Object::LiteralString(file.description.clone()),
    );
    filespec_dict.insert(
        "AFRelationship".to_string(),
        Object::Name(file.relationship.clone()),
    );
    let mut ef = BTreeMap::new();
    ef.insert("F".to_string(), Object::Reference(file_stream));
    ef.insert("UF".to_string(), Object::Reference(file_stream));
    filespec_dict.insert("EF".to_string(), Object::Dictionary(ef));
    write_object(&mut out, filespec, &Object::Dictionary(filespec_dict));

    // The associated-files array.
    offsets.insert(associated_files, out.len());
    write_object(
        &mut out,
        associated_files,
        &Object::Array(vec![Object::Reference(filespec)]),
    );

    // The XMP metadata stream.
    offsets.insert(metadata, out.len());
    let xmp = options.xmp.to_xml().into_bytes();
    let mut metadata_dict = BTreeMap::new();
    metadata_dict.insert("Type".to_string(), Object::Name("Metadata".to_string()));
    metadata_dict.insert("Subtype".to_string(), Object::Name("XML".to_string()));
    let metadata_data = if options.compress {
        crate::flate::compress(&xmp)
    } else {
        xmp.clone()
    };
    if options.compress {
        metadata_dict.insert(
            "Filter".to_string(),
            Object::Name("FlateDecode".to_string()),
        );
    }
    metadata_dict.insert(
        "Length".to_string(),
        Object::Integer(metadata_data.len() as i64),
    );
    write_object_head(&mut out, metadata, &Object::Dictionary(metadata_dict));
    out.extend_from_slice(b"stream\n");
    out.extend_from_slice(&metadata_data);
    out.extend_from_slice(b"\nendstream\nendobj\n");

    // The name tree that lists the attachment.
    offsets.insert(names, out.len());
    let mut names_map = BTreeMap::new();
    names_map.insert(
        "Names".to_string(),
        Object::Name("EmbeddedFiles".to_string()),
    );
    let mut names_dict = BTreeMap::new();
    names_dict.insert("EmbeddedFiles".to_string(), Object::Dictionary(names_map));
    write_object(&mut out, names, &Object::Dictionary(names_dict));

    // The cross-reference table.
    let xref_offset = out.len();
    let total = next.max(pdf.size);
    out.extend_from_slice(format!("xref\n0 {total}\n").as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for number in 1..total {
        match offsets.get(&number) {
            Some(offset) => {
                out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
            }
            None => out.extend_from_slice(b"0000000000 65535 f \n"),
        }
    }

    let mut trailer = BTreeMap::new();
    trailer.insert("Size".to_string(), Object::Integer(i64::from(total)));
    trailer.insert("Root".to_string(), Object::Reference(pdf.root));
    if let Some(info) = pdf.info {
        trailer.insert("Info".to_string(), Object::Reference(info));
    }
    out.extend_from_slice(b"trailer\n");
    out.extend_from_slice(Object::Dictionary(trailer).write().as_bytes());
    out.push(b'\n');
    out.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF").as_bytes());

    Ok(out)
}

/// Writes `N 0 obj` and the object, then `endobj`.
fn write_object(out: &mut Vec<u8>, number: u32, object: &Object) {
    write_object_head(out, number, object);
    out.extend_from_slice(b"endobj\n");
}

/// Writes `N 0 obj` and the object without `endobj`.
///
/// A stream has to place its data before `endobj`, so it writes its own.
fn write_object_head(out: &mut Vec<u8>, number: u32, object: &Object) {
    out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
    out.extend_from_slice(object.write().as_bytes());
    out.push(b'\n');
}

/// The catalog with the entries that point at the new pieces added.
///
/// The original bytes are kept and the new keys are inserted before the closing
/// `>>`, so anything the original catalog carried survives untouched.
fn updated_catalog(pdf: &Pdf, metadata: u32, associated_files: u32, names: u32) -> Result<Vec<u8>> {
    let catalog = pdf
        .catalog()
        .ok_or(Error::MissingPdfPart("the catalog object"))?;
    let closing = catalog
        .rfind(">>")
        .ok_or(Error::MissingPdfPart("the end of the catalog dictionary"))?;

    let mut additions = String::new();
    // Only add /Metadata if the original did not have one, so a document that
    // already carries metadata is not left with two.
    if !catalog.contains("/Metadata") {
        additions.push_str(&format!("/Metadata {metadata} 0 R\n"));
    }
    if !catalog.contains("/AF") {
        additions.push_str(&format!("/AF [{associated_files} 0 R]\n"));
    }
    if !catalog.contains("/Names") {
        additions.push_str(&format!("/Names {names} 0 R\n"));
    }
    if !catalog.contains("/PageMode") {
        additions.push_str("/PageMode /UseAttachments\n");
    }

    let mut updated = String::with_capacity(catalog.len() + additions.len());
    updated.push_str(&catalog[..closing]);
    updated.push_str(&additions);
    updated.push_str(&catalog[closing..]);
    Ok(updated.into_bytes())
}

/// Finds the next `N 0 obj` header at or after `from`.
fn find_object_header(bytes: &[u8], from: usize) -> Option<(u32, usize)> {
    let mut cursor = from;
    while cursor < bytes.len() {
        let position = find(bytes, cursor, b" 0 obj")?;
        // Walk back over the digits that precede it.
        let mut start = position;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        // A header at the start of a line, so a `0 obj` inside a string does not
        // look like one.
        let at_line_start = start == 0 || bytes[start - 1] == b'\n' || bytes[start - 1] == b'\r';
        if start < position && at_line_start {
            let number = std::str::from_utf8(&bytes[start..position])
                .ok()
                .and_then(|text| text.trim().parse::<u32>().ok());
            if let Some(number) = number {
                return Some((number, start));
            }
        }
        cursor = position + 1;
    }
    None
}

/// Finds `needle` at or after `from`.
fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if from >= bytes.len() || needle.len() > bytes.len() - from {
        return None;
    }
    bytes[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| offset + from)
}

/// Finds `needle` at or before `from`, searching backwards from the end.
fn rfind(bytes: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > bytes.len() {
        return None;
    }
    (0..=bytes.len() - needle.len())
        .rev()
        .find(|&start| &bytes[start..start + needle.len()] == needle)
}

/// Reads the object number after a key such as `/Root` in a trailer.
fn number_after(text: &str, key: &str) -> Option<u32> {
    let position = text.find(key)? + key.len();
    let rest = text[position..].trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Reads a literal string value for a key in a dictionary, unescaping it.
#[must_use]
pub fn dictionary_string(text: &str, key: &str) -> Option<String> {
    let position = text.find(key)? + key.len();
    let rest = text[position..].trim_start();
    if let Some(inner) = rest.strip_prefix('(') {
        let end = inner.find(')')?;
        return Some(read_literal_string(&inner[..end]));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xmp::{DocumentInfo, Level};

    /// A small but valid PDF with one page, as a tool would write it.
    fn minimal_pdf() -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"%PDF-1.4\n");
        let mut offsets = Vec::new();
        offsets.push(out.len());
        out.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        offsets.push(out.len());
        out.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\n");
        let xref = out.len();
        out.extend_from_slice(b"xref\n0 3\n0000000000 65535 f \n");
        for offset in &offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        out
    }

    fn options() -> WriteOptions {
        WriteOptions {
            xmp: Xmp::factur_x(
                Level::En16931,
                DocumentInfo {
                    title: "Invoice INV-2026-0001".to_string(),
                    author: "Seller GmbH".to_string(),
                    subject: "Invoice INV-2026-0001".to_string(),
                    producer: "crateworks pdfa".to_string(),
                    creator_tool: "crateworks".to_string(),
                    timestamp: "2026-10-21T14:47:53+00:00".to_string(),
                },
            ),
            timestamp: "2026-10-21T14:47:53+00:00".to_string(),
            compress: false,
        }
    }

    #[test]
    fn a_minimal_pdf_is_read() {
        let pdf = Pdf::parse(&minimal_pdf()).unwrap();
        assert_eq!(pdf.root_number(), 1);
        assert_eq!(pdf.max_object_number(), 2);
        assert!(pdf.catalog().unwrap().contains("/Catalog"));
    }

    #[test]
    fn embedding_produces_a_pdf_that_still_reads() {
        let out = embed(&minimal_pdf(), &EmbeddedFile::factur_x("<x/>"), &options()).unwrap();
        let pdf = Pdf::parse(&out).unwrap();
        assert_eq!(pdf.root_number(), 1);
        // Five new objects: the stream, the file spec, the array, the metadata
        // and the name tree.
        assert_eq!(pdf.max_object_number(), 7);
    }

    #[test]
    fn the_catalog_points_at_everything_that_was_added() {
        let out = embed(&minimal_pdf(), &EmbeddedFile::factur_x("<x/>"), &options()).unwrap();
        let pdf = Pdf::parse(&out).unwrap();
        let catalog = pdf.catalog().unwrap();
        assert!(catalog.contains("/Metadata 6 0 R"), "{catalog}");
        assert!(catalog.contains("/AF [5 0 R]"), "{catalog}");
        assert!(catalog.contains("/Names 7 0 R"), "{catalog}");
        assert!(catalog.contains("/PageMode /UseAttachments"), "{catalog}");
        // The original entries survive.
        assert!(catalog.contains("/Pages 2 0 R"), "{catalog}");
    }

    #[test]
    fn the_embedded_file_carries_the_md5_of_its_bytes() {
        let xml = b"<invoice/>";
        let out = embed(
            &minimal_pdf(),
            &EmbeddedFile::factur_x(xml.to_vec()),
            &options(),
        )
        .unwrap();
        // A PDF hex string is conventionally uppercase.
        let expected = crate::digest::hex(&crate::digest::md5(xml)).to_ascii_uppercase();
        let text = String::from_utf8_lossy(&out);
        assert!(
            text.contains(&format!("/CheckSum <{expected}>")),
            "{expected}"
        );
    }

    #[test]
    fn the_embedded_file_keeps_its_bytes_intact() {
        let xml = b"<?xml version=\"1.0\"?><rsm:CrossIndustryInvoice/>";
        let out = embed(
            &minimal_pdf(),
            &EmbeddedFile::factur_x(xml.to_vec()),
            &options(),
        )
        .unwrap();
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("rsm:CrossIndustryInvoice"));
    }

    #[test]
    fn the_xmp_is_readable_straight_out_of_the_file() {
        // Uncompressed by default, so a search finds it.
        let out = embed(&minimal_pdf(), &EmbeddedFile::factur_x("<x/>"), &options()).unwrap();
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("<pdfaid:part>3</pdfaid:part>"));
        assert!(text.contains("<fx:ConformanceLevel>EN 16931</fx:ConformanceLevel>"));
    }

    #[test]
    fn the_xref_offsets_point_at_their_objects() {
        let out = embed(&minimal_pdf(), &EmbeddedFile::factur_x("<x/>"), &options()).unwrap();
        let text = String::from_utf8_lossy(&out);
        // `startxref` also contains the word, so the table is found by the
        // newline that precedes it.
        let xref_start = text.find("\nxref\n").unwrap();
        let body = &text[xref_start..];
        // The first entry is object 0, the free one; object 1 follows it.
        let first_free = body.find("0000000000 65535 f ").unwrap();
        let mut lines = body[first_free..].lines();
        lines.next();
        for number in 1..=7u32 {
            let line = lines.next().unwrap();
            let offset: usize = line[..10].parse().unwrap();
            let at = &out[offset..];
            assert!(
                at.starts_with(format!("{number} 0 obj").as_bytes()),
                "entry {number} points at {:?}",
                String::from_utf8_lossy(&at[..24.min(at.len())])
            );
        }
    }

    #[test]
    fn a_document_that_already_has_metadata_does_not_get_a_second_one() {
        let mut original = minimal_pdf();
        let text = String::from_utf8_lossy(&original).replace(
            "/Type /Catalog /Pages 2 0 R",
            "/Type /Catalog /Pages 2 0 R /Metadata 9 0 R",
        );
        original = text.into_bytes();
        let out = embed(&original, &EmbeddedFile::factur_x("<x/>"), &options()).unwrap();
        let catalog = Pdf::parse(&out).unwrap().catalog().unwrap();
        assert_eq!(catalog.matches("/Metadata").count(), 1, "{catalog}");
    }

    #[test]
    fn an_object_stream_is_refused_rather_than_mangled() {
        let mut bytes = minimal_pdf();
        bytes.extend_from_slice(b"\n/ObjStm inside\n");
        assert!(matches!(Pdf::parse(&bytes), Err(Error::UnsupportedPdf(_))));
    }

    #[test]
    fn a_file_that_is_not_a_pdf_is_refused() {
        assert!(matches!(
            Pdf::parse(b"not a pdf"),
            Err(Error::MissingPdfPart(_))
        ));
    }

    #[test]
    fn a_string_value_is_read_back_unescaped() {
        assert_eq!(
            dictionary_string("<< /F (factur\\055x\\056xml) >>", "/F").unwrap(),
            "factur-x.xml"
        );
    }
}
