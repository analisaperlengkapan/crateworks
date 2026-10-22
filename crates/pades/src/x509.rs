//! The part of X.509 a signature verifier needs: who signed, when the
//! certificate is valid, and what public key it holds.
//!
//! This is deliberately not a certificate library. It reads the fields a report
//! has to name, and it says so in the docs. Certification path building, name
//! constraints, policies, revocation lookup: none of that is here, and a crate
//! that claims it is doing full RFC 5280 validation while doing this would be
//! lying about the one thing a verifier must never lie about.

use crate::der::{self, Element};
use crate::rsa::PublicKey;

/// What can go wrong while reading a certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The DER could not be read.
    Der(der::Error),
    /// A field the certificate must carry is absent.
    Missing(&'static str),
    /// The public key is of a kind this crate does not read.
    Unsupported(&'static str),
    /// A date is not one DER allows.
    BadTime,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Der(error) => write!(f, "the certificate is not valid DER: {error}"),
            Self::Missing(field) => write!(f, "the certificate has no {field}"),
            Self::Unsupported(field) => write!(f, "the {field} is not one this crate reads"),
            Self::BadTime => write!(f, "the certificate carries a date DER does not allow"),
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

/// A moment in UTC, to the second, as a certificate writes it.
///
/// There is no `chrono` here and none is wanted: a date in a certificate is read
/// and reported, never arithmetic beyond comparing two of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    /// The year.
    pub year: i32,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month, 1 to 31.
    pub day: u8,
    /// The hour, 0 to 23.
    pub hour: u8,
    /// The minute, 0 to 59.
    pub minute: u8,
    /// The second, 0 to 59.
    pub second: u8,
}

impl std::fmt::Display for Instant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// A certificate, read far enough to report on and to verify with.
#[derive(Debug, Clone)]
pub struct Certificate {
    /// The raw DER of the certificate, as it appeared in the container.
    pub der: Vec<u8>,
    /// The serial number, with the DER sign byte removed.
    pub serial: Vec<u8>,
    /// The raw DER of the issuer `Name`, for matching a signer identifier.
    pub issuer_bytes: Vec<u8>,
    /// The issuer, written as `CN=..., O=...`.
    pub issuer: String,
    /// The subject, written the same way.
    pub subject: String,
    /// The start of the validity window.
    pub not_before: Instant,
    /// The end of the validity window.
    pub not_after: Instant,
    /// Whether the certificate is a CA, from `basicConstraints`.
    pub is_ca: bool,
    /// The public key, when it is an RSA key.
    pub rsa: Option<PublicKey>,
    /// The public key algorithm in dotted form.
    pub key_algorithm: String,
}

impl Certificate {
    /// Reads a DER certificate.
    ///
    /// # Errors
    ///
    /// [`Error::Missing`] for a field the certificate has to carry, and
    /// [`Error::Unsupported`] for a public key this crate does not read.
    pub fn parse(der: &[u8]) -> Result<Self> {
        let certificate = der::one(der)?;
        if certificate.tag != der::SEQUENCE {
            return Err(Error::Missing("certificate sequence"));
        }
        let mut outer = certificate.reader();
        let tbs = outer.expect(der::SEQUENCE)?;

        let mut reader = tbs.reader();
        // version [0] EXPLICIT INTEGER, absent for v1.
        if let Some(version) = reader.optional(der::context_constructed(0))? {
            let mut inner = version.reader();
            inner.expect(der::INTEGER)?;
        }
        let serial = reader.expect(der::INTEGER)?;
        let _signature_algorithm = reader.expect(der::SEQUENCE)?;
        let issuer = reader.expect(der::SEQUENCE)?;
        let validity = reader.expect(der::SEQUENCE)?;
        let subject = reader.expect(der::SEQUENCE)?;
        let spki = reader.expect(der::SEQUENCE)?;

        let mut window = validity.reader();
        let not_before = read_time(&window.read()?)?;
        let not_after = read_time(&window.read()?)?;

        let mut key = spki.reader();
        let algorithm = key.expect(der::SEQUENCE)?;
        let key_algorithm = {
            let mut fields = algorithm.reader();
            fields.expect(der::OID)?.oid()?
        };
        let bits = key.expect(der::BIT_STRING)?;
        let rsa = if key_algorithm == "1.2.840.113549.1.1.1" {
            Some(PublicKey::from_der(bits.bit_string()?)?)
        } else {
            None
        };

        Ok(Self {
            der: der.to_vec(),
            serial: serial.unsigned_bytes().to_vec(),
            issuer_bytes: encode(&issuer),
            issuer: read_name(&issuer)?,
            subject: read_name(&subject)?,
            not_before,
            not_after,
            is_ca: read_is_ca(&tbs),
            rsa,
            key_algorithm,
        })
    }

    /// Whether `time` falls inside the validity window.
    #[must_use]
    pub fn valid_at(&self, time: Instant) -> bool {
        self.not_before <= time && time <= self.not_after
    }
}

/// Re-encodes an element exactly as it was read.
fn encode(element: &Element<'_>) -> Vec<u8> {
    let mut out = Vec::with_capacity(element.value.len() + 4);
    out.push(element.tag);
    crate::cms::write_length(&mut out, element.value.len());
    out.extend_from_slice(element.value);
    out
}

/// Reads a `Name` as `CN=..., O=...`, in the order the certificate lists them.
fn read_name(element: &Element<'_>) -> Result<String> {
    let mut reader = element.reader();
    let mut parts = Vec::new();
    while !reader.is_empty() {
        let set = reader.expect(der::SET)?;
        let mut set_reader = set.reader();
        while !set_reader.is_empty() {
            let sequence = set_reader.expect(der::SEQUENCE)?;
            let mut fields = sequence.reader();
            let kind = fields.expect(der::OID)?;
            let value = fields.read()?;
            let label = match kind.oid()?.as_str() {
                "2.5.4.3" => "CN",
                "2.5.4.6" => "C",
                "2.5.4.7" => "L",
                "2.5.4.8" => "ST",
                "2.5.4.10" => "O",
                "2.5.4.11" => "OU",
                "2.5.4.5" => "serialNumber",
                "1.2.840.113549.1.9.1" => "emailAddress",
                other => {
                    // An unrecognised attribute is still part of the name, and
                    // dropping it would make two different names print the same.
                    parts.push(format!("{other}={}", String::from_utf8_lossy(value.value)));
                    continue;
                }
            };
            let text = String::from_utf8_lossy(value.value);
            parts.push(format!("{label}={text}"));
        }
    }
    Ok(parts.join(", "))
}

/// Whether `basicConstraints` says this is a CA.
///
/// The extension is inside the `[3]` EXPLICIT extensions field, and it is
/// searched for by identifier rather than decoded positionally, because the
/// extensions appear in whatever order the issuer chose.
fn read_is_ca(tbs: &Element<'_>) -> bool {
    const BASIC_CONSTRAINTS: &str = "2.5.29.19";
    let mut reader = tbs.reader();
    // Walk to the `[3]` field.
    let mut extensions = None;
    while !reader.is_empty() {
        match reader.read() {
            Ok(element) if element.tag == der::context_constructed(3) => {
                extensions = Some(element);
                break;
            }
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    let Some(extensions) = extensions else {
        return false;
    };
    let Ok(sequence) = der::one(extensions.value) else {
        return false;
    };
    let mut list = sequence.reader();
    while !list.is_empty() {
        let Ok(extension) = list.expect(der::SEQUENCE) else {
            return false;
        };
        let mut fields = extension.reader();
        let Ok(kind) = fields.expect(der::OID) else {
            return false;
        };
        if kind.oid().as_deref() != Ok(BASIC_CONSTRAINTS) {
            continue;
        }
        // The `critical` BOOLEAN is optional and comes before the value, so
        // expecting the OCTET STRING straight after the identifier fails on any
        // certificate that marks the extension critical, which every CA does.
        let _ = fields.optional(der::BOOLEAN);
        // The value is an OCTET STRING holding the DER of the extension.
        let Ok(encoded) = fields.expect(der::OCTET_STRING) else {
            return false;
        };
        let Ok(inner) = der::one(encoded.value) else {
            return false;
        };
        let mut inner_reader = inner.reader();
        return match inner_reader.expect(der::BOOLEAN) {
            Ok(boolean) => boolean.boolean(),
            // An absent `cA` defaults to false.
            Err(_) => false,
        };
    }
    false
}

/// Reads a `UTCTime` or a `GeneralizedTime`.
fn read_time(element: &Element<'_>) -> Result<Instant> {
    let text = std::str::from_utf8(element.value).map_err(|_| Error::BadTime)?;
    let digits: Vec<u8> = text
        .bytes()
        .take_while(u8::is_ascii_digit)
        .map(|byte| byte - b'0')
        .collect();
    let number = |from: usize, to: usize| -> Result<i32> {
        let slice = digits.get(from..to).ok_or(Error::BadTime)?;
        Ok(slice
            .iter()
            .fold(0i32, |value, digit| value * 10 + i32::from(*digit)))
    };
    match element.tag {
        // UTCTime is YYMMDDHHMMSSZ, with the seconds optional.
        der::UTC_TIME => {
            let year = number(0, 2)?;
            // RFC 5280: 50..99 is 1950..1999, 00..49 is 2000..2049.
            let year = if year >= 50 { 1900 + year } else { 2000 + year };
            Ok(Instant {
                year,
                month: number(2, 4)? as u8,
                day: number(4, 6)? as u8,
                hour: number(6, 8)? as u8,
                minute: number(8, 10)? as u8,
                second: number(10, 12).unwrap_or(0) as u8,
            })
        }
        // GeneralizedTime is YYYYMMDDHHMMSSZ.
        der::GENERALIZED_TIME => Ok(Instant {
            year: number(0, 4)?,
            month: number(4, 6)? as u8,
            day: number(6, 8)? as u8,
            hour: number(8, 10)? as u8,
            minute: number(10, 12)? as u8,
            second: number(12, 14)? as u8,
        }),
        _ => Err(Error::BadTime),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The certificate pyHanko embedded in
    /// `crates/pdfa/tests/fixtures/signed_lt.pdf`.
    fn fixture() -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/signed_lt_cert.der");
        std::fs::read(&path).expect("the fixture is present")
    }

    #[test]
    fn reads_the_subject_and_issuer() {
        let certificate = Certificate::parse(&fixture()).unwrap();
        assert_eq!(certificate.subject, "CN=PAdES Corpus");
        assert_eq!(certificate.issuer, "CN=PAdES Corpus");
    }

    #[test]
    fn reads_the_validity_window() {
        let certificate = Certificate::parse(&fixture()).unwrap();
        assert_eq!(certificate.not_before.year, 2020);
        assert_eq!(certificate.not_after.year, 2040);
        assert_eq!(certificate.not_before.month, 1);
        assert_eq!(certificate.not_before.day, 1);
    }

    #[test]
    fn reads_the_rsa_public_key() {
        let certificate = Certificate::parse(&fixture()).unwrap();
        assert_eq!(certificate.key_algorithm, "1.2.840.113549.1.1.1");
        let key = certificate.rsa.expect("the fixture has an RSA key");
        assert_eq!(key.width(), 256, "a 2048-bit modulus");
    }

    #[test]
    fn reads_the_serial_number() {
        let certificate = Certificate::parse(&fixture()).unwrap();
        assert!(!certificate.serial.is_empty());
        // The DER sign byte is stripped, so the top byte is not zero.
        assert_ne!(certificate.serial[0], 0);
    }

    #[test]
    fn the_validity_window_brackets_a_known_moment() {
        let certificate = Certificate::parse(&fixture()).unwrap();
        let inside = Instant {
            year: 2030,
            month: 6,
            day: 15,
            hour: 12,
            minute: 0,
            second: 0,
        };
        assert!(certificate.valid_at(inside));
        let after = Instant {
            year: 2041,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        };
        assert!(!certificate.valid_at(after));
    }

    #[test]
    fn reads_the_ca_flag() {
        let certificate = Certificate::parse(&fixture()).unwrap();
        assert!(certificate.is_ca, "the fixture is self-signed and a CA");
    }

    #[test]
    fn rejects_something_that_is_not_a_certificate() {
        assert!(Certificate::parse(&[0x02, 0x01, 0x05]).is_err());
        assert!(Certificate::parse(b"").is_err());
    }

    #[test]
    fn formats_a_time() {
        let time = Instant {
            year: 2026,
            month: 10,
            day: 22,
            hour: 8,
            minute: 5,
            second: 3,
        };
        assert_eq!(time.to_string(), "2026-10-22T08:05:03Z");
    }
}
