//! Comparison against documents the reference implementation produced.
//!
//! The fixtures come from the `factur-x` Python package, which is what a real
//! user would otherwise reach for. Checked against that is worth more than
//! checking against our own writer, because the reference is the thing the tax
//! authority's validator was built to accept.

use pdfa::{Level, Xmp, attachment_name, check_factur_x, embedded_file};

/// The Factur-X document the reference implementation produced.
const REFERENCE_PDF: &[u8] = include_bytes!("fixtures/reference_facturx.pdf");
/// Its embedded invoice, extracted from the container.
const REFERENCE_XML: &[u8] = include_bytes!("fixtures/reference_embedded.xml");
/// Its XMP packet, extracted from the compressed metadata stream.
const REFERENCE_XMP: &str = include_str!("fixtures/reference_xmp.xml");

#[test]
fn the_reference_xmp_parses_and_says_what_it_should() {
    let xmp = Xmp::parse(REFERENCE_XMP).expect("the reference packet parses");
    assert_eq!(xmp.part, 3, "Factur-X is PDF/A-3");
    assert_eq!(xmp.conformance, 'B');
    assert_eq!(xmp.info.author, "Seller GmbH");

    let factur_x = xmp.factur_x.expect("a Factur-X description");
    assert_eq!(factur_x.level, Level::En16931);
    assert_eq!(factur_x.file_name, "factur-x.xml");
    assert_eq!(factur_x.document_type, "INVOICE");
    assert_eq!(factur_x.version, "1.0");
}

#[test]
fn every_property_the_reference_declares_is_one_we_declare() {
    // The extension schema advertises its properties; a reader relies on this
    // list to know which `fx:` fields to expect.
    for property in [
        "DocumentFileName",
        "DocumentType",
        "Version",
        "ConformanceLevel",
    ] {
        assert!(
            REFERENCE_XMP.contains(&format!(
                "<pdfaProperty:name>{property}</pdfaProperty:name>"
            )),
            "the reference declares {property}"
        );
        let ours = Xmp::factur_x(Level::En16931, Default::default()).to_xml();
        assert!(
            ours.contains(&format!(
                "<pdfaProperty:name>{property}</pdfaProperty:name>"
            )),
            "we declare {property}"
        );
    }
}

#[test]
fn the_reference_container_passes_the_checks_that_apply_to_it() {
    let report = check_factur_x(REFERENCE_PDF, Some("factur-x.xml"));
    assert_eq!(
        report.embedded_file.as_deref(),
        Some("factur-x.xml"),
        "the attachment name is read: {:?}",
        report.findings
    );
    assert!(
        !report.codes().contains(&"attachment-name"),
        "{:?}",
        report.findings
    );
    assert!(
        !report.codes().contains(&"metadata-not-referenced"),
        "{:?}",
        report.findings
    );
}

#[test]
fn the_reference_attachment_is_compressed_with_real_deflate() {
    // The reference compresses its attachment with real deflate, which this
    // crate does not inflate. Reading it must return nothing rather than
    // guessing, and the checker must say the checksum was not checked rather
    // than that it is wrong.
    assert_eq!(embedded_file(REFERENCE_PDF), None);
    let report = check_factur_x(REFERENCE_PDF, Some("factur-x.xml"));
    assert!(
        !report.codes().contains(&"checksum-mismatch"),
        "{:?}",
        report.findings
    );
    assert!(
        report.codes().contains(&"checksum-unchecked"),
        "{:?}",
        report.findings
    );
}

#[test]
fn our_writer_inflates_back_what_it_wrote() {
    // Our own compressed output is a stored zlib stream, which the checker can
    // inflate; this is the case the reader exists for.
    use pdfa::{DocumentInfo, EmbeddedFile, WriteOptions, embed};
    let mut offsets = Vec::new();
    let mut original: Vec<u8> = b"%PDF-1.4\n".to_vec();
    offsets.push(original.len());
    original.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(original.len());
    original.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\n");
    let xref = original.len();
    original.extend_from_slice(b"xref\n0 3\n0000000000 65535 f \n");
    for offset in &offsets {
        original.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    original.extend_from_slice(
        format!("trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );

    let options = WriteOptions {
        xmp: Xmp::factur_x(Level::En16931, DocumentInfo::default()),
        timestamp: "2026-10-21T14:47:53+00:00".to_string(),
        compress: true,
    };
    let container = embed(
        &original,
        &EmbeddedFile::factur_x(REFERENCE_XML.to_vec()),
        &options,
    )
    .unwrap();
    assert_eq!(embedded_file(&container).as_deref(), Some(REFERENCE_XML));
    let report = check_factur_x(&container, Some("factur-x.xml"));
    assert!(report.is_valid(), "{:?}", report.findings);
    assert!(
        !report.codes().contains(&"checksum-unchecked"),
        "{:?}",
        report.findings
    );
}

#[test]
fn the_extracted_reference_invoice_is_the_document_it_claims_to_be() {
    assert!(REFERENCE_XML.starts_with(b"<?xml version="));
    assert!(
        REFERENCE_XML
            .windows(30)
            .any(|w| w.starts_with(b"<rsm:CrossIndustryInvoice"))
    );
}

#[test]
fn the_attachment_name_helper_agrees_with_the_reference() {
    assert_eq!(
        attachment_name(REFERENCE_PDF).as_deref(),
        Some("factur-x.xml")
    );
}

#[test]
fn our_metadata_carries_the_same_facts_as_the_references() {
    // The values a validator checks, produced by us and by the reference.
    let ours = Xmp::parse(&Xmp::factur_x(Level::En16931, Default::default()).to_xml()).unwrap();
    let reference = Xmp::parse(REFERENCE_XMP).unwrap();
    assert_eq!(ours.part, reference.part);
    assert_eq!(ours.conformance, reference.conformance);
    assert_eq!(
        ours.factur_x.unwrap().level,
        reference.factur_x.unwrap().level
    );
}

#[test]
fn a_compressed_metadata_stream_does_not_read_as_missing() {
    // The reference compresses its metadata with real deflate, which this crate
    // does not inflate. It must be reported as unreadable, not as absent, or a
    // caller would chase a problem that is not there. Either way the report must
    // not claim the document has no metadata while /Metadata is clearly present.
    let report = check_factur_x(REFERENCE_PDF, Some("factur-x.xml"));
    assert!(
        !report.codes().contains(&"metadata-missing"),
        "a document with a /Metadata entry must not be reported as having none: {:?}",
        report.findings
    );
    // And the unreadable packet must be a warning, so the document is not
    // called invalid over a limitation of this reader.
    assert!(report.is_valid(), "{:?}", report.findings);
}
