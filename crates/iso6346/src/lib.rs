#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

use std::fmt;

/// The value each letter takes in the check-digit sum, indexed from `A`.
///
/// ISO 6346 gives a letter its position in the alphabet plus nine, then skips
/// every multiple of eleven: `A = 10`, `B = 12` (11 is skipped), `K = 21`,
/// `L = 23` (22 is skipped), `U = 32`, `V = 34` (33 is skipped), `Z = 38`. The
/// letters `I`, `O` and `Q` do not appear in real prefixes but still carry a
/// value, because the standard assigns one to every letter.
const LETTER_VALUES: [u32; 26] = [
    10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 34, 35, 36,
    37, 38,
];

/// The value of an ASCII letter, or `None` if the byte is not one.
fn letter_value(letter: u8) -> Option<u32> {
    if letter.is_ascii_uppercase() {
        Some(LETTER_VALUES[usize::from(letter - b'A')])
    } else {
        None
    }
}

/// The category of a container, from the fourth character of its code.
///
/// The letter says what the container is for. It is part of the code, not
/// something the check digit can be read from, so a caller can ask for it back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Category {
    /// `U`, a freight container.
    Freight,
    /// `J`, removable equipment carried on a container.
    Detachable,
    /// `Z`, a chassis or trailer.
    Chassis,
}

impl Category {
    /// The letter this category is written as.
    #[must_use]
    pub const fn letter(self) -> char {
        match self {
            Self::Freight => 'U',
            Self::Detachable => 'J',
            Self::Chassis => 'Z',
        }
    }

    const fn from_letter(letter: u8) -> Option<Self> {
        match letter {
            b'U' => Some(Self::Freight),
            b'J' => Some(Self::Detachable),
            b'Z' => Some(Self::Chassis),
            _ => None,
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Freight => "freight container",
            Self::Detachable => "detachable equipment",
            Self::Chassis => "chassis or trailer",
        })
    }
}

/// Why a container code was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input was empty once separators were removed.
    Empty,
    /// The input was not the length a container code has.
    WrongLength {
        /// The length a container code has.
        expected: usize,
        /// The length that was supplied.
        found: usize,
    },
    /// A character that cannot appear at that position.
    InvalidCharacter(char),
    /// The fourth character is not one of `U`, `J` or `Z`.
    UnknownCategory(char),
    /// The six-digit serial number is outside `0` to `999999`.
    SerialOutOfRange(u32),
    /// The code is the right shape but the check digit does not match.
    BadCheckDigit {
        /// The check digit the rest of the code implies.
        expected: u32,
        /// The check digit that was supplied.
        found: u32,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("code is empty"),
            Self::WrongLength { expected, found } => {
                write!(f, "expected {expected} characters, found {found}")
            }
            Self::InvalidCharacter(c) => write!(f, "invalid character {c:?}"),
            Self::UnknownCategory(c) => write!(f, "category {c:?} is not one of U, J or Z"),
            Self::SerialOutOfRange(n) => write!(f, "serial {n} is outside 0 to 999999"),
            Self::BadCheckDigit { expected, found } => {
                write!(f, "check digit is {found}, expected {expected}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// A validated container code, stored in canonical form.
///
/// The canonical form is the eleven characters upper-cased with every separator
/// removed: `CSQU3054383`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Container {
    owner: [u8; 3],
    category: Category,
    serial: u32,
    check: u32,
}

impl Container {
    /// Builds a code from its parts, computing the check digit.
    ///
    /// This is how a code is generated rather than checked: the check digit is
    /// whatever the other ten characters imply.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCharacter`] when `owner_code` is not three ASCII
    /// letters, and [`Error::SerialOutOfRange`] when `serial` is above `999999`.
    ///
    /// # Example
    ///
    /// ```
    /// use iso6346::{Category, Container};
    ///
    /// let container = Container::new("CSQ", Category::Freight, 305_438).unwrap();
    /// assert_eq!(container.to_string(), "CSQU3054383");
    /// ```
    pub fn new(owner_code: &str, category: Category, serial: u32) -> Result<Self> {
        let owner = owner_code.as_bytes();
        if owner.len() != 3 {
            return Err(Error::WrongLength {
                expected: 3,
                found: owner.len(),
            });
        }
        let mut owner_bytes = [0u8; 3];
        for (slot, &byte) in owner_bytes.iter_mut().zip(owner) {
            let upper = byte.to_ascii_uppercase();
            if letter_value(upper).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
            *slot = upper;
        }
        if serial > 999_999 {
            return Err(Error::SerialOutOfRange(serial));
        }
        let check = check_digit_unchecked(owner_bytes, category, serial);
        Ok(Self {
            owner: owner_bytes,
            category,
            serial,
            check,
        })
    }

    /// Parses and validates a container code.
    ///
    /// Separators (spaces, hyphens and slashes) are ignored and the input is
    /// upper-cased, so a code copied out of a shipping document can be passed
    /// as-is.
    ///
    /// # Errors
    ///
    /// Returns the reason the code is not a valid container code.
    ///
    /// # Example
    ///
    /// ```
    /// use iso6346::Container;
    ///
    /// let container = Container::parse("csqu-305438-3").unwrap();
    /// assert_eq!(container.to_string(), "CSQU3054383");
    /// // One digit changed, so the check digit no longer matches.
    /// assert!(Container::parse("CSQU3054384").is_err());
    /// ```
    pub fn parse(input: &str) -> Result<Self> {
        let cleaned: Vec<u8> = input
            .bytes()
            .filter(|b| !matches!(b, b' ' | b'-' | b'/'))
            .map(|b| b.to_ascii_uppercase())
            .collect();

        if cleaned.is_empty() {
            return Err(Error::Empty);
        }
        if cleaned.len() != 11 {
            return Err(Error::WrongLength {
                expected: 11,
                found: cleaned.len(),
            });
        }

        let mut owner = [0u8; 3];
        for (slot, &byte) in owner.iter_mut().zip(&cleaned[..3]) {
            if letter_value(byte).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
            *slot = byte;
        }

        let category = Category::from_letter(cleaned[3])
            .ok_or_else(|| Error::UnknownCategory(char::from(cleaned[3])))?;

        let digits = &cleaned[4..];
        let mut serial = 0u32;
        for &byte in &digits[..6] {
            if !byte.is_ascii_digit() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
            serial = serial * 10 + u32::from(byte - b'0');
        }

        if !digits[6].is_ascii_digit() {
            return Err(Error::InvalidCharacter(char::from(digits[6])));
        }
        let check = u32::from(digits[6] - b'0');

        let expected = check_digit_unchecked(owner, category, serial);
        if check != expected {
            return Err(Error::BadCheckDigit {
                expected,
                found: check,
            });
        }

        Ok(Self {
            owner,
            category,
            serial,
            check,
        })
    }

    /// The three-letter owner code, which identifies the operator.
    #[must_use]
    pub fn owner_code(&self) -> String {
        String::from_utf8(self.owner.to_vec()).expect("owner code is ASCII")
    }

    /// What the container is for.
    #[must_use]
    pub const fn category(&self) -> Category {
        self.category
    }

    /// The six-digit serial number, as a number.
    #[must_use]
    pub const fn serial(&self) -> u32 {
        self.serial
    }

    /// The check digit.
    #[must_use]
    pub const fn check_digit(&self) -> u32 {
        self.check
    }

    /// The canonical form of the code, eleven characters.
    #[must_use]
    pub fn as_str(&self) -> String {
        format!(
            "{}{}{:06}{}",
            self.owner_code(),
            self.category.letter(),
            self.serial,
            self.check
        )
    }
}

impl fmt::Display for Container {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_str())
    }
}

/// The check digit for already-validated parts.
fn check_digit_unchecked(owner: [u8; 3], category: Category, serial: u32) -> u32 {
    // Each character is weighted by two raised to its position, counting from
    // zero, across the ten characters before the check digit.
    let mut sum = 0u32;
    for (index, &byte) in owner.iter().enumerate() {
        sum += letter_value(byte).unwrap_or(0) * 2u32.pow(index as u32);
    }
    sum += letter_value(category.letter() as u8).unwrap_or(0) * 8;

    let serial_text = format!("{serial:06}");
    for (index, byte) in serial_text.bytes().enumerate() {
        sum += u32::from(byte - b'0') * 2u32.pow(index as u32 + 4);
    }

    let remainder = sum % 11;
    if remainder == 10 { 0 } else { remainder }
}

/// The check digit the first ten characters of a code imply.
///
/// `owner_code` is the three-letter owner code, `category` the fourth character
/// and `serial` the six-digit serial number. The result is a single digit, `0`
/// to `9`; a sum whose remainder is ten is written as `0`.
///
/// Returns `None` when `owner_code` is not three ASCII letters or `serial` is
/// above `999999`, rather than computing a digit from input a code could never
/// hold.
///
/// # Example
///
/// ```
/// use iso6346::{Category, check_digit};
///
/// assert_eq!(check_digit("CSQ", Category::Freight, 305_438), Some(3));
/// assert_eq!(check_digit("CSQ1", Category::Freight, 305_438), None);
/// ```
#[must_use]
pub fn check_digit(owner_code: &str, category: Category, serial: u32) -> Option<u32> {
    let owner = owner_code.as_bytes();
    if owner.len() != 3 || serial > 999_999 {
        return None;
    }
    let mut owner_bytes = [0u8; 3];
    for (slot, &byte) in owner_bytes.iter_mut().zip(owner) {
        let upper = byte.to_ascii_uppercase();
        letter_value(upper)?;
        *slot = upper;
    }
    Some(check_digit_unchecked(owner_bytes, category, serial))
}

/// Validates a container code without keeping the parsed value.
///
/// # Errors
///
/// Returns the reason the code is not a valid container code.
///
/// # Example
///
/// ```
/// use iso6346::validate;
///
/// assert!(validate("CSQU3054383").is_ok());
/// assert!(validate("CSQU3054384").is_err());
/// ```
pub fn validate(input: &str) -> Result<()> {
    Container::parse(input).map(|_| ())
}
