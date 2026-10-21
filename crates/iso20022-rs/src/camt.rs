//! `camt` cash management messages, including bank statements.
//!
//! The main message here is `camt.053`, the end-of-day bank statement, which is
//! the ISO 20022 replacement for the SWIFT MT940 format. [`BankStatement::parse`]
//! accepts the document as a whole, so a caller can pass the file straight from
//! a bank.

use crate::amount::Amount;
use crate::{Error, Result, find, find_all, optional_text, required_text};
use xmlkit::Element;

/// Whether a statement entry increases or decreases the balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditDebit {
    /// Money in.
    Credit,
    /// Money out.
    Debit,
}

impl CreditDebit {
    /// Reads the ISO 20022 `CdtDbtInd` element.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValue`] for a value other than `CRDT` or `DBIT`.
    pub fn parse(text: &str) -> Result<Self> {
        match text.trim() {
            "CRDT" => Ok(Self::Credit),
            "DBIT" => Ok(Self::Debit),
            other => Err(Error::InvalidValue {
                element: "CdtDbtInd",
                found: other.to_string(),
            }),
        }
    }

    /// The signed multiplier this indicator applies to an amount.
    #[must_use]
    pub const fn sign(self) -> f64 {
        match self {
            Self::Credit => 1.0,
            Self::Debit => -1.0,
        }
    }
}

/// One entry in a statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Signed amount: positive for a credit, negative for a debit.
    pub amount: Amount,
    /// Whether the entry is a credit or a debit.
    pub credit_debit: CreditDebit,
    /// Bank reference, if present.
    pub reference: Option<String>,
    /// The other party's name, if present.
    pub counterparty: Option<String>,
    /// Remittance information, the free text a payer supplied.
    pub remittance: Option<String>,
    /// Value date, `YYYY-MM-DD`, if present.
    pub value_date: Option<String>,
}

impl Entry {
    /// The amount with the credit/debit sign applied.
    #[must_use]
    pub fn signed_amount(&self) -> f64 {
        self.amount.amount * self.credit_debit.sign()
    }
}

/// A balance reported on a statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Balance {
    /// ISO 20022 balance type code, such as `OPBD` or `CLBD`.
    pub kind: String,
    /// The amount.
    pub amount: Amount,
    /// Whether the balance is a credit or a debit.
    pub credit_debit: Option<CreditDebit>,
}

/// A bank statement, typically a `camt.053` document.
#[derive(Debug, Clone, PartialEq)]
pub struct BankStatement {
    /// Statement identifier.
    pub id: String,
    /// Account IBAN, if the account is identified that way.
    pub iban: Option<String>,
    /// Account holder name, if present.
    pub owner: Option<String>,
    /// The opening balance, taken from the first `OPBD` balance.
    pub opening_balance: Option<Amount>,
    /// All reported balances.
    pub balances: Vec<Balance>,
    /// Statement entries, in document order.
    pub entries: Vec<Entry>,
}

impl BankStatement {
    /// Parses a complete `camt.053` document.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] for malformed input, or [`Error::MissingElement`]
    /// when the document has no statement.
    ///
    /// # Example
    ///
    /// ```
    /// use iso20022_rs::camt::BankStatement;
    ///
    /// let xml = r#"<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.08">
    ///   <BkToCstmrStmt><Stmt><Id>STMT-1</Id>
    ///     <Acct><Id><IBAN>DE89370400440532013000</IBAN></Id></Acct>
    ///     <Bal><Amt Ccy="EUR">10.00</Amt></Bal>
    ///   </Stmt></BkToCstmrStmt>
    /// </Document>"#;
    ///
    /// let statement = BankStatement::parse(xml).unwrap();
    /// assert_eq!(statement.id, "STMT-1");
    /// assert_eq!(statement.iban.as_deref(), Some("DE89370400440532013000"));
    /// ```
    pub fn parse(xml: &str) -> Result<Self> {
        let body = crate::document_body(xml)?;
        let stmt = find(&body, "Stmt").ok_or(Error::MissingElement("Stmt"))?;
        Self::from_element(stmt)
    }

    /// Reads a `<Stmt>` element.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingElement`] if `Id` is absent, or
    /// [`Error::InvalidValue`] for malformed amounts.
    pub fn from_element(stmt: &Element) -> Result<Self> {
        let id = required_text(stmt, "Id")?;
        let iban = find(stmt, "Acct").and_then(|acct| optional_text(acct, "IBAN"));
        let owner = find(stmt, "Acct").and_then(|acct| optional_text(acct, "Nm"));

        let mut balance_elements = Vec::new();
        find_all(stmt, "Bal", &mut balance_elements);
        let mut balances = Vec::with_capacity(balance_elements.len());
        for bal in balance_elements {
            let kind = balance_code(bal).unwrap_or_default();
            let Some(amt_el) = find(bal, "Amt") else {
                continue;
            };
            let amount = Amount::from_element(amt_el)?;
            let credit_debit =
                optional_text(bal, "CdtDbtInd").and_then(|t| CreditDebit::parse(&t).ok());
            balances.push(Balance {
                kind,
                amount,
                credit_debit,
            });
        }
        let opening_balance = balances
            .iter()
            .find(|b| b.kind == "OPBD")
            .map(|b| b.amount.clone());

        let mut entry_elements = Vec::new();
        find_all(stmt, "Ntry", &mut entry_elements);
        let mut entries = Vec::with_capacity(entry_elements.len());
        for ntry in entry_elements {
            let Some(amt_el) = find(ntry, "Amt") else {
                continue;
            };
            let amount = Amount::from_element(amt_el)?;
            let credit_debit = CreditDebit::parse(&required_text(ntry, "CdtDbtInd")?)?;
            let reference =
                optional_text(ntry, "AcctSvcrRef").or_else(|| optional_text(ntry, "NtryRef"));
            let counterparty = find(ntry, "RltdPties")
                .and_then(|p| find(p, "Cdtr").or_else(|| find(p, "Dbtr")))
                .and_then(|party| optional_text(party, "Nm"));
            let remittance = find(ntry, "RmtInf").and_then(|r| optional_text(r, "Ustrd"));
            let value_date = optional_text(ntry, "ValDt");
            entries.push(Entry {
                amount,
                credit_debit,
                reference,
                counterparty,
                remittance,
                value_date,
            });
        }

        Ok(Self {
            id,
            iban,
            owner,
            opening_balance,
            balances,
            entries,
        })
    }

    /// The closing balance, taken from the first `CLBD` balance.
    ///
    /// Falls back to the opening balance plus every entry when the document
    /// does not report one.
    #[must_use]
    pub fn closing_balance(&self) -> f64 {
        if let Some(bal) = self.balances.iter().find(|b| b.kind == "CLBD") {
            return bal.amount.amount * bal.credit_debit.map_or(1.0, CreditDebit::sign);
        }
        let opening = self.opening_balance.as_ref().map_or(0.0, |a| a.amount);
        opening + self.entries.iter().map(Entry::signed_amount).sum::<f64>()
    }

    /// Sum of the entries, ignoring any opening balance.
    #[must_use]
    pub fn total_of_entries(&self) -> f64 {
        self.entries.iter().map(Entry::signed_amount).sum()
    }

    /// The currency of the statement, taken from the opening balance or the
    /// first entry.
    #[must_use]
    pub fn currency(&self) -> Option<&crate::amount::Currency> {
        self.opening_balance
            .as_ref()
            .map(|a| &a.currency)
            .or_else(|| self.entries.first().map(|e| &e.amount.currency))
    }
}

/// Extracts the balance type code from the nested `Tp/CdOrPrtry/Cd` structure,
/// or from a `Prtry` element when a proprietary code is used.
fn balance_code(bal: &Element) -> Option<String> {
    find(bal, "CdOrPrtry")
        .and_then(|c| optional_text(c, "Cd").or_else(|| optional_text(c, "Prtry")))
        .or_else(|| optional_text(bal, "Cd"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATEMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.08">
  <BkToCstmrStmt>
    <GrpHdr><MsgId>MSG-1</MsgId></GrpHdr>
    <Stmt>
      <Id>STMT-001</Id>
      <Acct><Id><IBAN>DE89370400440532013000</IBAN></Id><Nm>Acme GmbH</Nm></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">1000.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
      </Bal>
      <Bal>
        <Tp><CdOrPrtry><Cd>CLBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">1250.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
      </Bal>
      <Ntry>
        <Amt Ccy="EUR">300.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <AcctSvcrRef>REF-1</AcctSvcrRef>
        <ValDt>2026-10-20</ValDt>
        <RltdPties><Dbtr><Nm>Jane Doe</Nm></Dbtr></RltdPties>
        <RmtInf><Ustrd>Invoice 42</Ustrd></RmtInf>
      </Ntry>
      <Ntry>
        <Amt Ccy="EUR">50.00</Amt>
        <CdtDbtInd>DBIT</CdtDbtInd>
        <AcctSvcrRef>REF-2</AcctSvcrRef>
      </Ntry>
    </Stmt>
  </BkToCstmrStmt>
</Document>"#;

    #[test]
    fn parses_a_full_statement() {
        let s = BankStatement::parse(STATEMENT).unwrap();
        assert_eq!(s.id, "STMT-001");
        assert_eq!(s.iban.as_deref(), Some("DE89370400440532013000"));
        assert_eq!(s.owner.as_deref(), Some("Acme GmbH"));
        assert_eq!(s.opening_balance.as_ref().unwrap().amount, 1000.0);
        assert_eq!(s.entries.len(), 2);
    }

    #[test]
    fn entries_carry_their_details() {
        let s = BankStatement::parse(STATEMENT).unwrap();
        let first = &s.entries[0];
        assert_eq!(first.credit_debit, CreditDebit::Credit);
        assert_eq!(first.signed_amount(), 300.0);
        assert_eq!(first.reference.as_deref(), Some("REF-1"));
        assert_eq!(first.counterparty.as_deref(), Some("Jane Doe"));
        assert_eq!(first.remittance.as_deref(), Some("Invoice 42"));
        assert_eq!(first.value_date.as_deref(), Some("2026-10-20"));

        let second = &s.entries[1];
        assert_eq!(second.credit_debit, CreditDebit::Debit);
        assert_eq!(second.signed_amount(), -50.0);
        assert!(second.counterparty.is_none());
    }

    #[test]
    fn closing_balance_prefers_the_reported_value() {
        let s = BankStatement::parse(STATEMENT).unwrap();
        assert_eq!(s.closing_balance(), 1250.0);
        assert_eq!(s.total_of_entries(), 250.0);
    }

    #[test]
    fn closing_balance_is_computed_when_absent() {
        let xml = r#"<Document><BkToCstmrStmt><Stmt>
            <Id>S1</Id>
            <Bal><Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp><Amt Ccy="EUR">100.00</Amt></Bal>
            <Ntry><Amt Ccy="EUR">10.00</Amt><CdtDbtInd>CRDT</CdtDbtInd></Ntry>
            <Ntry><Amt Ccy="EUR">25.00</Amt><CdtDbtInd>DBIT</CdtDbtInd></Ntry>
        </Stmt></BkToCstmrStmt></Document>"#;
        let s = BankStatement::parse(xml).unwrap();
        assert_eq!(s.closing_balance(), 85.0);
    }

    #[test]
    fn currency_comes_from_the_statement() {
        let s = BankStatement::parse(STATEMENT).unwrap();
        assert_eq!(s.currency().unwrap().as_str(), "EUR");
    }

    #[test]
    fn balances_are_collected_with_their_codes() {
        let s = BankStatement::parse(STATEMENT).unwrap();
        let codes: Vec<&str> = s.balances.iter().map(|b| b.kind.as_str()).collect();
        assert_eq!(codes, ["OPBD", "CLBD"]);
    }

    #[test]
    fn a_document_without_a_statement_is_an_error() {
        assert_eq!(
            BankStatement::parse("<Document><BkToCstmrStmt/></Document>"),
            Err(Error::MissingElement("Stmt"))
        );
    }

    #[test]
    fn a_statement_without_an_id_is_an_error() {
        let xml = "<Document><BkToCstmrStmt><Stmt><Ntry/></Stmt></BkToCstmrStmt></Document>";
        assert_eq!(BankStatement::parse(xml), Err(Error::MissingElement("Id")));
    }

    #[test]
    fn credit_debit_indicator_is_strict() {
        assert_eq!(CreditDebit::parse("CRDT"), Ok(CreditDebit::Credit));
        assert_eq!(CreditDebit::parse("DBIT"), Ok(CreditDebit::Debit));
        assert!(matches!(
            CreditDebit::parse("CREDIT"),
            Err(Error::InvalidValue { .. })
        ));
    }

    #[test]
    fn unknown_elements_are_ignored() {
        let xml = r#"<Document><BkToCstmrStmt><Stmt>
            <Id>S9</Id>
            <SomeFutureElement><Nested>ignored</Nested></SomeFutureElement>
            <Ntry><Amt Ccy="EUR">1.00</Amt><CdtDbtInd>CRDT</CdtDbtInd><Extra>x</Extra></Ntry>
        </Stmt></BkToCstmrStmt></Document>"#;
        let s = BankStatement::parse(xml).unwrap();
        assert_eq!(s.id, "S9");
        assert_eq!(s.entries.len(), 1);
    }
}
