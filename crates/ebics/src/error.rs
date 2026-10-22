//! What can go wrong.
//!
//! The errors are separated by layer, because the layer tells the caller what to
//! do about it: a malformed message is the caller's bug, a rejected return code
//! is the bank's answer, and a failed signature is either a wrong key or a
//! tampered message.

use std::fmt;

/// A result from this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// What went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The XML could not be parsed.
    Malformed {
        /// The parser's description of the problem.
        message: String,
    },
    /// A required element was absent or held an unusable value.
    MissingField(&'static str),
    /// The bank answered with a return code other than `000000`.
    Rejected {
        /// The technical return code.
        technical: String,
        /// The business return code, if the response carried one.
        business: Option<String>,
    },
    /// A signature or a digest did not match.
    ///
    /// This is the error that matters most: it means the message was altered, or
    /// the key is not the one the bank holds.
    BadSignature,
    /// The response was encrypted but this client holds no transaction key for
    /// it, or the key it holds did not decrypt the data.
    BadTransactionKey,
    /// The system clock is before the Unix epoch, so no timestamp can be made.
    Clock,
    /// A value was the right shape but outside what the protocol allows.
    Invalid(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { message } => write!(formatter, "malformed XML: {message}"),
            Self::MissingField(name) => write!(formatter, "the response has no {name}"),
            Self::Rejected {
                technical,
                business,
            } => match business {
                Some(business) => write!(
                    formatter,
                    "the bank refused the request: technical {technical}, business {business}"
                ),
                None => write!(
                    formatter,
                    "the bank refused the request: technical {technical}"
                ),
            },
            Self::BadSignature => formatter.write_str("a signature or digest did not match"),
            Self::BadTransactionKey => {
                formatter.write_str("the transaction key did not decrypt the order data")
            }
            Self::Clock => formatter.write_str("the system clock is before 1970"),
            Self::Invalid(what) => write!(formatter, "invalid {what}"),
        }
    }
}

impl std::error::Error for Error {}

/// A failed signature check is the same event whether it came from the padding,
/// the length or the comparison, so every RSA failure maps to one variant. The
/// distinction is deliberately lost: a caller must not learn *why* a signature
/// failed, only that it did.
impl From<crate::rsa::Error> for Error {
    fn from(_: crate::rsa::Error) -> Self {
        Self::BadSignature
    }
}

impl From<xmlkit::Error> for Error {
    fn from(error: xmlkit::Error) -> Self {
        Self::Malformed {
            message: error.message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn rejection_mentions_both_codes() {
        let error = Error::Rejected {
            technical: "011000".to_owned(),
            business: Some("090005".to_owned()),
        };
        let text = error.to_string();
        assert!(text.contains("011000"), "got {text}");
        assert!(text.contains("090005"), "got {text}");
    }

    #[test]
    fn a_rejection_without_a_business_code_still_reads_well() {
        let error = Error::Rejected {
            technical: "061002".to_owned(),
            business: None,
        };
        assert!(error.to_string().contains("061002"));
    }

    #[test]
    fn a_parse_error_converts() {
        let error = xmlkit::parse("<a><b></a>").expect_err("unbalanced");
        let converted: Error = error.into();
        assert!(matches!(converted, Error::Malformed { .. }));
    }
}
