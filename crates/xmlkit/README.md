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
means it cannot be used for XXE attacks. Namespace URIs are resolved; nothing
else about a schema is.

## Namespaces

The prefix is arbitrary and the URI is the identity, so matching on the name as
written finds `<soap:Body>` and misses `<env:Body>` even when both are the same
element. `find_namespaced` and `all_namespaced` match on the URI and the local
name, and resolve the prefix through the declarations on the element and its
ancestors, which is what a rebinding needs.

```rust
use xmlkit::{XML_NAMESPACE, attribute_namespaced, find_namespaced, parse};

let doc = parse(
    r#"<env:Envelope xmlns:env="urn:soap"><env:Body xml:lang="en"/></env:Envelope>"#,
).unwrap();

// Any prefix bound to the URI matches, and the prefix in the document is irrelevant.
assert!(find_namespaced(&doc, Some("urn:soap"), "Body").is_some());
assert!(find_namespaced(&doc, Some("urn:other"), "Body").is_none());

// `xml:` is bound without a declaration, and an unprefixed attribute is in no
// namespace even inside an element that declares a default one.
let body = find_namespaced(&doc, Some("urn:soap"), "Body").unwrap();
assert_eq!(attribute_namespaced(&doc, body, XML_NAMESPACE, "lang"), Some("en"));
```

An unbound prefix names nothing rather than "no namespace", so a document cannot
pass off `<evil:Body>` as an un-namespaced `Body` by leaving `evil` undeclared.

## Scope

* `parse` builds an `Element` tree from a string.
* `Element`'s `Display` impl writes it back.
* `find_namespaced`, `all_namespaced` and `Namespaces` resolve namespace URIs.
* No XPath, no schema validation, no DTD processing beyond skipping it.

The parser and the tree's `Drop` both keep their own stack rather than recursing,
so a deeply nested document cannot overflow the call stack; `MAX_DEPTH` is a
policy limit on nesting, not a safety one.

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
