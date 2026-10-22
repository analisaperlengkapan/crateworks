#![doc = include_str!("../README.md")]
mod escape;
mod reader;

pub use escape::Error;
pub use reader::{Properties, load, parse, parse_str};
