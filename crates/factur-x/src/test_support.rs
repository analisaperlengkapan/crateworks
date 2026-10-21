//! Shared test data.
//!
//! One invoice is used by the unit tests, the round-trip tests and the fixture
//! comparison, so a change to the model surfaces everywhere at once.

use crate::amount::{Amount, Currency};
use crate::model::{Address, Contact, Invoice, Line, Note, Party, TaxBreakdown, Totals};

/// A conforming invoice with one line at the German standard rate.
#[must_use]
pub fn sample_invoice() -> Invoice {
    Invoice {
        number: "INV-2026-0001".to_string(),
        issue_date: "2026-10-21".to_string(),
        type_code: "380".to_string(),
        currency: Currency::parse("EUR").unwrap(),
        tax_currency: None,
        guideline: "urn:cen.eu:en16931:2017".to_string(),
        business_process: None,
        due_date: None,
        buyer_reference: None,
        purchase_order: None,
        contract_reference: None,
        payment_terms: None,
        notes: Vec::new(),
        seller: Party {
            name: "Seller GmbH".to_string(),
            vat_identifier: Some("DE123456789".to_string()),
            tax_identifier: None,
            legal_identifier: None,
            legal_identifier_scheme: None,
            address: Address {
                country: "DE".to_string(),
                line1: Some("Hauptstrasse 1".to_string()),
                line2: None,
                city: Some("Berlin".to_string()),
                post_code: Some("10115".to_string()),
                subdivision: None,
            },
            contact: None,
        },
        buyer: Party {
            name: "Buyer AG".to_string(),
            vat_identifier: None,
            tax_identifier: None,
            legal_identifier: None,
            legal_identifier_scheme: None,
            address: Address {
                country: "DE".to_string(),
                line1: Some("Bahnhofstrasse 2".to_string()),
                line2: None,
                city: Some("Muenchen".to_string()),
                post_code: Some("80331".to_string()),
                subdivision: None,
            },
            contact: None,
        },
        lines: vec![Line {
            id: "1".to_string(),
            name: "Widget".to_string(),
            description: None,
            quantity: "10".to_string(),
            unit: "HUR".to_string(),
            unit_price: Amount::parse("100.00").unwrap(),
            line_total: Amount::parse("1000.00").unwrap(),
            vat_category: "S".to_string(),
            vat_rate: Some("19.00".to_string()),
        }],
        tax_breakdown: vec![TaxBreakdown {
            taxable_amount: Amount::parse("1000.00").unwrap(),
            tax_amount: Amount::parse("190.00").unwrap(),
            category: "S".to_string(),
            rate: Some("19.00".to_string()),
            exemption_reason: None,
        }],
        totals: Totals {
            line_total: Amount::parse("1000.00").unwrap(),
            tax_exclusive: Amount::parse("1000.00").unwrap(),
            tax_total: Amount::parse("190.00").unwrap(),
            tax_inclusive: Amount::parse("1190.00").unwrap(),
            prepaid: None,
            due_payable: Amount::parse("1190.00").unwrap(),
        },
    }
}

/// An invoice that exercises every optional part of the model.
#[must_use]
pub fn rich_invoice() -> Invoice {
    let mut invoice = sample_invoice();
    invoice.number = "INV-2026-0002".to_string();
    invoice.business_process = Some("A1".to_string());
    invoice.due_date = Some("2026-11-20".to_string());
    invoice.buyer_reference = Some("PO-77".to_string());
    invoice.purchase_order = Some("ORDER-9".to_string());
    invoice.payment_terms = Some("Payable within 30 days".to_string());
    invoice.notes = vec![Note {
        text: "Thanks for your business".to_string(),
        subject_code: Some("AAI".to_string()),
    }];
    invoice.seller.address.line2 = Some("2nd floor".to_string());
    invoice.seller.address.subdivision = Some("BE".to_string());
    invoice.seller.legal_identifier = Some("HRB12345".to_string());
    invoice.seller.legal_identifier_scheme = Some("FC".to_string());
    invoice.seller.contact = Some(Contact {
        name: Some("Anna".to_string()),
        phone: Some("+4930123".to_string()),
        email: Some("a@example.com".to_string()),
    });
    invoice.lines[0].description = Some("A fine widget".to_string());
    invoice.totals.prepaid = Some(Amount::parse("50.00").unwrap());
    invoice.totals.due_payable = Amount::parse("1140.00").unwrap();
    invoice
}
