//! `pain` payment initiation messages.
//!
//! A `pain.001` message is a customer instructing their bank to make one or
//! more payments. This module can both read such a message and build one, which
//! is the direction most applications need: they collect payment details from a
//! user and have to hand the bank a file in the format it expects.

use std::fmt::Write as _;

use crate::amount::Amount;
use crate::camt::CreditDebit;
use crate::{Error, Result, find, find_all, optional_text, required_text};
use crateworks_xml::Element;

/// One payment instruction inside a batch.
#[derive(Debug, Clone, PartialEq)]
pub struct CreditTransfer {
    /// Instruction identifier, unique within the message.
    pub instruction_id: String,
    /// Amount to transfer.
    pub amount: Amount,
    /// Name of the person or company receiving the money.
    pub creditor_name: String,
    /// Creditor IBAN.
    pub creditor_iban: String,
    /// Optional BIC of the creditor's bank.
    pub creditor_bic: Option<String>,
    /// Remittance information shown to the creditor.
    pub remittance: Option<String>,
}

/// A `pain.001` customer credit transfer initiation message.
#[derive(Debug, Clone, PartialEq)]
pub struct CreditTransferInitiation {
    /// Message identifier, assigned by the sender.
    pub message_id: String,
    /// Name of the account being debited.
    pub debtor_name: String,
    /// IBAN of the account being debited.
    pub debtor_iban: String,
    /// Optional BIC of the debtor's bank.
    pub debtor_bic: Option<String>,
    /// The payment instructions.
    pub transfers: Vec<CreditTransfer>,
}

/// Reads a BIC from an agent element such as `<DbtrAgt>`.
///
/// The code lives in `FinInstnId` and may be tagged `BICFI` (the current name)
/// or `BIC` (the older one), so both are accepted.
fn agent_bic(agent: &Element) -> Option<String> {
    let fin = find(agent, "FinInstnId").unwrap_or(agent);
    optional_text(fin, "BICFI").or_else(|| optional_text(fin, "BIC"))
}

impl CreditTransferInitiation {
    /// Reads a `pain.001` document.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`], [`Error::MissingElement`] or
    /// [`Error::InvalidValue`] as appropriate.
    pub fn parse(xml: &str) -> Result<Self> {
        let body = crate::document_body(xml)?;
        let group = find(&body, "GrpHdr").ok_or(Error::MissingElement("GrpHdr"))?;
        let message_id = required_text(group, "MsgId")?;
        let debtor_name = required_text(&body, "Dbtr").or_else(|_| {
            find(&body, "Dbtr")
                .map(Element::deep_text)
                .ok_or(Error::MissingElement("Dbtr"))
        })?;
        let debtor_iban = find(&body, "DbtrAcct")
            .and_then(|a| optional_text(a, "IBAN"))
            .ok_or(Error::MissingElement("DbtrAcct/IBAN"))?;
        let debtor_bic = find(&body, "DbtrAgt").and_then(agent_bic);

        let mut elements = Vec::new();
        find_all(&body, "CdtTrfTxInf", &mut elements);
        let mut transfers = Vec::with_capacity(elements.len());
        for tx in elements {
            transfers.push(Self::read_transfer(tx)?);
        }
        Ok(Self {
            message_id,
            debtor_name,
            debtor_iban,
            debtor_bic,
            transfers,
        })
    }

    fn read_transfer(tx: &Element) -> Result<CreditTransfer> {
        let instruction_id = find(tx, "PmtId")
            .and_then(|p| optional_text(p, "InstrId").or_else(|| optional_text(p, "EndToEndId")))
            .unwrap_or_else(|| "NOTPROVIDED".to_string());
        let amt_el = find(tx, "InstdAmt")
            .or_else(|| find(tx, "Amt"))
            .ok_or(Error::MissingElement("InstdAmt"))?;
        let amount = Amount::from_element(amt_el)?;
        let creditor_name = find(tx, "Cdtr")
            .and_then(|c| optional_text(c, "Nm"))
            .ok_or(Error::MissingElement("Cdtr/Nm"))?;
        let creditor_iban = find(tx, "CdtrAcct")
            .and_then(|a| optional_text(a, "IBAN"))
            .ok_or(Error::MissingElement("CdtrAcct/IBAN"))?;
        let creditor_bic = find(tx, "CdtrAgt").and_then(agent_bic);
        let remittance = find(tx, "RmtInf").and_then(|r| optional_text(r, "Ustrd"));
        Ok(CreditTransfer {
            instruction_id,
            amount,
            creditor_name,
            creditor_iban,
            creditor_bic,
            remittance,
        })
    }

    /// The sum of all transfers, in the currency of the first transfer.
    #[must_use]
    pub fn total(&self) -> f64 {
        self.transfers.iter().map(|t| t.amount.amount).sum()
    }

    /// The number of transfers, which the control sum must agree with.
    #[must_use]
    pub fn number_of_transfers(&self) -> usize {
        self.transfers.len()
    }

    /// Writes the message as a `pain.001.001.09` document.
    ///
    /// The control sum and the transaction count are computed from the
    /// transfers rather than taken from the caller, so the output is internally
    /// consistent by construction.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingElement`] when there are no transfers, because a
    /// bank rejects an empty batch.
    pub fn to_xml(&self) -> Result<String> {
        if self.transfers.is_empty() {
            return Err(Error::MissingElement("CdtTrfTxInf"));
        }
        let Some(first) = self.transfers.first() else {
            return Err(Error::MissingElement("CdtTrfTxInf"));
        };
        let _ = first;
        let now = "2026-01-01T00:00:00Z";
        let mut out = String::with_capacity(1024 + self.transfers.len() * 512);
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(
            "<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:pain.001.001.09\">\n  <CstmrCdtTrfInitn>\n",
        );
        out.push_str("    <GrpHdr>\n");
        let _ = writeln!(out, "      <MsgId>{}</MsgId>", escape(&self.message_id));
        let _ = writeln!(out, "      <CreDtTm>{now}</CreDtTm>");
        let _ = writeln!(out, "      <NbOfTxs>{}</NbOfTxs>", self.transfers.len());
        let _ = writeln!(
            out,
            "      <CtrlSum>{}</CtrlSum>",
            format_decimal(self.total())
        );
        out.push_str("      <InitgPty>\n");
        let _ = writeln!(out, "        <Nm>{}</Nm>", escape(&self.debtor_name));
        out.push_str("      </InitgPty>\n    </GrpHdr>\n");

        out.push_str("    <PmtInf>\n");
        let _ = writeln!(
            out,
            "      <PmtInfId>{}-PMT</PmtInfId>",
            escape(&self.message_id)
        );
        out.push_str("      <PmtMtd>TRF</PmtMtd>\n");
        let _ = writeln!(out, "      <NbOfTxs>{}</NbOfTxs>", self.transfers.len());
        let _ = writeln!(
            out,
            "      <CtrlSum>{}</CtrlSum>",
            format_decimal(self.total())
        );
        out.push_str("      <Dbtr>\n");
        let _ = writeln!(out, "        <Nm>{}</Nm>", escape(&self.debtor_name));
        out.push_str("      </Dbtr>\n");
        out.push_str("      <DbtrAcct>\n        <Id>\n");
        let _ = writeln!(out, "          <IBAN>{}</IBAN>", escape(&self.debtor_iban));
        out.push_str("        </Id>\n      </DbtrAcct>\n");
        if let Some(bic) = &self.debtor_bic {
            out.push_str("      <DbtrAgt>\n        <FinInstnId>\n");
            let _ = writeln!(out, "          <BICFI>{}</BICFI>", escape(bic));
            out.push_str("        </FinInstnId>\n      </DbtrAgt>\n");
        }

        for (i, t) in self.transfers.iter().enumerate() {
            out.push_str("      <CdtTrfTxInf>\n");
            out.push_str("        <PmtId>\n");
            let _ = writeln!(
                out,
                "          <InstrId>{}</InstrId>",
                escape(&t.instruction_id)
            );
            let _ = writeln!(
                out,
                "          <EndToEndId>{}</EndToEndId>",
                escape(&t.instruction_id)
            );
            out.push_str("        </PmtId>\n");
            let _ = writeln!(
                out,
                "        <Amt><InstdAmt Ccy=\"{}\">{}</InstdAmt></Amt>",
                escape(t.amount.currency.as_str()),
                format_decimal(t.amount.amount)
            );
            if let Some(bic) = &t.creditor_bic {
                out.push_str("        <CdtrAgt>\n          <FinInstnId>\n");
                let _ = writeln!(out, "            <BICFI>{}</BICFI>", escape(bic));
                out.push_str("          </FinInstnId>\n        </CdtrAgt>\n");
            }
            out.push_str("        <Cdtr>\n");
            let _ = writeln!(out, "          <Nm>{}</Nm>", escape(&t.creditor_name));
            out.push_str("        </Cdtr>\n");
            out.push_str("        <CdtrAcct>\n          <Id>\n");
            let _ = writeln!(out, "            <IBAN>{}</IBAN>", escape(&t.creditor_iban));
            out.push_str("          </Id>\n        </CdtrAcct>\n");
            if let Some(remit) = &t.remittance {
                out.push_str("        <RmtInf>\n");
                let _ = writeln!(out, "          <Ustrd>{}</Ustrd>", escape(remit));
                out.push_str("        </RmtInf>\n");
            }
            let _ = writeln!(out, "      </CdtTrfTxInf>");
            let _ = i;
        }
        out.push_str("    </PmtInf>\n  </CstmrCdtTrfInitn>\n</Document>\n");
        Ok(out)
    }
}

/// Formats a decimal with two places, the usual convention for currency.
fn format_decimal(value: f64) -> String {
    format!("{value:.2}")
}

/// Escapes text for XML content.
fn escape(s: &str) -> String {
    crateworks_xml::escape_text(s)
}

/// Whether a control sum matches the sum of the entries it covers.
///
/// Banks reject a file whose control sum disagrees with its entries, so a
/// generator should check this before sending.
#[must_use]
pub fn control_sum_matches(entries: &[f64], control_sum: f64) -> bool {
    // Compare at the scale the format uses, two decimal places.
    let sum: f64 = entries.iter().sum();
    (sum - control_sum).abs() < 0.005
}

/// Sign of an entry, re-exported so callers do not need the `camt` module.
#[must_use]
pub fn entry_sign(indicator: CreditDebit) -> f64 {
    indicator.sign()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAIN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.09">
  <CstmrCdtTrfInitn>
    <GrpHdr>
      <MsgId>MSG-2026-001</MsgId>
      <CreDtTm>2026-10-21T09:00:00Z</CreDtTm>
      <NbOfTxs>2</NbOfTxs>
      <CtrlSum>750.00</CtrlSum>
    </GrpHdr>
    <PmtInf>
      <PmtInfId>PMT-1</PmtInfId>
      <Dbtr><Nm>Acme GmbH</Nm></Dbtr>
      <DbtrAcct><Id><IBAN>DE89370400440532013000</IBAN></Id></DbtrAcct>
      <DbtrAgt><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></DbtrAgt>
      <CdtTrfTxInf>
        <PmtId><InstrId>INSTR-1</InstrId></PmtId>
        <Amt><InstdAmt Ccy="EUR">500.00</InstdAmt></Amt>
        <Cdtr><Nm>Supplier One</Nm></Cdtr>
        <CdtrAcct><Id><IBAN>GB82WEST12345698765432</IBAN></Id></CdtrAcct>
        <RmtInf><Ustrd>Invoice 1001</Ustrd></RmtInf>
      </CdtTrfTxInf>
      <CdtTrfTxInf>
        <PmtId><InstrId>INSTR-2</InstrId></PmtId>
        <Amt><InstdAmt Ccy="EUR">250.00</InstdAmt></Amt>
        <Cdtr><Nm>Supplier Two</Nm></Cdtr>
        <CdtrAcct><Id><IBAN>NL91ABNA0417164300</IBAN></Id></CdtrAcct>
      </CdtTrfTxInf>
    </PmtInf>
  </CstmrCdtTrfInitn>
</Document>"#;

    #[test]
    fn parses_a_payment_initiation() {
        let p = CreditTransferInitiation::parse(PAIN).unwrap();
        assert_eq!(p.message_id, "MSG-2026-001");
        assert_eq!(p.debtor_name, "Acme GmbH");
        assert_eq!(p.debtor_iban, "DE89370400440532013000");
        assert_eq!(p.debtor_bic.as_deref(), Some("DEUTDEFF"));
        assert_eq!(p.number_of_transfers(), 2);
        assert_eq!(p.total(), 750.0);
    }

    #[test]
    fn transfers_carry_their_details() {
        let p = CreditTransferInitiation::parse(PAIN).unwrap();
        let first = &p.transfers[0];
        assert_eq!(first.instruction_id, "INSTR-1");
        assert_eq!(first.amount.amount, 500.0);
        assert_eq!(first.amount.currency.as_str(), "EUR");
        assert_eq!(first.creditor_name, "Supplier One");
        assert_eq!(first.creditor_iban, "GB82WEST12345698765432");
        assert_eq!(first.remittance.as_deref(), Some("Invoice 1001"));
        assert!(p.transfers[1].remittance.is_none());
    }

    #[test]
    fn round_trip_through_generation() {
        let original = CreditTransferInitiation::parse(PAIN).unwrap();
        let xml = original.to_xml().unwrap();
        let reparsed = CreditTransferInitiation::parse(&xml).unwrap();
        assert_eq!(reparsed.message_id, original.message_id);
        assert_eq!(reparsed.debtor_iban, original.debtor_iban);
        assert_eq!(reparsed.transfers.len(), original.transfers.len());
        assert_eq!(reparsed.total(), original.total());
        assert_eq!(
            reparsed.transfers[0].creditor_iban,
            original.transfers[0].creditor_iban
        );
        assert_eq!(
            reparsed.transfers[0].remittance,
            original.transfers[0].remittance
        );
    }

    #[test]
    fn generated_control_sum_agrees_with_entries() {
        let p = CreditTransferInitiation::parse(PAIN).unwrap();
        let xml = p.to_xml().unwrap();
        let amounts: Vec<f64> = p.transfers.iter().map(|t| t.amount.amount).collect();
        assert!(control_sum_matches(&amounts, p.total()));
        assert!(xml.contains("<CtrlSum>750.00</CtrlSum>"));
        assert!(xml.contains("<NbOfTxs>2</NbOfTxs>"));
    }

    #[test]
    fn generated_xml_escapes_special_characters() {
        let mut p = CreditTransferInitiation::parse(PAIN).unwrap();
        p.transfers[0].creditor_name = "Smith & Sons <Ltd>".into();
        let xml = p.to_xml().unwrap();
        assert!(xml.contains("Smith &amp; Sons &lt;Ltd&gt;"));
        // Re-parsing must return the original text.
        let back = CreditTransferInitiation::parse(&xml).unwrap();
        assert_eq!(back.transfers[0].creditor_name, "Smith & Sons <Ltd>");
    }

    #[test]
    fn empty_batch_cannot_be_written() {
        let mut p = CreditTransferInitiation::parse(PAIN).unwrap();
        p.transfers.clear();
        assert_eq!(p.to_xml(), Err(Error::MissingElement("CdtTrfTxInf")));
    }

    #[test]
    fn missing_debtor_iban_is_an_error() {
        let xml = r#"<Document><CstmrCdtTrfInitn>
            <GrpHdr><MsgId>M</MsgId></GrpHdr>
            <PmtInf><Dbtr><Nm>X</Nm></Dbtr></PmtInf>
        </CstmrCdtTrfInitn></Document>"#;
        assert_eq!(
            CreditTransferInitiation::parse(xml),
            Err(Error::MissingElement("DbtrAcct/IBAN"))
        );
    }

    #[test]
    fn control_sum_comparison_uses_two_decimals() {
        assert!(control_sum_matches(&[0.1, 0.2], 0.3));
        assert!(!control_sum_matches(&[1.0], 1.01));
        assert!(control_sum_matches(&[1.0], 1.004));
    }

    #[test]
    fn entry_sign_follows_the_indicator() {
        assert_eq!(entry_sign(CreditDebit::Credit), 1.0);
        assert_eq!(entry_sign(CreditDebit::Debit), -1.0);
    }
}
