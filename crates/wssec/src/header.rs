//! The `Security` header and its `Timestamp` token.
//!
//! A server that accepts a token also wants to know it is fresh, or a captured
//! request can be replayed forever. The `Timestamp` token answers that: it states
//! when the message was created and when it stops being valid, and the server
//! rejects anything outside that window.

use xmlkit::Element;

use crate::namespaces;
use crate::token::UsernameToken;

/// A `wsu:Timestamp` token.
///
/// Both times are strings, not a date type, because the digest of a
/// `UsernameToken` covers the timestamp text exactly as written. A value that
/// round-trips through a date library and comes back with a different offset
/// would change the digest and break the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timestamp {
    /// When the message was created, in UTC.
    pub created: String,
    /// When it stops being valid, in UTC.
    pub expires: Option<String>,
}

impl Timestamp {
    /// A timestamp that was created now and does not expire.
    #[must_use]
    pub fn created(created: impl Into<String>) -> Self {
        Self {
            created: created.into(),
            expires: None,
        }
    }

    /// A timestamp with an expiry.
    #[must_use]
    pub fn new(created: impl Into<String>, expires: impl Into<String>) -> Self {
        Self {
            created: created.into(),
            expires: Some(expires.into()),
        }
    }

    /// Builds the `wsu:Timestamp` element.
    #[must_use]
    pub fn to_element(&self) -> Element {
        let mut element = Element::new("wsu:Timestamp");
        let mut created = Element::new("wsu:Created");
        created.push_text(&self.created);
        element.push(created);
        if let Some(expires) = &self.expires {
            let mut element_expires = Element::new("wsu:Expires");
            element_expires.push_text(expires);
            element.push(element_expires);
        }
        element
    }
}

/// The `wsse:Security` header, with the tokens to place in it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Security {
    /// The tokens, in the order they will be written.
    pub tokens: Vec<Element>,
}

impl Security {
    /// An empty header.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a user name token.
    #[must_use]
    pub fn with_username_token(mut self, token: &UsernameToken) -> Self {
        self.tokens.push(token.to_element());
        self
    }

    /// Adds a timestamp.
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: &Timestamp) -> Self {
        self.tokens.push(timestamp.to_element());
        self
    }

    /// Adds an already-built token.
    #[must_use]
    pub fn with_token(mut self, token: Element) -> Self {
        self.tokens.push(token);
        self
    }

    /// Builds the `wsse:Security` element, declaring the prefixes it uses.
    ///
    /// The `wsu` prefix is declared on the element itself so the header stands
    /// alone as a fragment, which is how a SOAP envelope includes it.
    #[must_use]
    pub fn to_element(&self) -> Element {
        let mut element = Element::new("wsse:Security");
        element.set_attr("xmlns:wsse", namespaces::WSSE);
        element.set_attr("xmlns:wsu", namespaces::WSU);
        for token in &self.tokens {
            element.push(token.clone());
        }
        element
    }

    /// The serialised header, ready for a SOAP `Header` element.
    #[must_use]
    pub fn to_xml(&self) -> String {
        self.to_element().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_timestamp_writes_both_times() {
        let element = Timestamp::new("2026-10-21T12:00:00Z", "2026-10-21T12:05:00Z").to_element();
        assert_eq!(
            element.child("wsu:Created").unwrap().text(),
            "2026-10-21T12:00:00Z"
        );
        assert_eq!(
            element.child("wsu:Expires").unwrap().text(),
            "2026-10-21T12:05:00Z"
        );
    }

    #[test]
    fn a_timestamp_without_an_expiry_omits_the_element() {
        let element = Timestamp::created("2026-10-21T12:00:00Z").to_element();
        assert!(element.child("wsu:Expires").is_none());
    }

    #[test]
    fn the_header_declares_both_namespaces() {
        let xml = Security::new().to_xml();
        assert!(xml.contains(namespaces::WSSE));
        assert!(xml.contains(namespaces::WSU));
        assert!(xml.starts_with("<wsse:Security"));
    }

    #[test]
    fn tokens_keep_the_order_they_were_added() {
        let xml = Security::new()
            .with_timestamp(&Timestamp::created("2026-10-21T12:00:00Z"))
            .with_username_token(&UsernameToken::text("scott", "tiger"))
            .to_xml();
        let timestamp = xml.find("wsu:Timestamp").unwrap();
        let token = xml.find("wsse:UsernameToken").unwrap();
        assert!(timestamp < token);
    }

    #[test]
    fn the_header_round_trips_through_the_parser() {
        let xml = Security::new()
            .with_username_token(&UsernameToken::text("scott", "tiger"))
            .to_xml();
        let parsed = xmlkit::parse(&xml).unwrap();
        assert_eq!(parsed.name, "wsse:Security");
        assert_eq!(
            parsed
                .child("wsse:UsernameToken")
                .and_then(|t| t.child("wsse:Username"))
                .unwrap()
                .text(),
            "scott"
        );
    }
}
