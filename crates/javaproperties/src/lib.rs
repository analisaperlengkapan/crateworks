//! Reading Java `.properties` files.
//!
//! This is the format behind `application.properties` in every Spring service
//! and `messages_id.properties` in every Java i18n bundle. It looks like a list
//! of `key=value` lines and is not: the rules are older than the format's common
//! use, and getting them wrong is quiet. A key with an escaped `=` splits in the
//! wrong place; a `\uXXXX` escape comes out as literal text; a file written by
//! Java comes out as mojibake because the default encoding is ISO-8859-1, not
//! UTF-8.
//!
//! # Example
//!
//! ```
//! use javaproperties::parse;
//!
//! let pairs = parse(b"name = cipansor\ncity\\:bandung = west java\n").unwrap();
//! assert_eq!(pairs[0], ("name".to_string(), "cipansor".to_string()));
//! // The escaped colon is part of the key, not a separator.
//! assert_eq!(pairs[1], ("city:bandung".to_string(), "west java".to_string()));
//! ```
//!
//! # The rules, such as they are
//!
//! - A line whose first non-blank character is `#` or `!` is a comment.
//! - A key ends at the first unescaped `=`, `:`, or run of whitespace. An
//!   escaped separator is part of the key.
//! - Whitespace after the separator is not part of the value; whitespace at the
//!   end of the line is.
//! - A line ending in an odd number of backslashes continues onto the next,
//!   whose leading whitespace is dropped.
//! - `\t`, `\n`, `\f`, `\r`, `\\`, and `\uXXXX` are escapes. A backslash before
//!   any other character is dropped, so `\q` is `q`.
//! - Bytes are ISO-8859-1 unless the caller says otherwise. `parse_str` is there
//!   for text that has already been decoded.
//!
//! # How it is checked
//!
//! The results are compared against Python's `javaproperties` package, which
//! replicates `java.util.Properties` and is itself tested against Java. That
//! matters more than checking the parser against itself: several of these rules
//! are counter-intuitive enough that a reasonable guess is wrong.

mod escape;
mod reader;

pub use escape::Error;
pub use reader::{Properties, load, parse, parse_str};
