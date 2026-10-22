//! The EBICS responses, and the order data inside them.
//!
//! A response carries two return codes: a technical one in the mutable header,
//! which says whether the message was processed at all, and a business one in
//! the body, which says what the bank did with the order. Both are read here,
//! because a caller that checks only the first will treat a refused payment as a
//! success.
//!
//! # The order data
//!
//! A download's order data is encrypted and then compressed, and the order is
//! the reverse: decompress, then decrypt. The transaction key is in the message,
//! wrapped under the customer's RSA key, so reading the data means:
//!
//! 1. RSA-decrypt the `TransactionKey` with the customer's private key.
//! 2. AES-128-CBC decrypt the `OrderData` with that key and the zero IV.
//! 3. Remove the EBICS padding.
//! 4. Inflate the result.
//!
//! Every step can fail on a message a bank would never send, so every step
//! returns an error rather than panicking.

use xmlkit::Element;

use crate::aes::{Aes128, BLOCK};
use crate::base64;
use crate::error::{Error, Result};
use crate::flate;
use crate::message::{descendant_text, find_descendant};
use crate::returncodes;
use crate::rsa::PrivateKey;

/// A parsed response.
#[derive(Debug, Clone)]
pub struct Response {
    /// The technical return code from the mutable header.
    pub technical_code: String,
    /// The business return code from the body, if the response carries one.
    pub business_code: Option<String>,
    /// The bank's report text, for a human to read.
    pub report_text: Option<String>,
    /// The transaction identifier, for a multi-part transfer.
    pub transaction_id: Option<String>,
    /// The base64 order data, still encrypted and compressed.
    pub order_data: Option<String>,
    /// The base64 transaction key, still wrapped.
    pub transaction_key: Option<String>,
    /// The bank's parameter timestamp, where the response carries one.
    pub timestamp: Option<String>,
}

impl Response {
    /// Parses a response.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Malformed`] if the XML cannot be parsed, and
    /// [`Error::MissingField`] if the technical return code is absent, which
    /// means the document is not an EBICS response.
    pub fn parse(document: &str) -> Result<Self> {
        let root = xmlkit::parse(document)?;
        let technical_code = find_descendant(&root, "ReturnCode")
            .map(Element::text)
            .map(|text| text.trim().to_owned())
            .ok_or(Error::MissingField("a technical ReturnCode"))?;

        // The body's return code is the business one. The header's is the
        // technical one, and it comes first in document order, so the second
        // occurrence is the business code.
        let mut codes = Vec::new();
        collect_texts(&root, "ReturnCode", &mut codes);
        let business_code = codes
            .into_iter()
            .nth(1)
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty());

        Ok(Self {
            technical_code,
            business_code,
            report_text: descendant_text(&root, "ReportText"),
            transaction_id: descendant_text(&root, "TransactionID"),
            order_data: descendant_text(&root, "OrderData"),
            transaction_key: descendant_text(&root, "TransactionKey"),
            timestamp: descendant_text(&root, "TimestampBankParameter"),
        })
    }

    /// Whether the bank accepted the message.
    #[must_use]
    pub fn is_success(&self) -> bool {
        returncodes::is_success(&self.technical_code)
    }

    /// Turns a refusal into an error, so a caller can use `?`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Rejected`] if the technical code is not `000000`.
    pub fn into_result(self) -> Result<Self> {
        if self.is_success() {
            Ok(self)
        } else {
            Err(Error::Rejected {
                technical: self.technical_code,
                business: self.business_code,
            })
        }
    }

    /// The meaning of the technical return code, where the specification gives
    /// one.
    #[must_use]
    pub fn technical_meaning(&self) -> Option<&'static returncodes::ReturnCode> {
        returncodes::technical(&self.technical_code)
    }

    /// Unwraps and decodes the order data.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingField`] if the response carries no order data or
    /// no transaction key, [`Error::BadTransactionKey`] if the key does not
    /// decrypt the data, and [`Error::Malformed`] if the inflated stream is not
    /// valid.
    pub fn order_data(&self, key: &PrivateKey) -> Result<Vec<u8>> {
        let wrapped = self
            .transaction_key
            .as_deref()
            .ok_or(Error::MissingField("a TransactionKey"))?;
        let data = self
            .order_data
            .as_deref()
            .ok_or(Error::MissingField("OrderData"))?;
        decrypt_order_data(wrapped, data, key)
    }
}

/// Decrypts order data given a wrapped transaction key.
///
/// This is the four steps from the module documentation, in order.
///
/// # Errors
///
/// As [`Response::order_data`].
pub fn decrypt_order_data(
    wrapped_transaction_key: &str,
    order_data: &str,
    key: &PrivateKey,
) -> Result<Vec<u8>> {
    let wrapped = base64::decode(wrapped_transaction_key.trim())
        .ok_or(Error::MissingField("a well-formed TransactionKey"))?;
    let ciphertext =
        base64::decode(order_data.trim()).ok_or(Error::MissingField("well-formed OrderData"))?;

    let transaction_key = key
        .decrypt_pkcs1v15(&wrapped)
        .map_err(|_| Error::BadTransactionKey)?;
    if transaction_key.len() < 16 {
        return Err(Error::BadTransactionKey);
    }
    let mut key_bytes = [0u8; 16];
    key_bytes.copy_from_slice(&transaction_key[..16]);

    if !ciphertext.len().is_multiple_of(BLOCK) || ciphertext.is_empty() {
        return Err(Error::BadTransactionKey);
    }
    let cipher = Aes128::new(&key_bytes);
    let decrypted = cipher.decrypt_cbc_zero_iv(&ciphertext);
    let unpadded = crate::aes::unpad(&decrypted).ok_or(Error::BadTransactionKey)?;
    flate::inflate(unpadded).ok_or(Error::Malformed {
        message: "the order data did not inflate".to_owned(),
    })
}

/// Collects the text of every descendant with the given local name, in document
/// order.
fn collect_texts(element: &Element, local: &str, found: &mut Vec<String>) {
    if xmlkit::local_name(&element.name) == local {
        found.push(element.text());
    }
    for node in &element.children {
        if let xmlkit::Node::Element(child) = node {
            collect_texts(child, local, found);
        }
    }
}

/// Encrypts order data the way an upload does: compress, pad, AES, base64.
///
/// This is the other half of [`decrypt_order_data`], and it stops short of
/// wrapping the transaction key: that is [`crate::rsa::encrypt_pkcs1v15`], which
/// the caller does with the bank's encryption key from an `HPB` response. The two
/// are separate because the key is wrapped once per transaction while the data
/// may be split across several segments.
#[must_use]
pub fn encrypt_order_data(order_data: &[u8], transaction_key: &[u8; 16]) -> String {
    let compressed = flate::compress(order_data);
    let padded = crate::aes::pad(&compressed);
    let cipher = Aes128::new(transaction_key);
    base64::encode(&cipher.encrypt_cbc_zero_iv(&padded))
}

#[cfg(test)]
mod tests {
    use super::{Response, decrypt_order_data, encrypt_order_data};
    use crate::rng::Rng;
    use crate::rsa::{PrivateKey, PublicKey};

    const MODULUS: &str = "2aWRwQ9/pII5l29p55PIXEWlJsvSn/d5KOpz/wnUEJ9AZvq2CitcgYhCG0qqn8Eb3G61qqQfNONhHUPdzc8Fndnv0Uqzg+Xtz4kSjHWZQZb069Po6jgLwUbmuuyfdlQpU4GdyrPHtFTd18tP20/5pmic1ui+9GQnyRBhoh55CTU=";
    const EXPONENT: &str = "AQAB";
    const PRIVATE: &str = "M1NfRvb/s+zd5f9pKypnopLt+nMwjXbPxL1dc3R+5RiZGG/hRA6rtwRKCqOWDyx8BIS7JK2lilRgz4T0ppc+jGbF3FIaqLzrr7b5hQJN97vR1f5PYZxGXQppx2qto2tQbm9Ct6K8UmRQTAI33ndB0yjhTUzXJnxl/lKnuBX0O/U=";

    fn key() -> PrivateKey {
        PrivateKey::from_base64(MODULUS, EXPONENT, PRIVATE).expect("usable")
    }

    /// A response with a technical code, a business code and report text.
    const RESPONSE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ebicsKeyManagementResponse xmlns="urn:org:ebics:H004" Version="H004" Revision="1">
  <header authenticate="true">
    <static/>
    <mutable>
      <ReturnCode>000000</ReturnCode>
      <ReportText>[EBICS_OK] OK</ReportText>
    </mutable>
  </header>
  <body>
    <ReturnCode authenticate="true">000000</ReturnCode>
    <TimestampBankParameter authenticate="true">2016-08-10T16:16:17.512Z</TimestampBankParameter>
  </body>
</ebicsKeyManagementResponse>"#;

    #[test]
    fn a_successful_response_parses() {
        let response = Response::parse(RESPONSE).expect("parses");
        assert!(response.is_success());
        assert_eq!(response.technical_code, "000000");
        assert_eq!(response.business_code.as_deref(), Some("000000"));
        assert_eq!(response.report_text.as_deref(), Some("[EBICS_OK] OK"));
        assert_eq!(
            response.timestamp.as_deref(),
            Some("2016-08-10T16:16:17.512Z")
        );
        assert!(response.into_result().is_ok());
    }

    /// The technical code comes from the header and the business code from the
    /// body, so a response that refuses the order must not look successful.
    #[test]
    fn a_refusal_becomes_an_error() {
        let refused = RESPONSE
            .replace(
                "<ReturnCode>000000</ReturnCode>",
                "<ReturnCode>011000</ReturnCode>",
            )
            .replace(
                "<ReturnCode authenticate=\"true\">000000</ReturnCode>",
                "<ReturnCode authenticate=\"true\">090005</ReturnCode>",
            );
        let response = Response::parse(&refused).expect("parses");
        assert!(!response.is_success());
        let error = response.into_result().expect_err("refused");
        let text = error.to_string();
        assert!(text.contains("011000"), "got {text}");
        assert!(text.contains("090005"), "got {text}");
    }

    #[test]
    fn a_response_without_a_return_code_is_not_a_response() {
        assert!(Response::parse("<a><b/></a>").is_err());
    }

    #[test]
    fn a_technical_code_has_a_meaning() {
        let response = Response::parse(RESPONSE).expect("parses");
        let entry = response.technical_meaning().expect("the code is known");
        assert_eq!(entry.symbol, "EBICS_OK");
    }

    /// The whole order-data path: encrypt here, decrypt here, and get the
    /// original bytes back.
    #[test]
    fn order_data_round_trips_through_encryption() {
        let rng = &mut Rng::from_seed([7; 32]);
        let transaction_key = [0x2bu8; 16];
        let plaintext = b"<order>a small document</order>".repeat(20);
        let public = PublicKey::from_base64(MODULUS, EXPONENT).expect("usable");

        let encrypted = encrypt_order_data(&plaintext, &transaction_key);

        // Wrap the transaction key under the public key, as a bank would.
        let wrapped =
            crate::rsa::encrypt_pkcs1v15(&public, &transaction_key, rng).expect("the key wraps");
        let wrapped_base64 = crate::base64::encode(&wrapped);

        let recovered = decrypt_order_data(&wrapped_base64, &encrypted, &key()).expect("decrypts");
        assert_eq!(recovered, plaintext);
    }

    /// A ciphertext that is not a whole number of blocks must be refused rather
    /// than silently truncated.
    #[test]
    fn a_ragged_ciphertext_is_refused() {
        let wrapped = crate::base64::encode(&[0u8; 128]);
        let ragged = crate::base64::encode(&[0u8; 17]);
        assert!(decrypt_order_data(&wrapped, &ragged, &key()).is_err());
    }

    #[test]
    fn a_wrong_transaction_key_is_refused() {
        let rng = &mut Rng::from_seed([8; 32]);
        let public = PublicKey::from_base64(MODULUS, EXPONENT).expect("usable");
        let right = [0x11u8; 16];
        let wrong = [0x22u8; 16];
        let plaintext = b"payload".repeat(30);

        let encrypted = encrypt_order_data(&plaintext, &right);
        let wrapped = crate::base64::encode(
            &crate::rsa::encrypt_pkcs1v15(&public, &wrong, rng).expect("wraps"),
        );
        assert!(decrypt_order_data(&wrapped, &encrypted, &key()).is_err());
    }
}
