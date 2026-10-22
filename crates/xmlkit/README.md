# xmlkit

[![crates.io](https://img.shields.io/crates/v/xmlkit.svg)](https://crates.io/crates/xmlkit)
[![docs.rs](https://docs.rs/xmlkit/badge.svg)](https://docs.rs/xmlkit)

A small XML parser, tree and serialiser with no dependencies.

```rust
use xmlkit::{parse, local_name};

let doc = parse(r#"<ns:Doc xmlns:ns="urn:x"><ns:Id>7</ns:Id></ns:Doc>"#).unwrap();
assert_eq!(local_name(&doc.name), "Doc");
assert_eq!(doc.deep_text(), "7");
```

Keeps comments and CDATA so that a parse and a re-serialise round-trips
faithfully. Namespace prefixes are preserved and `local_name` strips them for
matching, which is what a SOAP or ISO 20022 reader needs.

This crate exists because `iso20022-rs` and `soap-client` both need the same
small amount of XML handling, and neither needs a full DOM. It deliberately
covers a subset of XML: elements, attributes, text, CDATA, comments,
processing instructions and the five predefined entities plus numeric
character references. It does not resolve external entities, which also
means it cannot be used for XXE attacks.

## Scope

* `parse` builds an `Element` tree from a string.
* `Element`'s `Display` impl writes it back.
* Namespaces are preserved as part of the element name; nothing is resolved.

## Example

```
use xmlkit::{Element, parse};

let root = parse("<a x=\"1\"><b>hi</b></a>").unwrap();
assert_eq!(root.name, "a");
assert_eq!(root.attr("x"), Some("1"));
assert_eq!(root.child("b").unwrap().text(), "hi");
```

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
