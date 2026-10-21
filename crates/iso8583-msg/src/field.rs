//! Field descriptions.

use crate::codec::{DataEncoding, LengthEncoding, Padding};

/// How a field's length is expressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// The field always occupies exactly [`FieldSpec::max_len`] units.
    Fixed,
    /// One length unit precedes the data, so values run to nine (or fifteen
    /// for nibbles).
    Lvar,
    /// Two length units precede the data.
    Llvar,
    /// Three length units precede the data.
    Lllvar,
}

impl FieldType {
    /// How many length units precede the data.
    #[must_use]
    pub const fn prefix_units(self) -> usize {
        match self {
            Self::Fixed => 0,
            Self::Lvar => 1,
            Self::Llvar => 2,
            Self::Lllvar => 3,
        }
    }
}

/// Everything needed to encode or decode one field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSpec {
    /// How the field data itself is encoded.
    pub data: DataEncoding,
    /// How the length prefix is encoded. Ignored for fixed-length fields.
    pub length: LengthEncoding,
    /// Whether the field is fixed or variable length.
    pub kind: FieldType,
    /// The maximum length, in the unit implied by the two encodings: nibbles
    /// for binary-coded decimal, bytes otherwise. For a fixed-length field this
    /// is also the exact length.
    pub max_len: usize,
    /// Padding applied when a value does not fill the last unit.
    pub padding: Padding,
}

impl FieldSpec {
    /// A fixed-length field, which is what most numeric fields are.
    #[must_use]
    pub const fn fixed(data: DataEncoding, length: LengthEncoding, max_len: usize) -> Self {
        Self {
            data,
            length,
            kind: FieldType::Fixed,
            max_len,
            padding: Padding::None,
        }
    }

    /// A variable-length field whose length prefix has `units` units.
    #[must_use]
    pub const fn variable(
        data: DataEncoding,
        length: LengthEncoding,
        units: usize,
        max_len: usize,
    ) -> Self {
        Self {
            data,
            length,
            kind: match units {
                1 => FieldType::Lvar,
                2 => FieldType::Llvar,
                _ => FieldType::Lllvar,
            },
            max_len,
            padding: Padding::None,
        }
    }

    /// Returns the same field with a different padding rule.
    #[must_use]
    pub const fn with_padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// The unit a length is counted in: nibbles for binary-coded decimal, bytes
    /// for everything else.
    ///
    /// This matters because a BCD field holds two digits per byte, so a
    /// nineteen-digit account number occupies ten bytes and its length prefix
    /// reads nineteen.
    #[must_use]
    pub const fn length_unit(self) -> LengthUnit {
        match self.data {
            DataEncoding::Bcd => LengthUnit::Nibbles,
            _ => LengthUnit::Bytes,
        }
    }
}

/// What a length is counted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    /// Whole bytes.
    Bytes,
    /// Half bytes, as used by binary-coded decimal.
    Nibbles,
}
