//! Message specifications.
//!
//! A specification maps field numbers to [`FieldSpec`]s. Real switches differ
//! in their choices, so this is data: start from one of the defaults and
//! override the fields your switch lays out differently.

use std::collections::BTreeMap;

use crate::codec::{DataEncoding, LengthEncoding, Padding};
use crate::field::{FieldSpec, FieldType};
use crate::{Error, Result};

/// The field number holding the message type indicator.
pub const MTI_FIELD: u8 = 0;
/// The field number holding the primary bitmap.
pub const PRIMARY_BITMAP_FIELD: u8 = 1;

/// How every field of a message is laid out.
///
/// Field `0` is the message type indicator and field `1` is the bitmap chain;
/// neither is a data field. Fields `2` to `192` are the data fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    fields: BTreeMap<u8, FieldSpec>,
}

impl Spec {
    /// An empty specification. Fields added to it later.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
        }
    }

    /// Describes a field.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FieldOutOfRange`] if the field number is not 0-192.
    pub fn set(&mut self, field: u8, spec: FieldSpec) -> Result<&mut Self> {
        if field > crate::MAX_FIELDS {
            return Err(Error::FieldOutOfRange(field));
        }
        self.fields.insert(field, spec);
        Ok(self)
    }

    /// Describes a field, panicking if the number is out of range.
    ///
    /// Useful when building a specification from a literal, where an out-of-range
    /// number is a programming mistake rather than a runtime condition.
    ///
    /// # Panics
    ///
    /// Panics if `field` is greater than 192.
    #[must_use]
    pub fn with(mut self, field: u8, spec: FieldSpec) -> Self {
        assert!(field <= crate::MAX_FIELDS, "field {field} is out of range");
        self.fields.insert(field, spec);
        self
    }

    /// The description of a field.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownField`] if the field is not described.
    pub fn get(&self, field: u8) -> Result<FieldSpec> {
        self.fields
            .get(&field)
            .copied()
            .ok_or(Error::UnknownField(field))
    }

    /// Whether a field is described.
    #[must_use]
    pub fn contains(&self, field: u8) -> bool {
        self.fields.contains_key(&field)
    }

    /// How many fields are described.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Whether no fields are described.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Checks the specification for internal contradictions.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadSpec`] if a field cannot be encoded as described.
    pub fn validate(&self) -> Result<()> {
        let mti = self.get(MTI_FIELD)?;
        if mti.kind != FieldType::Fixed || mti.max_len != 4 {
            return Err(Error::BadSpec(
                "the message type indicator must be a fixed four-character field".to_string(),
            ));
        }
        let bitmap = self.get(PRIMARY_BITMAP_FIELD)?;
        // A bitmap always represents eight bytes. In ASCII that is sixteen
        // hexadecimal characters, in binary-coded decimal sixteen nibbles, and
        // in binary eight raw bytes.
        let represented = match bitmap.data {
            DataEncoding::Binary => bitmap.max_len,
            DataEncoding::Ascii | DataEncoding::Ebcdic | DataEncoding::Bcd => bitmap.max_len / 2,
        };
        if bitmap.kind != FieldType::Fixed || represented != 8 {
            return Err(Error::BadSpec(
                "the bitmap must be a fixed eight-byte field".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for Spec {
    fn default() -> Self {
        Self::new()
    }
}

/// A specification with every field in ASCII, which is the commonest layout for
/// the messages a payment terminal sees.
///
/// This mirrors the `default_ascii` specification of the widely used
/// `pyiso8583` library, so messages encoded here interoperate with it. Lengths
/// follow the ISO 8583:1987 conventions: a nineteen-digit account number, a
/// six-digit processing code, twelve-digit amounts, and so on.
#[must_use]
pub fn default_ascii() -> Spec {
    let mut spec = Spec::new();
    for (field, entry) in table() {
        spec = spec.with(field, entry);
    }
    spec
}

/// The field table, shared by the ASCII and EBCDIC defaults.
///
/// Lengths are the ISO 8583:1987 ones and match `pyiso8583`'s `default_ascii`.
fn table() -> Vec<(u8, FieldSpec)> {
    let a = DataEncoding::Ascii;
    let l = LengthEncoding::Ascii;
    let fixed = |max_len: usize| FieldSpec::fixed(a, l, max_len);
    let llvar = |max_len: usize| FieldSpec::variable(a, l, 2, max_len);
    let lllvar = |max_len: usize| FieldSpec::variable(a, l, 3, max_len);

    vec![
        (MTI_FIELD, fixed(4)),
        (PRIMARY_BITMAP_FIELD, fixed(16)),
        (2, llvar(19)),
        (3, fixed(6)),
        (4, fixed(12)),
        (5, fixed(12)),
        (6, fixed(12)),
        (7, fixed(10)),
        (8, fixed(8)),
        (9, fixed(8)),
        (10, fixed(8)),
        (11, fixed(6)),
        (12, fixed(6)),
        (13, fixed(4)),
        (14, fixed(4)),
        (15, fixed(4)),
        (16, fixed(4)),
        (17, fixed(4)),
        (18, fixed(4)),
        (19, fixed(3)),
        (20, fixed(3)),
        (21, fixed(3)),
        (22, fixed(3)),
        (23, fixed(3)),
        (24, fixed(3)),
        (25, fixed(2)),
        (26, fixed(2)),
        (27, fixed(1)),
        (28, fixed(8)),
        (29, fixed(8)),
        (30, fixed(8)),
        (31, fixed(8)),
        (32, llvar(11)),
        (33, llvar(11)),
        (34, llvar(28)),
        (35, llvar(37)),
        (36, lllvar(104)),
        (37, fixed(12)),
        (38, fixed(6)),
        (39, fixed(2)),
        (40, fixed(3)),
        (41, fixed(8)),
        (42, fixed(15)),
        (43, fixed(40)),
        (44, llvar(25)),
        (45, llvar(76)),
        (46, lllvar(999)),
        (47, lllvar(999)),
        (48, lllvar(999)),
        (49, fixed(3)),
        (50, fixed(3)),
        (51, fixed(3)),
        (52, fixed(16)),
        (53, fixed(16)),
        (54, lllvar(240)),
        (55, lllvar(510)),
        (56, lllvar(999)),
        (57, lllvar(999)),
        (58, lllvar(999)),
        (59, lllvar(999)),
        (60, lllvar(999)),
        (61, lllvar(999)),
        (62, lllvar(999)),
        (63, lllvar(999)),
        (64, fixed(16)),
        (65, fixed(16)),
        (66, fixed(1)),
        (67, fixed(2)),
        (68, fixed(3)),
        (69, fixed(3)),
        (70, fixed(3)),
        (71, fixed(4)),
        (72, fixed(4)),
        (73, fixed(6)),
        (74, fixed(10)),
        (75, fixed(10)),
        (76, fixed(10)),
        (77, fixed(10)),
        (78, fixed(10)),
        (79, fixed(10)),
        (80, fixed(10)),
        (81, fixed(10)),
        (82, fixed(12)),
        (83, fixed(12)),
        (84, fixed(12)),
        (85, fixed(12)),
        (86, fixed(16)),
        (87, fixed(16)),
        (88, fixed(16)),
        (89, fixed(16)),
        (90, fixed(42)),
        (91, fixed(1)),
        (92, fixed(2)),
        (93, fixed(5)),
        (94, fixed(7)),
        (95, fixed(42)),
        (96, fixed(16)),
        (97, fixed(17)),
        (98, fixed(25)),
        (99, llvar(11)),
        (100, llvar(11)),
        (101, llvar(17)),
        (102, llvar(28)),
        (103, llvar(28)),
        (104, lllvar(100)),
        (105, lllvar(999)),
        (106, lllvar(999)),
        (107, lllvar(999)),
        (108, lllvar(999)),
        (109, lllvar(999)),
        (110, lllvar(999)),
        (111, lllvar(999)),
        (112, lllvar(999)),
        (113, lllvar(999)),
        (114, lllvar(999)),
        (115, lllvar(999)),
        (116, lllvar(999)),
        (117, lllvar(999)),
        (118, lllvar(999)),
        (119, lllvar(999)),
        (120, lllvar(999)),
        (121, lllvar(999)),
        (122, lllvar(999)),
        (123, lllvar(999)),
        (124, lllvar(999)),
        (125, lllvar(999)),
        (126, lllvar(999)),
        (127, lllvar(999)),
        (128, fixed(16)),
    ]
}

/// A specification in which numeric fields are packed as binary-coded decimal,
/// halving their size on the wire.
///
/// This is the layout used by many high-volume switches, where a nineteen-digit
/// account number travels in ten bytes rather than nineteen. The message type
/// indicator and the bitmap become binary-coded decimal and binary respectively,
/// and numeric fields gain a right-hand zero pad so an odd number of digits
/// still fills whole bytes.
#[must_use]
pub fn default_bcd() -> Spec {
    let mut spec = Spec::new();
    for (field, entry) in table() {
        let entry = if field == MTI_FIELD {
            FieldSpec::fixed(DataEncoding::Bcd, LengthEncoding::Ascii, 4)
        } else if field == PRIMARY_BITMAP_FIELD {
            FieldSpec::fixed(DataEncoding::Binary, LengthEncoding::Ascii, 8)
        } else if BINARY_FIELDS.contains(&field) {
            FieldSpec::fixed(
                DataEncoding::Binary,
                LengthEncoding::Ascii,
                entry.max_len * 2,
            )
        } else if NUMERIC_FIELDS.contains(&field) {
            FieldSpec {
                data: DataEncoding::Bcd,
                padding: Padding::Right('0'),
                ..entry
            }
        } else {
            entry
        };
        spec = spec.with(field, entry);
    }
    spec
}

/// Fields that carry binary data rather than text, per ISO 8583:1987.
const BINARY_FIELDS: [u8; 6] = [52, 53, 64, 96, 128, 55];

/// Fields whose contents are digits, per ISO 8583:1987.
///
/// Only these become binary-coded decimal in [`default_bcd`]; everything else
/// stays character data, because a terminal identifier or a card acceptor name
/// contains letters.
const NUMERIC_FIELDS: [u8; 67] = [
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 49, 50, 51, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
    80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 91, 92, 93, 94, 97, 99, 100,
];

/// A specification with the whole message in EBCDIC code page 500, which is
/// what older mainframe switches expect.
#[must_use]
pub fn default_ebcdic() -> Spec {
    let mut spec = base(DataEncoding::Ebcdic, LengthEncoding::Ebcdic);
    spec = spec.with(
        MTI_FIELD,
        FieldSpec::fixed(DataEncoding::Ebcdic, LengthEncoding::Ebcdic, 4),
    );
    spec
}

/// The field table shared by the default specifications.
///
/// Lengths are the ISO 8583:1987 ones, and field 1 is the bitmap chain.
fn base(data: DataEncoding, length: LengthEncoding) -> Spec {
    let fixed = |max_len: usize| FieldSpec::fixed(data, length, max_len);
    let llvar = |max_len: usize| FieldSpec::variable(data, length, 2, max_len);
    let lllvar = |max_len: usize| FieldSpec::variable(data, length, 3, max_len);

    let mut spec = Spec::new();
    spec = spec.with(MTI_FIELD, fixed(4));
    spec = spec.with(PRIMARY_BITMAP_FIELD, fixed(16));

    // The lengths below come from the ISO 8583:1987 field table. Fields that
    // are described as `an` or `ans` in that table accept letters as well, but
    // the encoding does not depend on that, so it is not recorded here.
    let table: [(u8, FieldSpec); 127] = [
        (2, llvar(19)),     // Primary account number
        (3, fixed(6)),      // Processing code
        (4, fixed(12)),     // Amount, transaction
        (5, fixed(12)),     // Amount, settlement
        (6, fixed(12)),     // Amount, cardholder billing
        (7, fixed(10)),     // Transmission date and time
        (8, fixed(8)),      // Amount, cardholder billing fee
        (9, fixed(8)),      // Conversion rate, settlement
        (10, fixed(8)),     // Conversion rate, cardholder billing
        (11, fixed(6)),     // System trace audit number
        (12, fixed(6)),     // Time, local transaction
        (13, fixed(4)),     // Date, local transaction
        (14, fixed(4)),     // Date, expiration
        (15, fixed(4)),     // Date, settlement
        (16, fixed(4)),     // Date, conversion
        (17, fixed(4)),     // Date, capture
        (18, fixed(4)),     // Merchant type
        (19, fixed(3)),     // Acquiring institution country code
        (20, fixed(3)),     // Pan extended, country code
        (21, fixed(3)),     // Forwarding institution country code
        (22, fixed(3)),     // Point of service entry mode
        (23, fixed(3)),     // Application pan sequence number
        (24, fixed(3)),     // Network international identifier
        (25, fixed(2)),     // Point of service condition code
        (26, fixed(2)),     // Point of service capture code
        (27, fixed(1)),     // Authorising identification response length
        (28, fixed(8)),     // Amount, transaction fee
        (29, fixed(8)),     // Amount, settlement fee
        (30, fixed(8)),     // Amount, transaction processing fee
        (31, fixed(8)),     // Amount, settlement processing fee
        (32, llvar(11)),    // Acquiring institution identification code
        (33, llvar(11)),    // Forwarding institution identification code
        (34, llvar(28)),    // Primary account number, extended
        (35, llvar(37)),    // Track 2 data
        (36, lllvar(104)),  // Track 3 data
        (37, fixed(12)),    // Retrieval reference number
        (38, fixed(6)),     // Authorisation identification response
        (39, fixed(2)),     // Response code
        (40, fixed(3)),     // Service restriction code
        (41, fixed(8)),     // Card acceptor terminal identification
        (42, fixed(15)),    // Card acceptor identification code
        (43, fixed(40)),    // Card acceptor name and location
        (44, llvar(25)),    // Additional response data
        (45, llvar(76)),    // Track 1 data
        (46, lllvar(999)),  // Additional data, ISO
        (47, lllvar(999)),  // Additional data, national
        (48, lllvar(999)),  // Additional data, private
        (49, fixed(3)),     // Currency code, transaction
        (50, fixed(3)),     // Currency code, settlement
        (51, fixed(3)),     // Currency code, cardholder billing
        (52, fixed(8)),     // PIN block
        (53, fixed(16)),    // Security related control information
        (54, lllvar(120)),  // Additional amounts
        (55, lllvar(255)),  // ICC data
        (56, lllvar(999)),  // Reserved, ISO
        (57, lllvar(999)),  // Reserved, national
        (58, lllvar(999)),  // Reserved, national
        (59, lllvar(999)),  // Reserved, national
        (60, lllvar(999)),  // Reserved, national
        (61, lllvar(999)),  // Reserved, national
        (62, lllvar(999)),  // Reserved, private
        (63, lllvar(999)),  // Reserved, private
        (64, fixed(8)),     // MAC
        (65, fixed(8)),     // Bitmap, tertiary
        (66, fixed(1)),     // Amount, settlement fee
        (67, fixed(2)),     // Extended payment code
        (68, fixed(3)),     // Receiving institution country code
        (69, fixed(3)),     // Settlement institution country code
        (70, fixed(3)),     // Network management information code
        (71, fixed(4)),     // Message number
        (72, fixed(4)),     // Message number, last
        (73, fixed(6)),     // Date, action
        (74, fixed(10)),    // Credits, number
        (75, fixed(10)),    // Credits, reversal number
        (76, fixed(10)),    // Debits, number
        (77, fixed(10)),    // Debits, reversal number
        (78, fixed(10)),    // Transfer, number
        (79, fixed(10)),    // Transfer, reversal number
        (80, fixed(10)),    // Inquiries, number
        (81, fixed(10)),    // Authorisations, number
        (82, fixed(12)),    // Credits, processing fee amount
        (83, fixed(12)),    // Credits, transaction fee amount
        (84, fixed(12)),    // Debits, processing fee amount
        (85, fixed(12)),    // Debits, transaction fee amount
        (86, fixed(16)),    // Credits, amount
        (87, fixed(16)),    // Credits, reversal amount
        (88, fixed(16)),    // Debits, amount
        (89, fixed(16)),    // Debits, reversal amount
        (90, fixed(42)),    // Original data elements
        (91, fixed(1)),     // File update code
        (92, fixed(2)),     // File security code
        (93, fixed(5)),     // Response indicator
        (94, fixed(7)),     // Service indicator
        (95, fixed(42)),    // Replacement amounts
        (96, fixed(8)),     // Message security code
        (97, fixed(17)),    // Amount, net settlement
        (98, fixed(25)),    // Payee
        (99, llvar(11)),    // Settlement institution identification code
        (100, llvar(11)),   // Receiving institution identification code
        (101, llvar(17)),   // File name
        (102, llvar(28)),   // Account identification 1
        (103, llvar(28)),   // Account identification 2
        (104, lllvar(100)), // Transaction description
        (105, lllvar(999)), // Reserved, ISO
        (106, lllvar(999)), // Reserved, ISO
        (107, lllvar(999)), // Reserved, ISO
        (108, lllvar(999)), // Reserved, ISO
        (109, lllvar(999)), // Reserved, ISO
        (110, lllvar(999)), // Reserved, ISO
        (111, lllvar(999)), // Reserved, ISO
        (112, lllvar(999)), // Reserved, national
        (113, lllvar(999)), // Reserved, national
        (114, lllvar(999)), // Reserved, national
        (115, lllvar(999)), // Reserved, national
        (116, lllvar(999)), // Reserved, national
        (117, lllvar(999)), // Reserved, national
        (118, lllvar(999)), // Reserved, national
        (119, lllvar(999)), // Reserved, national
        (120, lllvar(999)), // Reserved, private
        (121, lllvar(999)), // Reserved, private
        (122, lllvar(999)), // Reserved, private
        (123, lllvar(999)), // Reserved, private
        (124, lllvar(999)), // Reserved, private
        (125, lllvar(999)), // Reserved, private
        (126, lllvar(999)), // Reserved, private
        (127, lllvar(999)), // Reserved, private
        (128, fixed(8)),    // MAC
    ];

    for (field, field_spec) in table {
        spec = spec.with(field, field_spec);
    }
    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_describe_every_field_up_to_128() {
        let spec = default_ascii();
        for field in 0..=128u8 {
            assert!(spec.contains(field), "field {field} missing");
        }
    }

    #[test]
    fn defaults_validate() {
        default_ascii().validate().unwrap();
        default_bcd().validate().unwrap();
        default_ebcdic().validate().unwrap();
    }

    #[test]
    fn a_spec_with_the_wrong_mti_width_is_rejected() {
        let mut spec = Spec::new();
        spec.set(
            MTI_FIELD,
            FieldSpec::fixed(DataEncoding::Ascii, LengthEncoding::Ascii, 2),
        )
        .unwrap();
        spec.set(
            PRIMARY_BITMAP_FIELD,
            FieldSpec::fixed(DataEncoding::Ascii, LengthEncoding::Ascii, 16),
        )
        .unwrap();
        assert!(matches!(spec.validate(), Err(Error::BadSpec(_))));
    }

    #[test]
    fn setting_an_out_of_range_field_is_rejected() {
        let mut spec = Spec::new();
        assert_eq!(
            spec.set(
                193,
                FieldSpec::fixed(DataEncoding::Ascii, LengthEncoding::Ascii, 1,)
            ),
            Err(Error::FieldOutOfRange(193))
        );
    }

    #[test]
    fn bcd_converts_numeric_fields_and_leaves_text_and_binary_alone() {
        let spec = default_bcd();
        // An amount is digits, so it packs two per byte.
        assert_eq!(spec.get(4).unwrap().data, DataEncoding::Bcd);
        // A terminal identifier contains letters.
        assert_eq!(spec.get(41).unwrap().data, DataEncoding::Ascii);
        // The PIN block and MAC are binary.
        assert_eq!(spec.get(52).unwrap().data, DataEncoding::Binary);
        assert_eq!(spec.get(64).unwrap().data, DataEncoding::Binary);
    }
}
