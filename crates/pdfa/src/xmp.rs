//! XMP metadata, as PDF/A requires it.
//!
//! PDF/A identifies itself through XMP, an XML format embedded in the document.
//! A reader decides whether a file claims to be PDF/A by looking for
//! `pdfaid:part` and `pdfaid:conformance` in this metadata, and Factur-X adds its
//! own description on top of that.
//!
//! The metadata is written as a raw XML string rather than through the element
//! tree, because an `xpacket` wrapper and a byte-order mark sit around the XML
//! and the exact bytes are part of what a validator checks. Building it from a
//! template keeps those bytes under control; [`Xmp::parse`] reads it back with
//! the workspace XML parser, where tolerance is what is wanted.

use xmlkit::local_name;

/// The `rdf` namespace.
pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
/// The `pdfaid` namespace, which carries the PDF/A part and conformance.
pub const PDFAID: &str = "http://www.aiim.org/pdfa/ns/id/";
/// The `dc` namespace.
pub const DC: &str = "http://purl.org/dc/elements/1.1/";
/// The `pdf` namespace.
pub const PDF: &str = "http://ns.adobe.com/pdf/1.3/";
/// The `xmp` namespace.
pub const XMP: &str = "http://ns.adobe.com/xap/1.0/";
/// The `pdfaExtension` namespace.
pub const PDFA_EXTENSION: &str = "http://www.aiim.org/pdfa/ns/extension/";
/// The `pdfaSchema` namespace.
pub const PDFA_SCHEMA: &str = "http://www.aiim.org/pdfa/ns/schema#";
/// The `pdfaProperty` namespace.
pub const PDFA_PROPERTY: &str = "http://www.aiim.org/pdfa/ns/property#";
/// The Factur-X extension schema namespace.
pub const FACTURX_SCHEMA: &str = "urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#";

/// The conformance level a Factur-X document declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// MINIMUM: only the header fields.
    Minimum,
    /// BASIC WL: the header, without lines.
    BasicWl,
    /// BASIC: lines, but not the full profile.
    Basic,
    /// EN 16931: the European standard's own profile.
    En16931,
    /// EXTENDED: everything the syntax allows.
    Extended,
}

impl Level {
    /// The spelling Factur-X uses in its XMP metadata.
    ///
    /// The spelling is not the same as the identifier: EN 16931 is written
    /// `EN 16931` in the XML and `en16931` elsewhere, and getting it wrong makes
    /// a conformant document look non-conformant.
    #[must_use]
    pub fn xmp_name(self) -> &'static str {
        match self {
            Self::Minimum => "MINIMUM",
            Self::BasicWl => "BASIC WL",
            Self::Basic => "BASIC",
            Self::En16931 => "EN 16931",
            Self::Extended => "EXTENDED",
        }
    }

    /// The spelling used in file names and code.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Minimum => "minimum",
            Self::BasicWl => "basicwl",
            Self::Basic => "basic",
            Self::En16931 => "en16931",
            Self::Extended => "extended",
        }
    }
}

/// What a document is, for the Factur-X description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacturX {
    /// The conformance level.
    pub level: Level,
    /// The name of the embedded XML file, conventionally `factur-x.xml`.
    pub file_name: String,
    /// The document type, conventionally `INVOICE`.
    pub document_type: String,
    /// The version of the schema, conventionally `1.0`.
    pub version: String,
}

impl FacturX {
    /// The conventional Factur-X description at the given level.
    #[must_use]
    pub fn new(level: Level) -> Self {
        Self {
            level,
            file_name: "factur-x.xml".to_string(),
            document_type: "INVOICE".to_string(),
            version: "1.0".to_string(),
        }
    }
}

/// The document properties PDF/A requires alongside the XMP.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentInfo {
    /// `dc:title`.
    pub title: String,
    /// `dc:creator`.
    pub author: String,
    /// `dc:description`.
    pub subject: String,
    /// `pdf:Producer`.
    pub producer: String,
    /// `xmp:CreatorTool`.
    pub creator_tool: String,
    /// The creation and modification time, in ISO 8601.
    ///
    /// PDF/A wants the same value in the XMP and in the document information
    /// dictionary, so it is carried once here.
    pub timestamp: String,
}

/// An XMP packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xmp {
    /// The PDF/A part, always 3 for Factur-X.
    pub part: u8,
    /// The PDF/A conformance, `B`, `U` or `A`.
    pub conformance: char,
    /// The document properties.
    pub info: DocumentInfo,
    /// The Factur-X description, when the document is one.
    pub factur_x: Option<FacturX>,
}

impl Xmp {
    /// A PDF/A-3B packet for a Factur-X invoice.
    #[must_use]
    pub fn factur_x(level: Level, info: DocumentInfo) -> Self {
        Self {
            part: 3,
            conformance: 'B',
            info,
            factur_x: Some(FacturX::new(level)),
        }
    }

    /// A plain PDF/A packet with no Factur-X description.
    #[must_use]
    pub fn pdf_a(part: u8, conformance: char, info: DocumentInfo) -> Self {
        Self {
            part,
            conformance,
            info,
            factur_x: None,
        }
    }

    /// Serialises the packet.
    ///
    /// The byte-order mark after `begin=` is not decoration: the XMP
    /// specification requires it, and a validator rejects a packet without one.
    #[must_use]
    pub fn to_xml(&self) -> String {
        let mut out = String::new();
        out.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
        out.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n");
        out.push_str(&format!("  <rdf:RDF xmlns:rdf=\"{RDF}\">\n"));

        out.push_str(&format!(
            "    <rdf:Description xmlns:pdfaid=\"{PDFAID}\" rdf:about=\"\">\n      <pdfaid:part>{}</pdfaid:part>\n      <pdfaid:conformance>{}</pdfaid:conformance>\n    </rdf:Description>\n",
            self.part, self.conformance
        ));

        out.push_str(&format!(
            "    <rdf:Description xmlns:dc=\"{DC}\" rdf:about=\"\">\n"
        ));
        out.push_str(&format!(
            "      <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:title>\n",
            xmlkit::escape_text(&self.info.title)
        ));
        out.push_str(&format!(
            "      <dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>\n",
            xmlkit::escape_text(&self.info.author)
        ));
        out.push_str(&format!(
            "      <dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:description>\n",
            xmlkit::escape_text(&self.info.subject)
        ));
        out.push_str("    </rdf:Description>\n");

        out.push_str(&format!(
            "    <rdf:Description xmlns:pdf=\"{PDF}\" rdf:about=\"\">\n      <pdf:Producer>{}</pdf:Producer>\n    </rdf:Description>\n",
            xmlkit::escape_text(&self.info.producer)
        ));
        out.push_str(&format!(
            "    <rdf:Description xmlns:xmp=\"{XMP}\" rdf:about=\"\">\n      <xmp:CreatorTool>{}</xmp:CreatorTool>\n      <xmp:CreateDate>{}</xmp:CreateDate>\n      <xmp:ModifyDate>{}</xmp:ModifyDate>\n    </rdf:Description>\n",
            xmlkit::escape_text(&self.info.creator_tool),
            xmlkit::escape_text(&self.info.timestamp),
            xmlkit::escape_text(&self.info.timestamp)
        ));

        if let Some(fx) = &self.factur_x {
            out.push_str(&format!(
                "    <rdf:Description xmlns:pdfaExtension=\"{PDFA_EXTENSION}\" xmlns:pdfaSchema=\"{PDFA_SCHEMA}\" xmlns:pdfaProperty=\"{PDFA_PROPERTY}\" rdf:about=\"\">\n"
            ));
            out.push_str(
                "      <pdfaExtension:schemas><rdf:Bag><rdf:li rdf:parseType=\"Resource\">\n",
            );
            out.push_str(
                "        <pdfaSchema:schema>Factur-X PDFA Extension Schema</pdfaSchema:schema>\n",
            );
            out.push_str(&format!(
                "        <pdfaSchema:namespaceURI>{FACTURX_SCHEMA}</pdfaSchema:namespaceURI>\n"
            ));
            out.push_str("        <pdfaSchema:prefix>fx</pdfaSchema:prefix>\n");
            out.push_str("        <pdfaSchema:property><rdf:Seq>\n");
            for (name, description) in [
                ("DocumentFileName", "The name of the embedded XML document"),
                (
                    "DocumentType",
                    "The type of the hybrid document in capital letters, e.g. INVOICE or ORDER",
                ),
                (
                    "Version",
                    "The actual version of the standard applying to the embedded XML document",
                ),
                (
                    "ConformanceLevel",
                    "The conformance level of the embedded XML document",
                ),
            ] {
                out.push_str("          <rdf:li rdf:parseType=\"Resource\">\n");
                out.push_str(&format!(
                    "            <pdfaProperty:name>{name}</pdfaProperty:name>\n"
                ));
                out.push_str("            <pdfaProperty:valueType>Text</pdfaProperty:valueType>\n");
                out.push_str(
                    "            <pdfaProperty:category>external</pdfaProperty:category>\n",
                );
                out.push_str(&format!(
                    "            <pdfaProperty:description>{}</pdfaProperty:description>\n",
                    xmlkit::escape_text(description)
                ));
                out.push_str("          </rdf:li>\n");
            }
            out.push_str("        </rdf:Seq></pdfaSchema:property>\n");
            out.push_str("      </rdf:li></rdf:Bag></pdfaExtension:schemas>\n");
            out.push_str("    </rdf:Description>\n");

            out.push_str(&format!(
                "    <rdf:Description xmlns:fx=\"{FACTURX_SCHEMA}\" rdf:about=\"\">\n"
            ));
            out.push_str(&format!(
                "      <fx:DocumentType>{}</fx:DocumentType>\n",
                xmlkit::escape_text(&fx.document_type)
            ));
            out.push_str(&format!(
                "      <fx:DocumentFileName>{}</fx:DocumentFileName>\n",
                xmlkit::escape_text(&fx.file_name)
            ));
            out.push_str(&format!(
                "      <fx:Version>{}</fx:Version>\n",
                xmlkit::escape_text(&fx.version)
            ));
            out.push_str(&format!(
                "      <fx:ConformanceLevel>{}</fx:ConformanceLevel>\n",
                xmlkit::escape_text(fx.level.xmp_name())
            ));
            out.push_str("    </rdf:Description>\n");
        }

        out.push_str("  </rdf:RDF>\n</x:xmpmeta>\n");
        out.push_str("<?xpacket end=\"w\"?>");
        out
    }

    /// Reads a packet back.
    ///
    /// # Errors
    ///
    /// Returns an error if the packet is not well-formed XML or carries no
    /// `pdfaid:part`, which is what makes it a PDF/A claim at all.
    pub fn parse(xml: &str) -> Result<Self, crate::Error> {
        // The `<?xpacket ...?>` wrapper is a processing instruction, and the
        // workspace parser accepts those only inside the root element. A real
        // packet always has them, so they are removed here rather than making
        // the parser more lenient.
        let document = strip_xpacket(xml);
        let root = xmlkit::parse(document)?;
        let mut part = None;
        let mut conformance = None;
        let mut info = DocumentInfo::default();
        let mut factur_x = None;

        for element in descendants(&root) {
            match local_name(&element.name) {
                "part" => part = element.text().trim().parse::<u8>().ok(),
                "conformance" => conformance = element.text().trim().chars().next(),
                "title" => info.title = deep_text(element),
                "creator" => info.author = deep_text(element),
                "description" => info.subject = deep_text(element),
                "Producer" => info.producer = element.text(),
                "CreatorTool" => info.creator_tool = element.text(),
                "CreateDate" => info.timestamp = element.text(),
                "DocumentFileName" => {
                    factur_x.get_or_insert_with(default_factur_x).file_name = element.text();
                }
                "DocumentType" => {
                    factur_x.get_or_insert_with(default_factur_x).document_type = element.text();
                }
                "Version" => {
                    factur_x.get_or_insert_with(default_factur_x).version = element.text();
                }
                "ConformanceLevel" => {
                    let text = element.text();
                    if let Some(level) = level_from_xmp(&text) {
                        factur_x.get_or_insert_with(default_factur_x).level = level;
                    }
                }
                _ => {}
            }
        }

        let Some(part) = part else {
            return Err(crate::Error::MissingXmp("pdfaid:part"));
        };
        let conformance = conformance.ok_or(crate::Error::MissingXmp("pdfaid:conformance"))?;
        Ok(Self {
            part,
            conformance,
            info,
            factur_x,
        })
    }
}

fn default_factur_x() -> FacturX {
    FacturX::new(Level::En16931)
}

/// The XML between the `xpacket` processing instructions.
fn strip_xpacket(xml: &str) -> &str {
    let start = xml.find("<?xpacket").map_or(0, |position| {
        xml[position..]
            .find("?>")
            .map_or(position, |offset| position + offset + 2)
    });
    let end = match xml.rfind("<?xpacket end") {
        Some(position) if position > start => position,
        _ => xml.len(),
    };
    xml[start..end].trim()
}

/// The text of an element and all its descendants.
///
/// A `dc:title` wraps its value in `rdf:Alt` and `rdf:li`, so the value is one
/// level down from the element the name suggests.
fn deep_text(element: &xmlkit::Element) -> String {
    let mut out = String::new();
    collect_text(element, &mut out);
    out.trim().to_string()
}

fn collect_text(element: &xmlkit::Element, out: &mut String) {
    for node in &element.children {
        match node {
            xmlkit::Node::Element(child) => collect_text(child, out),
            xmlkit::Node::Text(text) => out.push_str(text),
            xmlkit::Node::Comment(_) => {}
        }
    }
}

/// Every element in the tree, the root first.
fn descendants(root: &xmlkit::Element) -> Vec<&xmlkit::Element> {
    fn walk<'a>(element: &'a xmlkit::Element, out: &mut Vec<&'a xmlkit::Element>) {
        out.push(element);
        for node in &element.children {
            if let xmlkit::Node::Element(child) = node {
                walk(child, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

/// The level a `ConformanceLevel` spelling names.
#[must_use]
pub fn level_from_xmp(text: &str) -> Option<Level> {
    let normalised = text.trim().to_ascii_uppercase();
    match normalised.as_str() {
        "MINIMUM" => Some(Level::Minimum),
        "BASIC WL" => Some(Level::BasicWl),
        "BASIC" => Some(Level::Basic),
        "EN 16931" | "EN16931" => Some(Level::En16931),
        "EXTENDED" => Some(Level::Extended),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> DocumentInfo {
        DocumentInfo {
            title: "Invoice INV-2026-0001".to_string(),
            author: "Seller GmbH".to_string(),
            subject: "Invoice INV-2026-0001".to_string(),
            producer: "crateworks pdfa".to_string(),
            creator_tool: "crateworks".to_string(),
            timestamp: "2026-10-21T14:47:53+00:00".to_string(),
        }
    }

    #[test]
    fn the_packet_starts_with_the_byte_order_mark() {
        let xml = Xmp::factur_x(Level::En16931, info()).to_xml();
        assert!(xml.starts_with("<?xpacket begin=\"\u{feff}\""));
        assert!(xml.ends_with("<?xpacket end=\"w\"?>"));
    }

    #[test]
    fn the_packet_claims_pdf_a_3b() {
        let xml = Xmp::factur_x(Level::En16931, info()).to_xml();
        assert!(xml.contains("<pdfaid:part>3</pdfaid:part>"));
        assert!(xml.contains("<pdfaid:conformance>B</pdfaid:conformance>"));
    }

    #[test]
    fn a_packet_round_trips() {
        let original = Xmp::factur_x(Level::Extended, info());
        let back = Xmp::parse(&original.to_xml()).unwrap();
        assert_eq!(back.part, 3);
        assert_eq!(back.conformance, 'B');
        assert_eq!(back.info.title, "Invoice INV-2026-0001");
        assert_eq!(back.info.author, "Seller GmbH");
        assert_eq!(back.factur_x.unwrap().level, Level::Extended);
    }

    #[test]
    fn the_factur_x_level_is_spelled_the_way_the_standard_spells_it() {
        // "EN 16931" with a space, which is not the code form "en16931".
        let xml = Xmp::factur_x(Level::En16931, info()).to_xml();
        assert!(xml.contains("<fx:ConformanceLevel>EN 16931</fx:ConformanceLevel>"));
        assert_eq!(level_from_xmp("EN 16931"), Some(Level::En16931));
        assert_eq!(level_from_xmp("en16931"), Some(Level::En16931));
        assert_eq!(level_from_xmp("BASIC WL"), Some(Level::BasicWl));
        assert_eq!(level_from_xmp("nonsense"), None);
    }

    #[test]
    fn the_extension_schema_declares_all_four_properties() {
        let xml = Xmp::factur_x(Level::Basic, info()).to_xml();
        for property in [
            "DocumentFileName",
            "DocumentType",
            "Version",
            "ConformanceLevel",
        ] {
            assert!(
                xml.contains(&format!("<pdfaProperty:name>{property}")),
                "{property}"
            );
        }
    }

    #[test]
    fn a_plain_pdf_a_packet_has_no_factur_x_description() {
        let xml = Xmp::pdf_a(2, 'U', info()).to_xml();
        assert!(xml.contains("<pdfaid:part>2</pdfaid:part>"));
        assert!(xml.contains("<pdfaid:conformance>U</pdfaid:conformance>"));
        assert!(!xml.contains("fx:DocumentType"));
        assert!(Xmp::parse(&xml).unwrap().factur_x.is_none());
    }

    #[test]
    fn a_packet_without_a_part_is_rejected() {
        let xml = "<?xpacket begin=\"\u{feff}\"?><x:xmpmeta><rdf:RDF/></x:xmpmeta>";
        assert!(matches!(
            Xmp::parse(xml),
            Err(crate::Error::MissingXmp("pdfaid:part"))
        ));
    }

    #[test]
    fn text_with_xml_characters_is_escaped() {
        let mut info = info();
        info.title = "A & B <C>".to_string();
        let xml = Xmp::factur_x(Level::Basic, info).to_xml();
        assert!(xml.contains("A &amp; B &lt;C&gt;"));
        assert_eq!(Xmp::parse(&xml).unwrap().info.title, "A & B <C>");
    }
}
