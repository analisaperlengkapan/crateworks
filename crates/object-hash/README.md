# object-hash

A stable hash of a structured value.

Given a map, an array, a string, a number, nested in any combination, this
produces a short digest that is the same every time for values that are equal —
across runs, across processes, and across machines.

```rust
use object_hash::{Value, hash};

let one = Value::Map(vec![
    ("name".into(), Value::String("cipansor".into())),
    ("year".into(), Value::Number(2026.0)),
]);
// The same entries, inserted the other way round.
let two = Value::Map(vec![
    ("year".into(), Value::Number(2026.0)),
    ("name".into(), Value::String("cipansor".into())),
]);

assert_eq!(hash(&one), hash(&two));
```

## Why `Hash` is not enough

`std::hash::Hash` looks like the right tool and is not. The default hasher,
`RandomState`, is seeded per process, so the same value hashes differently every
time the program starts. That is exactly right for a `HashMap` — the seed is what
stops an attacker choosing collisions — and exactly wrong for anything that has
to be written down or compared across processes. The other half of the problem is
that a `HashMap` iterates in an unspecified order, so even with a fixed hasher two
equal maps can hash differently.

So the value is normalised first, into a canonical form that depends only on what
the value *is*, and then those bytes are hashed. This crate does both.

## What it is for

- **Cache keys** that hit the same entry for the same request.
- **ETags** that change when the content changes and not when a field is
  reordered.
- **Idempotency keys** that recognise a repeated request as the same action.
- **Change detection and deduplication** by comparing two digests.

## The equality rules

| Value | Rule |
|---|---|
| `Map` | Written in key order; insertion order does not matter |
| `Array` | Order matters |
| `Set` | Order does not matter |
| `Number` | An integer and an equal float are the same number; `-0.0` is `0`; `NaN`, `Infinity` and `-Infinity` have names |
| `String` | Carries its length in UTF-16 code units, so an emoji counts as two |
| `Null` | Its own value, distinct from the empty string |

The length prefix is what stops `{"ab": 1}` and `{"a": "b1"}` from writing the
same bytes.

## Compatibility with the npm package

The canonical form is the one the `object-hash` npm package produces, byte for
byte, and the digest is SHA-1 in lowercase hexadecimal, which is its default. A
digest computed here equals one computed there for the same value, so a Rust
service and a Node service can agree on a cache key without either knowing about
the other. That is why a map is written as an array of pairs, and why an integral
float has no decimal point: the reference writes them that way.

The one deliberate difference is JavaScript objects. An object there has a
prototype and a constructor, and the reference mixes those into the hash — so
`{}` in JavaScript hashes differently from an empty map here. A Rust caller has
no prototype to respect, and mixing one in would make the digest depend on the
library rather than on the value. `Value::Map` is the entries and nothing else.

## What it is not

SHA-1 is not used here as a security primitive. The digest names a value; it does
not protect it, and it is not a signature. `canonical` is exported, so if a
different digest is wanted, hash the canonical bytes with any other function.

## How it is checked

Against the `object-hash` npm package, over 51 cases: the reference's own
canonical byte stream and its default digest for numbers (including `NaN` and the
infinities), strings (including emoji and CJK), nested maps, arrays, sets, and the
sorting of unordered collections. All agree. SHA-1 is checked against the FIPS
180-1 vectors, including the one-million-character one.

Part of the [crateworks](https://github.com/analisaperlengkapan/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
