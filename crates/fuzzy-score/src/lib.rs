#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod difflib;
pub mod fuzz;
pub mod process;
mod unicode_tables;
pub mod utils;
mod word_chars;

pub use fuzz::{
    Options, partial_ratio, partial_token_set_ratio, partial_token_set_ratio_with_options,
    partial_token_sort_ratio, partial_token_sort_ratio_with_options, q_ratio, q_ratio_with_options,
    ratio, token_set_ratio, token_set_ratio_with_options, token_sort_ratio,
    token_sort_ratio_with_options, uq_ratio, uw_ratio, w_ratio,
};
pub use process::{Scorer, dedupe, extract, extract_one};
pub use utils::full_process;
