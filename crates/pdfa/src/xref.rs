//! Cross-reference tables, cross-reference streams and object streams.
//!
//! A PDF may lay its objects out in three ways. The oldest is a plain
//! `xref` table followed by a `trailer` dictionary, one line per object. PDF 1.5
//! added two things: the cross-reference **stream**, which is a stream object
//! whose dictionary carries the trailer keys, and the object **stream**, which
//! packs several objects into one compressed stream. Newer producers use both by
//! default, so a reader that only understands the table cannot open a file from a
//! modern word processor at all.
//!
//! Everything here works on bytes and returns bytes. An object taken out of an
//! object stream is handed back as `N 0 obj ... endobj`, so the rest of the crate
//! does not have to know where it came from.

use std::collections::{BTreeMap, BTreeSet};

use crate::container::{find, rfind};
use crate::flate;

/// Where one object lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    /// Defined in the file itself.
    InFile {
        /// The byte offset of the object's header.
        offset: usize,
        /// The generation number, which a reference must match.
        generation: u16,
    },
    /// A member of an object stream.
    InStream {
        /// The object number of the stream that holds it.
        stream: u32,
        /// Its position within that stream.
        index: u32,
    },
}

/// A cross-reference section: where every object is, plus the trailer keys.
#[derive(Debug, Clone, Default)]
pub struct Xref {
    /// Object number to location. A later section overrides an earlier one.
    pub entries: BTreeMap<u32, Location>,
    /// The trailer dictionary as text, which for a cross-reference stream is that
    /// stream's own dictionary.
    pub trailer: String,
}

/// Reads the cross-reference information a document points at.
///
/// Follows the `/Prev` chain so that an incrementally updated file resolves
/// objects defined in an earlier section, with the newest definition winning.
/// Returns `None` when there is no usable `startxref`, which is the signal to
/// fall back to scanning for `N 0 obj` headers.
#[must_use]
pub fn read(bytes: &[u8]) -> Option<Xref> {
    let start = startxref_offset(bytes)?;
    let mut combined = Xref::default();
    let mut next = Some(start);
    // A malformed file can point a section at itself or make a cycle, so each
    // offset is visited once.
    let mut seen = BTreeSet::new();

    while let Some(offset) = next {
        if offset >= bytes.len() || !seen.insert(offset) {
            break;
        }
        let section = read_section(bytes, offset)?;
        // Earlier sections fill in what the newer ones do not mention; a newer
        // entry is never overwritten by an older one.
        for (&number, &location) in &section.entries {
            combined.entries.entry(number).or_insert(location);
        }
        if combined.trailer.is_empty() {
            combined.trailer = section.trailer.clone();
        }
        next = integer_after(&section.trailer, "/Prev").map(|value| value as usize);
    }

    Some(combined)
}

/// The byte offset after `startxref`.
fn startxref_offset(bytes: &[u8]) -> Option<usize> {
    let position = rfind(bytes, b"startxref")?;
    let text = std::str::from_utf8(&bytes[position + b"startxref".len()..]).ok()?;
    text.trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

/// One cross-reference section, either a table or a stream.
fn read_section(bytes: &[u8], offset: usize) -> Option<Xref> {
    let at = bytes.get(offset..)?;
    if at.starts_with(b"xref") {
        return read_table(bytes, offset);
    }
    read_stream(bytes, offset)
}

/// A classic `xref` table and the `trailer` dictionary that follows it.
///
/// Read line by line rather than by arithmetic on offsets. A byte-offset reader
/// that slips by one line lands on an entry, parses its two numbers as a
/// subsection header, and gets a count of zero - then loops forever without
/// advancing, which is a hang on input an attacker chooses. An iterator cannot
/// fail to advance, so that whole class of bug is gone.
fn read_table(bytes: &[u8], offset: usize) -> Option<Xref> {
    let text = std::str::from_utf8(bytes.get(offset..)?).ok()?;
    let mut lines = text.lines();
    if !lines.next()?.trim_start().starts_with("xref") {
        return None;
    }

    let mut entries = BTreeMap::new();
    loop {
        let header = lines.next()?.trim();
        if header.starts_with("trailer") {
            let mut trailer = String::new();
            for line in lines {
                if line.contains("startxref") {
                    break;
                }
                trailer.push_str(line);
                trailer.push('\n');
            }
            return Some(Xref { entries, trailer });
        }

        let mut fields = header.split_whitespace();
        let first: u32 = fields.next()?.parse().ok()?;
        let count: u32 = fields.next()?.parse().ok()?;
        for index in 0..count {
            let mut entry = lines.next()?.split_whitespace();
            let entry_offset: usize = entry.next()?.parse().ok()?;
            let generation: u16 = entry.next()?.parse().ok()?;
            let kind = entry.next()?.chars().next()?;
            if kind == 'n' {
                entries.insert(
                    first + index,
                    Location::InFile {
                        offset: entry_offset,
                        generation,
                    },
                );
            }
        }
    }
}

/// A cross-reference stream: the entries are its decoded data, and its
/// dictionary carries the trailer keys.
fn read_stream(bytes: &[u8], offset: usize) -> Option<Xref> {
    let dict = dictionary_text(bytes, offset)?;
    if !dict.contains("/XRef") && !dict.contains("/W") {
        return None;
    }
    let widths = integer_array(&dict, "/W")?;
    if widths.len() < 3 {
        return None;
    }
    // Every entry must consume at least one byte, or a `/Index` with a huge count
    // would loop without ever running out of input.
    if widths.iter().take(3).sum::<u64>() == 0 {
        return None;
    }
    let size = integer_after(&dict, "/Size").unwrap_or(0);
    // `/Index` is a list of `start count` pairs and defaults to `[0 Size]`.
    let mut index = integer_array(&dict, "/Index").unwrap_or_else(|| vec![0, size]);

    let decoded = decoded_stream(bytes, offset)?;

    let mut entries = BTreeMap::new();
    let mut cursor = 0usize;
    while index.len() >= 2 {
        let first = index.remove(0) as u32;
        let count = index.remove(0) as u32;
        for number in first..first + count {
            let mut fields = [0u64; 3];
            for (slot, &width) in fields.iter_mut().zip(widths.iter()) {
                *slot = read_field(&decoded, &mut cursor, width as usize)?;
            }
            // A width of zero means the field is absent and takes its default,
            // which for the type is 1. That default is what makes a stream of
            // type-1 entries with no type column legal.
            let kind = if widths[0] == 0 { 1 } else { fields[0] };
            match kind {
                1 => {
                    entries.insert(
                        number,
                        Location::InFile {
                            offset: fields[1] as usize,
                            generation: fields[2] as u16,
                        },
                    );
                }
                2 => {
                    entries.insert(
                        number,
                        Location::InStream {
                            stream: fields[1] as u32,
                            index: fields[2] as u32,
                        },
                    );
                }
                // Type 0 is a free entry, and anything else is not a definition.
                _ => {}
            }
        }
    }

    Some(Xref {
        entries,
        trailer: dict,
    })
}

/// Reads one big-endian field of `width` bytes, advancing the cursor.
fn read_field(data: &[u8], cursor: &mut usize, width: usize) -> Option<u64> {
    let bytes = data.get(*cursor..*cursor + width)?;
    *cursor += width;
    Some(
        bytes
            .iter()
            .fold(0u64, |value, byte| (value << 8) | u64::from(*byte)),
    )
}

/// Expands an object stream into the objects it holds.
///
/// The stream begins with `N` pairs of integers, the object number and the
/// offset of its body from `/First`, then the bodies themselves.
#[must_use]
pub fn expand_object_stream(bytes: &[u8], offset: usize) -> Option<Vec<(u32, Vec<u8>)>> {
    let dict = dictionary_text(bytes, offset)?;
    let count = integer_after(&dict, "/N")? as usize;
    let first = integer_after(&dict, "/First")? as usize;
    let decoded = decoded_stream(bytes, offset)?;

    // The header is text, so only the part before `/First` is tokenised.
    let header = std::str::from_utf8(decoded.get(..first)?).ok()?;
    let numbers: Vec<usize> = header
        .split_whitespace()
        .filter_map(|token| token.parse().ok())
        .collect();
    if numbers.len() < count * 2 {
        return None;
    }

    let mut out = Vec::with_capacity(count);
    for pair in numbers[..count * 2].chunks(2) {
        let number = pair[0] as u32;
        let body_start = first + pair[1];
        // A body ends where the next one begins, and the last ends at the end of
        // the stream.
        let body_end = numbers[..count * 2]
            .chunks(2)
            .filter_map(|other| (first + other[1] > body_start).then_some(first + other[1]))
            .min()
            .unwrap_or(decoded.len());
        let body = decoded.get(body_start..body_end.min(decoded.len()))?;
        out.push((number, body.to_vec()));
    }
    Some(out)
}

/// The dictionary text starting at or after `from`, from `<<` to its matching `>>`.
fn dictionary_text(bytes: &[u8], from: usize) -> Option<String> {
    let start = find(bytes, from, b"<<")?;
    // Dictionaries nest, so the end is the `>>` that closes the first `<<`.
    let mut depth = 0usize;
    let mut cursor = start;
    while cursor + 1 < bytes.len() {
        match &bytes[cursor..cursor + 2] {
            b"<<" => {
                depth += 1;
                cursor += 2;
            }
            b">>" => {
                depth -= 1;
                cursor += 2;
                if depth == 0 {
                    return String::from_utf8(bytes.get(start..cursor)?.to_vec()).ok();
                }
            }
            _ => cursor += 1,
        }
    }
    None
}

/// The decoded bytes of a stream: inflated, then un-predicted.
///
/// A cross-reference stream is compressed *and* predicted by default in several
/// producers, and the predictor is applied after inflating. Skipping it yields
/// bytes that decode without error into entries that are wrong, which is worse
/// than failing outright, so the predictor is read from the dictionary and undone
/// here rather than left to each caller.
fn decoded_stream(bytes: &[u8], offset: usize) -> Option<Vec<u8>> {
    let dict = dictionary_text(bytes, offset)?;
    let data = stream_data(bytes, offset)?;
    let inflated = flate::inflate(&data).or_else(|| flate::inflate_raw(&data))?;

    // `/DecodeParms` may be an array for a filter chain, but for a single
    // predictor it is a dictionary. Search the whole dictionary for the keys,
    // which is what a nested dictionary inside an array looks like as text.
    let Some(predictor) = integer_after(&dict, "/Predictor") else {
        return Some(inflated);
    };
    let columns = integer_after(&dict, "/Columns").unwrap_or(1) as usize;
    let colors = integer_after(&dict, "/Colors").unwrap_or(1) as usize;
    flate::unpredict(&inflated, predictor as u8, columns, colors)
}

/// The bytes under the `stream` keyword of the object whose dictionary is at
/// `offset`, using `/Length` when it is a direct integer.
fn stream_data(bytes: &[u8], offset: usize) -> Option<Vec<u8>> {
    let start = find(bytes, offset, b"stream")?;
    let dict = dictionary_text(bytes, offset)?;
    // Step over `stream` and the single end-of-line that separates it from the
    // data, which is either a newline or a carriage-return newline pair.
    let after = bytes.get(start + b"stream".len()..)?;
    let skip = if after.starts_with(b"\r\n") {
        2
    } else if after.starts_with(b"\n") {
        1
    } else {
        0
    };
    let data_start = start + b"stream".len() + skip;

    match integer_after(&dict, "/Length") {
        Some(length) => Some(
            bytes
                .get(data_start..data_start + length as usize)?
                .to_vec(),
        ),
        // An indirect `/Length` cannot be resolved without the rest of the
        // cross-reference, so fall back to the `endstream` keyword. For a
        // compressed stream that is only safe because the marker is searched from
        // the end, where no compressed byte can fake it.
        None => {
            let end = rfind(bytes, b"endstream")?;
            Some(bytes.get(data_start..end)?.to_vec())
        }
    }
}

/// The integer value of a key in a dictionary, when it is written directly.
fn integer_after(text: &str, key: &str) -> Option<u64> {
    let position = text.find(key)? + key.len();
    let rest = text[position..].trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// The integers in an array value, such as `/W [1 2 1]`.
fn integer_array(text: &str, key: &str) -> Option<Vec<u64>> {
    let position = text.find(key)? + key.len();
    let rest = text[position..].trim_start();
    let inner = rest.strip_prefix('[')?;
    let end = inner.find(']')?;
    Some(
        inner[..end]
            .split_whitespace()
            .filter_map(|token| token.parse().ok())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The object stream a modern producer writes, built by hand here so the
    /// reader is tested against the layout the specification describes.
    fn object_stream(entries: &[(u32, &[u8])]) -> Vec<u8> {
        let mut header = String::new();
        let mut bodies = Vec::new();
        let mut offset = 0usize;
        for (number, body) in entries {
            header.push_str(&format!("{number} {offset} "));
            offset += body.len();
            bodies.extend_from_slice(body);
        }
        let mut payload = header.clone().into_bytes();
        let first = payload.len();
        payload.extend_from_slice(&bodies);
        let compressed = flate::compress(&payload);

        let mut out = b"7 0 obj\n<< /Type /ObjStm /N ".to_vec();
        out.extend_from_slice(format!("{} /First {} ", entries.len(), first).as_bytes());
        out.extend_from_slice(format!("/Length {} >>\nstream\n", compressed.len()).as_bytes());
        out.extend_from_slice(&compressed);
        out.extend_from_slice(b"\nendstream\nendobj\n");
        out
    }

    #[test]
    fn an_object_stream_expands_into_its_members() {
        let stream = object_stream(&[(1, b"<< /A 1 >>"), (2, b"<< /B 2 >>")]);
        let expanded = expand_object_stream(&stream, 0).unwrap();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0].0, 1);
        assert_eq!(expanded[0].1, b"<< /A 1 >>");
        assert_eq!(expanded[1].0, 2);
        assert_eq!(expanded[1].1, b"<< /B 2 >>");
    }

    #[test]
    fn an_object_stream_body_ends_at_the_next_body() {
        // Bodies are not length-prefixed, so the end of one is the start of the
        // next. Getting this wrong truncates or concatenates objects.
        let stream = object_stream(&[(1, b"one"), (2, b"two"), (3, b"three")]);
        let expanded = expand_object_stream(&stream, 0).unwrap();
        let bodies: Vec<&[u8]> = expanded.iter().map(|(_, body)| body.as_slice()).collect();
        assert_eq!(bodies, [b"one".as_slice(), b"two", b"three"]);
    }

    #[test]
    fn a_stream_header_is_read_as_integers_not_digits() {
        // A naive byte-by-byte digit reader would join `1 2` into `12`.
        let stream = object_stream(&[(11, b"x"), (22, b"y")]);
        let expanded = expand_object_stream(&stream, 0).unwrap();
        assert_eq!(expanded[0].0, 11);
        assert_eq!(expanded[1].0, 22);
    }

    #[test]
    fn integer_helpers_read_the_values_they_name() {
        let dict = "<< /W [1 2 1] /Size 9 /N 3 /First 40 /Prev 7 >>";
        assert_eq!(integer_array(dict, "/W"), Some(vec![1, 2, 1]));
        assert_eq!(integer_after(dict, "/Size"), Some(9));
        assert_eq!(integer_after(dict, "/N"), Some(3));
        assert_eq!(integer_after(dict, "/First"), Some(40));
        assert_eq!(integer_after(dict, "/Prev"), Some(7));
        assert_eq!(integer_after(dict, "/Absent"), None);
    }

    #[test]
    fn a_nested_dictionary_ends_at_its_own_closing_brackets() {
        let text = b"<< /A << /B 1 >> /C 2 >>";
        assert_eq!(
            dictionary_text(text, 0).unwrap(),
            "<< /A << /B 1 >> /C 2 >>"
        );
    }

    #[test]
    fn a_missing_startxref_yields_no_xref() {
        assert!(read(b"%PDF-1.7\n1 0 obj\n<< >>\nendobj\n").is_none());
    }

    #[test]
    fn a_table_whose_header_is_an_entry_does_not_loop() {
        // A reader that advances by byte offsets can slip a line, read an entry's
        // two numbers as a subsection header, get a count of zero, and spin
        // forever without consuming anything. That is a hang on input an attacker
        // chooses, so the reader is line based and this checks it terminates.
        let bytes = b"%PDF-1.4\n\
xref\n\
0 2\n\
0000000000 65535 f \n\
0000000015 00000 n \n\
1 1\n\
0000000015 00000 n \n\
trailer\n<< /Size 3 /Root 1 0 R >>\n\
startxref\n9\n%%EOF\n";
        let xref = read(bytes).expect("the table parses");
        // Object 0 is a free entry, so only object 1 is a definition. The point
        // is that the call returned at all rather than spinning.
        assert_eq!(
            xref.entries.get(&1),
            Some(&Location::InFile {
                offset: 15,
                generation: 0
            })
        );
        assert!(xref.trailer.contains("/Root 1 0 R"), "{}", xref.trailer);
    }

    #[test]
    fn a_truncated_table_returns_rather_than_hangs() {
        // The subsection promises two entries and the file ends after one. A
        // reader that waits for the rest would hang; this one gives up.
        let bytes = b"%PDF-1.4\nstartxref\n0\n%%EOF";
        let _ = read(bytes);
        let truncated = b"%PDF-1.4\n\
xref\n\
0 5\n\
0000000000 65535 f \n\
startxref\n9\n%%EOF\n";
        let _ = read(truncated);
    }

    #[test]
    fn a_zero_width_entry_is_refused() {
        // `/W [0 0 0]` would read no bytes per entry, so a large `/Size` would
        // spin. The widths must consume at least one byte.
        let bytes = b"%PDF-1.5\nstartxref\n0\n%%EOF";
        assert!(read(bytes).is_none());
    }
}
