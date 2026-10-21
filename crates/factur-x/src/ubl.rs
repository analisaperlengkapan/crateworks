//! Reading and writing the OASIS Universal Business Language syntax.
//!
//! UBL is the syntax used by Peppol and by most national e-invoicing mandates
//! outside France and Germany. It carries the same EN 16931 information as CII
//! under completely different element names, which is why the model records the
//! standard's BT numbers rather than either syntax's names.

use xmlkit::{Element, local_name};

use crate::amount::Currency;
use crate::model::{Address, Contact, Invoice, Line, Note, Party, TaxBreakdown, Totals};
use crate::{Error, Result};

/// The namespace of the invoice document.
pub const INVOICE: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
/// The namespace of the basic components.
pub const CBC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";
/// The namespace of the aggregate components.
pub const CAC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2";

/// The UBL version this crate writes.
pub const UBL_VERSION: &str = "2.1";

/// Builds the UBL document for an invoice.
#[must_use]
pub fn generate(invoice: &Invoice) -> Element {
    let mut root = Element::new("Invoice");
    root.set_attr("xmlns", INVOICE);
    root.set_attr("xmlns:cbc", CBC);
    root.set_attr("xmlns:cac", CAC);

    root.push_text_element("cbc:UBLVersionID", UBL_VERSION);
    root.push_text_element("cbc:CustomizationID", &invoice.guideline);
    if let Some(process) = &invoice.business_process {
        root.push_text_element("cbc:ProfileID", process);
    }
    root.push_text_element("cbc:ID", &invoice.number);
    root.push_text_element("cbc:IssueDate", &invoice.issue_date);
    if let Some(due) = &invoice.due_date {
        root.push_text_element("cbc:DueDate", due);
    }
    root.push_text_element("cbc:InvoiceTypeCode", &invoice.type_code);
    for note in &invoice.notes {
        root.push_text_element("cbc:Note", &encode_note(note));
    }
    root.push_text_element("cbc:DocumentCurrencyCode", invoice.currency.as_str());
    if let Some(tax_currency) = &invoice.tax_currency {
        root.push_text_element("cbc:TaxCurrencyCode", tax_currency.as_str());
    }
    if let Some(reference) = &invoice.buyer_reference {
        root.push_text_element("cbc:BuyerReference", reference);
    }
    if let Some(order) = &invoice.purchase_order {
        let mut element = Element::new("cac:OrderReference");
        element.push_text_element("cbc:ID", order);
        root.push(element);
    }
    if let Some(contract) = &invoice.contract_reference {
        let mut element = Element::new("cac:ContractDocumentReference");
        element.push_text_element("cbc:ID", contract);
        root.push(element);
    }

    let mut supplier = Element::new("cac:AccountingSupplierParty");
    supplier.push(party_element(&invoice.seller));
    root.push(supplier);

    let mut customer = Element::new("cac:AccountingCustomerParty");
    customer.push(party_element(&invoice.buyer));
    root.push(customer);

    if let Some(terms) = &invoice.payment_terms {
        let mut element = Element::new("cac:PaymentTerms");
        element.push_text_element("cbc:Note", terms);
        root.push(element);
    }

    root.push(tax_total_element(invoice));
    root.push(totals_element(&invoice.totals, invoice.currency.as_str()));

    for line in &invoice.lines {
        root.push(line_element(line, invoice.currency.as_str()));
    }

    root
}

/// A note's subject code is carried inside the text, wrapped in hash marks.
fn encode_note(note: &Note) -> String {
    match &note.subject_code {
        Some(subject) => format!("#{subject}#{}", note.text),
        None => note.text.clone(),
    }
}

/// Reads a note back, splitting off a leading subject code if there is one.
fn decode_note(text: &str) -> Note {
    if let Some(rest) = text.strip_prefix('#') {
        if let Some((subject, body)) = rest.split_once('#') {
            return Note {
                text: body.to_string(),
                subject_code: Some(subject.to_string()),
            };
        }
    }
    Note {
        text: text.to_string(),
        subject_code: None,
    }
}

/// `cac:Party`.
fn party_element(party: &Party) -> Element {
    let mut element = Element::new("cac:Party");
    element.push(address_element(&party.address));

    if let Some(vat) = &party.vat_identifier {
        element.push(tax_scheme_element(vat, "VAT"));
    }
    if let Some(tax) = &party.tax_identifier {
        element.push(tax_scheme_element(tax, "FC"));
    }

    let mut legal = Element::new("cac:PartyLegalEntity");
    legal.push_text_element("cbc:RegistrationName", &party.name);
    if let Some(identifier) = &party.legal_identifier {
        let mut id = Element::new("cbc:CompanyID");
        if let Some(scheme) = &party.legal_identifier_scheme {
            id.set_attr("schemeID", scheme);
        }
        id.push_text(identifier);
        legal.push(id);
    }
    element.push(legal);

    if let Some(contact) = &party.contact {
        element.push(contact_element(contact));
    }

    element
}

/// `cac:PostalAddress`.
fn address_element(address: &Address) -> Element {
    let mut element = Element::new("cac:PostalAddress");
    if let Some(line1) = &address.line1 {
        element.push_text_element("cbc:StreetName", line1);
    }
    if let Some(line2) = &address.line2 {
        element.push_text_element("cbc:AdditionalStreetName", line2);
    }
    if let Some(city) = &address.city {
        element.push_text_element("cbc:CityName", city);
    }
    if let Some(post_code) = &address.post_code {
        element.push_text_element("cbc:PostalZone", post_code);
    }
    if let Some(subdivision) = &address.subdivision {
        element.push_text_element("cbc:CountrySubentity", subdivision);
    }
    let mut country = Element::new("cac:Country");
    country.push_text_element("cbc:IdentificationCode", &address.country);
    element.push(country);
    element
}

/// `cac:Contact`.
fn contact_element(contact: &Contact) -> Element {
    let mut element = Element::new("cac:Contact");
    if let Some(name) = &contact.name {
        element.push_text_element("cbc:Name", name);
    }
    if let Some(phone) = &contact.phone {
        element.push_text_element("cbc:Telephone", phone);
    }
    if let Some(email) = &contact.email {
        element.push_text_element("cbc:ElectronicMail", email);
    }
    element
}

/// `cac:PartyTaxScheme`.
fn tax_scheme_element(identifier: &str, scheme: &str) -> Element {
    let mut element = Element::new("cac:PartyTaxScheme");
    element.push_text_element("cbc:CompanyID", identifier);
    let mut tax_scheme = Element::new("cac:TaxScheme");
    tax_scheme.push_text_element("cbc:ID", scheme);
    element.push(tax_scheme);
    element
}

/// `cac:TaxTotal`.
fn tax_total_element(invoice: &Invoice) -> Element {
    let mut element = Element::new("cac:TaxTotal");
    element.push_text_element_with_attr(
        "cbc:TaxAmount",
        "currencyID",
        invoice.currency.as_str(),
        invoice.totals.tax_total.as_str(),
    );
    for entry in &invoice.tax_breakdown {
        element.push(tax_subtotal_element(entry, invoice.currency.as_str()));
    }
    element
}

/// `cac:TaxSubtotal`.
fn tax_subtotal_element(entry: &TaxBreakdown, currency: &str) -> Element {
    let mut element = Element::new("cac:TaxSubtotal");
    element.push_text_element_with_attr(
        "cbc:TaxableAmount",
        "currencyID",
        currency,
        entry.taxable_amount.as_str(),
    );
    element.push_text_element_with_attr(
        "cbc:TaxAmount",
        "currencyID",
        currency,
        entry.tax_amount.as_str(),
    );
    let mut category = Element::new("cac:TaxCategory");
    category.push_text_element("cbc:ID", &entry.category);
    if let Some(rate) = &entry.rate {
        category.push_text_element("cbc:Percent", rate);
    }
    if let Some(reason) = &entry.exemption_reason {
        category.push_text_element("cbc:TaxExemptionReason", reason);
    }
    let mut scheme = Element::new("cac:TaxScheme");
    scheme.push_text_element("cbc:ID", "VAT");
    category.push(scheme);
    element.push(category);
    element
}

/// `cac:LegalMonetaryTotal`.
fn totals_element(totals: &Totals, currency: &str) -> Element {
    let mut element = Element::new("cac:LegalMonetaryTotal");
    for (name, value) in [
        ("cbc:LineExtensionAmount", &totals.line_total),
        ("cbc:TaxExclusiveAmount", &totals.tax_exclusive),
        ("cbc:TaxInclusiveAmount", &totals.tax_inclusive),
    ] {
        element.push_text_element_with_attr(name, "currencyID", currency, value.as_str());
    }
    if let Some(prepaid) = &totals.prepaid {
        element.push_text_element_with_attr(
            "cbc:PrepaidAmount",
            "currencyID",
            currency,
            prepaid.as_str(),
        );
    }
    element.push_text_element_with_attr(
        "cbc:PayableAmount",
        "currencyID",
        currency,
        totals.due_payable.as_str(),
    );
    element
}

/// `cac:InvoiceLine`.
fn line_element(line: &Line, currency: &str) -> Element {
    let mut element = Element::new("cac:InvoiceLine");
    element.push_text_element("cbc:ID", &line.id);
    element.push_text_element_with_attr(
        "cbc:InvoicedQuantity",
        "unitCode",
        &line.unit,
        &line.quantity,
    );
    element.push_text_element_with_attr(
        "cbc:LineExtensionAmount",
        "currencyID",
        currency,
        line.line_total.as_str(),
    );

    let mut item = Element::new("cac:Item");
    if let Some(description) = &line.description {
        item.push_text_element("cbc:Description", description);
    }
    item.push_text_element("cbc:Name", &line.name);
    let mut category = Element::new("cac:ClassifiedTaxCategory");
    category.push_text_element("cbc:ID", &line.vat_category);
    if let Some(rate) = &line.vat_rate {
        category.push_text_element("cbc:Percent", rate);
    }
    let mut scheme = Element::new("cac:TaxScheme");
    scheme.push_text_element("cbc:ID", "VAT");
    category.push(scheme);
    item.push(category);
    element.push(item);

    let mut price = Element::new("cac:Price");
    price.push_text_element_with_attr(
        "cbc:PriceAmount",
        "currencyID",
        currency,
        line.unit_price.as_str(),
    );
    element.push(price);

    element
}

/// Reads an invoice from a UBL document.
///
/// # Errors
///
/// Returns an error if a required element is missing or malformed.
pub fn parse(root: &Element) -> Result<Invoice> {
    let guideline = text_of(root, "CustomizationID")?;
    let business_process = optional_text(root, "ProfileID");
    let number = text_of(root, "ID")?;
    let issue_date = text_of(root, "IssueDate")?;
    let due_date = optional_text(root, "DueDate");
    let type_code = text_of(root, "InvoiceTypeCode")?;
    let currency = Currency::parse(&text_of(root, "DocumentCurrencyCode")?)?;
    let tax_currency = optional_text(root, "TaxCurrencyCode")
        .map(|c| Currency::parse(&c))
        .transpose()?;

    let notes = children(root, "Note")
        .map(|note| decode_note(&note.text()))
        .collect();

    let seller = root
        .child("cac:AccountingSupplierParty")
        .and_then(|p| child(p, "Party"))
        .ok_or_else(|| missing("AccountingSupplierParty"))?;
    let buyer = root
        .child("cac:AccountingCustomerParty")
        .and_then(|p| child(p, "Party"))
        .ok_or_else(|| missing("AccountingCustomerParty"))?;

    let mut lines = Vec::new();
    for item in children(root, "InvoiceLine") {
        lines.push(line_from(item)?);
    }

    let tax_total = root
        .child("cac:TaxTotal")
        .ok_or_else(|| missing("TaxTotal"))?;
    let mut tax_breakdown = Vec::new();
    for subtotal in children(tax_total, "TaxSubtotal") {
        let category = child(subtotal, "TaxCategory").ok_or_else(|| missing("TaxCategory"))?;
        tax_breakdown.push(TaxBreakdown {
            taxable_amount: amount_of(subtotal, "TaxableAmount")?,
            tax_amount: amount_of(subtotal, "TaxAmount")?,
            category: text_of(category, "ID")?,
            rate: optional_text(category, "Percent"),
            exemption_reason: optional_text(category, "TaxExemptionReason"),
        });
    }

    let total_element = root
        .child("cac:LegalMonetaryTotal")
        .ok_or_else(|| missing("LegalMonetaryTotal"))?;
    let totals = Totals {
        line_total: amount_of(total_element, "LineExtensionAmount")?,
        tax_exclusive: amount_of(total_element, "TaxExclusiveAmount")?,
        tax_total: amount_of(tax_total, "TaxAmount")?,
        tax_inclusive: amount_of(total_element, "TaxInclusiveAmount")?,
        prepaid: optional_amount(total_element, "PrepaidAmount")?,
        due_payable: amount_of(total_element, "PayableAmount")?,
    };

    Ok(Invoice {
        number,
        issue_date,
        type_code,
        currency,
        tax_currency,
        guideline,
        business_process,
        due_date,
        buyer_reference: optional_text(root, "BuyerReference"),
        purchase_order: root
            .child("cac:OrderReference")
            .and_then(|o| optional_text(o, "ID")),
        contract_reference: root
            .child("cac:ContractDocumentReference")
            .and_then(|o| optional_text(o, "ID")),
        payment_terms: root
            .child("cac:PaymentTerms")
            .and_then(|t| optional_text(t, "Note")),
        notes,
        seller: party_from(seller)?,
        buyer: party_from(buyer)?,
        lines,
        tax_breakdown,
        totals,
    })
}

/// Reads a line.
fn line_from(element: &Element) -> Result<Line> {
    let item = child(element, "Item").ok_or_else(|| missing("Item"))?;
    let category =
        child(item, "ClassifiedTaxCategory").ok_or_else(|| missing("ClassifiedTaxCategory"))?;
    let price = child(element, "Price").ok_or_else(|| missing("Price"))?;
    let quantity = child(element, "InvoicedQuantity").ok_or_else(|| missing("InvoicedQuantity"))?;

    Ok(Line {
        id: text_of(element, "ID")?,
        name: text_of(item, "Name")?,
        description: optional_text(item, "Description"),
        quantity: quantity.text(),
        unit: quantity.attr("unitCode").unwrap_or_default().to_string(),
        unit_price: amount_of(price, "PriceAmount")?,
        line_total: amount_of(element, "LineExtensionAmount")?,
        vat_category: text_of(category, "ID")?,
        vat_rate: optional_text(category, "Percent"),
    })
}

/// Reads a party.
fn party_from(element: &Element) -> Result<Party> {
    let postal = child(element, "PostalAddress");
    let address = Address {
        country: postal
            .and_then(|p| child(p, "Country"))
            .and_then(|c| optional_text(c, "IdentificationCode"))
            .ok_or_else(|| missing("IdentificationCode"))?,
        line1: postal.and_then(|p| optional_text(p, "StreetName")),
        line2: postal.and_then(|p| optional_text(p, "AdditionalStreetName")),
        city: postal.and_then(|p| optional_text(p, "CityName")),
        post_code: postal.and_then(|p| optional_text(p, "PostalZone")),
        subdivision: postal.and_then(|p| optional_text(p, "CountrySubentity")),
    };

    let legal = child(element, "PartyLegalEntity");
    let name = legal
        .and_then(|l| optional_text(l, "RegistrationName"))
        .or_else(|| {
            element
                .child("PartyName")
                .and_then(|n| optional_text(n, "Name"))
        })
        .ok_or_else(|| missing("RegistrationName"))?;
    let legal_id = legal.and_then(|l| child(l, "CompanyID"));

    let mut vat_identifier = None;
    let mut tax_identifier = None;
    for scheme in children(element, "PartyTaxScheme") {
        let Some(value) = child(scheme, "CompanyID").map(Element::text) else {
            continue;
        };
        let kind = child(scheme, "TaxScheme")
            .and_then(|t| optional_text(t, "ID"))
            .unwrap_or_default();
        if kind == "VAT" {
            vat_identifier = Some(value);
        } else {
            tax_identifier = Some(value);
        }
    }

    let contact = child(element, "Contact").map(|c| Contact {
        name: optional_text(c, "Name"),
        phone: optional_text(c, "Telephone"),
        email: optional_text(c, "ElectronicMail"),
    });

    Ok(Party {
        name,
        vat_identifier,
        tax_identifier,
        legal_identifier: legal_id.map(Element::text),
        legal_identifier_scheme: legal_id
            .and_then(|id| id.attr("schemeID"))
            .map(str::to_string),
        address,
        contact,
    })
}

/// Finds a direct child by local name, ignoring any namespace prefix.
fn child<'a>(element: &'a Element, name: &str) -> Option<&'a Element> {
    element.children.iter().find_map(|node| match node {
        xmlkit::Node::Element(e) if local_name(&e.name) == name => Some(e),
        _ => None,
    })
}

/// Iterates over direct children by local name, ignoring any namespace prefix.
fn children<'a>(element: &'a Element, name: &'a str) -> impl Iterator<Item = &'a Element> {
    element.children.iter().filter_map(move |node| match node {
        xmlkit::Node::Element(e) if local_name(&e.name) == name => Some(e),
        _ => None,
    })
}

/// Reads a required child's text.
fn text_of(element: &Element, name: &str) -> Result<String> {
    child(element, name)
        .map(Element::text)
        .ok_or_else(|| missing(name))
}

/// Reads an optional child's text, treating an empty element as absent.
fn optional_text(element: &Element, name: &str) -> Option<String> {
    child(element, name)
        .map(Element::text)
        .filter(|text| !text.is_empty())
}

/// Reads a required amount.
fn amount_of(element: &Element, name: &str) -> Result<crate::Amount> {
    crate::Amount::parse(&text_of(element, name)?)
}

/// Reads an optional amount.
fn optional_amount(element: &Element, name: &str) -> Result<Option<crate::Amount>> {
    optional_text(element, name)
        .map(|text| crate::Amount::parse(&text))
        .transpose()
}

fn missing(name: &str) -> Error {
    Error::MissingElement {
        name: name.to_string(),
    }
}

/// Convenience methods used by the generator above.
trait ElementExt {
    fn push_text_element(&mut self, name: &str, text: &str) -> &mut Self;
    fn push_text_element_with_attr(
        &mut self,
        name: &str,
        attribute: &str,
        value: &str,
        text: &str,
    ) -> &mut Self;
}

impl ElementExt for Element {
    fn push_text_element(&mut self, name: &str, text: &str) -> &mut Self {
        let mut element = Element::new(name);
        element.push_text(text);
        self.push(element);
        self
    }

    fn push_text_element_with_attr(
        &mut self,
        name: &str,
        attribute: &str,
        value: &str,
        text: &str,
    ) -> &mut Self {
        let mut element = Element::new(name);
        element.set_attr(attribute, value);
        element.push_text(text);
        self.push(element);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_invoice;

    #[test]
    fn the_root_element_carries_the_namespaces() {
        let root = generate(&sample_invoice());
        assert_eq!(root.name, "Invoice");
        assert_eq!(root.attr("xmlns:cac"), Some(CAC));
    }

    #[test]
    fn a_note_with_a_subject_code_is_encoded_in_one_string() {
        let note = Note {
            text: "Thanks".to_string(),
            subject_code: Some("AAI".to_string()),
        };
        assert_eq!(encode_note(&note), "#AAI#Thanks");
        assert_eq!(decode_note("#AAI#Thanks"), note);
    }

    #[test]
    fn a_note_without_a_subject_code_is_unchanged() {
        let note = Note {
            text: "Thanks".to_string(),
            subject_code: None,
        };
        assert_eq!(encode_note(&note), "Thanks");
        assert_eq!(decode_note("Thanks"), note);
        // A lone hash mark is text, not a subject code.
        assert_eq!(decode_note("#5 off").subject_code, None);
    }

    #[test]
    fn a_generated_document_parses_back_to_the_same_invoice() {
        let invoice = sample_invoice();
        let text = generate(&invoice).to_string();
        let parsed = xmlkit::parse(&text).unwrap();
        let back = parse(&parsed).unwrap();
        assert_eq!(back, invoice);
    }

    #[test]
    fn a_missing_supplier_is_reported() {
        let mut root = Element::new("Invoice");
        root.push_text_element("cbc:ID", "X");
        assert!(matches!(
            parse(&root).unwrap_err(),
            Error::MissingElement { .. }
        ));
    }
}
