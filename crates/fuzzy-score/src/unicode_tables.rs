//! Unicode tables taken from CPython, so that `full_process` reproduces
//! Python's `str.lower()`, `str.split()` and `str.strip()` exactly.
//!
//! Rust's own `char` tables are close but not identical: they track a newer
//! Unicode release, and `char::is_whitespace` follows `White_Space` rather
//! than Python's `str.isspace()`, which also covers the file, group, record
//! and unit separators U+001C..U+001F. These tables close the gap.

/// Codepoints Python's `str.isspace()` reports, and so the set `str.split()`
/// and `str.strip()` break on.
pub(crate) static PYTHON_WHITESPACE: [u32; 29] = [
    0x0009, 0x000A, 0x000B, 0x000C, 0x000D, 0x001C, 0x001D, 0x001E, 0x001F, 0x0020, 0x0085, 0x00A0,
    0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200A,
    0x2028, 0x2029, 0x202F, 0x205F, 0x3000,
];

/// Codepoints where Rust's `char::to_lowercase` differs from CPython's
/// `str.lower()`, mapped to what Python produces.
pub(crate) static PYTHON_LOWERCASE_OVERRIDES: [(u32, &str); 56] = [
    (0x1C89, "\u{1C89}"),
    (0xA7CB, "\u{A7CB}"),
    (0xA7CC, "\u{A7CC}"),
    (0xA7CE, "\u{A7CE}"),
    (0xA7D2, "\u{A7D2}"),
    (0xA7D4, "\u{A7D4}"),
    (0xA7DA, "\u{A7DA}"),
    (0xA7DC, "\u{A7DC}"),
    (0x10D50, "\u{10D50}"),
    (0x10D51, "\u{10D51}"),
    (0x10D52, "\u{10D52}"),
    (0x10D53, "\u{10D53}"),
    (0x10D54, "\u{10D54}"),
    (0x10D55, "\u{10D55}"),
    (0x10D56, "\u{10D56}"),
    (0x10D57, "\u{10D57}"),
    (0x10D58, "\u{10D58}"),
    (0x10D59, "\u{10D59}"),
    (0x10D5A, "\u{10D5A}"),
    (0x10D5B, "\u{10D5B}"),
    (0x10D5C, "\u{10D5C}"),
    (0x10D5D, "\u{10D5D}"),
    (0x10D5E, "\u{10D5E}"),
    (0x10D5F, "\u{10D5F}"),
    (0x10D60, "\u{10D60}"),
    (0x10D61, "\u{10D61}"),
    (0x10D62, "\u{10D62}"),
    (0x10D63, "\u{10D63}"),
    (0x10D64, "\u{10D64}"),
    (0x10D65, "\u{10D65}"),
    (0x16EA0, "\u{16EA0}"),
    (0x16EA1, "\u{16EA1}"),
    (0x16EA2, "\u{16EA2}"),
    (0x16EA3, "\u{16EA3}"),
    (0x16EA4, "\u{16EA4}"),
    (0x16EA5, "\u{16EA5}"),
    (0x16EA6, "\u{16EA6}"),
    (0x16EA7, "\u{16EA7}"),
    (0x16EA8, "\u{16EA8}"),
    (0x16EA9, "\u{16EA9}"),
    (0x16EAA, "\u{16EAA}"),
    (0x16EAB, "\u{16EAB}"),
    (0x16EAC, "\u{16EAC}"),
    (0x16EAD, "\u{16EAD}"),
    (0x16EAE, "\u{16EAE}"),
    (0x16EAF, "\u{16EAF}"),
    (0x16EB0, "\u{16EB0}"),
    (0x16EB1, "\u{16EB1}"),
    (0x16EB2, "\u{16EB2}"),
    (0x16EB3, "\u{16EB3}"),
    (0x16EB4, "\u{16EB4}"),
    (0x16EB5, "\u{16EB5}"),
    (0x16EB6, "\u{16EB6}"),
    (0x16EB7, "\u{16EB7}"),
    (0x16EB8, "\u{16EB8}"),
    (0x1E921, "\u{1E943}"),
];
