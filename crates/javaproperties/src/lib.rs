#![doc = include_str!("../README.md")]
mod escape;
mod reader;
mod writer;

pub use escape::Error;
pub use reader::{Properties, load, parse, parse_str};
pub use writer::{escape_key, escape_value, store};
