//! The EBICS message header.
//!
//! Every EBICS request carries a `header` with a `static` part and a `mutable`
//! part. The static part names the host, the partner and the user, gives the
//! order and its attribute, and — for a signed request — a nonce and a
//! timestamp. The mutable part carries the transaction phase during a multi-part
//! transfer.
//!
//! The element order in `static` is fixed by the schema and is not the order in
//! which a caller happens to have the values, so the header is built here rather
//! than assembled by the caller.
//!
//! # Security medium
//!
//! `SecurityMedium` is `0000` for a message that travels over the internet
//! without a customer-supplied transport medium. The other values select a
//! medium the bank and the customer agreed on, and a client that sends the wrong
//! one is rejected.

use std::fmt::Write as _;

use crate::error::{Error, Result};
use crate::time;

/// The security medium that means "ordinary internet transport".
pub const SECURITY_MEDIUM_INTERNET: &str = "0000";

/// Whether an order's data carries a customer signature and how it is packed.
///
/// The value is five characters. The schema fixes the value per order type
/// rather than leaving it to the client, and only three values are defined for
/// the order types that use order data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderAttribute {
    /// Order data with a signature, ZIP-compressed, hybrid-encrypted. `OZHNN`.
    WithSignatureCompressedEncrypted,
    /// A signature only, ZIP-compressed, hybrid-encrypted. `UZHNN`.
    SignatureCompressedEncrypted,
    /// Order data without a signature, ZIP-compressed, hybrid-encrypted. `DZHNN`.
    WithoutSignatureCompressedEncrypted,
    /// No order data: the key-management and key-exchange orders. `DZNNN`.
    NoOrderData,
    /// A value the schema allows for a specific order but does not name.
    ///
    /// The schema fixes the attribute per order type, and the three named values
    /// above do not cover every order, so an uncommon order keeps its literal
    /// value here rather than being refused.
    Other,
}

impl OrderAttribute {
    /// The five-character code that goes on the wire.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::WithSignatureCompressedEncrypted => "OZHNN",
            Self::SignatureCompressedEncrypted => "UZHNN",
            Self::WithoutSignatureCompressedEncrypted => "DZHNN",
            Self::NoOrderData => "DZNNN",
            Self::Other => "",
        }
    }

    /// Reads an attribute code.
    ///
    /// Returns [`OrderAttribute::Other`] for a code the schema permits for a
    /// specific order but does not name.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "OZHNN" => Some(Self::WithSignatureCompressedEncrypted),
            "UZHNN" => Some(Self::SignatureCompressedEncrypted),
            "DZHNN" => Some(Self::WithoutSignatureCompressedEncrypted),
            "DZNNN" => Some(Self::NoOrderData),
            _ => None,
        }
    }
}

/// The mutable header, which carries the transaction phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionPhase {
    /// The first message of a transfer, which carries the order data.
    Initialisation,
    /// The message that acknowledges receipt and asks for the next part.
    Transfer,
    /// The message that closes a download after the last part was received.
    Receipt,
}

impl TransactionPhase {
    /// The element name for the phase.
    #[must_use]
    pub fn element(self) -> &'static str {
        match self {
            Self::Initialisation => "TransactionPhase",
            Self::Transfer => "TransactionPhase",
            Self::Receipt => "TransactionPhase",
        }
    }

    /// The text of the phase element.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Self::Initialisation => "Initialisation",
            Self::Transfer => "Transfer",
            Self::Receipt => "Receipt",
        }
    }

    /// Reads a phase from its text.
    #[must_use]
    pub fn from_text(text: &str) -> Option<Self> {
        match text {
            "Initialisation" => Some(Self::Initialisation),
            "Transfer" => Some(Self::Transfer),
            "Receipt" => Some(Self::Receipt),
            _ => None,
        }
    }
}

/// The values that go into the static part of a header.
#[derive(Debug, Clone)]
pub struct StaticHeader {
    /// The host identifier the bank issued.
    pub host_id: String,
    /// The partner identifier the bank issued.
    pub partner_id: String,
    /// The user identifier the bank issued.
    pub user_id: String,
    /// The order type, for example `HPB`, `HKD` or `CCT`.
    pub order_type: String,
    /// The order attribute the schema fixes for that order type.
    pub order_attribute: OrderAttribute,
    /// The security medium, normally [`SECURITY_MEDIUM_INTERNET`].
    pub security_medium: String,
    /// A random value that makes the message unique. Absent for an unsecured
    /// request, which carries neither nonce nor timestamp.
    pub nonce: Option<String>,
    /// The creation time. Absent for an unsecured request.
    pub timestamp: Option<String>,
    /// The product name sent to the bank, which appears in the bank's logs.
    pub product: String,
    /// The product's language, a two-letter code.
    pub product_language: String,
    /// The transaction identifier, present once a transaction exists.
    pub transaction_id: Option<String>,
}

impl StaticHeader {
    /// A header for a signed request, with a fresh nonce and the current time.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Clock`] if the system clock is before the epoch, which
    /// makes a timestamp impossible.
    pub fn signed(
        host_id: impl Into<String>,
        partner_id: impl Into<String>,
        user_id: impl Into<String>,
        order_type: impl Into<String>,
        order_attribute: OrderAttribute,
        nonce: impl Into<String>,
    ) -> Result<Self> {
        let timestamp = time::now().ok_or(Error::Clock)?;
        Ok(Self {
            host_id: host_id.into(),
            partner_id: partner_id.into(),
            user_id: user_id.into(),
            order_type: order_type.into(),
            order_attribute,
            security_medium: SECURITY_MEDIUM_INTERNET.to_owned(),
            nonce: Some(nonce.into()),
            timestamp: Some(timestamp),
            product: crate::PRODUCT.to_owned(),
            product_language: "en".to_owned(),
            transaction_id: None,
        })
    }

    /// A header for an unsecured request, which carries no nonce and no
    /// timestamp. Only `INI` and `HIA` are sent this way.
    #[must_use]
    pub fn unsecured(
        host_id: impl Into<String>,
        partner_id: impl Into<String>,
        user_id: impl Into<String>,
        order_type: impl Into<String>,
    ) -> Self {
        Self {
            host_id: host_id.into(),
            partner_id: partner_id.into(),
            user_id: user_id.into(),
            order_type: order_type.into(),
            order_attribute: OrderAttribute::NoOrderData,
            security_medium: SECURITY_MEDIUM_INTERNET.to_owned(),
            nonce: None,
            timestamp: None,
            product: crate::PRODUCT.to_owned(),
            product_language: "en".to_owned(),
            transaction_id: None,
        }
    }

    /// Renders the `static` element.
    #[must_use]
    pub fn to_static_xml(&self) -> String {
        let mut out = String::from("<static>");
        push_text(&mut out, "HostID", &self.host_id);
        if let Some(nonce) = &self.nonce {
            push_text(&mut out, "Nonce", nonce);
        }
        if let Some(timestamp) = &self.timestamp {
            push_text(&mut out, "Timestamp", timestamp);
        }
        push_text(&mut out, "PartnerID", &self.partner_id);
        push_text(&mut out, "UserID", &self.user_id);
        let _ = write!(
            out,
            "<Product Language=\"{}\">{}</Product>",
            crate::xml_escape(&self.product_language),
            crate::xml_escape(&self.product)
        );
        let _ = write!(
            out,
            "<OrderDetails><OrderType>{}</OrderType><OrderAttribute>{}</OrderAttribute></OrderDetails>",
            crate::xml_escape(&self.order_type),
            self.order_attribute.code()
        );
        if let Some(transaction_id) = &self.transaction_id {
            push_text(&mut out, "TransactionID", transaction_id);
        }
        push_text(&mut out, "SecurityMedium", &self.security_medium);
        out.push_str("</static>");
        out
    }

    /// Renders the `mutable` element for a phase, or an empty one for the first
    /// message of a transfer.
    #[must_use]
    pub fn to_mutable_xml(phase: Option<TransactionPhase>) -> String {
        match phase {
            Some(phase) => format!(
                "<mutable><{tag}>{text}</{tag}></mutable>",
                tag = phase.element(),
                text = phase.text()
            ),
            None => "<mutable></mutable>".to_owned(),
        }
    }
}

fn push_text(out: &mut String, tag: &str, value: &str) {
    let _ = write!(out, "<{tag}>{}</{tag}>", crate::xml_escape(value));
}

#[cfg(test)]
mod tests {
    use super::{OrderAttribute, StaticHeader, TransactionPhase};

    #[test]
    fn order_attribute_codes_round_trip() {
        for attribute in [
            OrderAttribute::WithSignatureCompressedEncrypted,
            OrderAttribute::SignatureCompressedEncrypted,
            OrderAttribute::WithoutSignatureCompressedEncrypted,
            OrderAttribute::NoOrderData,
        ] {
            assert_eq!(OrderAttribute::from_code(attribute.code()), Some(attribute));
        }
    }

    #[test]
    fn an_unknown_attribute_is_not_named() {
        assert_eq!(OrderAttribute::from_code("DZZZZ"), None);
        assert_eq!(OrderAttribute::Other.code(), "");
    }

    #[test]
    fn transaction_phases_round_trip() {
        for phase in [
            TransactionPhase::Initialisation,
            TransactionPhase::Transfer,
            TransactionPhase::Receipt,
        ] {
            assert_eq!(TransactionPhase::from_text(phase.text()), Some(phase));
        }
    }

    /// The element order is fixed by the schema, so it is asserted rather than
    /// left to a caller's insertion order.
    #[test]
    fn the_static_header_follows_the_schema_order() {
        let header = StaticHeader {
            host_id: "HOST".to_owned(),
            partner_id: "PARTNER".to_owned(),
            user_id: "USER".to_owned(),
            order_type: "HPB".to_owned(),
            order_attribute: OrderAttribute::WithoutSignatureCompressedEncrypted,
            security_medium: "0000".to_owned(),
            nonce: Some("abc".to_owned()),
            timestamp: Some("2026-10-22T11:44:26.627Z".to_owned()),
            product: "crateworks".to_owned(),
            product_language: "en".to_owned(),
            transaction_id: None,
        };
        let xml = header.to_static_xml();
        let order = [
            "<HostID>",
            "<Nonce>",
            "<Timestamp>",
            "<PartnerID>",
            "<UserID>",
            "<Product",
            "<OrderDetails>",
            "<SecurityMedium>",
        ];
        let mut cursor = 0;
        for tag in order {
            let found = xml[cursor..]
                .find(tag)
                .unwrap_or_else(|| panic!("{tag} missing after position {cursor} in {xml}"));
            cursor += found + tag.len();
        }
    }

    /// An unsecured header must not carry a nonce or a timestamp, because the
    /// schema forbids them and a bank rejects the message.
    #[test]
    fn an_unsecured_header_has_neither_nonce_nor_timestamp() {
        let header = StaticHeader::unsecured("HOST", "PARTNER", "USER", "INI");
        let xml = header.to_static_xml();
        assert!(!xml.contains("<Nonce>"), "got {xml}");
        assert!(!xml.contains("<Timestamp>"), "got {xml}");
        assert!(
            xml.contains("<OrderAttribute>DZNNN</OrderAttribute>"),
            "got {xml}"
        );
    }

    #[test]
    fn values_are_escaped() {
        let header = StaticHeader::unsecured("H&ST", "PARTNER", "USER", "INI");
        assert!(header.to_static_xml().contains("<HostID>H&amp;ST</HostID>"));
    }

    #[test]
    fn the_mutable_header_is_never_self_closing() {
        assert_eq!(
            StaticHeader::to_mutable_xml(None),
            "<mutable></mutable>",
            "a short tag would break the canonical form"
        );
        assert_eq!(
            StaticHeader::to_mutable_xml(Some(TransactionPhase::Transfer)),
            "<mutable><TransactionPhase>Transfer</TransactionPhase></mutable>"
        );
    }
}
