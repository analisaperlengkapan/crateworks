//! The message itself.

use std::collections::BTreeMap;

use crate::bitmap;
use crate::codec;
use crate::field::LengthUnit;
use crate::spec::{MTI_FIELD, Spec};
use crate::{Error, MAX_FIELDS, Result};

/// An ISO 8583 message: a message type indicator plus a set of numbered fields.
///
/// Fields are kept as strings in ascending order, which is the order they are
/// encoded in. Setting a field to an empty string removes it, because a field
/// that is present but empty is not representable in a bitmap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    mti: String,
    fields: BTreeMap<u8, String>,
}

impl Message {
    /// A message with the given message type indicator and no fields.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadMti`] unless the indicator is exactly four
    /// characters.
    pub fn new(mti: &str) -> Result<Self> {
        if mti.len() != 4 || !mti.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(Error::BadMti(mti.to_string()));
        }
        Ok(Self {
            mti: mti.to_ascii_uppercase(),
            fields: BTreeMap::new(),
        })
    }

    /// The message type indicator.
    #[must_use]
    pub fn mti(&self) -> &str {
        &self.mti
    }

    /// Replaces the message type indicator.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadMti`] unless the indicator is exactly four
    /// characters.
    pub fn set_mti(&mut self, mti: &str) -> Result<()> {
        if mti.len() != 4 || !mti.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(Error::BadMti(mti.to_string()));
        }
        self.mti = mti.to_ascii_uppercase();
        Ok(())
    }

    /// Sets a field, replacing any previous value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FieldOutOfRange`] if the field number is 0, 1, or
    /// greater than 192. Field 0 is the message type indicator and field 1 is
    /// the bitmap, so neither can be set as data.
    pub fn set(&mut self, field: u8, value: &str) -> Result<&mut Self> {
        if field == MTI_FIELD || field == 1 || field > MAX_FIELDS {
            return Err(Error::FieldOutOfRange(field));
        }
        if value.is_empty() {
            self.fields.remove(&field);
        } else {
            self.fields.insert(field, value.to_string());
        }
        Ok(self)
    }

    /// The value of a field, if it is present.
    #[must_use]
    pub fn get(&self, field: u8) -> Option<&str> {
        self.fields.get(&field).map(String::as_str)
    }

    /// Removes a field, returning its previous value.
    pub fn remove(&mut self, field: u8) -> Option<String> {
        self.fields.remove(&field)
    }

    /// Whether a field is present.
    #[must_use]
    pub fn contains(&self, field: u8) -> bool {
        self.fields.contains_key(&field)
    }

    /// The field numbers present, in ascending order.
    pub fn fields(&self) -> impl Iterator<Item = u8> + '_ {
        self.fields.keys().copied()
    }

    /// How many data fields are present.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Whether no data fields are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Encodes the message.
    ///
    /// # Errors
    ///
    /// Returns an error if the specification is inconsistent, a field is not
    /// described, or a value does not fit its field.
    pub fn encode(&self, spec: &Spec) -> Result<Vec<u8>> {
        spec.validate()?;

        let mut out = codec::encode_data(MTI_FIELD, &self.mti, &spec.get(MTI_FIELD)?)?;

        let present: std::collections::BTreeSet<u8> = self.fields.keys().copied().collect();
        out.extend(bitmap::encode(&present, &spec.get(1)?)?);

        for (field, value) in &self.fields {
            let field_spec = spec.get(*field)?;
            let encoded = codec::encode_data(*field, value, &field_spec)?;

            let unit = field_spec.length_unit();
            let units = match unit {
                LengthUnit::Bytes => encoded.len(),
                LengthUnit::Nibbles => value.len(),
            };
            out.extend(codec::encode_length(units, &field_spec)?);
            out.extend(encoded);
        }

        Ok(out)
    }

    /// Decodes a message.
    ///
    /// # Errors
    ///
    /// Returns an error if the specification is inconsistent, the input is
    /// empty or truncated, or the bitmap and the data disagree.
    ///
    /// # Example
    ///
    /// ```
    /// use iso8583_msg::{default_ascii, Message};
    ///
    /// let spec = default_ascii();
    /// let mut message = Message::new("0200").unwrap();
    /// message.set(4, "000000001000").unwrap();
    ///
    /// let bytes = message.encode(&spec).unwrap();
    /// let decoded = Message::decode(&bytes, &spec).unwrap();
    ///
    /// assert_eq!(decoded.mti(), "0200");
    /// assert_eq!(decoded.get(4), Some("000000001000"));
    /// ```
    pub fn decode(bytes: &[u8], spec: &Spec) -> Result<Self> {
        if bytes.is_empty() {
            return Err(Error::Empty);
        }
        spec.validate()?;

        let mti_spec = spec.get(MTI_FIELD)?;
        let mti_width = codec::units_to_bytes(mti_spec.max_len, mti_spec.length_unit());
        if bytes.len() < mti_width {
            return Err(Error::Truncated {
                field: MTI_FIELD,
                needed: mti_width,
                available: bytes.len(),
            });
        }
        let mti = codec::decode_data(MTI_FIELD, &bytes[..mti_width], &mti_spec)?;
        let mut cursor = mti_width;

        let bitmap_spec = spec.get(1)?;
        let (present, consumed) = bitmap::decode(&bytes[cursor..], &bitmap_spec)?;
        cursor += consumed;

        let mut message = Self::new(&mti)?;
        for field in present {
            let field_spec = spec.get(field)?;
            let (units, prefix) = codec::decode_length(field, &bytes[cursor..], &field_spec)?;
            cursor += prefix;

            let width = codec::units_to_bytes(units, field_spec.length_unit());
            if bytes.len() < cursor + width {
                return Err(Error::Truncated {
                    field,
                    needed: width,
                    available: bytes.len().saturating_sub(cursor),
                });
            }
            let value = codec::decode_data(field, &bytes[cursor..cursor + width], &field_spec)?;
            cursor += width;
            message.fields.insert(field, value);
        }

        if cursor != bytes.len() {
            return Err(Error::TrailingBytes(bytes.len() - cursor));
        }

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::default_ascii;

    #[test]
    fn a_message_type_indicator_must_be_four_characters() {
        assert!(Message::new("0200").is_ok());
        assert!(Message::new("020").is_err());
        assert!(Message::new("02000").is_err());
        assert!(Message::new("02-0").is_err());
    }

    #[test]
    fn setting_an_empty_value_removes_the_field() {
        let mut msg = Message::new("0200").unwrap();
        msg.set(4, "000000010000").unwrap();
        assert!(msg.contains(4));
        msg.set(4, "").unwrap();
        assert!(!msg.contains(4));
    }

    #[test]
    fn the_mti_and_bitmap_cannot_be_set_as_fields() {
        let mut msg = Message::new("0200").unwrap();
        assert_eq!(msg.set(0, "0200"), Err(Error::FieldOutOfRange(0)));
        assert_eq!(msg.set(1, "00"), Err(Error::FieldOutOfRange(1)));
        assert_eq!(msg.set(193, "x"), Err(Error::FieldOutOfRange(193)));
    }

    #[test]
    fn encode_then_decode_round_trips() {
        let spec = default_ascii();
        let mut msg = Message::new("0200").unwrap();
        msg.set(2, "4111111111111111").unwrap();
        msg.set(3, "000000").unwrap();
        msg.set(4, "000000010000").unwrap();
        msg.set(7, "0101120000").unwrap();
        msg.set(11, "123456").unwrap();
        msg.set(39, "00").unwrap();
        msg.set(41, "TERM0001").unwrap();
        msg.set(70, "301").unwrap();

        let bytes = msg.encode(&spec).unwrap();
        let back = Message::decode(&bytes, &spec).unwrap();
        assert_eq!(back, msg);
    }

    #[test]
    fn decoding_an_empty_slice_reports_empty() {
        assert_eq!(Message::decode(&[], &default_ascii()), Err(Error::Empty));
    }
}
