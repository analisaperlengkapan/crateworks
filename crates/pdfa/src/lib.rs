#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod conformance;
pub mod container;
pub mod digest;
pub mod flate;
pub mod object;
pub mod xmp;
pub mod xref;

pub use conformance::{
    Finding, Report, Severity, attachment_name, check_factur_x, embedded_file, metadata_packet,
};
pub use container::{EmbeddedFile, Pdf, WriteOptions, embed};
pub use xmp::{DocumentInfo, FacturX, Level, Xmp};

/// A failure while reading or writing a PDF/A container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The file is not well-formed XML.
    Xml(String),
    /// A part of the PDF that has to be there is not.
    MissingPdfPart(&'static str),
    /// The metadata is missing something required.
    MissingXmp(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xml(message) => write!(f, "invalid XML: {message}"),
            Self::MissingPdfPart(part) => write!(f, "the document has no {part}"),
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
