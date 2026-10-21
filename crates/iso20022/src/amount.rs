//! Monetary amounts and currency codes.

use std::fmt::{self, Display};

use crate::{Error, Result};

/// A three-letter currency code.
///
/// The code is kept as written rather than restricted to a fixed list, because
/// ISO 20022 messages do carry currencies outside the common set and rejecting
/// them would lose data. [`Currency::is_well_formed`] checks the shape only.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Currency(String);

impl Currency {
    /// Wraps a currency code, upper-casing it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValue`] unless the code is three ASCII letters.
    pub fn parse(code: &str) -> Result<Self> {
        let upper = code.trim().to_uppercase();
        if upper.len() != 3 || !upper.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(Error::InvalidValue {
                element: "Ccy",
                found: code.to_string(),
            });
        }
        Ok(Self(upper))
    }

    /// The code as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the code looks like a currency. Always true for a value built by
    /// [`Currency::parse`], and useful for documenting the invariant.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        self.0.len() == 3 && self.0.chars().all(|c| c.is_ascii_alphabetic())
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An amount with its currency.
///
/// ISO 20022 stores the value as a decimal string with a scale that depends on
/// the currency, so `f64` is not exact for every case. It is used here because
/// statement reconciliation and reporting work at this precision, and the
/// original text is kept in [`Amount::text`] so an exact value is never lost.
#[derive(Debug, Clone, PartialEq)]
pub struct Amount {
    /// The numeric value.
    pub amount: f64,
    /// The currency the amount is expressed in.
    pub currency: Currency,
    /// The value exactly as it appeared in the document.
    pub text: String,
}

impl Amount {
    /// Reads an `<Amt Ccy="...">` element.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValue`] when the currency attribute is absent or
    /// malformed, or when the text is not a number.
    pub fn from_element(element: &crateworks_xml::Element) -> Result<Self> {
        let currency = element.attr("Ccy").ok_or(Error::InvalidValue {
            element: "Amt",
            found: "missing Ccy attribute".into(),
        })?;
        let currency = Currency::parse(currency)?;
        let text = element.deep_text().trim().to_string();
        let amount = text.parse::<f64>().map_err(|_| Error::InvalidValue {
            element: "Amt",
            found: text.clone(),
        })?;
        Ok(Self {
            amount,
            currency,
            text,
        })
    }

    /// Whether the amount is a credit (money in).
    #[must_use]
    pub fn is_credit(&self) -> bool {
        self.amount >= 0.0
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.text, self.currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crateworks_xml::parse;

    #[test]
    fn parses_amount_with_currency() {
        let e = parse(r#"<Amt Ccy="EUR">1250.75</Amt>"#).unwrap();
        let a = Amount::from_element(&e).unwrap();
        assert_eq!(a.amount, 1250.75);
        assert_eq!(a.currency.as_str(), "EUR");
        assert_eq!(a.text, "1250.75");
        assert!(a.is_credit());
    }

    #[test]
    fn negative_amount_is_a_debit() {
        let e = parse(r#"<Amt Ccy="USD">-40.00</Amt>"#).unwrap();
        let a = Amount::from_element(&e).unwrap();
        assert!(!a.is_credit());
        assert_eq!(a.to_string(), "-40.00 USD");
    }

    #[test]
    fn currency_code_is_validated() {
        assert!(Currency::parse("eur").is_ok());
        assert_eq!(
            Currency::parse("EURO"),
            Err(Error::InvalidValue {
                element: "Ccy",
                found: "EURO".into()
            })
        );
        assert!(Currency::parse("E1R").is_err());
        assert!(Currency::parse("EU").is_err());
    }

    #[test]
    fn missing_currency_attribute_is_an_error() {
        let e = parse("<Amt>10.00</Amt>").unwrap();
        assert!(matches!(
            Amount::from_element(&e),
            Err(Error::InvalidValue { .. })
        ));
    }

    #[test]
    fn non_numeric_text_is_an_error() {
        let e = parse(r#"<Amt Ccy="EUR">abc</Amt>"#).unwrap();
        assert!(matches!(
            Amount::from_element(&e),
            Err(Error::InvalidValue { .. })
        ));
    }

    #[test]
    fn currency_display_and_shape_check() {
        let c = Currency::parse("GBP").unwrap();
        assert_eq!(c.to_string(), "GBP");
        assert!(c.is_well_formed());
    }
}
