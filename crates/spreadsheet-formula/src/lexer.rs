//! Tokeniser for formula text.

use crate::Error;

/// A lexical token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A numeric literal.
    Number(f64),
    /// A string literal, with surrounding quotes removed and doubled quotes
    /// collapsed.
    Text(String),
    /// A cell reference such as `A1`, or a range when followed by `:`.
    Cell {
        /// Zero-based column index.
        column: u32,
        /// Zero-based row index.
        row: u32,
        /// Absolute column, written with `$`.
        abs_column: bool,
        /// Absolute row, written with `$`.
        abs_row: bool,
        /// Whether a sheet qualifier was present.
        sheet: Option<String>,
    },
    /// An identifier, which is a function name or a boolean literal.
    Ident(String),
    /// An operator or punctuation character.
    Op(char),
}

/// Splits `input` into tokens.
///
/// # Errors
///
/// Returns [`Error::Lex`] with the offset of the first problem.
pub fn tokenize(input: &str) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\n' | '\r' => i += 1,
            '"' => {
                let (text, next) = read_string(&chars, i)?;
                tokens.push(Token::Text(text));
                i = next;
            }
            '0'..='9' | '.' => {
                let (num, next) = read_number(&chars, i)?;
                tokens.push(Token::Number(num));
                i = next;
            }
            '\'' => {
                // A quoted sheet name, for example 'My Sheet'!A1.
                let (name, next) = read_quoted_sheet(&chars, i)?;
                let (cell, end) = read_cell(&chars, next, Some(name))?;
                tokens.push(cell);
                i = end;
            }
            c if c.is_alphabetic() || c == '_' || c == '$' => {
                let (cell, next) = read_cell(&chars, i, None)?;
                tokens.push(cell);
                i = next;
            }
            '+' | '-' | '*' | '/' | '^' | '&' | '%' | '=' | '<' | '>' | '(' | ')' | ',' | ':' => {
                tokens.push(Token::Op(c));
                i += 1;
            }
            other => {
                return Err(Error::Lex {
                    position: i,
                    message: format!("unexpected character {other:?}"),
                });
            }
        }
    }
    Ok(tokens)
}

fn read_string(chars: &[char], start: usize) -> Result<(String, usize), Error> {
    let mut out = String::new();
    let mut i = start + 1;
    while i < chars.len() {
        if chars[i] == '"' {
            // A doubled quote inside a string is an escaped quote.
            if i + 1 < chars.len() && chars[i + 1] == '"' {
                out.push('"');
                i += 2;
                continue;
            }
            return Ok((out, i + 1));
        }
        out.push(chars[i]);
        i += 1;
    }
    Err(Error::Lex {
        position: start,
        message: "unterminated string".into(),
    })
}

fn read_number(chars: &[char], start: usize) -> Result<(f64, usize), Error> {
    let mut i = start;
    let mut seen_dot = false;
    let mut seen_exp = false;
    while i < chars.len() {
        match chars[i] {
            '0'..='9' => i += 1,
            '.' if !seen_dot && !seen_exp => {
                seen_dot = true;
                i += 1;
            }
            'e' | 'E' if !seen_exp => {
                seen_exp = true;
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
            }
            _ => break,
        }
    }
    let text: String = chars[start..i].iter().collect();
    text.parse::<f64>().map(|n| (n, i)).map_err(|_| Error::Lex {
        position: start,
        message: format!("bad number {text:?}"),
    })
}

fn read_quoted_sheet(chars: &[char], start: usize) -> Result<(String, usize), Error> {
    let mut out = String::new();
    let mut i = start + 1;
    while i < chars.len() {
        if chars[i] == '\'' {
            if i + 1 < chars.len() && chars[i + 1] == '\'' {
                out.push('\'');
                i += 2;
                continue;
            }
            return Ok((out, i + 1));
        }
        out.push(chars[i]);
        i += 1;
    }
    Err(Error::Lex {
        position: start,
        message: "unterminated sheet name".into(),
    })
}

/// Reads an identifier or cell reference.
///
/// Decides between the two by looking for digits at the end: `A1` is a cell,
/// `SUM` is an identifier.
fn read_cell(chars: &[char], start: usize, sheet: Option<String>) -> Result<(Token, usize), Error> {
    // Called at "!" with a name already read, for example after a quoted sheet
    // name. Consume the separator and read the cell, keeping that name.
    if chars.get(start) == Some(&'!') {
        return read_cell(chars, start + 1, sheet);
    }
    let mut i = start;
    let mut abs_column = false;
    if i < chars.len() && chars[i] == '$' {
        abs_column = true;
        i += 1;
    }
    let letters_start = i;
    while i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '_') {
        i += 1;
    }
    let letters: String = chars[letters_start..i].iter().collect();

    let mut abs_row = false;
    if i < chars.len() && chars[i] == '$' {
        abs_row = true;
        i += 1;
    }
    let digits_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    let digits: String = chars[digits_start..i].iter().collect();

    // A sheet qualifier is followed by "!" and then the cell. The sheet name
    // is everything read so far, digits included, because names like "Sheet1"
    // are common.
    if i < chars.len() && chars[i] == '!' {
        let (cell, end) = read_cell(chars, i + 1, Some(format!("{letters}{digits}")))?;
        return Ok((cell, end));
    }

    if digits.is_empty() {
        if abs_column {
            return Err(Error::Lex {
                position: start,
                message: "expected a cell reference after '$'".into(),
            });
        }
        if letters.is_empty() {
            return Err(Error::Lex {
                position: start,
                message: "expected a name".into(),
            });
        }
        return Ok((Token::Ident(letters), i));
    }
    if letters.is_empty() {
        return Err(Error::Lex {
            position: start,
            message: "cell reference must start with a column letter".into(),
        });
    }
    let column = column_index(&letters);
    let row: u32 = digits.parse::<u32>().map_err(|_| Error::Lex {
        position: digits_start,
        message: format!("bad row {digits:?}"),
    })?;
    if row == 0 {
        return Err(Error::Lex {
            position: digits_start,
            message: "row numbers start at 1".into(),
        });
    }
    Ok((
        Token::Cell {
            column,
            row: row - 1,
            abs_column,
            abs_row,
            sheet,
        },
        i,
    ))
}

/// Converts a column name to a zero-based index, so `A` is 0 and `AA` is 26.
#[must_use]
pub fn column_index(letters: &str) -> u32 {
    letters
        .chars()
        .filter(char::is_ascii_alphabetic)
        .fold(0u32, |acc, c| {
            acc * 26 + (c.to_ascii_uppercase() as u32 - 'A' as u32 + 1)
        })
        .saturating_sub(1)
}

/// Converts a zero-based column index to its letters.
#[must_use]
pub fn column_name(mut index: u32) -> String {
    let mut out = Vec::new();
    loop {
        out.push((b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    out.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_numbers_operators_and_parens() {
        let t = tokenize("1+2.5*(3-1)").unwrap();
        assert_eq!(
            t,
            vec![
                Token::Number(1.0),
                Token::Op('+'),
                Token::Number(2.5),
                Token::Op('*'),
                Token::Op('('),
                Token::Number(3.0),
                Token::Op('-'),
                Token::Number(1.0),
                Token::Op(')'),
            ]
        );
    }

    #[test]
    fn tokenizes_cell_references() {
        let t = tokenize("A1").unwrap();
        assert_eq!(
            t,
            vec![Token::Cell {
                column: 0,
                row: 0,
                abs_column: false,
                abs_row: false,
                sheet: None
            }]
        );
        let t = tokenize("AB12").unwrap();
        assert_eq!(
            t,
            vec![Token::Cell {
                column: 27,
                row: 11,
                abs_column: false,
                abs_row: false,
                sheet: None
            }]
        );
    }

    #[test]
    fn tokenizes_absolute_references() {
        let t = tokenize("$A$1").unwrap();
        assert_eq!(
            t,
            vec![Token::Cell {
                column: 0,
                row: 0,
                abs_column: true,
                abs_row: true,
                sheet: None
            }]
        );
    }

    #[test]
    fn tokenizes_sheet_qualified_references() {
        let t = tokenize("Sheet1!B2").unwrap();
        assert_eq!(
            t,
            vec![Token::Cell {
                column: 1,
                row: 1,
                abs_column: false,
                abs_row: false,
                sheet: Some("Sheet1".into())
            }]
        );
        let t = tokenize("'My Sheet'!A1").unwrap();
        assert_eq!(
            t,
            vec![Token::Cell {
                column: 0,
                row: 0,
                abs_column: false,
                abs_row: false,
                sheet: Some("My Sheet".into())
            }]
        );
    }

    #[test]
    fn distinguishes_identifiers_from_cells() {
        assert_eq!(tokenize("SUM").unwrap(), vec![Token::Ident("SUM".into())]);
        assert_eq!(tokenize("SUM(").unwrap()[0], Token::Ident("SUM".into()));
    }

    #[test]
    fn tokenizes_strings_with_escaped_quotes() {
        assert_eq!(tokenize("\"hi\"").unwrap(), vec![Token::Text("hi".into())]);
        assert_eq!(
            tokenize("\"say \"\"hi\"\"\"").unwrap(),
            vec![Token::Text("say \"hi\"".into())]
        );
        assert!(tokenize("\"unterminated").is_err());
    }

    #[test]
    fn tokenizes_scientific_notation() {
        assert_eq!(tokenize("1e3").unwrap(), vec![Token::Number(1000.0)]);
        assert_eq!(tokenize("1.5E-2").unwrap(), vec![Token::Number(0.015)]);
    }

    #[test]
    fn rejects_bad_characters_and_rows() {
        assert!(matches!(tokenize("1 # 2"), Err(Error::Lex { .. })));
        assert!(matches!(tokenize("A0"), Err(Error::Lex { .. })));
        assert!(matches!(tokenize("$"), Err(Error::Lex { .. })));
    }

    #[test]
    fn column_index_and_name_round_trip() {
        assert_eq!(column_index("A"), 0);
        assert_eq!(column_index("Z"), 25);
        assert_eq!(column_index("AA"), 26);
        assert_eq!(column_index("AB"), 27);
        assert_eq!(column_index("ZZ"), 701);
        for i in [0u32, 1, 25, 26, 27, 701, 702] {
            assert_eq!(column_index(&column_name(i)), i, "column {i}");
        }
    }

    #[test]
    fn ranges_are_three_tokens() {
        let t = tokenize("A1:B3").unwrap();
        assert_eq!(t.len(), 3);
        assert_eq!(t[1], Token::Op(':'));
    }
}
