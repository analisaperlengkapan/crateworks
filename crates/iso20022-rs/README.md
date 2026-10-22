# iso20022-rs

[![crates.io](https://img.shields.io/crates/v/iso20022-rs.svg)](https://crates.io/crates/iso20022-rs)
[![docs.rs](https://docs.rs/iso20022-rs/badge.svg)](https://docs.rs/iso20022-rs)

ISO 20022 payment messages: `camt` bank statements and `pain` payment
initiation.

```rust
use iso20022_rs::camt::BankStatement;

let xml = r#"<Document><BkToCstmrStmt><Stmt>
  <Id>S1</Id>
  <Bal><Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp><Amt Ccy="EUR">100.00</Amt></Bal>
  <Ntry><Amt Ccy="EUR">25.00</Amt><CdtDbtInd>CRDT</CdtDbtInd></Ntry>
</Stmt></BkToCstmrStmt></Document>"#;

let s = BankStatement::parse(xml).unwrap();
assert_eq!(s.id, "S1");
assert_eq!(s.closing_balance(), 125.0);
```

`pain.001` can be read and generated, with the control sum and transaction count
computed from the transfers so the output is internally consistent.

ISO 20022 is the XML message standard that is replacing the older SWIFT MT
formats. It is organised into business areas whose message names carry a
three-letter prefix:

* `pain` - payment initiation, sent by a customer to their bank.
* `pacs` - payment clearing and settlement, sent between banks.
* `camt` - cash management, including account statements.

## Example

```
use iso20022_rs::camt::BankStatement;

let xml = r#"<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.08">
  <BkToCstmrStmt>
    <Stmt>
      <Id>STMT-001</Id>
      <Acct><Id><IBAN>DE89370400440532013000</IBAN></Id></Acct>
      <Bal><Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp><Amt Ccy="EUR">1000.00</Amt></Bal>
      <Ntry><Amt Ccy="EUR">250.00</Amt><CdtDbtInd>CRDT</CdtDbtInd></Ntry>
    </Stmt>
  </BkToCstmrStmt>
</Document>"#;

let statement = BankStatement::parse(xml).unwrap();
assert_eq!(statement.id, "STMT-001");
assert_eq!(statement.iban.as_deref(), Some("DE89370400440532013000"));
assert_eq!(statement.opening_balance.as_ref().unwrap().amount, 1000.0);
assert_eq!(statement.entries.len(), 1);
assert_eq!(statement.closing_balance(), 1250.0);
```

## Scope

Each message is modelled with the fields an application actually needs,
rather than the full generated schema. Unknown elements are ignored, so a
document from a newer schema version still parses. Namespace URIs are
recorded but not resolved.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
