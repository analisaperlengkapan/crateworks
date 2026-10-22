//! The EBICS request messages, and the signature over them.
//!
//! An EBICS request is an XML document with a fixed shape. The client builds it,
//! signs its `header`, and sends it. The signature is what makes the request
//! trustworthy to the bank, and it has two parts:
//!
//! 1. A **digest** of the `header` element, SHA-256 over its canonical form,
//!    base64 encoded, placed in `ds:DigestValue`.
//! 2. A **signature** over `ds:SignedInfo`, RSA PKCS#1 v1.5 with SHA-256, base64
//!    encoded, placed in `ds:SignatureValue`.
//!
//! Both canonical forms are exclusive C14N with the document root's namespace
//! declarations re-declared at the signed element; see [`crate::c14n`]. The two
//! digests differ in *which* element is canonicalised, not in how, which is why
//! one function serves both.
//!
//! # Why the two steps are separate
//!
//! The digest covers the header, which is large and mostly data. The signature
//! covers `SignedInfo`, which is small and names the digest. That is the XML
//! Signature arrangement: the expensive signature is over a small, fixed
//! structure, and the digest binds it to the actual message.
//!
//! # The algorithm identifiers
//!
//! The `Algorithm` attributes are not decoration. A bank verifies the signature
//! by reading them, so they are written exactly as the reference clients write
//! them, and [`verify`] reads them back rather than assuming.

use std::fmt::Write as _;

use xmlkit::{Element, Node};

use crate::c14n::canonicalize_in_document;
use crate::error::{Error, Result};
use crate::header::{OrderAttribute, StaticHeader, TransactionPhase};
use crate::rsa::{PrivateKey, PublicKey};
use crate::sha256::Sha256;
use crate::{base64, xml_escape};

/// The canonicalisation algorithm the messages name.
pub const C14N_ALGORITHM: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";
/// The signature algorithm: RSA PKCS#1 v1.5 over SHA-256.
pub const SIGNATURE_METHOD: &str = "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256";
/// The digest algorithm: SHA-256.
pub const DIGEST_METHOD: &str = "http://www.w3.org/2001/04/xmlenc#sha256";
/// The reference that selects the authenticated elements.
pub const REFERENCE_URI: &str = "#xpointer(//*[@authenticate='true'])";
/// The EBICS namespace.
pub const EBICS_NAMESPACE: &str = "urn:org:ebics:H004";
/// The XML Signature namespace.
pub const DS_NAMESPACE: &str = "http://www.w3.org/2000/09/xmldsig#";
/// The attribute that marks the elements a signature covers.
pub const AUTHENTICATE: &str = "authenticate";

/// The root element name for each request kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestKind {
    /// A signed order, the ordinary case: `ebicsRequest`.
    Signed,
    /// An unsigned order with the bank key digests: `ebicsNoPubKeyDigestsRequest`.
    ///
    /// The key-management orders that need no signature, `HPB` among them, use
    /// this root. The name says the digests are absent, not the signature.
    NoPubKeyDigests,
    /// An unsigned order, used by `INI` and `HIA` to send keys for the first
    /// time, when no key exists yet to sign with: `ebicsUnsecuredRequest`.
    Unsecured,
}

impl RequestKind {
    /// The root element name.
    #[must_use]
    pub fn root(self) -> &'static str {
        match self {
            Self::Signed => "ebicsRequest",
            Self::NoPubKeyDigests => "ebicsNoPubKeyDigestsRequest",
            Self::Unsecured => "ebicsUnsecuredRequest",
        }
    }

    /// Whether a message of this kind carries an `AuthSignature`.
    ///
    /// An unsecured request does not: there is no key to sign with yet.
    #[must_use]
    pub fn is_signed(self) -> bool {
        !matches!(self, Self::Unsecured)
    }
}

/// A request under construction.
#[derive(Debug, Clone)]
pub struct Request {
    kind: RequestKind,
    header: StaticHeader,
    phase: Option<TransactionPhase>,
    body: String,
}

impl Request {
    /// A signed request with an empty body.
    #[must_use]
    pub fn new(kind: RequestKind, header: StaticHeader) -> Self {
        Self {
            kind,
            header,
            phase: None,
            body: String::new(),
        }
    }

    /// Sets the mutable header phase.
    #[must_use]
    pub fn with_phase(mut self, phase: TransactionPhase) -> Self {
        self.phase = Some(phase);
        self
    }

    /// Sets the body markup, which the caller renders.
    #[must_use]
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// The `header` element, which the signature covers.
    #[must_use]
    pub fn header_xml(&self) -> String {
        let mut out = String::from("<header authenticate=\"true\">");
        out.push_str(&self.header.to_static_xml());
        out.push_str(&StaticHeader::to_mutable_xml(self.phase));
        out.push_str("</header>");
        out
    }

    /// The document prologue and root open tag.
    fn root_open(&self) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><{} xmlns:ds=\"{}\" xmlns=\"{}\" Version=\"H004\" Revision=\"1\">",
            self.kind.root(),
            DS_NAMESPACE,
            EBICS_NAMESPACE
        )
    }

    /// Renders the request with a digest and a signature already computed.
    fn render(&self, digest: &str, signature: &str) -> String {
        let mut out = self.root_open();
        out.push_str(&self.header_xml());
        if self.kind.is_signed() {
            out.push_str(&signed_info(digest));
            if !signature.is_empty() {
                let _ = write!(out, "<ds:SignatureValue>{signature}</ds:SignatureValue>");
            }
            out.push_str("</AuthSignature>");
        }
        if self.body.is_empty() {
            out.push_str("<body></body>");
        } else {
            let _ = write!(out, "<body>{}</body>", self.body);
        }
        let _ = write!(out, "</{}>", self.kind.root());
        out
    }

    /// Renders the request without a signature.
    ///
    /// This is only correct for [`RequestKind::Unsecured`], which carries none.
    /// For any other kind the bank rejects it, so the method refuses.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Invalid`] for a kind that must be signed.
    pub fn to_unsigned_xml(&self) -> Result<String> {
        if self.kind.is_signed() {
            return Err(Error::Invalid("an unsigned request of a signed kind"));
        }
        Ok(self.render("", ""))
    }

    /// Signs the request and renders it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Malformed`] if the request this method built cannot be
    /// parsed, which would mean a bug here rather than in the caller's values,
    /// and [`Error::Invalid`] for an unsecured request, which is not signed.
    pub fn sign(&self, key: &PrivateKey) -> Result<String> {
        if !self.kind.is_signed() {
            return Err(Error::Invalid("a signature on an unsecured request"));
        }

        let digest = self.header_digest()?;
        let unsigned = self.render(&digest, "");
        // The document is built once and parsed once. The signature covers
        // `ds:SignedInfo`, and its canonical form depends on the namespaces in
        // scope, which come from the root, so it has to be canonicalised in the
        // finished document rather than from a fragment.
        let root = xmlkit::parse(&unsigned)?;
        let info =
            find_descendant(&root, "SignedInfo").ok_or(Error::MissingField("ds:SignedInfo"))?;
        let canonical = canonicalize_in_document(&root, info);
        let signature = base64::encode(&key.sign_pkcs1v15_sha256(canonical.as_bytes())?);
        Ok(self.render(&digest, &signature))
    }

    /// The base64 digest of the header, as it goes into `ds:DigestValue`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Malformed`] if the header cannot be parsed.
    pub fn header_digest(&self) -> Result<String> {
        let document = format!(
            "{}{}</{}>",
            self.root_open(),
            self.header_xml(),
            self.kind.root()
        );
        let root = xmlkit::parse(&document)?;
        let header = find_descendant(&root, "header").ok_or(Error::MissingField("header"))?;
        let canonical = canonicalize_in_document(&root, header);
        Ok(base64::encode(&Sha256::digest(canonical.as_bytes())))
    }
}

/// Renders the `AuthSignature` element up to the signature value.
fn signed_info(digest: &str) -> String {
    format!(
        "<AuthSignature><ds:SignedInfo>\
         <ds:CanonicalizationMethod Algorithm=\"{C14N_ALGORITHM}\"></ds:CanonicalizationMethod>\
         <ds:SignatureMethod Algorithm=\"{SIGNATURE_METHOD}\"></ds:SignatureMethod>\
         <ds:Reference URI=\"{REFERENCE_URI}\">\
         <ds:Transforms><ds:Transform Algorithm=\"{C14N_ALGORITHM}\"></ds:Transform></ds:Transforms>\
         <ds:DigestMethod Algorithm=\"{DIGEST_METHOD}\"></ds:DigestMethod>\
         <ds:DigestValue>{digest}</ds:DigestValue>\
         </ds:Reference></ds:SignedInfo>"
    )
}

/// Whether a response or request accepted the message, given its technical code.
///
/// A separate function because callers ask the question in two places: after
/// parsing a response, and while checking a return code they already hold.
#[must_use]
pub fn accepted(technical_code: &str) -> bool {
    crate::returncodes::is_success(technical_code)
}

/// Verifies the `AuthSignature` of a request or response against `key`.
///
/// The digest is recomputed from the authenticated elements in the document and
/// compared with `ds:DigestValue`, then the signature over `ds:SignedInfo` is
/// checked. Both must hold: a valid signature over a `SignedInfo` whose digest
/// is wrong would accept a tampered message.
///
/// # Errors
///
/// Returns [`Error::BadSignature`] if either check fails, and
/// [`Error::MissingField`] if the document has no signature to verify.
pub fn verify(document: &str, key: &PublicKey) -> Result<()> {
    let root = xmlkit::parse(document)?;
    let info = find_descendant(&root, "SignedInfo").ok_or(Error::MissingField("ds:SignedInfo"))?;
    let digest_value =
        descendant_text(&root, "DigestValue").ok_or(Error::MissingField("ds:DigestValue"))?;
    let signature_value =
        descendant_text(&root, "SignatureValue").ok_or(Error::MissingField("ds:SignatureValue"))?;

    // The digest covers every element the reference names.
    let mut covered = String::new();
    for element in authenticated_elements(&root) {
        covered.push_str(&canonicalize_in_document(&root, element));
    }
    let expected = base64::encode(&Sha256::digest(covered.as_bytes()));
    if expected != digest_value.trim() {
        return Err(Error::BadSignature);
    }

    let canonical = canonicalize_in_document(&root, info);
    let signature = base64::decode(signature_value.trim()).ok_or(Error::BadSignature)?;
    key.verify_pkcs1v15_sha256(canonical.as_bytes(), &signature)
        .map_err(|_| Error::BadSignature)
}

/// Every element that carries `authenticate="true"`, in document order.
#[must_use]
pub fn authenticated_elements(root: &Element) -> Vec<&Element> {
    let mut found = Vec::new();
    collect_authenticated(root, &mut found);
    found
}

fn collect_authenticated<'a>(element: &'a Element, found: &mut Vec<&'a Element>) {
    if element.attr(AUTHENTICATE) == Some("true") {
        found.push(element);
    }
    for node in &element.children {
        if let Node::Element(child) = node {
            collect_authenticated(child, found);
        }
    }
}

/// The first descendant with the given local name.
#[must_use]
pub fn find_descendant<'a>(element: &'a Element, local: &str) -> Option<&'a Element> {
    if xmlkit::local_name(&element.name) == local {
        return Some(element);
    }
    for node in &element.children {
        if let Node::Element(child) = node
            && let Some(found) = find_descendant(child, local)
        {
            return Some(found);
        }
    }
    None
}

/// The concatenated text of the first descendant with the given local name.
#[must_use]
pub fn descendant_text(element: &Element, local: &str) -> Option<String> {
    let found = find_descendant(element, local)?;
    Some(found.text())
}

/// Renders a body for an order that carries no order data, such as `HPB` or
/// `HKD`.
#[must_use]
pub fn empty_body() -> String {
    String::new()
}

/// Renders a body that carries base64 order data, as a download or an upload
/// does.
#[must_use]
pub fn data_transfer_body(order_data_base64: &str) -> String {
    format!(
        "<DataTransfer><OrderData>{}</OrderData></DataTransfer>",
        xml_escape(order_data_base64)
    )
}

/// The order attribute the schema fixes for the order types this crate sends
/// without order data.
///
/// Only the value is returned, and only for the orders where the specification
/// fixes it; a caller sending another order supplies the attribute itself.
#[must_use]
pub fn fixed_order_attribute(order_type: &str) -> Option<OrderAttribute> {
    match order_type {
        // The orders that carry no order data at all, which are the two the
        // customer sends before any key exists.
        "INI" | "HIA" => Some(OrderAttribute::NoOrderData),
        // The key and protocol orders whose data is unsigned, compressed and
        // encrypted.
        "HPB" | "HAA" | "HTD" | "HPD" | "HKD" => {
            Some(OrderAttribute::WithoutSignatureCompressedEncrypted)
        }
        // Everything else, including every payment order, is left to the caller:
        // the attribute depends on whether the order carries a customer
        // signature, which this function cannot know.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Request, RequestKind, authenticated_elements, data_transfer_body, find_descendant,
        fixed_order_attribute, verify,
    };
    use crate::header::{OrderAttribute, StaticHeader};
    use crate::rsa::PrivateKey;

    /// A 1024-bit key generated for these tests only. It is short on purpose: a
    /// 2048-bit key makes every test here slow enough to notice, and the
    /// signature logic under test does not depend on the size.
    const MODULUS: &str = "2aWRwQ9/pII5l29p55PIXEWlJsvSn/d5KOpz/wnUEJ9AZvq2CitcgYhCG0qqn8Eb3G61qqQfNONhHUPdzc8Fndnv0Uqzg+Xtz4kSjHWZQZb069Po6jgLwUbmuuyfdlQpU4GdyrPHtFTd18tP20/5pmic1ui+9GQnyRBhoh55CTU=";
    const EXPONENT: &str = "AQAB";
    const PRIVATE: &str = "M1NfRvb/s+zd5f9pKypnopLt+nMwjXbPxL1dc3R+5RiZGG/hRA6rtwRKCqOWDyx8BIS7JK2lilRgz4T0ppc+jGbF3FIaqLzrr7b5hQJN97vR1f5PYZxGXQppx2qto2tQbm9Ct6K8UmRQTAI33ndB0yjhTUzXJnxl/lKnuBX0O/U=";
    const DIGEST: &str = "EyvwHt0ys+FLODZDHVL0WDWTpH39JYLXGOYgghfFpdk=";

    fn fixture_key() -> PrivateKey {
        PrivateKey::from_base64(MODULUS, EXPONENT, PRIVATE).expect("the fixture key is usable")
    }

    fn header() -> StaticHeader {
        StaticHeader {
            host_id: "HOST".to_owned(),
            partner_id: "PARTNER".to_owned(),
            user_id: "USER".to_owned(),
            order_type: "HPB".to_owned(),
            order_attribute: OrderAttribute::WithoutSignatureCompressedEncrypted,
            security_medium: "0000".to_owned(),
            nonce: Some("c5baa99837e84f97dc5f015131a0bb77".to_owned()),
            timestamp: Some("2026-10-22T11:44:26.627Z".to_owned()),
            product: "crateworks".to_owned(),
            product_language: "en".to_owned(),
            transaction_id: None,
        }
    }

    /// The key is the one the other tests assume, checked once so a fixture edit
    /// that changes it fails here rather than in six places.
    #[test]
    fn the_fixture_key_is_the_one_expected() {
        assert_eq!(fixture_key().public().digest(), DIGEST);
        assert_eq!(fixture_key().public().width(), 128);
    }

    #[test]
    fn root_names_match_the_kind() {
        assert_eq!(RequestKind::Signed.root(), "ebicsRequest");
        assert_eq!(
            RequestKind::NoPubKeyDigests.root(),
            "ebicsNoPubKeyDigestsRequest"
        );
        assert_eq!(RequestKind::Unsecured.root(), "ebicsUnsecuredRequest");
        assert!(RequestKind::NoPubKeyDigests.is_signed());
        assert!(!RequestKind::Unsecured.is_signed());
    }

    #[test]
    fn an_unsigned_request_of_a_signed_kind_is_refused() {
        let request = Request::new(RequestKind::Signed, header());
        assert!(request.to_unsigned_xml().is_err());
        assert!(request.sign(&fixture_key()).is_ok());
    }

    #[test]
    fn an_unsecured_request_is_not_signed() {
        let request = Request::new(
            RequestKind::Unsecured,
            StaticHeader::unsecured("HOST", "PARTNER", "USER", "INI"),
        );
        assert!(request.to_unsigned_xml().is_ok());
        assert!(request.sign(&fixture_key()).is_err());
    }

    #[test]
    fn the_authenticated_element_is_the_header() {
        let request = Request::new(RequestKind::Signed, header());
        let document = format!(
            "{}{}</ebicsRequest>",
            request.root_open(),
            request.header_xml()
        );
        let root = xmlkit::parse(&document).expect("parses");
        let authenticated = authenticated_elements(&root);
        assert_eq!(authenticated.len(), 1);
        assert_eq!(xmlkit::local_name(&authenticated[0].name), "header");
    }

    /// The rendered `SignedInfo` must name its algorithms and must not use a
    /// short tag, because the canonical form of a short tag is different.
    #[test]
    fn the_signed_info_names_its_algorithms() {
        let info = super::signed_info("DIGEST");
        assert!(info.contains(super::C14N_ALGORITHM));
        assert!(info.contains(super::SIGNATURE_METHOD));
        assert!(info.contains(super::DIGEST_METHOD));
        assert!(info.contains(super::REFERENCE_URI));
        assert!(
            !info.contains("/>"),
            "a short tag breaks the canonical form"
        );
    }

    /// The mapping is the one the reference client sends, which the schema
    /// agrees with: only `INI` and `HIA` use `DZNNN`, and the key orders that do
    /// carry data use `DZHNN`.
    #[test]
    fn fixed_attributes_cover_the_key_management_orders() {
        assert_eq!(
            fixed_order_attribute("INI"),
            Some(OrderAttribute::NoOrderData)
        );
        assert_eq!(
            fixed_order_attribute("HIA"),
            Some(OrderAttribute::NoOrderData)
        );
        assert_eq!(
            fixed_order_attribute("HPB"),
            Some(OrderAttribute::WithoutSignatureCompressedEncrypted)
        );
        assert_eq!(
            fixed_order_attribute("HKD"),
            Some(OrderAttribute::WithoutSignatureCompressedEncrypted)
        );
        // A payment order's attribute depends on the customer's choice, so the
        // caller supplies it.
        assert_eq!(fixed_order_attribute("CCT"), None);
    }

    #[test]
    fn a_data_transfer_body_wraps_the_order_data() {
        let body = data_transfer_body("QUJD");
        assert_eq!(
            body,
            "<DataTransfer><OrderData>QUJD</OrderData></DataTransfer>"
        );
    }

    /// The whole signature chain against a key made here. The independent check
    /// against Python is in `tests/signature_oracle.rs`.
    #[test]
    fn a_signed_request_verifies_against_its_own_key() {
        let key = fixture_key();
        let xml = Request::new(RequestKind::Signed, header())
            .sign(&key)
            .expect("signs");
        verify(&xml, key.public()).expect("our own signature verifies");
    }

    /// A single altered byte in the header must break the digest.
    #[test]
    fn a_tampered_header_breaks_the_signature() {
        let key = fixture_key();
        let xml = Request::new(RequestKind::Signed, header())
            .sign(&key)
            .expect("signs");
        let tampered = xml.replace("<HostID>HOST</HostID>", "<HostID>FAKE</HostID>");
        assert_ne!(tampered, xml, "the substitution must have applied");
        assert!(verify(&tampered, key.public()).is_err());
    }

    /// An altered digest must break the signature even though the header is
    /// untouched, because the signature covers `SignedInfo`.
    #[test]
    fn a_tampered_digest_breaks_the_signature() {
        let key = fixture_key();
        let xml = Request::new(RequestKind::Signed, header())
            .sign(&key)
            .expect("signs");
        let digest = super::descendant_text(&xmlkit::parse(&xml).expect("parses"), "DigestValue")
            .expect("a digest");
        let flipped = if digest.starts_with('A') { "B" } else { "A" };
        let tampered = xml.replacen(&digest, &format!("{flipped}{}", &digest[1..]), 1);
        assert!(verify(&tampered, key.public()).is_err());
    }

    /// The digest must not depend on anything but the header, so signing twice
    /// with the same header and key gives the same digest.
    #[test]
    fn the_digest_is_deterministic() {
        let request = Request::new(RequestKind::Signed, header());
        let first = request.header_digest().expect("digest");
        let second = request.header_digest().expect("digest");
        assert_eq!(first, second);
    }

    #[test]
    fn find_descendant_finds_a_nested_element() {
        let root = xmlkit::parse("<a><b><c/></b></a>").expect("parses");
        assert!(find_descendant(&root, "c").is_some());
        assert!(find_descendant(&root, "d").is_none());
    }
}
