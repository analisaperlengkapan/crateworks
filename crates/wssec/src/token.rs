//! The `UsernameToken` security token.
//!
//! This is the part of WS-Security that almost every legacy service actually
//! uses: a username, a password, and sometimes a nonce and a timestamp. The
//! OASIS UsernameToken Profile 1.1 defines it, and it comes in two flavours.
//!
//! With `PasswordText` the password goes on the wire as it is, which is only
//! acceptable over TLS. With `PasswordDigest` the password never travels:
//!
//! ```text
//! digest = Base64( SHA1( nonce + created + password ) )
//! ```
//!
//! # The nonce is bytes, not the base64 text
//!
//! The specification defines the nonce as a random byte sequence, base64 encoded
//! *for transport only*. The digest is taken over the raw bytes, and the `Nonce`
//! element carries the base64 form. This crate follows the specification.
//!
//! It is worth saying plainly because the most widely used Python client, `zeep`,
//! does not: it hands the base64 *text* to the digest, so its digest is taken
//! over 24 bytes of ASCII rather than the 16 random bytes, and it base64 encodes
//! the result a second time in the element. A server written against the
//! specification rejects that. [`digest_with_text_nonce`] reproduces
//! the other behaviour for the rare server that was written to match it.

use xmlkit::Element;

use crate::base64;
use crate::digest::sha1;
use crate::namespaces;

/// How the password is carried in the token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Password {
    /// The password itself, in clear text.
    ///
    /// The profile sends this value as it is; only the digest and the nonce are
    /// base64. The reference client `zeep` writes it plainly, and a server that
    /// compared it against a stored password would reject a base64 form.
    Text(String),
    /// `Base64(SHA1(nonce + created + password))`.
    ///
    /// The digest is computed from the fields the caller supplies, so the caller
    /// can reproduce it exactly in a test.
    Digest(String),
}

impl Password {
    /// The `Type` attribute this password carries.
    #[must_use]
    pub fn type_uri(&self) -> &'static str {
        match self {
            Self::Text(_) => namespaces::PASSWORD_TEXT,
            Self::Digest(_) => namespaces::PASSWORD_DIGEST,
        }
    }

    /// The value to place inside the `Password` element.
    #[must_use]
    pub fn wire_value(&self) -> String {
        match self {
            Self::Text(password) => password.clone(),
            Self::Digest(digest) => digest.clone(),
        }
    }
}

/// A `UsernameToken`, ready to place inside a `Security` header.
///
/// Build one with [`UsernameToken::text`] or [`UsernameToken::digest`]. A digest
/// token needs a nonce and a creation time, and those are inputs rather than
/// something generated here: a library that silently calls a clock and a random
/// number generator cannot be tested, and the caller usually has to record the
/// timestamp anyway to know when the token expires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsernameToken {
    /// The user name.
    pub username: String,
    /// The password, in whichever form was chosen.
    pub password: Option<Password>,
    /// The raw nonce bytes, when a digest is used.
    pub nonce: Option<Vec<u8>>,
    /// The creation time, as it will appear in the `wsu:Created` element.
    pub created: Option<String>,
}

impl UsernameToken {
    /// A token carrying the password as text.
    #[must_use]
    pub fn text(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: Some(Password::Text(password.into())),
            nonce: None,
            created: None,
        }
    }

    /// A token carrying only a user name.
    #[must_use]
    pub fn username_only(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: None,
            nonce: None,
            created: None,
        }
    }

    /// A token whose password is the standard digest.
    ///
    /// `nonce` is the raw byte sequence; `created` is the timestamp exactly as
    /// it will be written, because the digest covers that text.
    ///
    /// # Example
    ///
    /// ```
    /// use wssec::UsernameToken;
    ///
    /// let token = UsernameToken::digest(
    ///     "user",
    ///     "secret",
    ///     b"nonce bytes".to_vec(),
    ///     "2026-01-01T00:00:00Z",
    /// );
    ///
    /// let element = token.to_element();
    /// assert_eq!(element.name, "wsse:UsernameToken");
    /// // The password never appears in the document in clear text.
    /// assert!(!element.deep_text().contains("secret"));
    /// ```
    #[must_use]
    pub fn digest(
        username: impl Into<String>,
        password: impl Into<String>,
        nonce: impl Into<Vec<u8>>,
        created: impl Into<String>,
    ) -> Self {
        let nonce = nonce.into();
        let created = created.into();
        let digest = digest_of(&nonce, &created, password.into().as_bytes());
        Self {
            username: username.into(),
            password: Some(Password::Digest(base64::encode(&digest))),
            nonce: Some(nonce),
            created: Some(created),
        }
    }

    /// A token whose password is digested from an already-hashed password.
    ///
    /// Some servers store `SHA1(password)` and expect the digest to be taken
    /// over that, which the profile allows.
    #[must_use]
    pub fn digest_of_hashed(
        username: impl Into<String>,
        password: impl Into<String>,
        nonce: impl Into<Vec<u8>>,
        created: impl Into<String>,
    ) -> Self {
        let nonce = nonce.into();
        let created = created.into();
        let hashed = sha1(password.into().as_bytes());
        let digest = digest_of(&nonce, &created, &hashed);
        Self {
            username: username.into(),
            password: Some(Password::Digest(base64::encode(&digest))),
            nonce: Some(nonce),
            created: Some(created),
        }
    }

    /// Builds the `wsse:UsernameToken` element.
    #[must_use]
    pub fn to_element(&self) -> Element {
        let mut token = Element::new("wsse:UsernameToken");
        let mut username = Element::new("wsse:Username");
        username.push_text(&self.username);
        token.push(username);

        if let Some(password) = &self.password {
            let mut element = Element::new("wsse:Password");
            element.set_attr("Type", password.type_uri());
            element.push_text(password.wire_value());
            token.push(element);
        }

        if let Some(nonce) = &self.nonce {
            let mut element = Element::new("wsse:Nonce");
            element.set_attr("EncodingType", namespaces::BASE64_BINARY);
            element.push_text(base64::encode(nonce));
            token.push(element);
        }

        if let Some(created) = &self.created {
            let mut element = Element::new("wsu:Created");
            element.push_text(created);
            token.push(element);
        }

        token
    }
}

/// The raw `SHA1(nonce + created + password)` digest.
fn digest_of(nonce: &[u8], created: &str, password: &[u8]) -> [u8; 20] {
    let mut input = Vec::with_capacity(nonce.len() + created.len() + password.len());
    input.extend_from_slice(nonce);
    input.extend_from_slice(created.as_bytes());
    input.extend_from_slice(password);
    sha1(&input)
}

/// Reproduces the digest `zeep` computes, for servers written to match it.
///
/// The nonce argument is the base64 *text* that will appear in the document;
/// its ASCII bytes are hashed as if they were the nonce, and the resulting digest
/// is base64 encoded. See the module documentation for why this differs from the
/// specification.
#[must_use]
pub fn digest_with_text_nonce(nonce_text: &str, created: &str, password: &[u8]) -> String {
    base64::encode(&digest_of(nonce_text.as_bytes(), created, password))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The nonce used throughout the reference fixtures: bytes 0x00 to 0x0f.
    fn nonce() -> Vec<u8> {
        (0..16u8).collect()
    }

    const CREATED: &str = "2026-10-21T12:00:00+00:00";

    #[test]
    fn a_text_token_carries_the_password_as_it_is() {
        // The profile sends a text password in clear text. Base64 here would
        // look plausible and fail against every real server.
        let token = UsernameToken::text("scott", "tiger");
        let element = token.to_element();
        let password = element.child("wsse:Password").unwrap();
        assert_eq!(password.attr("Type"), Some(namespaces::PASSWORD_TEXT));
        assert_eq!(password.text(), "tiger");
    }

    #[test]
    fn a_username_only_token_has_no_password_element() {
        let element = UsernameToken::username_only("scott").to_element();
        assert!(element.child("wsse:Password").is_none());
        assert_eq!(element.child("wsse:Username").unwrap().text(), "scott");
    }

    #[test]
    fn the_digest_matches_the_reference_value() {
        // Verified independently against Python's hashlib and against zeep.
        let token = UsernameToken::digest("NNK", "secret", nonce(), CREATED);
        let Some(Password::Digest(digest)) = &token.password else {
            panic!("expected a digest");
        };
        assert_eq!(digest, "g01uA646Xg1x0SE/Xfa3CM/TLMI=");
    }

    #[test]
    fn the_hashed_password_variant_matches_its_reference_value() {
        let token = UsernameToken::digest_of_hashed("NNK", "secret", nonce(), CREATED);
        let Some(Password::Digest(digest)) = &token.password else {
            panic!("expected a digest");
        };
        assert_eq!(digest, "do+w3jR3fUf6DcOSPuf0u2tJZoc=");
    }

    #[test]
    fn the_nonce_element_carries_the_base64_of_the_raw_bytes() {
        let element = UsernameToken::digest("NNK", "secret", nonce(), CREATED).to_element();
        let nonce_element = element.child("wsse:Nonce").unwrap();
        assert_eq!(nonce_element.text(), "AAECAwQFBgcICQoLDA0ODw==");
        assert_eq!(
            nonce_element.attr("EncodingType"),
            Some(namespaces::BASE64_BINARY)
        );
        assert_eq!(element.child("wsu:Created").unwrap().text(), CREATED);
    }

    #[test]
    fn the_specification_and_zeep_digests_are_different() {
        // The two conventions must not be confused: they hash different bytes.
        let specification = digest_of(&nonce(), CREATED, b"secret");
        let zeep = digest_with_text_nonce("AAECAwQFBgcICQoLDA0ODw==", CREATED, b"secret");
        assert_eq!(
            base64::encode(&specification),
            "g01uA646Xg1x0SE/Xfa3CM/TLMI="
        );
        assert_eq!(zeep, "HGLe52r9t2cKn9XnWrjJuN9jGKw=");
        assert_ne!(base64::encode(&specification), zeep);
    }

    #[test]
    fn the_created_time_feeds_the_digest_verbatim() {
        // A different spelling of the same instant gives a different digest, so
        // the caller must write the timestamp it digested.
        let a = UsernameToken::digest("u", "p", nonce(), "2026-10-21T12:00:00Z");
        let b = UsernameToken::digest("u", "p", nonce(), "2026-10-21T12:00:00+00:00");
        assert_ne!(a.password, b.password);
    }
}
