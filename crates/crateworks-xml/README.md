# crateworks-xml

A small XML parser, tree and serialiser with no dependencies.

```rust
use crateworks_xml::{parse, local_name};

let doc = parse(r#"<ns:Doc xmlns:ns="urn:x"><ns:Id>7</ns:Id></ns:Doc>"#).unwrap();
assert_eq!(local_name(&doc.name), "Doc");
assert_eq!(doc.deep_text(), "7");
```

Keeps comments and CDATA so that a parse and a re-serialise round-trips
faithfully. Namespace prefixes are preserved and `local_name` strips them for
matching, which is what a SOAP or ISO 20022 reader needs.

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under Apache-2.0 or MIT.
