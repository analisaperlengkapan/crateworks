# iso20022-rs

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

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
