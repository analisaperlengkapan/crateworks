//! PDF/A conformance, XMP metadata, and the Factur-X container.
//!
//! `factur-x` in this workspace writes the EN 16931 invoice as XML. That XML is
//! not the invoice a tax authority accepts: it has to travel inside a PDF/A-3
//! document as an attached file, with metadata that says so. This crate is that
//! missing half.
//!
//! # Example
//!
//! ```
//! use pdfa::{DocumentInfo, EmbeddedFile, Level, WriteOptions, Xmp, embed};
//!
//! // Any ordinary PDF, produced by whatever the caller already uses.
//! let original = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
//! 2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\n\
//! xref\n0 3\n0000000000 65535 f \n0000000009 00000 n \n0000000063 00000 n \n\
//! trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n133\n%%EOF\n";
//!
//! let xml = b"<rsm:CrossIndustryInvoice/>";
//! let info = DocumentInfo {
//!     title: "Invoice INV-2026-0001".into(),
//!     author: "Seller GmbH".into(),
//!     subject: "Invoice INV-2026-0001".into(),
//!     producer: "crateworks".into(),
//!     creator_tool: "crateworks".into(),
//!     timestamp: "2026-10-21T14:47:53+00:00".into(),
//! };
//! let options = WriteOptions {
//!     xmp: Xmp::factur_x(Level::En16931, info),
//!     timestamp: "2026-10-21T14:47:53+00:00".into(),
//!     compress: false,
//! };
//!
//! let container = embed(original, &EmbeddedFile::factur_x(xml.to_vec()), &options).unwrap();
//!
//! // And check what was built.
//! let report = pdfa::check_factur_x(&container, Some("factur-x.xml"));
//! assert!(report.is_valid(), "{:?}", report.findings);
//! assert_eq!(pdfa::embedded_file(&container).unwrap(), xml);
//! ```
//!
//! # What the checker proves
//!
//! [`check_factur_x`] verifies the container contract: the metadata claims
//! PDF/A-3, the catalog points at it, the attachment is present under the agreed
//! name, it declares a relationship, and its checksum matches its bytes. That
//! last one is the failure that actually happens in practice, because the XML is
//! regenerated and the checksum is left behind.
//!
//! It does not apply the full ISO 19005 rules. Device-independent colour, every
//! font embedded, no encryption, transparency restrictions: that is veraPDF's
//! job and a small crate claiming it would be lying. The report says what was
//! checked so a caller who reads "valid" knows exactly what that means.
//!
//! # What it refuses
//!
//! A PDF whose objects live in object streams, or that uses a cross-reference
//! stream, is refused rather than mangled. Both are legal and both are common in
//! output from newer tools, and handling them means parsing compressed object
//! containers. Rewriting a file the parser does not fully understand would risk
//! corrupting it silently, which is worse than saying no.

#![forbid(unsafe_code)]

/// The README, compiled as a doctest so its example cannot drift from the API.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

pub mod conformance;
pub mod container;
pub mod digest;
pub mod flate;
pub mod object;
pub mod xmp;

pub use conformance::{Finding, Report, Severity, attachment_name, check_factur_x, embedded_file};
pub use container::{EmbeddedFile, Pdf, WriteOptions, embed};
pub use xmp::{DocumentInfo, FacturX, Level, Xmp};

/// A failure while reading or writing a PDF/A container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The file is not well-formed XML.
    Xml(String),
    /// A part of the PDF that has to be there is not.
    MissingPdfPart(&'static str),
    /// A PDF feature this crate does not handle.
    UnsupportedPdf(&'static str),
    /// The metadata is missing something required.
    MissingXmp(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xml(message) => write!(f, "invalid XML: {message}"),
            Self::MissingPdfPart(part) => write!(f, "the document has no {part}"),
            Self::UnsupportedPdf(what) => write!(f, "unsupported PDF feature: {what}"),
            Self::MissingXmp(field) => write!(f, "the XMP metadata has no {field}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<xmlkit::Error> for Error {
    fn from(error: xmlkit::Error) -> Self {
        Self::Xml(error.to_string())
    }
}

/// The result type for this crate.
pub type Result<T> = std::result::Result<T, Error>;
