//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use pdfa::{DocumentInfo, EmbeddedFile, Level, WriteOptions, Xmp, embed};

    // Any ordinary PDF, produced by whatever the caller already uses.
    let original = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Count 0 /Kids [] >>\nendobj\n\
xref\n0 3\n0000000000 65535 f \n0000000009 00000 n \n0000000063 00000 n \n\
trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n133\n%%EOF\n";

    let xml = b"<rsm:CrossIndustryInvoice/>";
    let info = DocumentInfo {
        title: "Invoice INV-2026-0001".into(),
        author: "Seller GmbH".into(),
        subject: "Invoice INV-2026-0001".into(),
        producer: "crateworks".into(),
        creator_tool: "crateworks".into(),
        timestamp: "2026-10-21T14:47:53+00:00".into(),
    };
    let options = WriteOptions {
        xmp: Xmp::factur_x(Level::En16931, info),
        timestamp: "2026-10-21T14:47:53+00:00".into(),
        compress: false,
    };

    let container = embed(original, &EmbeddedFile::factur_x(xml.to_vec()), &options).unwrap();

    // And check what was built.
    let report = pdfa::check_factur_x(&container, Some("factur-x.xml"));
    assert!(report.is_valid(), "{:?}", report.findings);
    assert_eq!(pdfa::embedded_file(&container).unwrap(), xml);
}
