//! Base64, as WS-Security needs it.
//!
//! WS-Security carries the nonce and the password digest as base64, and the
//! workspace has no base64 crate, so this module supplies the little that is
//! needed. Both the standard alphabet and the URL-safe one are here because a
//! caller may hand back a nonce it received over a channel that used either.

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// The alphabet to encode with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alphabet {
    /// `A-Z a-z 0-9 + /`, padded with `=`.
    Standard,
    /// `A-Z a-z 0-9 - _`, as used in URLs and filenames.
    UrlSafe,
}

impl Alphabet {
    fn table(self) -> &'static [u8; 64] {
        match self {
            Self::Standard => STANDARD,
            Self::UrlSafe => URL_SAFE,
        }
    }

    /// Whether the padding character is appended.
    ///
    /// WS-Security uses padded base64, but a URL-safe token often drops it.
    fn padded(self) -> bool {
        matches!(self, Self::Standard)
    }
}

/// Encodes bytes as base64.
#[must_use]
pub fn encode(data: &[u8]) -> String {
    encode_with(data, Alphabet::Standard)
}

/// Encodes bytes as base64 in the chosen alphabet.
#[must_use]
pub fn encode_with(data: &[u8], alphabet: Alphabet) -> String {
    let table = alphabet.table();
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(table[(b0 >> 2) as usize] as char);
        out.push(table[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(table[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else if alphabet.padded() {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(table[(b2 & 0x3f) as usize] as char);
        } else if alphabet.padded() {
            out.push('=');
        }
    }
    out
}

/// Decodes base64, accepting either alphabet and optional padding.
///
/// # Errors
///
/// Returns [`DecodeError`] if the input contains a character outside both
/// alphabets or a group that cannot be decoded.
pub fn decode(text: &str) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut buffer: u32 = 0;
    let mut bits = 0u32;
    let mut seen_padding = false;
    for byte in text.bytes() {
        if byte == b'=' {
            seen_padding = true;
            continue;
        }
        // A character after the padding is malformed rather than extra data.
        if seen_padding {
            return Err(DecodeError::TrailingData);
        }
        if byte.is_ascii_whitespace() {
            continue;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return Err(DecodeError::InvalidCharacter(byte as char)),
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    // A leftover of six bits can only be a zero pad; anything else is truncated.
    if bits >= 6 || (buffer & ((1 << bits) - 1)) != 0 {
        return Err(DecodeError::Truncated);
    }
    Ok(out)
}

/// Why base64 decoding failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// A character outside both alphabets.
    InvalidCharacter(char),
    /// The input ended in the middle of a group.
    Truncated,
    /// Characters followed the padding.
    TrailingData,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCharacter(c) => write!(f, "invalid base64 character {c:?}"),
            Self::Truncated => f.write_str("base64 input is truncated"),
            Self::TrailingData => f.write_str("base64 input has data after the padding"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_vectors_encode() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "Zg==");
        assert_eq!(encode(b"fo"), "Zm8=");
        assert_eq!(encode(b"foo"), "Zm9v");
        assert_eq!(encode(b"foob"), "Zm9vYg==");
        assert_eq!(encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn the_standard_vectors_decode() {
        for (text, expected) in [
            ("", &b""[..]),
            ("Zg==", b"f"),
            ("Zm8=", b"fo"),
            ("Zm9v", b"foo"),
            ("Zm9vYg==", b"foob"),
            ("Zm9vYmE=", b"fooba"),
            ("Zm9vYmFy", b"foobar"),
        ] {
            assert_eq!(decode(text).unwrap(), expected, "{text}");
        }
    }

    #[test]
    fn all_two_hundred_and_fifty_six_byte_values_round_trip() {
        let data: Vec<u8> = (0..=255u8).collect();
        assert_eq!(decode(&encode(&data)).unwrap(), data);
    }

    #[test]
    fn a_sixteen_byte_nonce_encodes_as_the_oracle_does() {
        // The nonce used in the reference fixtures: bytes 0x00 through 0x0f.
        let nonce: Vec<u8> = (0..16u8).collect();
        assert_eq!(encode(&nonce), "AAECAwQFBgcICQoLDA0ODw==");
    }

    #[test]
    fn the_url_safe_alphabet_differs_only_in_the_last_two_symbols() {
        assert_eq!(encode_with(&[0xfb, 0xef, 0xbe], Alphabet::Standard), "++++");
        assert_eq!(encode_with(&[0xfb, 0xef, 0xbe], Alphabet::UrlSafe), "----");
        // And either decodes back to the same bytes.
        assert_eq!(decode("++++").unwrap(), decode("----").unwrap());
    }

    #[test]
    fn padding_is_optional_on_decode() {
        assert_eq!(decode("Zg").unwrap(), b"f");
        assert_eq!(decode("Zm8").unwrap(), b"fo");
    }

    #[test]
    fn malformed_input_is_rejected() {
        assert_eq!(decode("Zg==").unwrap(), b"f");
        assert!(matches!(
            decode("Zm9v!"),
            Err(DecodeError::InvalidCharacter('!'))
        ));
        assert!(matches!(decode("Zg==Zg=="), Err(DecodeError::TrailingData)));
        assert!(matches!(decode("A"), Err(DecodeError::Truncated)));
    }
}
