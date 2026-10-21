//! The namespaces WS-Security documents are built from.
//!
//! These are long and easy to mistype, and a wrong one produces a document the
//! server rejects with no explanation, so they are named here once.

/// The `wsse` prefix: the security extension namespace.
pub const WSSE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";
/// The `wsu` prefix: the utility namespace, which carries `Created` and `Expires`.
pub const WSU: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd";
/// The UsernameToken profile, which defines the two password types.
pub const USERNAME_TOKEN_PROFILE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0";
/// The SOAP message security namespace, which defines the base64 encoding type.
pub const SOAP_MESSAGE_SECURITY: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0";
/// The SOAP 1.1 envelope namespace.
pub const SOAP_11_ENVELOPE: &str = "http://schemas.xmlsoap.org/soap/envelope/";
/// The SOAP 1.2 envelope namespace.
pub const SOAP_12_ENVELOPE: &str = "http://www.w3.org/2003/05/soap-envelope";

/// The `Type` of a plaintext password.
pub const PASSWORD_TEXT: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText";
/// The `Type` of a digested password.
pub const PASSWORD_DIGEST: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest";
/// The `EncodingType` of a base64 nonce.
pub const BASE64_BINARY: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary";
