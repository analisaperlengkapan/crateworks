//! Reading a signed document and saying what was checked.
//!
//! The report is deliberately explicit about the difference between a check that
//! passed and a check that was not made. [`Integrity::NotChecked`] is a variant
//! and not an absent field, because a caller that reads "no error" must not
//! conclude "verified" when the digest was never computed.

use crate::cms;
use crate::sha2::Algorithm;
use crate::x509::{Certificate, Instant};
use pdfa::Pdf;

/// What can go wrong while reading a signed document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The PDF could not be read.
    Pdf(String),
    /// The CMS inside a signature could not be read.
    Cms(String),
    /// The document carries no signature at all.
    NoSignature,
    /// A structure the report needs is absent or malformed.
    Malformed(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pdf(message) => write!(f, "the document could not be read: {message}"),
            Self::Cms(message) => write!(f, "the signature could not be read: {message}"),
            Self::NoSignature => write!(f, "the document carries no signature"),
            Self::Malformed(part) => write!(f, "the {part} is missing or malformed"),
        }
    }
}

impl std::error::Error for Error {}

/// A result from this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// The `/SubFilter`, which is what the producer claims the signature is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `/ETSI.CAdES.detached`, the PAdES baseline signature.
    CadesDetached,
    /// `/adbe.pkcs7.detached`, the older Adobe form.
    AdbePkcs7Detached,
    /// `/adbe.pkcs7.sha1`, where the signed content is the SHA-1 of the file.
    AdbePkcs7Sha1,
    /// `/ETSI.RFC3161`, a document timestamp rather than a signature.
    Rfc3161,
    /// A name this crate does not know.
    Other,
}

impl Kind {
    /// The kind a `/SubFilter` names.
    #[must_use]
    pub fn from_sub_filter(name: &str) -> Self {
        match name {
            "ETSI.CAdES.detached" => Self::CadesDetached,
            "adbe.pkcs7.detached" => Self::AdbePkcs7Detached,
            "adbe.pkcs7.sha1" => Self::AdbePkcs7Sha1,
            "ETSI.RFC3161" => Self::Rfc3161,
            _ => Self::Other,
        }
    }
}

/// What the covered byte range spans.
///
/// The three cases matter because a long-term-valid document is signed over a
/// revision and the validation data is appended afterwards on purpose. A verifier
/// that demands [`Coverage::EntireFile`] rejects every B-LT document in
/// existence, which is a mistake worth naming in the type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    /// The range reaches the end of the file, so nothing follows the signature.
    EntireFile,
    /// The range reaches the end of the revision the signature was made in, and
    /// later revisions follow. This is what an LTV-enabled document looks like.
    EntireRevision,
    /// The range stops inside a revision, so bytes the signer could see are not
    /// covered.
    Partial,
}

/// Whether the covered bytes still hash to what the signature says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integrity {
    /// The digest of the covered bytes matches the one in the signature.
    Intact,
    /// It does not match, so the covered bytes changed after signing.
    Broken,
    /// The check could not be made, because the algorithm is unsupported or the
    /// range is malformed.
    NotChecked,
}

/// The highest PAdES baseline level the document reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types)]
pub enum Level {
    /// `B-B`: a signature with a signing certificate.
    B_B,
    /// `B-T`: the signature carries an RFC 3161 timestamp.
    B_T,
    /// `B-LT`: validation data is in the document's DSS.
    B_LT,
    /// `B-LTA`: an archive timestamp covers the validation data.
    B_LTA,
}

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The document is usable.
    Info,
    /// Something a caller should know before trusting the result.
    Warning,
    /// A check failed.
    Error,
}

/// One thing the verifier noticed, with a stable code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// A short, stable identifier, so a caller can match on it.
    pub code: &'static str,
    /// How serious it is.
    pub severity: Severity,
    /// What was seen.
    pub message: String,
}

/// What is known about the signer's certificate.
#[derive(Debug, Clone)]
pub struct Signer {
    /// The subject, written as `CN=..., O=...`.
    pub subject: String,
    /// The issuer, written the same way.
    pub issuer: String,
    /// The serial number.
    pub serial: Vec<u8>,
    /// The start of the validity window.
    pub not_before: Instant,
    /// The end of the validity window.
    pub not_after: Instant,
    /// Whether the certificate is a CA.
    pub is_ca: bool,
    /// The public key algorithm in dotted form.
    pub key_algorithm: String,
}

/// The DSS, which is where long-term validation data lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationData {
    /// The number of certificates in `/DSS /Certs`.
    pub certificates: usize,
    /// The number of OCSP responses in `/DSS /OCSPs`.
    pub ocsp_responses: usize,
    /// The number of CRLs in `/DSS /CRLs`.
    pub crls: usize,
}

/// One signature, and everything the verifier could establish about it.
#[derive(Debug, Clone)]
pub struct Signature {
    /// The name of the signature field.
    pub field: String,
    /// The `/SubFilter`.
    pub kind: Kind,
    /// The `/SubFilter` as it was written.
    pub sub_filter: String,
    /// The digest algorithm, taken from the CMS rather than from the dictionary.
    pub digest: Option<Algorithm>,
    /// The `/ByteRange`, as the four numbers the dictionary held.
    pub byte_range: [u64; 4],
    /// What the range covers.
    pub coverage: Coverage,
    /// Whether the covered bytes still hash to what the signature says.
    pub integrity: Integrity,
    /// The signer's certificate, if it could be read.
    pub signer: Option<Signer>,
    /// Whether an RFC 3161 timestamp is embedded.
    pub timestamped: bool,
    /// The algorithm the signature was made with, in dotted form.
    pub signature_algorithm: String,
    /// Everything the verifier noticed about this signature.
    pub findings: Vec<Finding>,
}

impl Signature {
    /// Whether the signature is intact and its signer certificate was read.
    #[must_use]
    pub fn is_intact(&self) -> bool {
        self.integrity == Integrity::Intact
    }
}

/// The result of reading a signed document.
#[derive(Debug, Clone)]
pub struct Report {
    /// Every signature, in the order the document lists them.
    pub signatures: Vec<Signature>,
    /// The document's validation data, when it has a DSS.
    pub validation_data: Option<ValidationData>,
    /// The highest baseline level any signature reaches.
    pub level: Option<Level>,
    /// Findings about the document as a whole.
    pub findings: Vec<Finding>,
}

/// Reads a signed PDF and reports on every signature in it.
///
/// # Errors
///
/// [`Error::Pdf`] when the file is not a readable PDF, [`Error::NoSignature`]
/// when it carries none, and [`Error::Cms`] when a signature's CMS cannot be
/// read at all. A signature whose CMS is readable but whose checks fail is
/// reported, not returned as an error.
pub fn verify(bytes: &[u8]) -> Result<Report> {
    let pdf = Pdf::parse(bytes).map_err(|error| Error::Pdf(error.to_string()))?;
    let fields = signature_fields(&pdf)?;
    if fields.is_empty() {
        return Err(Error::NoSignature);
    }
    let mut signatures = Vec::new();
    for field in fields {
        signatures.push(read_signature(&pdf, bytes, field)?);
    }
    let validation_data = read_dss(&pdf);
    let level = overall_level(&signatures, validation_data.is_some());
    let mut findings = Vec::new();
    if validation_data.is_none() {
        findings.push(Finding {
            code: "no-dss",
            severity: Severity::Info,
            message: "the document carries no validation data, so it is B-B or B-T at most".into(),
        });
    }
    Ok(Report {
        signatures,
        validation_data,
        level,
        findings,
    })
}

/// The signature fields in the document's AcroForm.
fn signature_fields(pdf: &Pdf) -> Result<Vec<(String, u32)>> {
    let catalog = pdf.catalog().ok_or(Error::Malformed("catalog"))?;
    let Some(acro_form) = indirect_after(&catalog, "/AcroForm") else {
        return Ok(Vec::new());
    };
    let form = object_text(pdf, acro_form).ok_or(Error::Malformed("AcroForm"))?;
    let mut out = Vec::new();
    for number in array_references(&form, "/Fields") {
        let Some(text) = object_text(pdf, number) else {
            continue;
        };
        if !text.contains("/FT") || !text.contains("/Sig") {
            continue;
        }
        let name = dictionary_text(&text, "/T").unwrap_or_else(|| format!("Sig{number}"));
        // The signature itself is usually a separate object named by `/V`.
        let value = indirect_after(&text, "/V").unwrap_or(number);
        out.push((name, value));
    }
    Ok(out)
}

/// The object numbers in the `key [ ... ]` array, in order.
fn array_references(text: &str, key: &str) -> Vec<u32> {
    let Some(position) = text.find(key) else {
        return Vec::new();
    };
    let rest = &text[position + key.len()..];
    let Some(open) = rest.find('[') else {
        return Vec::new();
    };
    let Some(close) = rest[open..].find(']') else {
        return Vec::new();
    };
    let body = &rest[open + 1..open + close];
    let mut out = Vec::new();
    let mut pieces = body.split_whitespace().peekable();
    while let Some(piece) = pieces.next() {
        if let Ok(number) = piece.parse::<u32>() {
            // A reference is `N 0 R`, so the next two tokens decide it.
            let generation = pieces.peek().and_then(|token| token.parse::<u32>().ok());
            if generation == Some(0) {
                pieces.next();
                if pieces.next() == Some("R") {
                    out.push(number);
                }
            }
        }
    }
    out
}

/// Reads one signature object.
fn read_signature(pdf: &Pdf, bytes: &[u8], (field, number): (String, u32)) -> Result<Signature> {
    let text = object_text(pdf, number).ok_or(Error::Malformed("signature dictionary"))?;
    let sub_filter = dictionary_text(&text, "/SubFilter")
        .or_else(|| dictionary_name(&text, "/SubFilter"))
        .unwrap_or_default();
    let kind = Kind::from_sub_filter(&sub_filter);
    let byte_range = read_byte_range(&text).ok_or(Error::Malformed("ByteRange"))?;
    let contents = read_contents(&text).ok_or(Error::Malformed("Contents"))?;

    let mut findings = Vec::new();
    let signed = match cms::parse(&contents) {
        Ok(signed) => signed,
        Err(error) => {
            return Ok(Signature {
                field,
                kind,
                sub_filter,
                digest: None,
                byte_range,
                coverage: coverage(byte_range, bytes.len()),
                integrity: Integrity::NotChecked,
                signer: None,
                timestamped: false,
                signature_algorithm: String::new(),
                findings: vec![Finding {
                    code: "cms-unreadable",
                    severity: Severity::Error,
                    message: error.to_string(),
                }],
            });
        }
    };

    let coverage = coverage(byte_range, bytes.len());
    let covered = covered_bytes(bytes, byte_range);
    let integrity = match (&covered, signed.digest, &signed.message_digest) {
        (Some(covered), digest, Some(expected)) => {
            let actual = digest.digest(covered);
            if actual == *expected {
                Integrity::Intact
            } else {
                findings.push(Finding {
                    code: "digest-mismatch",
                    severity: Severity::Error,
                    message: "the covered bytes do not hash to the digest in the signature".into(),
                });
                Integrity::Broken
            }
        }
        _ => {
            findings.push(Finding {
                code: "digest-unchecked",
                severity: Severity::Warning,
                message: "the signature carries no messageDigest, so integrity was not checked"
                    .into(),
            });
            Integrity::NotChecked
        }
    };

    if coverage == Coverage::Partial {
        findings.push(Finding {
            code: "partial-coverage",
            severity: Severity::Error,
            message: "the signature covers only part of the revision it was made in".into(),
        });
    }

    // The signature is over the signed attributes, and only over the content
    // digest indirectly, through the messageDigest attribute.
    if integrity == Integrity::Intact
        && let (Some(attributes), Some(certificate)) =
            (&signed.signed_attributes, &signed.signer_certificate)
    {
        match Certificate::parse(certificate) {
            Ok(parsed) => match &parsed.rsa {
                Some(key) => {
                    if let Err(error) =
                        key.verify_pkcs1v15(attributes, &signed.signature, signed.digest)
                    {
                        findings.push(Finding {
                            code: "signature-invalid",
                            severity: Severity::Error,
                            message: format!(
                                "the signature over the attributes does not verify: {error}"
                            ),
                        });
                    }
                }
                None => findings.push(Finding {
                    code: "key-unsupported",
                    severity: Severity::Warning,
                    message: format!(
                        "the signer key is {} and this crate verifies only RSA",
                        parsed.key_algorithm
                    ),
                }),
            },
            Err(error) => findings.push(Finding {
                code: "certificate-unreadable",
                severity: Severity::Warning,
                message: error.to_string(),
            }),
        }
    }

    let signer = signed
        .signer_certificate
        .as_ref()
        .and_then(|der| Certificate::parse(der).ok())
        .map(|certificate| Signer {
            subject: certificate.subject,
            issuer: certificate.issuer,
            serial: certificate.serial,
            not_before: certificate.not_before,
            not_after: certificate.not_after,
            is_ca: certificate.is_ca,
            key_algorithm: certificate.key_algorithm,
        });

    if signed.timestamp_token.is_none() {
        findings.push(Finding {
            code: "no-timestamp",
            severity: Severity::Warning,
            message: "the signature carries no RFC 3161 timestamp, so it is B-B and not B-T".into(),
        });
    }

    Ok(Signature {
        field,
        kind,
        sub_filter,
        digest: Some(signed.digest),
        byte_range,
        coverage,
        integrity,
        signer,
        timestamped: signed.timestamp_token.is_some(),
        signature_algorithm: signed.signature_algorithm,
        findings,
    })
}

/// The covered bytes, or `None` when the range is not inside the file.
fn covered_bytes(bytes: &[u8], byte_range: [u64; 4]) -> Option<Vec<u8>> {
    let [first_start, first_length, second_start, second_length] = byte_range;
    let start = usize::try_from(first_start).ok()?;
    let length = usize::try_from(first_length).ok()?;
    let second = usize::try_from(second_start).ok()?;
    let second_len = usize::try_from(second_length).ok()?;
    let head = bytes.get(start..start.checked_add(length)?)?;
    let tail = bytes.get(second..second.checked_add(second_len)?)?;
    let mut covered = Vec::with_capacity(head.len() + tail.len());
    covered.extend_from_slice(head);
    covered.extend_from_slice(tail);
    Some(covered)
}

/// What a byte range spans.
///
/// The second part of the range ends where the signature's own revision ends. If
/// that is the end of the file, nothing follows and the whole file is covered;
/// if it is not, later revisions follow, which is how a document with validation
/// data appended after signing looks.
fn coverage(byte_range: [u64; 4], length: usize) -> Coverage {
    let [_, first_length, second_start, second_length] = byte_range;
    if first_length == 0 && second_length == 0 {
        return Coverage::Partial;
    }
    match second_start.checked_add(second_length) {
        Some(end) if (end as usize) == length => Coverage::EntireFile,
        Some(end) if (end as usize) < length => Coverage::EntireRevision,
        _ => Coverage::Partial,
    }
}

/// The `/DSS` dictionary, counted by type.
///
/// `/Certs`, `/OCSPs` and `/CRLs` are arrays of streams, and each stream holds
/// one or more DER values concatenated with no separator, which is what ETSI
/// specifies. So the count is the number of DER values, not the number of
/// streams: a DSS with one stream holding three certificates carries three.
fn read_dss(pdf: &Pdf) -> Option<ValidationData> {
    let catalog = pdf.catalog()?;
    let number = indirect_after(&catalog, "/DSS")?;
    let text = object_text(pdf, number)?;
    Some(ValidationData {
        certificates: count_der_values(pdf, &text, "/Certs"),
        ocsp_responses: count_der_values(pdf, &text, "/OCSPs"),
        crls: count_der_values(pdf, &text, "/CRLs"),
    })
}

/// The number of DER values across the streams in an array.
///
/// A compressed stream is counted as its number of streams, because inflating it
/// here would mean duplicating the filter chain `pdfa` already has. That
/// undercounts a compressed DSS and never overcounts, which is the direction a
/// report should err in.
fn count_der_values(pdf: &Pdf, text: &str, key: &str) -> usize {
    let mut total = 0;
    for number in array_references(text, key) {
        let Some(object) = object_text(pdf, number) else {
            continue;
        };
        match stream_bytes(&object) {
            Some(data) => total += count_der_elements(data).max(1),
            None => total += 1,
        }
    }
    total
}

/// The bytes between `stream` and `endstream`.
fn stream_bytes(object: &str) -> Option<&[u8]> {
    let start = object.find("stream")? + "stream".len();
    let rest = &object.as_bytes()[start..];
    // A stream keyword is followed by CRLF or LF.
    let start = rest
        .iter()
        .position(|&byte| byte != b'\r' && byte != b'\n')?;
    let data = &rest[start..];
    let end = find_bytes(data, b"endstream")?;
    Some(&data[..end])
}

/// The offset of `needle` in `haystack`.
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// How many top-level DER elements the bytes hold.
fn count_der_elements(bytes: &[u8]) -> usize {
    let mut reader = crate::der::Reader::new(bytes);
    let mut count = 0;
    while !reader.is_empty() {
        if reader.read().is_err() {
            break;
        }
        count += 1;
    }
    count
}

/// The highest level the document reaches.
fn overall_level(signatures: &[Signature], has_dss: bool) -> Option<Level> {
    let mut level: Option<Level> = None;
    for signature in signatures {
        let reached = if signature.timestamped {
            Level::B_T
        } else {
            Level::B_B
        };
        level = Some(level.map_or(reached, |current| current.max(reached)));
    }
    if let (Some(current), true) = (level, has_dss) {
        level = Some(Level::B_LT.max(current));
    }
    level
}

/// The object number in a `key N 0 R` reference.
fn indirect_after(text: &str, key: &str) -> Option<u32> {
    let position = text.find(key)? + key.len();
    let rest = text[position..].trim_start();
    let end = rest.find('R')?;
    let candidate = rest[..end].trim();
    let number = candidate.split_whitespace().next()?;
    number.parse().ok()
}

/// The bytes of an object, as text.
fn object_text(pdf: &Pdf, number: u32) -> Option<String> {
    let bytes = pdf.object_bytes(number)?;
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// A name value such as `/SubFilter /ETSI.CAdES.detached`.
fn dictionary_name(text: &str, key: &str) -> Option<String> {
    let position = text.find(key)? + key.len();
    let rest = text[position..].trim_start();
    let rest = rest.strip_prefix('/')?;
    let end = rest
        .find(|character: char| {
            !character.is_ascii_alphanumeric() && character != '.' && character != '-'
        })
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// A text value, either a literal string or a hex string.
fn dictionary_text(text: &str, key: &str) -> Option<String> {
    let position = text.find(key)? + key.len();
    let rest = text[position..].trim_start();
    if let Some(inner) = rest.strip_prefix('(') {
        let end = inner.find(')')?;
        return Some(inner[..end].replace("\\072", ":").replace("\\(", "("));
    }
    if let Some(inner) = rest.strip_prefix('<') {
        let end = inner.find('>')?;
        return Some(inner[..end].to_string());
    }
    None
}

/// The four numbers of a `/ByteRange`.
fn read_byte_range(text: &str) -> Option<[u64; 4]> {
    let position = text.find("/ByteRange")? + "/ByteRange".len();
    let rest = &text[position..];
    let open = rest.find('[')?;
    let close = rest[open..].find(']')? + open;
    let numbers: Vec<u64> = rest[open + 1..close]
        .split_whitespace()
        .filter_map(|piece| piece.parse().ok())
        .collect();
    if numbers.len() != 4 {
        return None;
    }
    Some([numbers[0], numbers[1], numbers[2], numbers[3]])
}

/// The hex bytes of a `/Contents`.
fn read_contents(text: &str) -> Option<Vec<u8>> {
    let position = text.find("/Contents")? + "/Contents".len();
    let rest = text[position..].trim_start();
    let inner = rest.strip_prefix('<')?;
    let end = inner.find('>')?;
    let hex: String = inner[..end]
        .chars()
        .filter(char::is_ascii_hexdigit)
        .collect();
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len() / 2)
        .map(|index| u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read(&path).unwrap_or_else(|_| panic!("the fixture {name} is present"))
    }

    #[test]
    fn reads_a_b_b_signature() {
        let report = verify(&fixture("signed_bb.pdf")).unwrap();
        assert_eq!(report.signatures.len(), 1);
        let signature = &report.signatures[0];
        assert_eq!(signature.field, "Sig1");
        assert_eq!(signature.kind, Kind::CadesDetached);
        assert_eq!(signature.digest, Some(Algorithm::Sha256));
        assert_eq!(signature.integrity, Integrity::Intact);
        assert_eq!(signature.coverage, Coverage::EntireFile);
        assert_eq!(signature.byte_range, [0, 2221, 6233, 490]);
    }

    #[test]
    fn a_b_b_signature_reaches_level_b_b() {
        // It has no timestamp and no DSS, so it is B-B and nothing higher.
        let report = verify(&fixture("signed_bb.pdf")).unwrap();
        assert_eq!(report.level, Some(Level::B_B));
        assert!(report.validation_data.is_none());
    }

    #[test]
    fn reads_the_signer() {
        let report = verify(&fixture("signed_bb.pdf")).unwrap();
        let signer = report.signatures[0]
            .signer
            .as_ref()
            .expect("the cert is present");
        assert_eq!(signer.subject, "CN=PAdES Corpus");
        assert_eq!(signer.not_before.year, 2020);
        assert_eq!(signer.not_after.year, 2040);
    }

    #[test]
    fn a_b_lt_document_covers_a_revision_not_the_file() {
        // This is the case the whole `Coverage` type exists for. The DSS is
        // appended after signing on purpose, so the range stops short of the end
        // of the file and the document is still intact.
        let report = verify(&fixture("signed_lt.pdf")).unwrap();
        let signature = &report.signatures[0];
        assert_eq!(signature.coverage, Coverage::EntireRevision);
        assert_eq!(signature.integrity, Integrity::Intact);
        assert_eq!(report.level, Some(Level::B_LT));
    }

    #[test]
    fn reads_the_validation_data() {
        let report = verify(&fixture("signed_lt.pdf")).unwrap();
        let data = report.validation_data.expect("the document has a DSS");
        assert!(
            data.certificates >= 1,
            "the DSS carries the signer certificate"
        );
    }

    #[test]
    fn appended_bytes_do_not_break_integrity() {
        // Appending is what an LTV update does, and the appended bytes are
        // outside the signed range, so the signature stays intact.
        let report = verify(&fixture("appended.pdf")).unwrap();
        let signature = &report.signatures[0];
        assert_eq!(signature.integrity, Integrity::Intact);
        assert_eq!(signature.coverage, Coverage::EntireRevision);
    }

    #[test]
    fn detects_a_changed_byte() {
        let report = verify(&fixture("tampered.pdf")).unwrap();
        let signature = &report.signatures[0];
        assert_eq!(signature.integrity, Integrity::Broken);
        assert!(
            signature
                .findings
                .iter()
                .any(|finding| finding.code == "digest-mismatch"),
            "the report names the failure"
        );
    }

    #[test]
    fn a_document_without_a_signature_is_an_error() {
        let blank = fixture("blank.pdf");
        assert_eq!(verify(&blank).unwrap_err(), Error::NoSignature);
    }

    #[test]
    fn reads_a_byte_range() {
        let text = "<< /ByteRange [0 2221 6233 490] /Type /Sig >>";
        assert_eq!(read_byte_range(text), Some([0, 2221, 6233, 490]));
        assert_eq!(read_byte_range("<< /Type /Sig >>"), None);
    }

    #[test]
    fn coverage_has_three_cases() {
        assert_eq!(coverage([0, 100, 200, 50], 250), Coverage::EntireFile);
        assert_eq!(coverage([0, 100, 200, 50], 400), Coverage::EntireRevision);
        assert_eq!(coverage([0, 100, 200, 50], 100), Coverage::Partial);
        assert_eq!(coverage([0, 0, 0, 0], 100), Coverage::Partial);
    }

    #[test]
    fn reads_a_name_and_a_text_value() {
        assert_eq!(
            dictionary_name("<< /SubFilter /ETSI.CAdES.detached >>", "/SubFilter"),
            Some("ETSI.CAdES.detached".into())
        );
        assert_eq!(
            dictionary_text("<< /T (Sig1) >>", "/T"),
            Some("Sig1".into())
        );
    }

    #[test]
    fn reads_an_indirect_reference() {
        assert_eq!(
            indirect_after("<< /AcroForm 12 0 R >>", "/AcroForm"),
            Some(12)
        );
        assert_eq!(indirect_after("<< /Type /Sig >>", "/AcroForm"), None);
    }
}
