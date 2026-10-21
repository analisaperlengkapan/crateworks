//! A corpus of randomly built values, hashed by the reference and by this crate.
//!
//! The curated fixtures cover the cases somebody thought of. This covers the
//! ones nobody did: 4,000 values built at random out of nesting arrays, sets and
//! maps, mixing numbers at both ends of the range, strings that are empty or
//! hold emoji, CJK or U+FFFF, and repeated keys and entries. The reference
//! `object-hash` npm package wrote both the canonical form and the digest for
//! each one.
//!
//! Two real bugs came out of this corpus and nowhere else:
//!
//! * Numbers at the edges of the range: Rust writes `1e21` as twenty-two digits
//!   where JavaScript writes `1e+21`, and `1e-7` as `0.0000001` where JavaScript
//!   writes `1e-7`.
//! * Ordering entries above the basic plane: JavaScript sorts by UTF-16 code
//!   unit, so an emoji (U+D83D…) sorts *before* U+FFFF. Comparing UTF-8 bytes
//!   reverses those two, which changed the digest of any set holding both.
//!
//! The descriptions are a small tagged JSON, parsed here rather than pulled in
//! as a dependency, since this crate has none.

use object_hash::{Value, canonical, hash};

const CORPUS: &str = include_str!("fixtures/fuzz.tsv");

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            at: 0,
        }
    }

    fn peek(&self) -> u8 {
        self.bytes[self.at]
    }

    fn take(&mut self, expected: u8) {
        assert_eq!(
            self.bytes[self.at], expected,
            "expected {:?} at byte {}",
            expected as char, self.at
        );
        self.at += 1;
    }

    fn literal(&mut self, text: &str) {
        assert!(
            self.bytes[self.at..].starts_with(text.as_bytes()),
            "expected {text:?} at byte {}",
            self.at
        );
        self.at += text.len();
    }

    fn value(&mut self) -> Value {
        match self.peek() {
            b'n' if self.bytes[self.at..].starts_with(b"null") => {
                self.at += 4;
                Value::Null
            }
            b't' => {
                self.literal("true");
                Value::Bool(true)
            }
            b'f' => {
                self.literal("false");
                Value::Bool(false)
            }
            b'"' => Value::String(self.string()),
            b'{' => self.tagged(),
            other => panic!("unexpected {:?} at byte {}", other as char, self.at),
        }
    }

    /// One of `{"n":..}`, `{"a":[..]}`, `{"s":[..]}` or `{"m":[[key,value],..]}`.
    fn tagged(&mut self) -> Value {
        self.take(b'{');
        self.take(b'"');
        let tag = self.peek();
        self.at += 1;
        self.take(b'"');
        self.take(b':');
        let value = match tag {
            b'n' => {
                let text = self.string();
                Value::Number(match text.as_str() {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    _ => text.parse().unwrap_or_else(|_| panic!("number {text:?}")),
                })
            }
            b'a' => Value::Array(self.list()),
            b's' => Value::Set(self.list()),
            b'm' => Value::Map(self.pairs()),
            other => panic!("tag {:?}", other as char),
        };
        self.take(b'}');
        value
    }

    fn list(&mut self) -> Vec<Value> {
        self.take(b'[');
        let mut items = Vec::new();
        if self.peek() == b']' {
            self.at += 1;
            return items;
        }
        loop {
            items.push(self.value());
            match self.peek() {
                b',' => self.at += 1,
                _ => {
                    self.take(b']');
                    return items;
                }
            }
        }
    }

    fn pairs(&mut self) -> Vec<(String, Value)> {
        self.take(b'[');
        let mut pairs = Vec::new();
        if self.peek() == b']' {
            self.at += 1;
            return pairs;
        }
        loop {
            self.take(b'[');
            let key = self.string();
            self.take(b',');
            let value = self.value();
            self.take(b']');
            pairs.push((key, value));
            match self.peek() {
                b',' => self.at += 1,
                _ => {
                    self.take(b']');
                    return pairs;
                }
            }
        }
    }

    fn string(&mut self) -> String {
        self.take(b'"');
        let mut text = String::new();
        loop {
            match self.peek() {
                b'"' => {
                    self.at += 1;
                    return text;
                }
                b'\\' => {
                    self.at += 1;
                    let escape = self.peek();
                    self.at += 1;
                    match escape {
                        b'u' => {
                            let first = self.hex4();
                            // A lead surrogate is always followed by its trail.
                            let code = if (0xD800..0xDC00).contains(&first) {
                                self.take(b'\\');
                                self.take(b'u');
                                let second = self.hex4();
                                0x1_0000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                            } else {
                                first
                            };
                            text.push(char::from_u32(code).expect("scalar value"));
                        }
                        b'n' => text.push('\n'),
                        b't' => text.push('\t'),
                        b'r' => text.push('\r'),
                        b'"' => text.push('"'),
                        b'\\' => text.push('\\'),
                        other => panic!("escape {:?}", other as char),
                    }
                }
                _ => {
                    // Copy the run of bytes up to the next quote or backslash.
                    let start = self.at;
                    while !matches!(self.peek(), b'"' | b'\\') {
                        self.at += 1;
                    }
                    text.push_str(std::str::from_utf8(&self.bytes[start..self.at]).expect("utf-8"));
                }
            }
        }
    }

    fn hex4(&mut self) -> u32 {
        let digits =
            std::str::from_utf8(&self.bytes[self.at..self.at + 4]).expect("ascii hex digits");
        self.at += 4;
        u32::from_str_radix(digits, 16).expect("hex")
    }
}

/// Each line is a description, the reference's canonical form, and its digest.
fn corpus() -> Vec<(String, String, String)> {
    CORPUS
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 3, "malformed line: {line:?}");
            (
                fields[0].to_string(),
                fields[1].to_string(),
                fields[2].to_string(),
            )
        })
        .collect()
}

#[test]
fn every_random_value_matches_the_reference() {
    let cases = corpus();
    assert!(cases.len() >= 4000, "only {} cases", cases.len());

    let mut failures = Vec::new();
    for (description, canonical_form, digest) in &cases {
        let mut reader = Reader::new(description);
        let value = reader.value();
        assert_eq!(
            reader.at,
            description.len(),
            "not all of {description:?} was read"
        );

        let got_canonical = canonical(&value);
        if got_canonical != *canonical_form {
            failures.push(format!(
                "{description}\n  canonical\n    expected {canonical_form:?}\n    got      {got_canonical:?}"
            ));
            continue;
        }
        let got_digest = hash(&value);
        if got_digest != *digest {
            failures.push(format!(
                "{description}\n  digest\n    expected {digest}\n    got      {got_digest}"
            ));
        }
    }

    assert!(
        failures.len() < 10,
        "{} failures, first few:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
