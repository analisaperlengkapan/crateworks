//! Recursive-descent parser turning tokens into an expression tree.
//!
//! Operator precedence follows the spreadsheet convention, from lowest to
//! highest: comparison, concatenation with `&`, addition, multiplication,
//! exponentiation, unary sign, percent, then primary expressions. Comparison is
//! left-associative like the rest, which differs from mathematics but matches
//! what spreadsheet users see.

use crate::Error;
use crate::lexer::{Token, tokenize};

/// A parsed formula expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal number.
    Number(f64),
    /// A literal string.
    Text(String),
    /// A literal boolean, written as `TRUE` or `FALSE`.
    Bool(bool),
    /// A reference to a single cell.
    Cell {
        /// Zero-based column.
        column: u32,
        /// Zero-based row.
        row: u32,
        /// Sheet qualifier, if any.
        sheet: Option<String>,
    },
    /// A rectangular range between two corners, inclusive.
    Range {
        /// First corner.
        start: Box<Expr>,
        /// Second corner.
        end: Box<Expr>,
        /// Sheet qualifier, if any.
        sheet: Option<String>,
    },
    /// A unary operation.
    Unary {
        /// The operator, one of `-`, `+` or `%`.
        op: char,
        /// Operand.
        operand: Box<Expr>,
    },
    /// A binary operation.
    Binary {
        /// The operator.
        op: BinOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },
    /// A function call.
    Call {
        /// Function name, upper-cased.
        name: String,
        /// Arguments in order.
        args: Vec<Expr>,
    },
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Subtract,
    /// `*`
    Multiply,
    /// `/`
    Divide,
    /// `^`
    Power,
    /// `&`, text concatenation.
    Concat,
    /// `=`
    Equal,
    /// `<>`
    NotEqual,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,
}

impl BinOp {
    /// The operator as it is written.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Power => "^",
            Self::Concat => "&",
            Self::Equal => "=",
            Self::NotEqual => "<>",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
        }
    }
}

/// Parses a formula, with or without a leading `=`.
///
/// # Errors
///
/// Returns [`Error::Lex`] or [`Error::Parse`].
///
/// # Example
///
/// ```
/// use spreadsheet_formula::{parse_formula, Expr};
///
/// // The leading `=` is optional.
/// assert_eq!(parse_formula("=1+2").unwrap(), parse_formula("1+2").unwrap());
/// assert!(matches!(parse_formula("1+2").unwrap(), Expr::Binary { .. }));
///
/// assert!(parse_formula("1+").is_err());
/// ```
pub fn parse_formula(input: &str) -> Result<Expr, Error> {
    let text = input.strip_prefix('=').unwrap_or(input);
    let tokens = tokenize(text)?;
    if tokens.is_empty() {
        return Err(Error::Parse {
            position: 0,
            message: "empty formula".into(),
        });
    }
    let mut p = Parser { tokens, pos: 0 };
    let expr = p.parse_comparison()?;
    if p.pos != p.tokens.len() {
        return Err(Error::Parse {
            position: p.pos,
            message: format!("unexpected trailing token {:?}", p.tokens[p.pos]),
        });
    }
    Ok(expr)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn eat_op(&mut self, c: char) -> bool {
        if self.peek() == Some(&Token::Op(c)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn parse_comparison(&mut self) -> Result<Expr, Error> {
        let mut left = self.parse_concat()?;
        loop {
            let op = if self.eat_op('=') {
                BinOp::Equal
            } else if self.eat_op('<') {
                if self.eat_op('>') {
                    BinOp::NotEqual
                } else if self.eat_op('=') {
                    BinOp::LessEqual
                } else {
                    BinOp::Less
                }
            } else if self.eat_op('>') {
                if self.eat_op('=') {
                    BinOp::GreaterEqual
                } else {
                    BinOp::Greater
                }
            } else {
                return Ok(left);
            };
            let right = self.parse_concat()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
    }

    fn parse_concat(&mut self) -> Result<Expr, Error> {
        let mut left = self.parse_additive()?;
        while self.eat_op('&') {
            let right = self.parse_additive()?;
            left = Expr::Binary {
                op: BinOp::Concat,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, Error> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = if self.eat_op('+') {
                BinOp::Add
            } else if self.eat_op('-') {
                BinOp::Subtract
            } else {
                return Ok(left);
            };
            let right = self.parse_multiplicative()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, Error> {
        let mut left = self.parse_power()?;
        loop {
            let op = if self.eat_op('*') {
                BinOp::Multiply
            } else if self.eat_op('/') {
                BinOp::Divide
            } else {
                return Ok(left);
            };
            let right = self.parse_power()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
    }

    fn parse_power(&mut self) -> Result<Expr, Error> {
        let base = self.parse_unary()?;
        if self.eat_op('^') {
            // Right-associative, so 2^3^2 is 2^(3^2).
            let exponent = self.parse_power()?;
            return Ok(Expr::Binary {
                op: BinOp::Power,
                left: Box::new(base),
                right: Box::new(exponent),
            });
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<Expr, Error> {
        if self.eat_op('-') {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: '-',
                operand: Box::new(operand),
            });
        }
        if self.eat_op('+') {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: '+',
                operand: Box::new(operand),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, Error> {
        let mut expr = self.parse_primary()?;
        while self.eat_op('%') {
            expr = Expr::Unary {
                op: '%',
                operand: Box::new(expr),
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, Error> {
        let token = self.peek().cloned().ok_or_else(|| Error::Parse {
            position: self.pos,
            message: "unexpected end of formula".into(),
        })?;
        match token {
            Token::Number(n) => {
                self.pos += 1;
                Ok(Expr::Number(n))
            }
            Token::Text(t) => {
                self.pos += 1;
                Ok(Expr::Text(t))
            }
            Token::Ident(name) => {
                self.pos += 1;
                let upper = name.to_ascii_uppercase();
                if self.eat_op('(') {
                    let args = self.parse_args()?;
                    return Ok(Expr::Call { name: upper, args });
                }
                match upper.as_str() {
                    "TRUE" => Ok(Expr::Bool(true)),
                    "FALSE" => Ok(Expr::Bool(false)),
                    _ => Err(Error::Parse {
                        position: self.pos - 1,
                        message: format!("{name:?} is not a cell reference, function or boolean"),
                    }),
                }
            }
            Token::Cell {
                column, row, sheet, ..
            } => {
                self.pos += 1;
                let start = Expr::Cell {
                    column,
                    row,
                    sheet: sheet.clone(),
                };
                if self.eat_op(':') {
                    let end = self.parse_primary()?;
                    return Ok(Expr::Range {
                        start: Box::new(start),
                        end: Box::new(end),
                        sheet,
                    });
                }
                Ok(start)
            }
            Token::Op('(') => {
                self.pos += 1;
                let inner = self.parse_comparison()?;
                if !self.eat_op(')') {
                    return Err(Error::Parse {
                        position: self.pos,
                        message: "expected closing parenthesis".into(),
                    });
                }
                Ok(inner)
            }
            other => Err(Error::Parse {
                position: self.pos,
                message: format!("unexpected token {other:?}"),
            }),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, Error> {
        let mut args = Vec::new();
        if self.eat_op(')') {
            return Ok(args);
        }
        loop {
            args.push(self.parse_comparison()?);
            if self.eat_op(',') {
                continue;
            }
            if self.eat_op(')') {
                return Ok(args);
            }
            return Err(Error::Parse {
                position: self.pos,
                message: "expected ',' or ')' in argument list".into(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(n: f64) -> Expr {
        Expr::Number(n)
    }

    fn bin(op: BinOp, l: Expr, r: Expr) -> Expr {
        Expr::Binary {
            op,
            left: Box::new(l),
            right: Box::new(r),
        }
    }

    #[test]
    fn parses_arithmetic_with_precedence() {
        assert_eq!(
            parse_formula("1+2*3").unwrap(),
            bin(
                BinOp::Add,
                num(1.0),
                bin(BinOp::Multiply, num(2.0), num(3.0))
            )
        );
    }

    #[test]
    fn parentheses_override_precedence() {
        assert_eq!(
            parse_formula("(1+2)*3").unwrap(),
            bin(
                BinOp::Multiply,
                bin(BinOp::Add, num(1.0), num(2.0)),
                num(3.0)
            )
        );
    }

    #[test]
    fn power_is_right_associative() {
        assert_eq!(
            parse_formula("2^3^2").unwrap(),
            bin(
                BinOp::Power,
                num(2.0),
                bin(BinOp::Power, num(3.0), num(2.0))
            )
        );
    }

    #[test]
    fn unary_minus_binds_tighter_than_power_base() {
        // -2^2 parses as (-2)^2 in spreadsheet precedence for the unary sign.
        let parsed = parse_formula("-2^2").unwrap();
        assert_eq!(
            parsed,
            bin(
                BinOp::Power,
                Expr::Unary {
                    op: '-',
                    operand: Box::new(num(2.0))
                },
                num(2.0)
            )
        );
    }

    #[test]
    fn leading_equals_is_optional() {
        assert_eq!(
            parse_formula("=1+1").unwrap(),
            parse_formula("1+1").unwrap()
        );
    }

    #[test]
    fn parses_comparison_operators() {
        assert!(matches!(
            parse_formula("A1<>B1").unwrap(),
            Expr::Binary {
                op: BinOp::NotEqual,
                ..
            }
        ));
        assert!(matches!(
            parse_formula("A1<=B1").unwrap(),
            Expr::Binary {
                op: BinOp::LessEqual,
                ..
            }
        ));
        assert!(matches!(
            parse_formula("A1>=B1").unwrap(),
            Expr::Binary {
                op: BinOp::GreaterEqual,
                ..
            }
        ));
        assert!(matches!(
            parse_formula("A1<B1").unwrap(),
            Expr::Binary {
                op: BinOp::Less,
                ..
            }
        ));
    }

    #[test]
    fn parses_function_calls_including_nested() {
        let e = parse_formula("SUM(A1:A3, MAX(B1,B2))").unwrap();
        let Expr::Call { name, args } = e else {
            panic!("expected call, got {e:?}")
        };
        assert_eq!(name, "SUM");
        assert_eq!(args.len(), 2);
        assert!(matches!(args[0], Expr::Range { .. }));
        assert!(matches!(&args[1], Expr::Call { name, .. } if name == "MAX"));
    }

    #[test]
    fn parses_zero_argument_calls() {
        let e = parse_formula("PI()").unwrap();
        assert_eq!(
            e,
            Expr::Call {
                name: "PI".into(),
                args: vec![]
            }
        );
    }

    #[test]
    fn parses_booleans_and_text() {
        assert_eq!(parse_formula("TRUE").unwrap(), Expr::Bool(true));
        assert_eq!(parse_formula("FALSE").unwrap(), Expr::Bool(false));
        assert_eq!(parse_formula("\"hi\"").unwrap(), Expr::Text("hi".into()));
    }

    #[test]
    fn parses_percent_as_postfix() {
        let e = parse_formula("50%").unwrap();
        assert_eq!(
            e,
            Expr::Unary {
                op: '%',
                operand: Box::new(num(50.0))
            }
        );
    }

    #[test]
    fn rejects_trailing_tokens_and_empty_input() {
        assert!(matches!(parse_formula("1 2"), Err(Error::Parse { .. })));
        assert!(matches!(parse_formula(""), Err(Error::Parse { .. })));
        assert!(matches!(parse_formula("="), Err(Error::Parse { .. })));
    }

    #[test]
    fn rejects_unbalanced_parentheses() {
        assert!(matches!(parse_formula("(1+2"), Err(Error::Parse { .. })));
        assert!(matches!(parse_formula("1+2)"), Err(Error::Parse { .. })));
    }

    #[test]
    fn rejects_bare_identifiers() {
        assert!(matches!(parse_formula("FOO"), Err(Error::Parse { .. })));
    }
}
