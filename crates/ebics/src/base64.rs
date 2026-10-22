//! Base64, as EBICS carries it.
//!
//! Every binary value in an EBICS message is base64: the key modulus and
//! exponent, the transaction key, the encrypted order data, the digests and the
//! signature values. The workspace has no base64 crate and the policy is
//! std-only, so the codec lives here. It is the same code as the one in this
//! workspace's `wssec` crate, which needed it for WS-Security nonces; a base64
//! is a base64 whatever the envelope.

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes `data` as standard base64 with `=` padding.
#[must_use]
pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map_or(0, u32::from);
        let b2 = chunk.get(2).copied().map_or(0, u32::from);
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(STANDARD[(triple >> 18) as usize & 0x3f] as char);
        out.push(STANDARD[(triple >> 12) as usize & 0x3f] as char);
        if chunk.len() > 1 {
            out.push(STANDARD[(triple >> 6) as usize & 0x3f] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(STANDARD[triple as usize & 0x3f] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Decodes standard base64, with or without padding.
///
/// Whitespace is ignored, because a bank's response may carry the value across
/// lines. Returns `None` for a character outside the alphabet, for a length that
/// cannot be a base64 string, and for padding that does not sit where it should.
#[must_use]
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut accumulator: u32 = 0;
    let mut bits = 0u32;
    let mut padding = 0usize;

    for byte in text.bytes() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if byte == b'=' {
            // Padding may only complete a final, partial quantum. A `=` seen
            // when no bits are pending follows a whole group and is malformed.
            if bits == 0 {
                return None;
            }
            padding += 1;
            continue;
        }
        // Padding may only be the last one or two characters.
        if padding > 0 {
            return None;
        }
        let value = match byte {
            b'A'..=b'Z' => u32::from(byte - b'A'),
            b'a'..=b'z' => u32::from(byte - b'a') + 26,
            b'0'..=b'9' => u32::from(byte - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        accumulator = (accumulator << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((accumulator >> bits) as u8);
        }
    }

    if padding > 2 || bits >= 6 {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    /// RFC 4648's own test vectors.
    #[test]
    fn rfc4648_vectors() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (plain, encoded) in cases {
            assert_eq!(encode(plain.as_bytes()), encoded, "encoding {plain}");
            assert_eq!(
                decode(encoded).unwrap(),
                plain.as_bytes(),
                "decoding {encoded}"
            );
        }
    }

    #[test]
    fn rejects_malformed() {
        assert!(decode("Zm9v!").is_none());
        assert!(decode("Zm9v=").is_none());
        assert!(decode("Z").is_none());
        assert!(decode("====").is_none());
    }

    #[test]
    fn ignores_whitespace() {
        assert_eq!(decode("Zm9v\nYmFy").unwrap(), b"foobar");
    }
}
