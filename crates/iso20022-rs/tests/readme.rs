//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use iso20022_rs::camt::BankStatement;

    let xml = r#"<Document><BkToCstmrStmt><Stmt>
  <Id>S1</Id>
  <Bal><Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp><Amt Ccy="EUR">100.00</Amt></Bal>
  <Ntry><Amt Ccy="EUR">25.00</Amt><CdtDbtInd>CRDT</CdtDbtInd></Ntry>
</Stmt></BkToCstmrStmt></Document>"#;

    let s = BankStatement::parse(xml).unwrap();
    assert_eq!(s.id, "S1");
    assert_eq!(s.closing_balance(), 125.0);
}
