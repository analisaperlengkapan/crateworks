//! The invoice model.
//!
//! The types mirror EN 16931, the European standard for the semantic content of
//! an electronic invoice. Every field carries its BT or BG number in its doc
//! comment, because that number is how the standard and every implementation
//! guide refers to it. The numbers matter: a mapping table from "BT-31" to
//! "seller VAT identifier" is the only reliable way to talk about this data
//! across the CII and UBL syntaxes, which use completely different element
//! names for the same thing.
//!
//! A `BG` is a group, so a field named `BG-25` in a document is a list of
//! invoice lines rather than a single value.

use crate::amount::{Amount, Currency};

/// An invoice or credit note.
///
/// Only the fields EN 16931 marks as required are non-optional. The rest are
/// `Option` or empty collections, because a conforming invoice does not have to
/// carry them.
#[derive(Debug, Clone, PartialEq)]
pub struct Invoice {
    /// BT-1: the invoice number assigned by the seller.
    pub number: String,
    /// BT-2: the date the invoice was issued, as `YYYY-MM-DD`.
    pub issue_date: String,
    /// BT-3: the document type code. `380` is an invoice and `381` a credit
    /// note, but the field is kept as text because the list is long.
    pub type_code: String,
    /// BT-5: the currency the invoice is expressed in.
    pub currency: Currency,
    /// BT-6: the tax accounting currency, when it differs from [`Self::currency`].
    pub tax_currency: Option<Currency>,
    /// BT-24: the specification identifier, which says which rules the invoice
    /// claims to follow.
    pub guideline: String,
    /// BT-23: the business process identifier.
    pub business_process: Option<String>,
    /// BT-9: the date payment is due.
    pub due_date: Option<String>,
    /// BT-10: the buyer's reference, such as a purchase order number.
    pub buyer_reference: Option<String>,
    /// BT-13: the purchase order reference.
    pub purchase_order: Option<String>,
    /// BT-12: the contract reference.
    pub contract_reference: Option<String>,
    /// BT-20: the payment terms in free text.
    pub payment_terms: Option<String>,
    /// BG-1: free-text notes, each with an optional subject code.
    pub notes: Vec<Note>,
    /// BG-4: the seller.
    pub seller: Party,
    /// BG-7: the buyer.
    pub buyer: Party,
    /// BG-25: the invoice lines.
    pub lines: Vec<Line>,
    /// BG-23: the VAT breakdown, one entry per rate and category.
    pub tax_breakdown: Vec<TaxBreakdown>,
    /// BG-22: the document totals.
    pub totals: Totals,
}

/// BG-1: a free-text note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// BT-22: the text.
    pub text: String,
    /// BT-21: the subject code, such as `AAI` for general information.
    pub subject_code: Option<String>,
}

/// BG-4 and BG-7: a seller or a buyer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Party {
    /// BT-27 or BT-44: the name.
    pub name: String,
    /// BT-31 or BT-48: the VAT identifier, without spaces.
    pub vat_identifier: Option<String>,
    /// BT-32 or BT-49: the tax registration identifier for a country where the
    /// party has no VAT number.
    pub tax_identifier: Option<String>,
    /// BT-30 or BT-47: the legal registration identifier, such as a company
    /// register number.
    pub legal_identifier: Option<String>,
    /// The scheme of [`Self::legal_identifier`], such as `FC` for a French
    /// company register.
    pub legal_identifier_scheme: Option<String>,
    /// BG-5 or BG-8: the postal address.
    pub address: Address,
    /// BG-6 or BG-9: the contact.
    pub contact: Option<Contact>,
}

/// BG-5 and BG-8: a postal address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    /// BT-40 or BT-55: the country code, two letters.
    pub country: String,
    /// BT-35 or BT-50: the first address line.
    pub line1: Option<String>,
    /// BT-36 or BT-51: the second address line.
    pub line2: Option<String>,
    /// BT-37 or BT-52: the city.
    pub city: Option<String>,
    /// BT-38 or BT-53: the post code.
    pub post_code: Option<String>,
    /// BT-39 or BT-54: the country subdivision.
    pub subdivision: Option<String>,
}

/// BG-6 and BG-9: a contact at a party.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    /// BT-41 or BT-56: the contact's name.
    pub name: Option<String>,
    /// BT-42 or BT-57: the telephone number.
    pub phone: Option<String>,
    /// BT-43 or BT-58: the email address.
    pub email: Option<String>,
}

/// BG-25: an invoice line.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// BT-126: the line number.
    pub id: String,
    /// BT-153: the item name.
    pub name: String,
    /// BT-154: the item description.
    pub description: Option<String>,
    /// BT-129: the quantity billed.
    pub quantity: String,
    /// BT-130: the unit of measure, a UN/ECE Recommendation 20 code such as
    /// `HUR` for an hour or `C62` for one piece.
    pub unit: String,
    /// BT-146: the price per unit, excluding tax.
    pub unit_price: Amount,
    /// BT-131: the line total, excluding tax.
    pub line_total: Amount,
    /// BT-151: the VAT category code, such as `S` for standard rate.
    pub vat_category: String,
    /// BT-152: the VAT rate as a percentage.
    pub vat_rate: Option<String>,
}

/// BG-23: one entry of the VAT breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct TaxBreakdown {
    /// BT-116: the taxable amount for this rate and category.
    pub taxable_amount: Amount,
    /// BT-117: the tax amount.
    pub tax_amount: Amount,
    /// BT-118: the VAT category code.
    pub category: String,
    /// BT-119: the VAT rate as a percentage.
    pub rate: Option<String>,
    /// BT-120: the reason the category applies, when the category is not `S`.
    pub exemption_reason: Option<String>,
}

/// BG-22: the document totals.
#[derive(Debug, Clone, PartialEq)]
pub struct Totals {
    /// BT-106: the sum of all line totals.
    pub line_total: Amount,
    /// BT-109: the total excluding tax.
    pub tax_exclusive: Amount,
    /// BT-110: the total tax.
    pub tax_total: Amount,
    /// BT-112: the total including tax.
    pub tax_inclusive: Amount,
    /// BT-113: the amount already paid.
    pub prepaid: Option<Amount>,
    /// BT-115: the amount now payable.
    pub due_payable: Amount,
}

impl Invoice {
    /// The VAT category codes used, in the order they first appear.
    #[must_use]
    pub fn categories(&self) -> Vec<&str> {
        let mut seen = Vec::new();
        for entry in &self.tax_breakdown {
            if !seen.contains(&entry.category.as_str()) {
                seen.push(entry.category.as_str());
            }
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_invoice;

    #[test]
    fn the_sample_invoice_has_the_expected_shape() {
        let invoice = sample_invoice();
        assert_eq!(invoice.number, "INV-2026-0001");
        assert_eq!(invoice.lines.len(), 1);
        assert_eq!(invoice.tax_breakdown.len(), 1);
        assert_eq!(invoice.seller.address.country, "DE");
    }

    #[test]
    fn categories_are_listed_once_in_order() {
        let mut invoice = sample_invoice();
        invoice.tax_breakdown.push(TaxBreakdown {
            taxable_amount: Amount::parse("500.00").unwrap(),
            tax_amount: Amount::parse("35.00").unwrap(),
            category: "AA".to_string(),
            rate: Some("7.00".to_string()),
            exemption_reason: None,
        });
        assert_eq!(invoice.categories(), vec!["S", "AA"]);
    }
}
