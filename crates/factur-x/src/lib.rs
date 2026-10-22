#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod cii;
pub mod model;
pub mod ubl;
pub mod validate;

mod amount;

#[doc(hidden)]
pub mod test_support;

pub use amount::{Amount, Currency};
pub use model::Invoice;
pub use validate::{Report, Violation, validate};

use xmlkit::Element;

/// Why a document could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A required element was not present.
    MissingElement {
        /// The element name that was expected.
        name: String,
    },
    /// A value was not in the form the field requires.
    InvalidValue {
        /// The field that was wrong.
        field: String,
        /// What was found.
        found: String,
    },
    /// The XML itself was not well formed.
    Xml(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingElement { name } => write!(f, "missing required element {name}"),
            Self::InvalidValue { field, found } => {
                write!(f, "{field} has an invalid value {found:?}")
            }
            Self::Xml(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<xmlkit::Error> for Error {
    fn from(error: xmlkit::Error) -> Self {
        Self::Xml(error.to_string())
    }
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Which XML syntax a document uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    /// UN/CEFACT Cross Industry Invoice, used by Factur-X and ZUGFeRD.
    Cii,
    /// OASIS Universal Business Language, used by Peppol.
    Ubl,
}

impl Syntax {
    /// The namespace URI of the document element.
    #[must_use]
    pub fn namespace(self) -> &'static str {
        match self {
            Self::Cii => cii::RSM,
            Self::Ubl => ubl::INVOICE,
        }
    }
}

/// Writes an invoice as an XML document.
///
/// The result is a complete document with a declaration, so it can be written
/// to a file as is.
///
/// # Example
///
/// ```
/// use factur_x::{generate, Syntax};
///
/// // A conforming sample invoice; see `test_support`.
/// let invoice = factur_x::test_support::sample_invoice();
/// let xml = generate(&invoice, Syntax::Cii);
///
/// assert!(xml.starts_with("<?xml"));
/// assert!(xml.contains(&invoice.number));
/// ```
#[must_use]
pub fn generate(invoice: &Invoice, syntax: Syntax) -> String {
    let root = match syntax {
        Syntax::Cii => cii::generate(invoice),
        Syntax::Ubl => ubl::generate(invoice),
    };
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{root}")
}

/// Reads an invoice from an XML document.
///
/// # Errors
///
/// Returns [`Error::Xml`] if the text is not well-formed XML or the root element
/// does not match the syntax, and [`Error::MissingElement`] if a required field
/// is absent.
pub fn parse(xml: &str, syntax: Syntax) -> Result<Invoice> {
    let root = xmlkit::parse(xml)?;
    let expected = match syntax {
        Syntax::Cii => "CrossIndustryInvoice",
        Syntax::Ubl => "Invoice",
    };
    if xmlkit::local_name(&root.name) != expected {
        return Err(Error::InvalidValue {
            field: "root element".to_string(),
            found: root.name.clone(),
        });
    }
    match syntax {
        Syntax::Cii => cii::parse(&root),
        Syntax::Ubl => ubl::parse(&root),
    }
}

/// Detects the syntax from a document's root element name.
#[must_use]
pub fn detect_syntax(xml: &str) -> Option<Syntax> {
    let root = xmlkit::parse(xml).ok()?;
    match xmlkit::local_name(&root.name) {
        "CrossIndustryInvoice" => Some(Syntax::Cii),
        "Invoice" => Some(Syntax::Ubl),
        _ => None,
    }
}

/// The CII document for an invoice, without the XML declaration.
#[must_use]
pub fn cii_document(invoice: &Invoice) -> Element {
    cii::generate(invoice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_syntaxes_round_trip_the_sample_invoice() {
        let invoice = test_support::sample_invoice();
        for syntax in [Syntax::Cii, Syntax::Ubl] {
            let xml = generate(&invoice, syntax);
            let back = parse(&xml, syntax).unwrap();
            assert_eq!(back, invoice, "{syntax:?}");
        }
    }

    #[test]
    fn both_syntaxes_round_trip_the_rich_invoice() {
        let invoice = test_support::rich_invoice();
        for syntax in [Syntax::Cii, Syntax::Ubl] {
            let xml = generate(&invoice, syntax);
            let back = parse(&xml, syntax).unwrap();
            assert_eq!(back, invoice, "{syntax:?}");
        }
    }

    #[test]
    fn the_generated_document_starts_with_a_declaration() {
        let xml = generate(&test_support::sample_invoice(), Syntax::Cii);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    }

    #[test]
    fn the_syntax_can_be_detected_from_the_document() {
        let invoice = test_support::sample_invoice();
        assert_eq!(
            detect_syntax(&generate(&invoice, Syntax::Cii)),
            Some(Syntax::Cii)
        );
        assert_eq!(
            detect_syntax(&generate(&invoice, Syntax::Ubl)),
            Some(Syntax::Ubl)
        );
        assert_eq!(detect_syntax("<Other/>"), None);
        assert_eq!(detect_syntax("not xml"), None);
    }

    #[test]
    fn parsing_with_the_wrong_syntax_is_an_error() {
        let xml = generate(&test_support::sample_invoice(), Syntax::Cii);
        assert!(parse(&xml, Syntax::Ubl).is_err());
    }

    #[test]
    fn malformed_xml_is_reported() {
        assert!(matches!(parse("<a>", Syntax::Ubl), Err(Error::Xml(_))));
    }

    #[test]
    fn the_namespaces_are_the_standard_ones() {
        assert_eq!(
            Syntax::Cii.namespace(),
            "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100"
        );
        assert_eq!(
            Syntax::Ubl.namespace(),
            "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
        );
    }
}
