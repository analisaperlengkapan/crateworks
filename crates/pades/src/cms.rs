//! The CMS `SignedData` structure inside a PDF signature's `/Contents`.
//!
//! This is the half of a PAdES signature that is DER rather than PDF. What is
//! read here is what a verifier needs: the digest algorithm, the signed
//! attributes, the signature bytes, and the certificates the signer sent along.
//! Nothing is re-encoded: the signed attributes are hashed exactly as they
//! appear in the document, with the one tag substitution RFC 5652 requires.

use crate::der::{self, Element};
use crate::sha2::Algorithm;

/// What can go wrong while reading a signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The DER could not be read.
    Der(der::Error),
    /// The content type is not `signedData`, so this is not a CMS signature.
    NotSignedData,
    /// A field the signature must carry is absent.
    Missing(&'static str),
    /// A field is present but not a value this crate handles.
    Unsupported(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Der(error) => write!(f, "the signature is not valid DER: {error}"),
            Self::NotSignedData => write!(f, "the content is not a CMS SignedData"),
            Self::Missing(field) => write!(f, "the signature has no {field}"),
            Self::Unsupported(field) => write!(f, "the {field} is not one this crate handles"),
        }
    }
}

impl std::error::Error for Error {}

impl From<der::Error> for Error {
    fn from(error: der::Error) -> Self {
        Self::Der(error)
    }
}

/// A result from this module.
pub type Result<T> = std::result::Result<T, Error>;

/// The `signedData` content type, `1.2.840.113549.1.7.2`.
const SIGNED_DATA: &str = "1.2.840.113549.1.7.2";
/// The `data` content type, `1.2.840.113549.1.7.1`.
const DATA: &str = "1.2.840.113549.1.7.1";
/// The `messageDigest` signed attribute, `1.2.840.113549.1.9.4`.
const MESSAGE_DIGEST: &str = "1.2.840.113549.1.9.4";
/// The `contentType` signed attribute, `1.2.840.113549.1.9.3`.
const CONTENT_TYPE: &str = "1.2.840.113549.1.9.3";
/// The `signingTime` signed attribute, `1.2.840.113549.1.9.5`.
const SIGNING_TIME: &str = "1.2.840.113549.1.9.5";
/// The `signatureTimeStampToken` unsigned attribute, `1.2.840.113549.1.9.16.2.14`.
const SIGNATURE_TIMESTAMP: &str = "1.2.840.113549.1.9.16.2.14";

/// A parsed CMS `SignedData`.
#[derive(Debug, Clone)]
pub struct SignedData {
    /// The digest algorithm the signer used.
    pub digest: Algorithm,
    /// The signed attributes, as they were signed: the bytes to hash when
    /// checking the signature, with the tag already replaced by `SET`.
    pub signed_attributes: Option<Vec<u8>>,
    /// The digest of the content, from the `messageDigest` attribute.
    pub message_digest: Option<Vec<u8>>,
    /// The time the signer claims, from the `signingTime` attribute, if present.
    pub signing_time: Option<String>,
    /// The signature over the signed attributes.
    pub signature: Vec<u8>,
    /// The algorithm the signature was made with, in dotted form.
    pub signature_algorithm: String,
    /// The certificate the signer identified, DER-encoded, if it was sent.
    pub signer_certificate: Option<Vec<u8>>,
    /// Every certificate in the container, DER-encoded.
    pub certificates: Vec<Vec<u8>>,
    /// The embedded RFC 3161 timestamp token, DER-encoded, if present.
    pub timestamp_token: Option<Vec<u8>>,
}

/// Parses the DER of a PDF signature's `/Contents`.
///
/// The bytes are usually the `ContentInfo` wrapping a `SignedData` directly,
/// which is what RFC 5652 defines and what ETSI requires. A few producers wrap
/// it in an extra OCTET STRING, so that is unwrapped when the first read fails.
///
/// # Errors
///
/// [`Error::NotSignedData`] when the content is not a CMS signature, and
/// [`Error::Missing`] for a field that has to be there.
pub fn parse(bytes: &[u8]) -> Result<SignedData> {
    match parse_inner(bytes) {
        Ok(signed) => Ok(signed),
        Err(Error::Der(_)) => {
            // One producer wraps the ContentInfo in an OCTET STRING. Accepting it
            // is cheap and refusing it would reject real documents.
            let mut outer = der::Reader::new(bytes);
            let element = outer.read()?;
            if element.tag != der::OCTET_STRING {
                return Err(Error::NotSignedData);
            }
            parse_inner(element.value)
        }
        Err(other) => Err(other),
    }
}

fn parse_inner(bytes: &[u8]) -> Result<SignedData> {
    // A PDF `/Contents` is a fixed-size placeholder, so the DER is followed by
    // zero padding. The element is read and whatever follows it is ignored;
    // requiring the DER to fill the buffer would reject every real signature.
    let mut outer = der::Reader::new(bytes);
    let content_info = outer.read()?;
    if content_info.tag != der::SEQUENCE {
        return Err(Error::NotSignedData);
    }
    let mut reader = content_info.reader();
    let content_type = reader.expect(der::OID)?;
    if content_type.oid()? != SIGNED_DATA {
        return Err(Error::NotSignedData);
    }
    let wrapped = reader.expect(der::context_constructed(0))?;
    let signed_data = der::one(wrapped.value)?;
    if signed_data.tag != der::SEQUENCE {
        return Err(Error::NotSignedData);
    }
    read_signed_data(&signed_data)
}

/// Reads the `SignedData` SEQUENCE: version, digests, content, certificates,
/// then the signer infos.
fn read_signed_data(signed_data: &Element<'_>) -> Result<SignedData> {
    let mut reader = signed_data.reader();
    let _version = reader.expect(der::INTEGER)?;

    // digestAlgorithms: a SET OF AlgorithmIdentifier.
    let digest_algorithms = reader.expect(der::SET)?;
    let mut digests = digest_algorithms.reader();
    let first = digests.read()?;
    let digest = read_algorithm(&first)?;

    // encapContentInfo: the content type, then an optional `[0]` content.
    let content_info = reader.expect(der::SEQUENCE)?;
    let mut content = content_info.reader();
    let content_type = content.expect(der::OID)?;
    if content_type.oid()? != DATA {
        // A detached signature over something other than `data` is not what a
        // PDF uses. Its digest would still be in the signed attributes, but the
        // document would not mean what a caller thinks it means.
        return Err(Error::Unsupported("encapsulated content type"));
    }

    // certificates: `[0]` IMPLICIT SET OF CertificateChoices, optional.
    let certificates = match reader.optional(der::context_constructed(0))? {
        Some(element) => read_certificates(&element)?,
        None => Vec::new(),
    };

    // signerInfos: a SET OF SignerInfo. A PDF signature carries one; more than
    // one is legal CMS but not a shape a PDF signature uses.
    let signer_infos = reader.expect(der::SET)?;
    let mut signers = signer_infos.reader();
    let signer = signers.read()?;
    let mut signed = read_signer_info(&signer, &certificates)?;
    signed.digest = digest;
    Ok(signed)
}

/// Reads the certificates in a `[0]` IMPLICIT SET.
///
/// The certificates are kept as the exact bytes they were read as, because the
/// signer identifier is matched against their encoding, not against a re-encoding
/// that might differ.
fn read_certificates(element: &Element<'_>) -> Result<Vec<Vec<u8>>> {
    let mut reader = element.reader();
    let mut out = Vec::new();
    while !reader.is_empty() {
        let before = reader.remaining();
        reader.read()?;
        let consumed = before.len() - reader.remaining().len();
        out.push(before[..consumed].to_vec());
    }
    Ok(out)
}

/// Reads one `SignerInfo`.
fn read_signer_info(element: &Element<'_>, certificates: &[Vec<u8>]) -> Result<SignedData> {
    let mut reader = element.reader();
    let _version = reader.expect(der::INTEGER)?;

    // sid: a CHOICE. `issuerAndSerialNumber` is a plain SEQUENCE, and
    // `subjectKeyIdentifier` is `[0]`. RFC 5652 defines it that way, and reading
    // the SEQUENCE case as a context tag makes every ordinary signature fail.
    let sid = reader.read()?;
    let issuer_and_serial = match sid.tag {
        der::SEQUENCE => Some(sid),
        tag if tag == der::context(0) => None,
        _ => return Err(Error::Missing("signer identifier")),
    };

    let digest_algorithm = reader.expect(der::SEQUENCE)?;
    let digest = read_algorithm(&digest_algorithm)?;

    // signedAttrs: `[0]` IMPLICIT SET OF Attribute, optional but present for a
    // CAdES signature, which PAdES requires.
    let attributes = match reader.optional(der::context_constructed(0))? {
        Some(attributes) => read_signed_attributes(&attributes)?,
        None => SignedAttributes::default(),
    };

    let signature_algorithm = reader.expect(der::SEQUENCE)?;
    let signature_oid = read_algorithm_oid(&signature_algorithm)?;
    let signature = reader.expect(der::OCTET_STRING)?;

    // unsignedAttrs: `[1]`, where an RFC 3161 timestamp lives.
    let mut timestamp_token = None;
    if let Some(attributes) = reader.optional(der::context_constructed(1))? {
        timestamp_token = find_timestamp(&attributes)?;
    }

    let signer_certificate = issuer_and_serial
        .and_then(|sid| match_certificate(certificates, sid))
        .or_else(|| certificates.first().cloned());

    Ok(SignedData {
        digest,
        signed_attributes: attributes.encoded,
        message_digest: attributes.message_digest,
        signing_time: attributes.signing_time,
        signature: signature.value.to_vec(),
        signature_algorithm: signature_oid,
        signer_certificate,
        certificates: certificates.to_vec(),
        timestamp_token,
    })
}

/// Reads a SET OF Attribute, returning the attributes this crate uses.
///
/// The signed-attribute bytes are re-emitted with the tag changed from `[0]` to
/// `SET`, because that is what RFC 5652 says the signature is computed over. The
/// substitution is the whole reason this returns bytes and not a structure: the
/// signature was made over the encoding, so the encoding is what has to be
/// hashed, and re-encoding the attributes could produce a different one.
fn read_signed_attributes(element: &Element<'_>) -> Result<SignedAttributes> {
    let mut reader = element.reader();
    let mut message_digest = None;
    let mut signing_time = None;
    while !reader.is_empty() {
        let attribute = reader.expect(der::SEQUENCE)?;
        let mut fields = attribute.reader();
        let kind = fields.expect(der::OID)?;
        let values = fields.expect(der::SET)?;
        let mut list = values.reader();
        match kind.oid()?.as_str() {
            MESSAGE_DIGEST => {
                if let Ok(value) = list.read() {
                    message_digest = Some(value.value.to_vec());
                }
            }
            CONTENT_TYPE => {
                if let Ok(value) = list.read()
                    && value.oid()? != DATA
                {
                    return Err(Error::Unsupported("signed content type"));
                }
            }
            SIGNING_TIME => {
                if let Ok(value) = list.read() {
                    signing_time = Some(String::from_utf8_lossy(value.value).into_owned());
                }
            }
            _ => {}
        }
    }
    // `[0]` and `SET` differ in exactly one bit, so the substitution is a
    // single-byte change and every other byte is preserved.
    let mut bytes = element.value.to_vec();
    let mut encoded = Vec::with_capacity(bytes.len() + 4);
    encoded.push(der::SET);
    write_length(&mut encoded, bytes.len());
    encoded.append(&mut bytes);
    Ok(SignedAttributes {
        encoded: Some(encoded),
        message_digest,
        signing_time,
    })
}

/// The signed attributes a verifier uses, and the bytes they were signed as.
#[derive(Debug, Default)]
struct SignedAttributes {
    /// The attributes re-encoded with the `SET` tag the signature covers.
    encoded: Option<Vec<u8>>,
    /// The `messageDigest` attribute.
    message_digest: Option<Vec<u8>>,
    /// The `signingTime` attribute.
    signing_time: Option<String>,
}

/// Finds the RFC 3161 timestamp token among the unsigned attributes.
fn find_timestamp(element: &Element<'_>) -> Result<Option<Vec<u8>>> {
    let mut reader = element.reader();
    while !reader.is_empty() {
        let attribute = reader.expect(der::SEQUENCE)?;
        let mut fields = attribute.reader();
        let kind = fields.expect(der::OID)?;
        let values = fields.expect(der::SET)?;
        if kind.oid()? == SIGNATURE_TIMESTAMP {
            let mut list = values.reader();
            if let Ok(value) = list.read() {
                return Ok(Some(value.value.to_vec()));
            }
        }
    }
    Ok(None)
}

/// Reads an `AlgorithmIdentifier`, returning the digest it names.
fn read_algorithm(element: &Element<'_>) -> Result<Algorithm> {
    let oid = read_algorithm_oid(element)?;
    Algorithm::from_oid(&oid).ok_or(Error::Unsupported("digest algorithm"))
}

/// The object identifier inside an `AlgorithmIdentifier`.
fn read_algorithm_oid(element: &Element<'_>) -> Result<String> {
    let mut reader = element.reader();
    let oid = reader.expect(der::OID)?;
    Ok(oid.oid()?)
}

/// Finds the certificate whose issuer and serial match the signer identifier.
fn match_certificate(certificates: &[Vec<u8>], sid: Element<'_>) -> Option<Vec<u8>> {
    let mut fields = sid.reader();
    let issuer = fields.expect(der::SEQUENCE).ok()?;
    let serial = fields.expect(der::INTEGER).ok()?;
    let serial = serial.unsigned_bytes();
    for certificate in certificates {
        // A certificate that cannot be read is skipped, not fatal: the signer
        // identifier may name a certificate this crate has no use for.
        let Ok(parsed) = crate::x509::Certificate::parse(certificate) else {
            continue;
        };
        if parsed.serial == serial && parsed.issuer_bytes == issuer.value {
            return Some(certificate.clone());
        }
    }
    None
}

/// Writes a DER length, short or long form.
///
/// Shared with the X.509 reader, which re-encodes a `Name` to match against a
/// signer identifier.
pub(crate) fn write_length(out: &mut Vec<u8>, length: usize) {
    if length < 0x80 {
        out.push(length as u8);
        return;
    }
    let bytes = length.to_be_bytes();
    let first = bytes
        .iter()
        .position(|&byte| byte != 0)
        .unwrap_or(bytes.len() - 1);
    let significant = &bytes[first..];
    out.push(0x80 | significant.len() as u8);
    out.extend_from_slice(significant);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `/Contents` of `crates/pdfa/tests/fixtures/signed_lt.pdf`, the DER a
    /// real pyHanko signature produced. Kept as a fixture rather than a literal
    /// so the test reads what a producer actually wrote.
    fn fixture() -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/signed_lt_contents.hex");
        let text = std::fs::read_to_string(&path).expect("the fixture is present");
        let clean: String = text.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        (0..clean.len() / 2)
            .map(|index| u8::from_str_radix(&clean[index * 2..index * 2 + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn reads_the_digest_and_algorithm() {
        let signed = parse(&fixture()).unwrap();
        assert_eq!(signed.digest, Algorithm::Sha256);
        assert_eq!(signed.signature_algorithm, "1.2.840.113549.1.1.11");
    }

    #[test]
    fn reads_the_message_digest() {
        let signed = parse(&fixture()).unwrap();
        let digest = signed.message_digest.expect("the attribute is present");
        assert_eq!(digest.len(), 32, "SHA-256 is 32 bytes");
    }

    #[test]
    fn the_signed_attributes_start_with_the_set_tag() {
        // The signature is computed over the attributes with the tag replaced,
        // so getting this byte wrong makes every verification fail.
        let signed = parse(&fixture()).unwrap();
        let attributes = signed
            .signed_attributes
            .expect("the attributes are present");
        assert_eq!(attributes[0], der::SET);
        assert_eq!(attributes[0], 0x31);
    }

    #[test]
    fn reads_the_signature_and_the_certificate() {
        let signed = parse(&fixture()).unwrap();
        assert_eq!(signed.signature.len(), 256, "a 2048-bit signature");
        assert_eq!(signed.certificates.len(), 1);
        assert!(signed.signer_certificate.is_some());
    }

    #[test]
    fn a_pades_signature_carries_no_signing_time() {
        // pyHanko writes no `signingTime` for a PAdES signature: ETSI expects the
        // time to come from an RFC 3161 timestamp, which is what `B-T` adds. A
        // test that demanded the attribute would be asserting something the
        // profile forbids.
        let signed = parse(&fixture()).unwrap();
        assert_eq!(signed.signing_time, None);
    }

    #[test]
    fn rejects_something_that_is_not_a_signature() {
        assert_eq!(
            parse(&[0x02, 0x01, 0x05]).unwrap_err(),
            Error::NotSignedData
        );
        assert!(parse(b"not der at all").is_err());
    }

    #[test]
    fn writes_a_short_length() {
        let mut out = Vec::new();
        write_length(&mut out, 5);
        assert_eq!(out, vec![0x05]);
    }

    #[test]
    fn writes_a_long_length() {
        let mut out = Vec::new();
        write_length(&mut out, 200);
        assert_eq!(out, vec![0x81, 0xC8]);
        let mut out = Vec::new();
        write_length(&mut out, 0x1234);
        assert_eq!(out, vec![0x82, 0x12, 0x34]);
    }
}
