//! WS-Security headers for SOAP services.
//!
//! `soap-client` in this workspace builds and reads SOAP envelopes, and its
//! documentation says plainly that it does not implement WS-Security. This crate
//! is that missing half. Almost every bank, government and ERP endpoint requires
//! a `Security` header, and without one the request is rejected before the
//! service ever looks at it.
//!
//! # Example
//!
//! ```
//! use wssec::{Security, Timestamp, UsernameToken};
//!
//! // The nonce and the timestamp are inputs, not something generated here, so
//! // the caller can record them and a test can reproduce the digest.
//! let nonce = b"0123456789abcdef".to_vec();
//! let created = "2026-10-21T12:00:00Z";
//!
//! let token = UsernameToken::digest("NNK", "secret", nonce, created);
//! let header = Security::new()
//!     .with_timestamp(&Timestamp::new(created, "2026-10-21T12:05:00Z"))
//!     .with_username_token(&token)
//!     .to_xml();
//!
//! assert!(header.contains("wsse:UsernameToken"));
//! assert!(header.contains("PasswordDigest"));
//! assert!(header.contains("wsu:Timestamp"));
//! // The password itself is nowhere in the document.
//! assert!(!header.contains("secret"));
//! ```
//!
//! The result is a header fragment. Pass it to `soap_client::Envelope` with
//! `with_header_xml` and the two crates compose into a request that an
//! enterprise service will accept.
//!
//! # What this crate does not do
//!
//! It does not sign messages, does not carry X.509 certificates, and does not
//! talk to a hardware token. `Signature` and `EncryptedData` are the other half
//! of WS-Security, and they need a certificate story that this crate
//! deliberately leaves alone. Nor does it generate a nonce or read a clock: a
//! library that did would be untestable and would hide the timestamp the caller
//! has to record anyway.

#![forbid(unsafe_code)]

/// The README, compiled as a doctest so its example cannot drift from the API.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

pub mod base64;
pub mod digest;
pub mod header;
pub mod namespaces;
pub mod token;

pub use header::{Security, Timestamp};
pub use token::{Password, UsernameToken, digest_with_text_nonce};

/// A failure while reading or building a security header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The document is not well-formed XML.
    Xml(String),
    /// A field a digest needs was not supplied.
    MissingField(&'static str),
    /// A nonce was not valid base64.
    Nonce(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xml(message) => write!(f, "invalid XML: {message}"),
            Self::MissingField(name) => write!(f, "{name} is required"),
            Self::Nonce(message) => write!(f, "invalid nonce: {message}"),
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

/// Reads a `wsse:Security` header out of a SOAP envelope or a fragment.
///
/// This is what a server, or a test, needs: given a request, what was the user
/// name, and was the password a digest or plain text.
///
/// # Errors
///
/// Returns an error if the document is not well-formed XML.
pub fn parse_security(xml: &str) -> Result<Vec<ParsedToken>> {
    let root = xmlkit::parse(xml)?;
    let security = find_security(&root).ok_or(Error::MissingField("Security"))?;
    let mut tokens = Vec::new();
    for node in &security.children {
        let xmlkit::Node::Element(element) = node else {
            continue;
        };
        match xmlkit::local_name(&element.name) {
            "UsernameToken" => tokens.push(ParsedToken::Username(read_username_token(element)?)),
            "Timestamp" => tokens.push(ParsedToken::Timestamp(Timestamp {
                created: child_text(element, "Created").unwrap_or_default(),
                expires: child_text(element, "Expires"),
            })),
            _ => {}
        }
    }
    Ok(tokens)
}

/// A token read back from a header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedToken {
    /// A user name token.
    Username(ParsedUsernameToken),
    /// A timestamp.
    Timestamp(Timestamp),
}

/// A user name token as read from a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUsernameToken {
    /// The user name.
    pub username: String,
    /// The password value exactly as it appeared, still base64 if it was text.
    pub password: Option<String>,
    /// Whether the password carried the digest type.
    pub is_digest: bool,
    /// The nonce, decoded from base64, when present.
    pub nonce: Option<Vec<u8>>,
    /// The creation time, when present.
    pub created: Option<String>,
}

impl ParsedUsernameToken {
    /// The `PasswordText` password as it travelled.
    ///
    /// The profile sends a text password in clear text, so no decoding is
    /// needed; only a digest cannot be reversed.
    ///
    /// # Errors
    ///
    /// Returns an error if the token carried no password, or carried a digest.
    pub fn plaintext_password(&self) -> Result<String> {
        let Some(password) = &self.password else {
            return Err(Error::MissingField("Password"));
        };
        if self.is_digest {
            return Err(Error::MissingField("plaintext password (this is a digest)"));
        }
        Ok(password.clone())
    }

    /// Checks a candidate password against a digest token.
    ///
    /// Returns `false` for a text token, where there is no digest to check.
    #[must_use]
    pub fn matches_digest(&self, candidate: &str) -> bool {
        if !self.is_digest {
            return false;
        }
        let (Some(nonce), Some(created), Some(digest)) =
            (&self.nonce, &self.created, &self.password)
        else {
            return false;
        };
        let expected = token::UsernameToken::digest("", candidate, nonce.clone(), created.clone());
        match expected.password {
            Some(Password::Digest(expected)) => constant_time_eq(&expected, digest),
            _ => false,
        }
    }
}

/// Compares two strings without leaking where they differ through timing.
///
/// The comparison is over the digest, which is not itself secret, but a server
/// comparing digests is a comparison against a value derived from a password, so
/// the habit is worth keeping.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut difference = 0u8;
    for (x, y) in a.iter().zip(b) {
        difference |= x ^ y;
    }
    difference == 0
}

/// Finds the `Security` element, however deeply it is nested.
fn find_security(element: &xmlkit::Element) -> Option<&xmlkit::Element> {
    if xmlkit::local_name(&element.name) == "Security" {
        return Some(element);
    }
    for node in &element.children {
        if let xmlkit::Node::Element(child) = node {
            if let Some(found) = find_security(child) {
                return Some(found);
            }
        }
    }
    None
}

/// Reads a `UsernameToken` element.
fn read_username_token(element: &xmlkit::Element) -> Result<ParsedUsernameToken> {
    let username = child_text(element, "Username").unwrap_or_default();
    let password_element = find_child(element, "Password");
    let password = password_element.map(xmlkit::Element::text);
    let is_digest = password_element
        .and_then(|p| p.attr("Type"))
        .is_some_and(|t| t.ends_with("#PasswordDigest"));
    let nonce = match child_text(element, "Nonce") {
        Some(text) => Some(base64::decode(&text).map_err(|e| Error::Nonce(e.to_string()))?),
        None => None,
    };
    Ok(ParsedUsernameToken {
        username,
        password,
        is_digest,
        nonce,
        created: child_text(element, "Created"),
    })
}

/// The text of a direct child, by local name.
fn child_text(element: &xmlkit::Element, name: &str) -> Option<String> {
    find_child(element, name).map(xmlkit::Element::text)
}

/// Finds a direct child by local name, ignoring the namespace prefix.
fn find_child<'a>(element: &'a xmlkit::Element, name: &str) -> Option<&'a xmlkit::Element> {
    element.children.iter().find_map(|node| match node {
        xmlkit::Node::Element(child) if xmlkit::local_name(&child.name) == name => Some(child),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_written_here_reads_back_here() {
        let nonce: Vec<u8> = (0..16u8).collect();
        let xml = Security::new()
            .with_username_token(&UsernameToken::digest(
                "NNK",
                "secret",
                nonce.clone(),
                "2026-10-21T12:00:00+00:00",
            ))
            .to_xml();

        let tokens = parse_security(&xml).unwrap();
        let ParsedToken::Username(token) = &tokens[0] else {
            panic!("expected a user name token");
        };
        assert_eq!(token.username, "NNK");
        assert!(token.is_digest);
        assert_eq!(token.nonce.as_deref(), Some(nonce.as_slice()));
        assert!(token.matches_digest("secret"));
        assert!(!token.matches_digest("wrong"));
    }

    #[test]
    fn a_text_password_reads_back_as_plaintext() {
        let xml = Security::new()
            .with_username_token(&UsernameToken::text("scott", "tiger"))
            .to_xml();
        let tokens = parse_security(&xml).unwrap();
        let ParsedToken::Username(token) = &tokens[0] else {
            panic!("expected a user name token");
        };
        assert!(!token.is_digest);
        assert_eq!(token.plaintext_password().unwrap(), "tiger");
    }

    #[test]
    fn a_digest_token_has_no_plaintext() {
        let xml = Security::new()
            .with_username_token(&UsernameToken::digest(
                "u",
                "p",
                vec![1, 2, 3],
                "2026-01-01T00:00:00Z",
            ))
            .to_xml();
        let tokens = parse_security(&xml).unwrap();
        let ParsedToken::Username(token) = &tokens[0] else {
            panic!("expected a user name token");
        };
        assert!(token.plaintext_password().is_err());
    }

    #[test]
    fn a_header_nested_in_a_soap_envelope_is_found() {
        let envelope = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Header><wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
    <wsse:UsernameToken><wsse:Username>scott</wsse:Username></wsse:UsernameToken>
  </wsse:Security></soap:Header>
  <soap:Body/>
</soap:Envelope>"#;
        let tokens = parse_security(envelope).unwrap();
        let ParsedToken::Username(token) = &tokens[0] else {
            panic!("expected a user name token");
        };
        assert_eq!(token.username, "scott");
    }

    #[test]
    fn a_document_without_a_security_element_is_reported() {
        assert!(matches!(
            parse_security("<a><b/></a>"),
            Err(Error::MissingField("Security"))
        ));
    }

    #[test]
    fn the_readme_example_composes_with_a_soap_envelope() {
        // The point of the crate: the fragment fits a SOAP Header.
        let header = Security::new()
            .with_username_token(&UsernameToken::text("scott", "tiger"))
            .to_xml();
        let envelope = format!(
            r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
  <soap:Header>{header}</soap:Header>
  <soap:Body><Ping/></soap:Body>
</soap:Envelope>"#
        );
        let parsed = xmlkit::parse(&envelope).unwrap();
        assert!(parsed.child("soap:Header").is_some());
        assert_eq!(parse_security(&envelope).unwrap().len(), 1);
    }
}
