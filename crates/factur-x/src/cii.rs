//! Reading and writing the UN/CEFACT Cross Industry Invoice syntax.
//!
//! CII is the syntax Factur-X and ZUGFeRD use. Element names are long because
//! they come from the UN/CEFACT core components, and every element sits in one
//! of four namespaces: `rsm` for the document, `ram` for the business
//! information entities, `udt` for the data types and `qdt` for the qualified
//! ones.

use xmlkit::{Element, local_name};

use crate::amount::{Amount, Currency};
use crate::model::{Address, Contact, Invoice, Line, Note, Party, TaxBreakdown, Totals};
use crate::{Error, Result};

/// The namespace of the document element.
pub const RSM: &str = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
/// The namespace of the business information entities.
pub const RAM: &str =
    "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100";
/// The namespace of the unqualified data types.
pub const UDT: &str = "urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100";
/// The namespace of the qualified data types.
pub const QDT: &str = "urn:un:unece:uncefact:data:standard:QualifiedDataType:100";

/// The specification identifier of the base EN 16931 rules.
pub const EN16931: &str = "urn:cen.eu:en16931:2017";

/// Builds the CII document for an invoice.
#[must_use]
pub fn generate(invoice: &Invoice) -> Element {
    let mut root = Element::new("rsm:CrossIndustryInvoice");
    root.set_attr("xmlns:rsm", RSM);
    root.set_attr("xmlns:ram", RAM);
    root.set_attr("xmlns:udt", UDT);
    root.set_attr("xmlns:qdt", QDT);

    root.push(context(invoice));
    root.push(document(invoice));

    let mut transaction = Element::new("rsm:SupplyChainTradeTransaction");
    for line in &invoice.lines {
        transaction.push(line_element(line));
    }
    transaction.push(agreement(invoice));
    transaction.push(delivery());
    transaction.push(settlement(invoice));
    root.push(transaction);

    root
}

/// `rsm:ExchangedDocumentContext`.
fn context(invoice: &Invoice) -> Element {
    let mut element = Element::new("rsm:ExchangedDocumentContext");
    if let Some(process) = &invoice.business_process {
        let mut parameter = Element::new("ram:BusinessProcessSpecifiedDocumentContextParameter");
        parameter.push_text_element("ram:ID", process);
        element.push(parameter);
    }
    let mut guideline = Element::new("ram:GuidelineSpecifiedDocumentContextParameter");
    guideline.push_text_element("ram:ID", &invoice.guideline);
    element.push(guideline);
    element
}

/// `rsm:ExchangedDocument`.
fn document(invoice: &Invoice) -> Element {
    let mut element = Element::new("rsm:ExchangedDocument");
    element.push_text_element("ram:ID", &invoice.number);
    element.push_text_element("ram:TypeCode", &invoice.type_code);

    let mut issue = Element::new("ram:IssueDateTime");
    issue.push_text_element_with_attr(
        "udt:DateTimeString",
        "format",
        "102",
        &compact_date(&invoice.issue_date),
    );
    element.push(issue);

    for note in &invoice.notes {
        element.push(note_element(note));
    }
    element
}

/// `ram:IncludedNote`.
fn note_element(note: &Note) -> Element {
    let mut element = Element::new("ram:IncludedNote");
    element.push_text_element("ram:Content", &note.text);
    if let Some(subject) = &note.subject_code {
        element.push_text_element("ram:SubjectCode", subject);
    }
    element
}

/// `ram:IncludedSupplyChainTradeLineItem`.
fn line_element(line: &Line) -> Element {
    let mut element = Element::new("ram:IncludedSupplyChainTradeLineItem");

    let mut document = Element::new("ram:AssociatedDocumentLineDocument");
    document.push_text_element("ram:LineID", &line.id);
    element.push(document);

    let mut product = Element::new("ram:SpecifiedTradeProduct");
    product.push_text_element("ram:Name", &line.name);
    if let Some(description) = &line.description {
        product.push_text_element("ram:Description", description);
    }
    element.push(product);

    let mut agreement = Element::new("ram:SpecifiedLineTradeAgreement");
    let mut price = Element::new("ram:NetPriceProductTradePrice");
    price.push_text_element("ram:ChargeAmount", line.unit_price.as_str());
    agreement.push(price);
    element.push(agreement);

    let mut delivery = Element::new("ram:SpecifiedLineTradeDelivery");
    let mut quantity = Element::new("ram:BilledQuantity");
    quantity.set_attr("unitCode", &line.unit);
    quantity.push_text(&line.quantity);
    delivery.push(quantity);
    element.push(delivery);

    let mut settlement = Element::new("ram:SpecifiedLineTradeSettlement");
    let mut tax = Element::new("ram:ApplicableTradeTax");
    tax.push_text_element("ram:TypeCode", "VAT");
    tax.push_text_element("ram:CategoryCode", &line.vat_category);
    if let Some(rate) = &line.vat_rate {
        tax.push_text_element("ram:RateApplicablePercent", rate);
    }
    settlement.push(tax);

    let mut summation = Element::new("ram:SpecifiedTradeSettlementLineMonetarySummation");
    summation.push_text_element("ram:LineTotalAmount", line.line_total.as_str());
    settlement.push(summation);
    element.push(settlement);

    element
}

/// `ram:ApplicableHeaderTradeAgreement`.
fn agreement(invoice: &Invoice) -> Element {
    let mut element = Element::new("ram:ApplicableHeaderTradeAgreement");
    if let Some(reference) = &invoice.buyer_reference {
        element.push_text_element("ram:BuyerReference", reference);
    }
    element.push(party_element("ram:SellerTradeParty", &invoice.seller));
    element.push(party_element("ram:BuyerTradeParty", &invoice.buyer));
    if let Some(order) = &invoice.purchase_order {
        let mut document = Element::new("ram:BuyerOrderReferencedDocument");
        document.push_text_element("ram:IssuerAssignedID", order);
        element.push(document);
    }
    if let Some(contract) = &invoice.contract_reference {
        let mut document = Element::new("ram:ContractReferencedDocument");
        document.push_text_element("ram:IssuerAssignedID", contract);
        element.push(document);
    }
    element
}

/// `ram:ApplicableHeaderTradeDelivery`, which EN 16931 allows to be empty.
fn delivery() -> Element {
    let mut element = Element::new("ram:ApplicableHeaderTradeDelivery");
    element.set_attr("xsi:nil", "true");
    element
}

/// `ram:ApplicableHeaderTradeSettlement`.
fn settlement(invoice: &Invoice) -> Element {
    let mut element = Element::new("ram:ApplicableHeaderTradeSettlement");
    element.push_text_element("ram:InvoiceCurrencyCode", invoice.currency.as_str());
    if let Some(tax_currency) = &invoice.tax_currency {
        element.push_text_element("ram:TaxCurrencyCode", tax_currency.as_str());
    }

    for entry in &invoice.tax_breakdown {
        let mut tax = Element::new("ram:ApplicableTradeTax");
        tax.push_text_element("ram:CalculatedAmount", entry.tax_amount.as_str());
        tax.push_text_element("ram:TypeCode", "VAT");
        if let Some(reason) = &entry.exemption_reason {
            tax.push_text_element("ram:ExemptionReason", reason);
        }
        tax.push_text_element("ram:BasisAmount", entry.taxable_amount.as_str());
        tax.push_text_element("ram:CategoryCode", &entry.category);
        if let Some(rate) = &entry.rate {
            tax.push_text_element("ram:RateApplicablePercent", rate);
        }
        element.push(tax);
    }

    if invoice.due_date.is_some() || invoice.payment_terms.is_some() {
        let mut terms = Element::new("ram:SpecifiedTradePaymentTerms");
        if let Some(text) = &invoice.payment_terms {
            terms.push_text_element("ram:Description", text);
        }
        if let Some(due) = &invoice.due_date {
            let mut date = Element::new("ram:DueDateDateTime");
            date.push_text_element_with_attr(
                "udt:DateTimeString",
                "format",
                "102",
                &compact_date(due),
            );
            terms.push(date);
        }
        element.push(terms);
    }

    element.push(totals_element(&invoice.totals, invoice.currency.as_str()));
    element
}

/// `ram:SpecifiedTradeSettlementHeaderMonetarySummation`.
fn totals_element(totals: &Totals, currency: &str) -> Element {
    let mut element = Element::new("ram:SpecifiedTradeSettlementHeaderMonetarySummation");
    element.push_text_element("ram:LineTotalAmount", totals.line_total.as_str());
    element.push_text_element("ram:TaxBasisTotalAmount", totals.tax_exclusive.as_str());
    element.push_text_element_with_attr(
        "ram:TaxTotalAmount",
        "currencyID",
        currency,
        totals.tax_total.as_str(),
    );
    element.push_text_element("ram:GrandTotalAmount", totals.tax_inclusive.as_str());
    if let Some(prepaid) = &totals.prepaid {
        element.push_text_element("ram:TotalPrepaidAmount", prepaid.as_str());
    }
    element.push_text_element("ram:DuePayableAmount", totals.due_payable.as_str());
    element
}

/// A `ram:SellerTradeParty` or `ram:BuyerTradeParty`.
fn party_element(name: &str, party: &Party) -> Element {
    let mut element = Element::new(name);
    element.push_text_element("ram:Name", &party.name);

    if let Some(value) = &party.legal_identifier {
        let mut organisation = Element::new("ram:SpecifiedLegalOrganization");
        let mut id = Element::new("ram:ID");
        if let Some(scheme) = &party.legal_identifier_scheme {
            id.set_attr("schemeID", scheme);
        }
        id.push_text(value);
        organisation.push(id);
        element.push(organisation);
    }

    if let Some(contact) = &party.contact {
        element.push(contact_element(contact));
    }

    element.push(address_element(&party.address));

    if let Some(vat) = &party.vat_identifier {
        element.push(registration_element("VA", vat));
    }
    if let Some(tax) = &party.tax_identifier {
        element.push(registration_element("FC", tax));
    }
    element
}

/// `ram:DefinedTradeContact`.
fn contact_element(contact: &Contact) -> Element {
    let mut element = Element::new("ram:DefinedTradeContact");
    if let Some(name) = &contact.name {
        element.push_text_element("ram:PersonName", name);
    }
    if let Some(phone) = &contact.phone {
        let mut communication = Element::new("ram:TelephoneUniversalCommunication");
        communication.push_text_element("ram:CompleteNumber", phone);
        element.push(communication);
    }
    if let Some(email) = &contact.email {
        let mut communication = Element::new("ram:EmailURIUniversalCommunication");
        communication.push_text_element("ram:URIID", email);
        element.push(communication);
    }
    element
}

/// `ram:PostalTradeAddress`.
fn address_element(address: &Address) -> Element {
    let mut element = Element::new("ram:PostalTradeAddress");
    if let Some(post_code) = &address.post_code {
        element.push_text_element("ram:PostcodeCode", post_code);
    }
    if let Some(line1) = &address.line1 {
        element.push_text_element("ram:LineOne", line1);
    }
    if let Some(line2) = &address.line2 {
        element.push_text_element("ram:LineTwo", line2);
    }
    if let Some(city) = &address.city {
        element.push_text_element("ram:CityName", city);
    }
    element.push_text_element("ram:CountryID", &address.country);
    if let Some(subdivision) = &address.subdivision {
        element.push_text_element("ram:CountrySubDivisionName", subdivision);
    }
    element
}

/// `ram:SpecifiedTaxRegistration`.
fn registration_element(scheme: &str, value: &str) -> Element {
    let mut element = Element::new("ram:SpecifiedTaxRegistration");
    let mut id = Element::new("ram:ID");
    id.set_attr("schemeID", scheme);
    id.push_text(value);
    element.push(id);
    element
}

/// Turns `YYYY-MM-DD` into the `YYYYMMDD` that the `102` date format means.
fn compact_date(date: &str) -> String {
    date.chars().filter(char::is_ascii_digit).collect()
}

/// Reads an invoice from a CII document.
///
/// # Errors
///
/// Returns an error if a required element is missing or malformed.
pub fn parse(root: &Element) -> Result<Invoice> {
    let document = child(root, "ExchangedDocument").ok_or_else(|| missing("ExchangedDocument"))?;
    let transaction = child(root, "SupplyChainTradeTransaction")
        .ok_or_else(|| missing("SupplyChainTradeTransaction"))?;

    let context = child(root, "ExchangedDocumentContext");
    let guideline = context
        .and_then(|c| child(c, "GuidelineSpecifiedDocumentContextParameter"))
        .and_then(|p| child(p, "ID"))
        .map(Element::text)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| missing("GuidelineSpecifiedDocumentContextParameter"))?;
    let business_process = context
        .and_then(|c| child(c, "BusinessProcessSpecifiedDocumentContextParameter"))
        .and_then(|p| child(p, "ID"))
        .map(Element::text)
        .filter(|t| !t.is_empty());

    let number = text_of(document, "ID")?;
    let type_code = text_of(document, "TypeCode")?;
    let issue_date = child(document, "IssueDateTime")
        .and_then(|d| child(d, "DateTimeString"))
        .map(Element::text)
        .map(|t| expand_date(&t))
        .ok_or_else(|| missing("IssueDateTime"))?;

    // The note's text lives in a `ram:Content` element rather than directly in
    // the note, so `Element::text` would come back empty.
    let mut notes = Vec::new();
    for note in children(document, "IncludedNote") {
        notes.push(Note {
            text: text_of(note, "Content")?,
            subject_code: optional_text(note, "SubjectCode"),
        });
    }

    let agreement = child(transaction, "ApplicableHeaderTradeAgreement")
        .ok_or_else(|| missing("ApplicableHeaderTradeAgreement"))?;
    let settlement = child(transaction, "ApplicableHeaderTradeSettlement")
        .ok_or_else(|| missing("ApplicableHeaderTradeSettlement"))?;

    let currency = Currency::parse(&text_of(settlement, "InvoiceCurrencyCode")?)?;
    let tax_currency = optional_text(settlement, "TaxCurrencyCode")
        .map(|c| Currency::parse(&c))
        .transpose()?;

    let seller = party_from(
        child(agreement, "SellerTradeParty").ok_or_else(|| missing("SellerTradeParty"))?,
    )?;
    let buyer =
        party_from(child(agreement, "BuyerTradeParty").ok_or_else(|| missing("BuyerTradeParty"))?)?;

    let mut lines = Vec::new();
    for item in children(transaction, "IncludedSupplyChainTradeLineItem") {
        lines.push(line_from(item)?);
    }

    let mut tax_breakdown = Vec::new();
    for tax in children(settlement, "ApplicableTradeTax") {
        tax_breakdown.push(TaxBreakdown {
            taxable_amount: amount_of(tax, "BasisAmount")?,
            tax_amount: amount_of(tax, "CalculatedAmount")?,
            category: text_of(tax, "CategoryCode")?,
            rate: optional_text(tax, "RateApplicablePercent"),
            exemption_reason: optional_text(tax, "ExemptionReason"),
        });
    }

    let totals_element = child(
        settlement,
        "SpecifiedTradeSettlementHeaderMonetarySummation",
    )
    .ok_or_else(|| missing("SpecifiedTradeSettlementHeaderMonetarySummation"))?;
    let totals = Totals {
        line_total: amount_of(totals_element, "LineTotalAmount")?,
        tax_exclusive: amount_of(totals_element, "TaxBasisTotalAmount")?,
        tax_total: amount_of(totals_element, "TaxTotalAmount")?,
        tax_inclusive: amount_of(totals_element, "GrandTotalAmount")?,
        prepaid: optional_amount(totals_element, "TotalPrepaidAmount")?,
        due_payable: amount_of(totals_element, "DuePayableAmount")?,
    };

    let terms = child(settlement, "SpecifiedTradePaymentTerms");
    let payment_terms = terms.and_then(|t| optional_text(t, "Description"));
    let due_date = terms
        .and_then(|t| child(t, "DueDateDateTime"))
        .and_then(|d| child(d, "DateTimeString"))
        .map(Element::text)
        .map(|t| expand_date(&t));

    Ok(Invoice {
        number,
        issue_date,
        type_code,
        currency,
        tax_currency,
        guideline,
        business_process,
        due_date,
        buyer_reference: optional_text(agreement, "BuyerReference"),
        purchase_order: child(agreement, "BuyerOrderReferencedDocument")
            .and_then(|d| optional_text(d, "IssuerAssignedID")),
        contract_reference: child(agreement, "ContractReferencedDocument")
            .and_then(|d| optional_text(d, "IssuerAssignedID")),
        payment_terms,
        notes,
        seller,
        buyer,
        lines,
        tax_breakdown,
        totals,
    })
}

/// Reads a line.
fn line_from(item: &Element) -> Result<Line> {
    let document = child(item, "AssociatedDocumentLineDocument")
        .ok_or_else(|| missing("AssociatedDocumentLineDocument"))?;
    let product =
        child(item, "SpecifiedTradeProduct").ok_or_else(|| missing("SpecifiedTradeProduct"))?;
    let agreement = child(item, "SpecifiedLineTradeAgreement")
        .ok_or_else(|| missing("SpecifiedLineTradeAgreement"))?;
    let price = child(agreement, "NetPriceProductTradePrice")
        .ok_or_else(|| missing("NetPriceProductTradePrice"))?;
    let delivery = child(item, "SpecifiedLineTradeDelivery")
        .ok_or_else(|| missing("SpecifiedLineTradeDelivery"))?;
    let settlement = child(item, "SpecifiedLineTradeSettlement")
        .ok_or_else(|| missing("SpecifiedLineTradeSettlement"))?;
    let tax =
        child(settlement, "ApplicableTradeTax").ok_or_else(|| missing("ApplicableTradeTax"))?;
    let summation = child(settlement, "SpecifiedTradeSettlementLineMonetarySummation")
        .ok_or_else(|| missing("SpecifiedTradeSettlementLineMonetarySummation"))?;

    let quantity_element =
        child(delivery, "BilledQuantity").ok_or_else(|| missing("BilledQuantity"))?;
    let unit = quantity_element
        .attr("unitCode")
        .unwrap_or_default()
        .to_string();

    Ok(Line {
        id: text_of(document, "LineID")?,
        name: text_of(product, "Name")?,
        description: optional_text(product, "Description"),
        quantity: quantity_element.text(),
        unit,
        unit_price: amount_of(price, "ChargeAmount")?,
        line_total: amount_of(summation, "LineTotalAmount")?,
        vat_category: text_of(tax, "CategoryCode")?,
        vat_rate: optional_text(tax, "RateApplicablePercent"),
    })
}

/// Reads a party.
fn party_from(element: &Element) -> Result<Party> {
    let address_element = child(element, "PostalTradeAddress");
    let address = Address {
        country: address_element
            .and_then(|a| optional_text(a, "CountryID"))
            .ok_or_else(|| missing("CountryID"))?,
        line1: address_element.and_then(|a| optional_text(a, "LineOne")),
        line2: address_element.and_then(|a| optional_text(a, "LineTwo")),
        city: address_element.and_then(|a| optional_text(a, "CityName")),
        post_code: address_element.and_then(|a| optional_text(a, "PostcodeCode")),
        subdivision: address_element.and_then(|a| optional_text(a, "CountrySubDivisionName")),
    };

    // BT-30 is a single legal registration identifier, so only the first one
    // is meaningful and any further ones are ignored.
    let legal = child(element, "SpecifiedLegalOrganization")
        .and_then(|organisation| child(organisation, "ID"));

    let mut vat_identifier = None;
    let mut tax_identifier = None;
    for registration in children(element, "SpecifiedTaxRegistration") {
        let Some(id) = child(registration, "ID") else {
            continue;
        };
        match id.attr("schemeID") {
            Some("VA") => vat_identifier = Some(id.text()),
            _ => tax_identifier = Some(id.text()),
        }
    }

    let contact = child(element, "DefinedTradeContact").map(|c| Contact {
        name: optional_text(c, "PersonName"),
        phone: child(c, "TelephoneUniversalCommunication")
            .and_then(|t| optional_text(t, "CompleteNumber")),
        email: child(c, "EmailURIUniversalCommunication").and_then(|t| optional_text(t, "URIID")),
    });

    Ok(Party {
        name: text_of(element, "Name")?,
        vat_identifier,
        tax_identifier,
        legal_identifier: legal.map(Element::text),
        legal_identifier_scheme: legal.and_then(|id| id.attr("schemeID")).map(str::to_string),
        address,
        contact,
    })
}

/// Turns `YYYYMMDD` into `YYYY-MM-DD`, leaving anything else alone.
fn expand_date(text: &str) -> String {
    if text.len() == 8 && text.bytes().all(|b| b.is_ascii_digit()) {
        format!("{}-{}-{}", &text[..4], &text[4..6], &text[6..])
    } else {
        text.to_string()
    }
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
fn amount_of(element: &Element, name: &str) -> Result<Amount> {
    Amount::parse(&text_of(element, name)?)
}

/// Reads an optional amount.
fn optional_amount(element: &Element, name: &str) -> Result<Option<Amount>> {
    optional_text(element, name)
        .map(|text| Amount::parse(&text))
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
    fn the_document_element_carries_the_namespaces() {
        let root = generate(&sample_invoice());
        assert_eq!(root.name, "rsm:CrossIndustryInvoice");
        assert_eq!(root.attr("xmlns:ram"), Some(RAM));
    }

    #[test]
    fn dates_are_compacted_and_expanded() {
        assert_eq!(compact_date("2026-10-21"), "20261021");
        assert_eq!(expand_date("20261021"), "2026-10-21");
        assert_eq!(expand_date("not-a-date"), "not-a-date");
    }

    #[test]
    fn a_generated_document_parses_back_to_the_same_invoice() {
        let invoice = sample_invoice();
        let root = generate(&invoice);
        let text = root.to_string();
        let parsed = xmlkit::parse(&text).unwrap();
        let back = parse(&parsed).unwrap();
        assert_eq!(back, invoice);
    }

    #[test]
    fn a_missing_document_element_is_reported() {
        let root = Element::new("rsm:CrossIndustryInvoice");
        let error = parse(&root).unwrap_err();
        assert!(matches!(error, Error::MissingElement { .. }), "{error:?}");
    }
}
