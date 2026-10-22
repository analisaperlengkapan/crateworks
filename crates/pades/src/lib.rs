#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub use verify::{
    Coverage, Error, Finding, Integrity, Kind, Level, Report, Severity, Signature, Signer,
    ValidationData, verify,
};

pub mod cms;
pub mod der;
pub mod rsa;
pub mod sha2;
pub mod verify;
pub mod x509;
