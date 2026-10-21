//! Checking whether a PDF is a conformant PDF/A-3 container.
//!
//! # What this checks, and what it does not
//!
//! Full PDF/A validation is the ISO 19005 rules applied to everything a PDF can
//! contain: colour spaces must be device-independent, every font must be
//! embedded, no encryption, no transparency in the wrong place, and dozens more.
//! That is what veraPDF does, and a small crate that claimed to do it would be
//! lying.
//!
//! What this checks is the container contract that Factur-X adds on top of
//! PDF/A-3, which is a much smaller and completely specified set:
//!
//! * the file starts with a PDF header,
//! * it carries XMP metadata that claims PDF/A-3,
//! * the catalog points at that metadata with `/Metadata`,
//! * an embedded file with the agreed name is present,
//! * the embedded file's `/CheckSum` matches the bytes, which is the failure
//!   that actually happens: the XML is edited and the checksum is not updated,
//! * the embedded file's `/AFRelationship` is set, as PDF/A-3 requires.
//!
//! The distinction is stated in the report rather than hidden, because a caller
//! who reads "valid" and ships a document to a tax authority should know exactly
//! what was and was not proven.
//!
//! # Reading bytes, not text
//!
//! Everything here searches the raw bytes. A PDF carries a binary comment on its
//! second line, and that comment is not valid UTF-8; a lossy conversion replaces
//! those bytes with U+FFFD, which shifts every offset after them so that a
//! string index and a byte index stop agreeing. A stream's `/Length` is measured
//! in bytes for the same reason.

use crate::container::Pdf;
use crate::digest::{hex, md5};
use crate::xmp::Xmp;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The document is not a usable PDF/A container at all.
    Error,
    /// The document is readable but something a validator would flag.
    Warning,
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// How serious it is.
    pub severity: Severity,
    /// A short name, stable enough to match on.
    pub code: &'static str,
    /// What was found, in terms a caller can act on.
    pub message: String,
}

/// The result of checking a document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// What was found, in the order the checks ran.
    pub findings: Vec<Finding>,
    /// The embedded file's name, when one was found.
    pub embedded_file: Option<String>,
}

impl Report {
    /// Whether nothing serious was found.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self
            .findings
            .iter()
            .any(|finding| finding.severity == Severity::Error)
    }

    /// Whether there is nothing to report at all.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// The codes that were reported.
    #[must_use]
    pub fn codes(&self) -> Vec<&'static str> {
        self.findings.iter().map(|finding| finding.code).collect()
    }

    fn error(&mut self, code: &'static str, message: impl Into<String>) {
        self.findings.push(Finding {
            severity: Severity::Error,
            code,
            message: message.into(),
        });
    }

    fn warn(&mut self, code: &'static str, message: impl Into<String>) {
        self.findings.push(Finding {
            severity: Severity::Warning,
            code,
            message: message.into(),
        });
    }
}

/// Checks a Factur-X container.
///
/// `expected_file_name` is the attachment the caller expects, `factur-x.xml` for
/// an invoice. Pass `None` to accept whatever name is present.
#[must_use]
pub fn check_factur_x(bytes: &[u8], expected_file_name: Option<&str>) -> Report {
    let mut report = Report::default();

    let pdf = match Pdf::parse(bytes) {
        Ok(pdf) => pdf,
        Err(error) => {
            report.error("structure", error.to_string());
            return report;
        }
    };

    check_metadata(bytes, &pdf, &mut report);
    report.embedded_file = check_attachment(bytes, &pdf, &mut report, expected_file_name);
    report
}

/// Checks that the metadata is present, is PDF/A-3, and is reachable.
fn check_metadata(bytes: &[u8], pdf: &Pdf, report: &mut Report) {
    let catalog = pdf.catalog().unwrap_or_default();
    if !catalog.contains("/Metadata") {
        report.error(
            "metadata-not-referenced",
            "the catalog has no /Metadata entry, so the document does not claim PDF/A",
        );
    }

    let Some(packet) = metadata_packet(bytes) else {
        // A metadata stream that exists but cannot be read is a different matter
        // from one that is absent, and the caller needs to tell them apart: the
        // first is a limitation of this reader, the second is a document that
        // does not claim PDF/A at all.
        if find_from(bytes, 0, b"/Type /Metadata").is_some() {
            report.warn(
                "metadata-unreadable",
                "the metadata stream is compressed and could not be read, so its claims were not checked",
            );
        } else {
            report.error(
                "metadata-missing",
                "the document carries no XMP packet, so it cannot claim PDF/A",
            );
        }
        return;
    };

    match Xmp::parse(&packet) {
        Ok(xmp) => {
            if xmp.part != 3 {
                report.error(
                    "pdfa-part",
                    format!(
                        "Factur-X requires PDF/A-3 and the metadata claims part {}",
                        xmp.part
                    ),
                );
            }
            if xmp.factur_x.is_none() {
                report.warn(
                    "factur-x-metadata-missing",
                    "the XMP does not describe an embedded Factur-X document",
                );
            }
        }
        Err(error) => report.error("metadata-malformed", error.to_string()),
    }
}

/// Checks the embedded attachment, returning its name.
fn check_attachment(
    bytes: &[u8],
    pdf: &Pdf,
    report: &mut Report,
    expected_file_name: Option<&str>,
) -> Option<String> {
    let catalog = pdf.catalog().unwrap_or_default();
    if !catalog.contains("/AF") && !catalog.contains("/Names") {
        report.error(
            "attachment-missing",
            "the catalog has neither /AF nor /Names, so nothing is embedded",
        );
        return None;
    }
    if find_from(bytes, 0, b"/Type /EmbeddedFile").is_none() {
        report.error(
            "attachment-missing",
            "no object of type /EmbeddedFile was found",
        );
        return None;
    }

    let name = filespec_name(bytes);
    if let Some(expected) = expected_file_name {
        let present = name.as_deref() == Some(expected)
            || find_from(bytes, 0, format!("({expected})").as_bytes()).is_some();
        if !present {
            report.error(
                "attachment-name",
                format!("an embedded file is present but not one named {expected}"),
            );
        }
    }

    if find_from(bytes, 0, b"/AFRelationship").is_none() {
        report.warn(
            "afrelationship-missing",
            "PDF/A-3 requires the attached file to declare an /AFRelationship",
        );
    }

    check_checksum(bytes, report);
    name
}

/// Checks the embedded file's `/CheckSum` against its bytes.
///
/// This is the failure worth catching: the XML is regenerated and the checksum
/// in the container is left at its old value, which a validator rejects and a
/// reader may not.
fn check_checksum(bytes: &[u8], report: &mut Report) {
    let Some(checksum_position) = find_from(bytes, 0, b"/CheckSum") else {
        report.warn(
            "checksum-missing",
            "the embedded file declares no /CheckSum",
        );
        return;
    };
    let rest = bytes[checksum_position + b"/CheckSum".len()..].trim_ascii_start();
    let Some(rest) = rest.strip_prefix(b"<") else {
        report.warn("checksum-malformed", "the /CheckSum is not a hex string");
        return;
    };
    let Some(close) = rest.iter().position(|byte| *byte == b'>') else {
        report.warn("checksum-malformed", "the /CheckSum is not a hex string");
        return;
    };
    let hex_digits = String::from_utf8_lossy(&rest[..close]).into_owned();

    let Some(data) = embedded_stream_data(bytes) else {
        report.warn(
            "checksum-unchecked",
            "the embedded stream could not be read, so its checksum was not checked",
        );
        return;
    };

    let actual = hex(&md5(&data));
    if !actual.eq_ignore_ascii_case(&hex_digits) {
        report.error(
            "checksum-mismatch",
            format!(
                "the embedded file hashes to {actual} but the container says {}",
                hex_digits.to_ascii_lowercase()
            ),
        );
    }
}

/// The XMP packet, inflating it if it was written compressed.
fn metadata_packet(bytes: &[u8]) -> Option<String> {
    if let Some(start) = find_from(bytes, 0, b"<?xpacket") {
        let end = find_from(bytes, start, b"<?xpacket end")
            .map(|at| at + b"<?xpacket end=\"w\"?>".len())
            .unwrap_or(bytes.len());
        return Some(String::from_utf8_lossy(bytes.get(start..end)?).into_owned());
    }

    // Compressed: read the metadata stream and inflate a stored zlib stream,
    // which is the form this crate writes. A packet compressed with real deflate
    // is not readable here, so nothing is returned.
    let stream_start = find_from(bytes, 0, b"/Type /Metadata")?;
    let data = stream_bytes(bytes, stream_start)?;
    let inflated = crate::flate::inflate_stored(&data)?;
    String::from_utf8(inflated).ok()
}

/// The bytes of an embedded file stream, decompressed if it is stored.
///
/// The stream's `/Length` is used to find the end of the data. Searching for the
/// `endstream` keyword instead would be wrong for a compressed stream, whose
/// bytes are arbitrary and can contain a newline, so stripping the separator
/// would eat a real byte.
fn embedded_stream_data(bytes: &[u8]) -> Option<Vec<u8>> {
    let stream_start = find_from(bytes, 0, b"/Type /EmbeddedFile")?;
    let start = object_start(bytes, stream_start);
    let stream_keyword = find_from(bytes, start, b"stream")?;
    let compressed = find_from(bytes.get(start..stream_keyword)?, 0, b"/Filter").is_some();
    let data = stream_bytes(bytes, stream_start)?;
    if compressed {
        // This crate writes a stored zlib stream, which can be inflated without
        // a full deflate implementation. Anything else is reported as unchecked.
        return crate::flate::inflate_stored(&data);
    }
    Some(data)
}

/// The raw bytes under `stream` in the object that contains `marker`.
fn stream_bytes(bytes: &[u8], marker: usize) -> Option<Vec<u8>> {
    let start = object_start(bytes, marker);
    let stream_keyword = find_from(bytes, start, b"stream")?;
    let length = length_value(bytes.get(start..stream_keyword)?)?;
    // Step over `stream` and the single newline that separates it from the data.
    let after_keyword = bytes.get(stream_keyword + b"stream".len()..)?;
    let skip = if after_keyword.starts_with(b"\r\n") {
        2
    } else if after_keyword.starts_with(b"\n") {
        1
    } else {
        0
    };
    let data_offset = stream_keyword + b"stream".len() + skip;
    Some(bytes.get(data_offset..data_offset + length)?.to_vec())
}

/// The integer after `/Length` in a stream dictionary.
fn length_value(header: &[u8]) -> Option<usize> {
    let at = find_from(header, 0, b"/Length")? + b"/Length".len();
    let rest = header.get(at..)?;
    let digits: Vec<u8> = rest
        .iter()
        .skip_while(|byte| byte.is_ascii_whitespace())
        .take_while(|byte| byte.is_ascii_digit())
        .copied()
        .collect();
    std::str::from_utf8(&digits).ok()?.parse().ok()
}

/// Finds a byte sequence at or after `from`.
fn find_from(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| offset + from)
}

/// The index where the object containing `before` starts.
///
/// An object header is `N 0 obj` at the beginning of a line, so the start is the
/// first byte after the preceding newline. Counting digits backwards from the
/// `obj` keyword does not work: the header is `N 0 obj`, and the `0` sits
/// between the number and the keyword.
fn object_start(bytes: &[u8], before: usize) -> usize {
    let at = find_backwards(bytes, before, b" obj").map_or(0, |at| at + 1);
    let mut start = at;
    while start > 0 && bytes[start - 1] != b'\n' && bytes[start - 1] != b'\r' {
        start -= 1;
    }
    start
}

/// Finds the last occurrence of `needle` that starts before `before`.
fn find_backwards(bytes: &[u8], before: usize, needle: &[u8]) -> Option<usize> {
    let limit = before.min(bytes.len());
    if needle.len() > limit {
        return None;
    }
    (0..=limit - needle.len())
        .rev()
        .find(|&start| &bytes[start..start + needle.len()] == needle)
}

/// Reads the name out of the file specification, for callers that want it.
#[must_use]
pub fn attachment_name(bytes: &[u8]) -> Option<String> {
    Pdf::parse(bytes).ok()?;
    filespec_name(bytes)
}

/// The `/UF` or `/F` name of the file specification object.
///
/// The name is the one followed by a literal string. An `/EF` sub-dictionary
/// also has `/F` and `/UF` keys, but their values are references, so looking for
/// the string is what tells the two apart.
fn filespec_name(bytes: &[u8]) -> Option<String> {
    let position = find_from(bytes, 0, b"/Type /Filespec")?;
    let start = object_start(bytes, position);
    let end = find_from(bytes, position, b"endobj").unwrap_or(bytes.len());
    let object = bytes.get(start..end)?;

    for key in [b"/UF".as_slice(), b"/F".as_slice()] {
        let mut search = object;
        while let Some(at) = find_from(search, 0, key) {
            let rest = search[at + key.len()..].trim_ascii_start();
            if let Some(inner) = rest.strip_prefix(b"(")
                && let Some(close) = inner.iter().position(|byte| *byte == b')')
            {
                let text = String::from_utf8_lossy(&inner[..close]);
                return Some(crate::object::read_literal_string(&text));
            }
            search = &search[at + key.len()..];
        }
    }
    None
}

/// The embedded file's bytes, if they can be read.
#[must_use]
pub fn embedded_file(bytes: &[u8]) -> Option<Vec<u8>> {
    embedded_stream_data(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::{EmbeddedFile, WriteOptions, embed};
    use crate::xmp::{DocumentInfo, Level};

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
            xmp: Xmp::factur_x(Level::En16931, DocumentInfo::default()),
            timestamp: "2026-10-21T14:47:53+00:00".to_string(),
            compress: false,
        }
    }

    fn built() -> Vec<u8> {
        embed(
            &minimal_pdf(),
            &EmbeddedFile::factur_x(b"<invoice/>".to_vec()),
            &options(),
        )
        .unwrap()
    }

    #[test]
    fn a_document_we_built_passes_our_own_checks() {
        let report = check_factur_x(&built(), Some("factur-x.xml"));
        assert!(report.is_valid(), "{:?}", report.findings);
        assert_eq!(report.embedded_file.as_deref(), Some("factur-x.xml"));
    }

    #[test]
    fn the_embedded_file_reads_back_unchanged() {
        assert_eq!(embedded_file(&built()).unwrap(), b"<invoice/>");
    }

    #[test]
    fn a_plain_pdf_without_metadata_is_reported() {
        let report = check_factur_x(&minimal_pdf(), Some("factur-x.xml"));
        assert!(!report.is_valid());
        assert!(
            report.codes().contains(&"metadata-missing"),
            "{:?}",
            report.codes()
        );
    }

    #[test]
    fn an_edit_that_forgets_the_checksum_is_caught() {
        // The failure this check exists for: change the XML, leave the checksum.
        let built = built();
        let text = String::from_utf8_lossy(&built).replace("<invoice/>", "<invoice>x</invoice>");
        let report = check_factur_x(text.as_bytes(), Some("factur-x.xml"));
        assert!(
            report.codes().contains(&"checksum-mismatch"),
            "{:?}",
            report.findings
        );
        assert!(!report.is_valid());
    }

    #[test]
    fn the_wrong_attachment_name_is_caught() {
        let report = check_factur_x(&built(), Some("order-x.xml"));
        assert!(
            report.codes().contains(&"attachment-name"),
            "{:?}",
            report.findings
        );
    }

    #[test]
    fn a_compressed_stream_is_inflated_rather_than_reported_as_wrong() {
        let mut opts = options();
        opts.compress = true;
        let built = embed(
            &minimal_pdf(),
            &EmbeddedFile::factur_x(b"<invoice/>".to_vec()),
            &opts,
        )
        .unwrap();
        assert_eq!(embedded_file(&built).unwrap(), b"<invoice/>");
        let report = check_factur_x(&built, Some("factur-x.xml"));
        assert!(report.is_valid(), "{:?}", report.findings);
    }

    #[test]
    fn the_attachment_name_helper_agrees_with_the_report() {
        assert_eq!(attachment_name(&built()).as_deref(), Some("factur-x.xml"));
    }

    #[test]
    fn the_binary_comment_does_not_shift_offsets() {
        // A PDF carries non-UTF-8 bytes on its second line, and reading the file
        // as text would move every offset after them. The attachment still reads
        // back, which is what proves the byte handling is right.
        let built = built();
        assert!(built.windows(4).any(|w| w == b"\xe2\xe3\xcf\xd3"));
        assert_eq!(embedded_file(&built).unwrap(), b"<invoice/>");
    }
}
