# iso6346

Shipping container codes, the eleven-character identifiers painted on the side
of every freight container: `CSQU3054383`. The crate parses one, validates its
check digit, and generates the check digit for a new code.

ISO 6346 defines the code, and the BIC (Bureau International des Containers et
du Transport Intermodal) registers the owner prefixes. The check digit is the
part that catches a mistyped digit before a container is loaded onto the wrong
ship, and it is the whole reason to use a crate rather than a string.

## Example

```rust
use iso6346::{Category, Container, check_digit, validate};

// Parse and validate. Case and separators do not matter.
let container = Container::parse("csqu-305438-3").unwrap();
assert_eq!(container.owner_code(), "CSQ");
assert_eq!(container.category(), Category::Freight);
assert_eq!(container.serial(), 305_438);
assert_eq!(container.check_digit(), 3);
assert_eq!(container.to_string(), "CSQU3054383");

// A single wrong digit fails the check.
assert!(validate("CSQU3054384").is_err());

// Generate a code: the check digit is whatever the other ten characters imply.
let generated = Container::new("CSQ", Category::Freight, 305_438).unwrap();
assert_eq!(generated.to_string(), "CSQU3054383");

// Or ask for the digit on its own, to complete a partial code.
assert_eq!(check_digit("CSQ", Category::Freight, 305_438), Some(3));
```

## The code

An ISO 6346 code is four letters and seven digits:

| Part | Example | Meaning |
|---|---|---|
| Owner code | `CSQ` | Three letters, registered with the BIC |
| Category | `U` | `U` freight container, `J` detachable equipment, `Z` chassis |
| Serial | `305438` | Six digits, `000000` to `999999` |
| Check digit | `3` | One digit, computed from the ten before it |

The check digit is the sum of each character's value times two raised to its
position, modulo eleven. A letter's value is its position in the alphabet plus
nine, skipping every multiple of eleven, so `A` is 10, `B` is 12, `K` is 21, `L`
is 23 and `Z` is 38. A remainder of ten is written as `0`.

## What it does not do

It does not check that an owner code is registered. The BIC's register is a
dataset that changes, and a crate that shipped a copy would be wrong within a
month. A code can be well formed, have a correct check digit, and still name an
operator that does not exist.

## How it is checked

Against the `container-validator` npm package by Sameer Shemna
(<https://github.com/sameershemna/Container-validator>), over a generated corpus
of four thousand codes. The crate is an independent reimplementation; the
algorithm is ISO 6346's.

Part of the [crateworks](https://github.com/cipansor/crateworks) workspace.
Licensed under the Apache License, Version 2.0, copyright Yayasan Pesantren
Cipansor and contributors.
