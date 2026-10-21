# javaproperties

Reading Java `.properties` files.

This is the format behind `application.properties` in every Spring service and
`messages_id.properties` in every Java i18n bundle. It looks like a list of
`key=value` lines and is not: the rules are older than the format's common use,
and getting them wrong is quiet.

```rust
use javaproperties::parse;

let pairs = parse(b"name = cipansor\ncity\\:bandung = west java\n").unwrap();
assert_eq!(pairs[0], ("name".to_string(), "cipansor".to_string()));
// The escaped colon is part of the key, not a separator.
assert_eq!(pairs[1], ("city:bandung".to_string(), "west java".to_string()));
```

## Why this is not a five-line parser

Every rule below is one somebody gets wrong:

- A line whose first non-blank character is `#` or `!` is a comment.
- A key ends at the first unescaped `=`, `:`, or run of whitespace. `a\=b=c`
  splits into the key `a=b` and the value `c`; `a\\=b` splits after the doubled
  backslash into the key `a\` and the value `b`.
- Whitespace after the separator is not part of the value; whitespace at the end
  of the line is.
- A line ending in an odd number of backslashes continues onto the next, whose
  leading whitespace is dropped and nothing put in its place, so `one\` then
  `two` is `onetwo`.
- `\t`, `\n`, `\f`, `\r`, `\\` and `\uXXXX` are escapes. A backslash before any
  other character is dropped, so `\q` is `q`.
- A `\u` not followed by four hex digits is an error, not a literal `u`.
- Characters above the basic plane are written as a surrogate pair, and are
  rejoined into one character.
- Bytes are ISO-8859-1 unless you say otherwise. `parse` takes bytes and reads
  them that way; `parse_str` takes text you have already decoded. Reading a Java
  file as UTF-8 is the classic source of mojibake.

A key given twice keeps both entries in order, and `Properties::get` returns the
last, which is what Java does.

## How it is checked

Against Python's `javaproperties` package, which replicates
`java.util.Properties` and is itself tested against Java. The corpus is a
hand-written set that pins each rule above, plus **six thousand random cases**
built from the format's own punctuation: separators, escapes, `\uXXXX` sequences,
line continuations, comments, all four line endings and the ISO-8859-1 default
all mixed into the same document. All 6,084 agree.

The random corpus is generated with a fixed seed, so it can be rebuilt exactly,
and the cases where two rules meet are the ones it exists for: a key that ends in
a backslash just before a comment, a `\uXXXX` split across a continuation, a
separator that is also an escaped character. Those are where the obvious
implementation and the reference part ways.

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
