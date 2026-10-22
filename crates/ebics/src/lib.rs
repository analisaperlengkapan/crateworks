#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod aes;
pub mod base64;
pub mod bignum;
pub mod c14n;
pub mod error;
pub mod flate;
pub mod header;
pub mod message;
pub mod response;
pub mod returncodes;
pub mod rng;
pub mod rsa;
pub mod sha256;
pub mod time;

/// The product name this crate sends in every header.
pub const PRODUCT: &str = "crateworks";

/// Escapes text for XML content and for a double-quoted attribute value.
///
/// The two cases differ only for the quote character, and both callers need a
/// value that is safe in either position, so one function escapes both.
#[must_use]
pub fn xml_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}
