//! Comparison against documents the reference implementation produced.
//!
//! The fixtures come from the `factur-x` Python package, which is what a real
//! user would otherwise reach for. Checked against that is worth more than
//! checking against our own writer, because the reference is the thing the tax
//! authority's validator was built to accept.

use pdfa::{Level, Xmp, attachment_name, check_factur_x, embedded_file};

/// Decodes a hex string.
fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
        .collect()
}

/// zlib streams paired with the plain text they hold.
///
/// Produced by Python's `zlib`, which is the reference implementation and never
/// saw this code. Between them the vectors cover stored, fixed-Huffman and
/// dynamic-Huffman blocks, which is what a real `FlateDecode` stream may be.
const ZLIB_VECTORS: &str = include_str!("fixtures/zlib_vectors.txt");

#[test]
fn inflate_matches_zlib_on_every_vector() {
    let mut checked = 0;
    for (number, line) in ZLIB_VECTORS.lines().enumerate() {
        let (compressed, plain) = line.split_once('\t').expect("hex\thex");
        let compressed = unhex(compressed);
        let plain = unhex(plain);
        assert_eq!(
            pdfa::flate::inflate(&compressed).as_deref(),
            Some(plain.as_slice()),
            "vector {number} ({} compressed bytes)",
            compressed.len()
        );
        checked += 1;
    }
    assert!(checked > 40, "only {checked} vectors were read");
}

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
fn the_reference_attachment_is_read_now_that_deflate_is_inflated() {
    // The reference compresses its attachment with real deflate. This crate used
    // to read only stored zlib streams, so the reference's own attachment came
    // back as nothing and its checksum could not be checked. It inflates now.
    assert_eq!(
        embedded_file(REFERENCE_PDF).as_deref(),
        Some(REFERENCE_XML),
        "the reference's own attachment must be readable"
    );
    let report = check_factur_x(REFERENCE_PDF, Some("factur-x.xml"));
    assert!(
        !report.codes().contains(&"checksum-mismatch"),
        "{:?}",
        report.findings
    );
    assert!(
        !report.codes().contains(&"checksum-unchecked"),
        "the checksum must be checked now: {:?}",
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
fn the_reference_metadata_stream_is_read_now_that_deflate_is_inflated() {
    // The reference compresses its XMP packet with real deflate. It used to come
    // back as unreadable, which was a limitation of this reader rather than a
    // fault in the document. Reading it is what makes the packet checkable at all.
    let packet = pdfa::conformance::metadata_packet(REFERENCE_PDF)
        .expect("the reference's compressed metadata must be readable");
    let xmp = Xmp::parse(&packet).expect("the inflated packet parses");
    assert_eq!(xmp.part, 3);
    assert_eq!(xmp.conformance, 'B');
    assert_eq!(xmp.info.author, "Seller GmbH");
    assert_eq!(
        xmp.factur_x.expect("a Factur-X description").level,
        Level::En16931
    );

    let report = check_factur_x(REFERENCE_PDF, Some("factur-x.xml"));
    assert!(
        !report.codes().contains(&"metadata-missing"),
        "{:?}",
        report.findings
    );
    assert!(report.is_valid(), "{:?}", report.findings);
}

/// A PDF whose objects live in an object stream, written by pikepdf (qpdf).
///
/// This is the layout a modern producer emits by default and the one this crate
/// used to reject outright.
const OBJECT_STREAM_PDF: &[u8] = include_bytes!("fixtures/object_stream.pdf");

#[test]
fn a_document_with_object_streams_parses() {
    let pdf = pdfa::Pdf::parse(OBJECT_STREAM_PDF).expect("an object-stream PDF must parse");
    // The catalog and the pages object both live inside the object stream, so
    // reaching them proves the stream was expanded and not merely located.
    let catalog = pdf.catalog().expect("a catalog");
    assert!(catalog.contains("/Catalog"), "catalog was: {catalog}");
    assert!(catalog.contains("/Pages"), "catalog was: {catalog}");
    let pages = pdf.object_bytes(3).expect("the pages object");
    assert!(
        String::from_utf8_lossy(pages).contains("/Pages"),
        "pages was: {}",
        String::from_utf8_lossy(pages)
    );
}

#[test]
fn the_reference_xref_stream_is_decoded_with_its_predictor() {
    // pikepdf writes the cross-reference stream with a PNG predictor, so the
    // entries only come out right when the predictor is undone. Reading a
    // predictor's output as entries yields plausible nonsense, which is why this
    // is checked against the numbers the file itself states.
    let xref = pdfa::xref::read(OBJECT_STREAM_PDF).expect("a cross-reference section");
    assert_eq!(
        xref.entries.get(&2),
        Some(&pdfa::xref::Location::InStream {
            stream: 1,
            index: 0
        })
    );
    assert_eq!(
        xref.entries.get(&6),
        Some(&pdfa::xref::Location::InFile {
            offset: 264,
            generation: 0
        })
    );
    assert!(xref.trailer.contains("/Root 2 0 R"), "{}", xref.trailer);
}

#[test]
fn embedding_into_an_object_stream_document_produces_a_readable_file() {
    // The whole point of reading object streams: a user hands this crate a
    // modern PDF and gets a Factur-X container back. The container has to remain
    // readable, which means every object the original held must survive, pages
    // included. A dropped pages object is what makes a strict reader report that
    // it cannot find the page tree.
    use pdfa::{DocumentInfo, EmbeddedFile, WriteOptions, embed};
    let options = WriteOptions {
        xmp: Xmp::factur_x(Level::En16931, DocumentInfo::default()),
        timestamp: "2026-10-21T00:00:00+00:00".to_string(),
        compress: true,
    };
    let invoice = b"<invoice>hello</invoice>".to_vec();
    let container = embed(
        OBJECT_STREAM_PDF,
        &EmbeddedFile::factur_x(invoice.clone()),
        &options,
    )
    .expect("embedding into an object-stream PDF must work");

    // Every object the original defined is still present, and the pages object
    // still says it is one.
    let pdf = pdfa::Pdf::parse(&container).expect("the container parses");
    let pages = pdf.object_bytes(3).expect("the pages object survived");
    assert!(
        String::from_utf8_lossy(pages).contains("/Pages"),
        "pages was: {}",
        String::from_utf8_lossy(pages)
    );
    assert_eq!(
        pdfa::embedded_file(&container).as_deref(),
        Some(invoice.as_slice())
    );

    // And the attachment is reachable the way a reader looks for it: the catalog
    // points at a name tree, and that tree lists the file in a `/Names` array of
    // `(name) reference` pairs. A name written where the array belongs produces a
    // file no reader can find the attachment in, which is what this guards.
    let catalog = pdf.catalog().expect("a catalog");
    let names_number = catalog
        .split_once("/Names")
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .and_then(|token| token.parse::<u32>().ok())
        .unwrap_or_else(|| panic!("the catalog must point at a name tree: {catalog}"));
    let names = pdf.object_bytes(names_number).expect("the name tree");
    let names = String::from_utf8_lossy(names);
    assert!(names.contains("/EmbeddedFiles"), "names tree was: {names}");
    let listing = names
        .split_once("/Names")
        .map(|(_, rest)| rest.trim_start().to_string())
        .unwrap_or_default();
    assert!(
        listing.starts_with("[(factur-x.xml)"),
        "the listing must be an array starting with the file name: {listing}"
    );
    assert!(
        listing.contains("0 R]"),
        "the listing must end with a reference: {listing}"
    );

    let report = check_factur_x(&container, Some("factur-x.xml"));
    assert!(report.is_valid(), "{:?}", report.findings);
}
