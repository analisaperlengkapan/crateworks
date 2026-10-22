//! The built-in function library.
//!
//! Every function receives flattened arguments: a scalar argument becomes a
//! single value and a range becomes one value per cell, in row-major order.
//! Aggregating functions therefore do not need to know whether they were given
//! `A1:A3` or `A1,A2,A3`.
//!
//! `IF` and `IFERROR` are not here, because they must not evaluate every
//! argument; the evaluator handles them directly.

use crate::Error;
use crate::value::{Error as ValueError, Value};

/// A flattened argument: one value for a scalar, many for a range.
#[derive(Debug, Clone)]
pub(crate) struct Arg(pub Vec<Value>);

impl Arg {
    fn numbers(&self) -> impl Iterator<Item = f64> + '_ {
        self.0.iter().filter_map(|v| v.as_number().ok())
    }

    /// Values that count as numeric: a number, a boolean, or text that reads as
    /// a number. An empty cell is not numeric, which is what `COUNT` requires.
    fn countable(&self) -> usize {
        self.0
            .iter()
            .filter(|v| !v.is_empty() && v.as_number().is_ok())
            .count()
    }
}

/// Calls a function by name.
pub(crate) fn call(name: &str, args: &[Arg]) -> Result<Value, Error> {
    match name {
        "SUM" => {
            require_at_least(name, args, 1)?;
            Ok(Value::Number(args.iter().flat_map(Arg::numbers).sum()))
        }
        "PRODUCT" => {
            require_at_least(name, args, 1)?;
            let product = args.iter().flat_map(Arg::numbers).product::<f64>();
            Ok(Value::Number(product))
        }
        "AVERAGE" => {
            require_at_least(name, args, 1)?;
            let values: Vec<f64> = args.iter().flat_map(Arg::numbers).collect();
            if values.is_empty() {
                return Ok(Value::Empty);
            }
            let count = values.len() as f64;
            Ok(Value::Number(values.iter().sum::<f64>() / count))
        }
        "MEDIAN" => {
            require_at_least(name, args, 1)?;
            let mut values: Vec<f64> = args.iter().flat_map(Arg::numbers).collect();
            if values.is_empty() {
                return Ok(Value::Empty);
            }
            values.sort_by(f64::total_cmp);
            let mid = values.len() / 2;
            let median = if values.len().is_multiple_of(2) {
                f64::midpoint(values[mid - 1], values[mid])
            } else {
                values[mid]
            };
            Ok(Value::Number(median))
        }
        "MIN" => {
            require_at_least(name, args, 1)?;
            Ok(min_max(args, true))
        }
        "MAX" => {
            require_at_least(name, args, 1)?;
            Ok(min_max(args, false))
        }
        "COUNT" => {
            require_at_least(name, args, 1)?;
            let n: usize = args.iter().map(Arg::countable).sum();
            Ok(Value::Number(n as f64))
        }
        "COUNTA" => {
            require_at_least(name, args, 1)?;
            let n = args
                .iter()
                .flat_map(|a| a.0.iter())
                .filter(|v| !v.is_empty())
                .count();
            Ok(Value::Number(n as f64))
        }
        "ABS" => unary_number(name, args, f64::abs),
        "SIGN" => unary_number(name, args, |n| {
            if n > 0.0 {
                1.0
            } else if n < 0.0 {
                -1.0
            } else {
                0.0
            }
        }),
        "SQRT" => unary_number(name, args, f64::sqrt),
        "INT" => unary_number(name, args, f64::floor),
        "ROUND" => {
            let (value, digits) = number_and_digits(name, args)?;
            Ok(Value::Number(round_to(value, digits, RoundMode::Half)))
        }
        "ROUNDUP" => {
            let (value, digits) = number_and_digits(name, args)?;
            Ok(Value::Number(round_to(value, digits, RoundMode::Up)))
        }
        "ROUNDDOWN" => {
            let (value, digits) = number_and_digits(name, args)?;
            Ok(Value::Number(round_to(value, digits, RoundMode::Down)))
        }
        "MOD" => {
            require_arity(name, args, 2)?;
            let a = first_number(name, &args[0])?;
            let b = first_number(name, &args[1])?;
            if b == 0.0 {
                return Err(Error::Arithmetic(crate::value::Error::DivisionByZero));
            }
            // Spreadsheet MOD follows the sign of the divisor.
            Ok(Value::Number(a - b * (a / b).floor()))
        }
        "POWER" => {
            require_arity(name, args, 2)?;
            let a = first_number(name, &args[0])?;
            let b = first_number(name, &args[1])?;
            Ok(Value::Number(a.powf(b)))
        }
        "PI" => {
            require_arity(name, args, 0)?;
            Ok(Value::Number(std::f64::consts::PI))
        }
        "COUNTIF" => {
            require_arity(name, args, 2)?;
            let matched = args[0].0.iter().filter(|v| criterion(v, &args[1])).count();
            Ok(Value::Number(matched as f64))
        }
        "SUMIF" => {
            require_between(name, args, 2, 3)?;
            let values = matching_values(args);
            Ok(Value::Number(values.iter().sum()))
        }
        "AVERAGEIF" => {
            require_between(name, args, 2, 3)?;
            let values = matching_values(args);
            if values.is_empty() {
                return Ok(Value::Empty);
            }
            let count = values.len() as f64;
            Ok(Value::Number(values.iter().sum::<f64>() / count))
        }
        "COUNTBLANK" => {
            require_arity(name, args, 1)?;
            let blanks = args[0]
                .0
                .iter()
                .filter(|v| v.is_empty() || v.to_string().trim().is_empty())
                .count();
            Ok(Value::Number(blanks as f64))
        }
        "IFS" => {
            // Pairs of condition and result, then an optional final fallback.
            // Every condition is already evaluated by the caller, so this cannot
            // short-circuit the way `IF` does; it picks, it does not save work.
            require_at_least(name, args, 2)?;
            let mut i = 0;
            while i + 1 < args.len() {
                if first_value(&args[i]).as_bool() {
                    return Ok(first_value(&args[i + 1]));
                }
                i += 2;
            }
            if args.len() % 2 == 1 {
                return Ok(first_value(&args[args.len() - 1]));
            }
            Err(Error::NoMatchingCondition)
        }
        "EXACT" => {
            require_arity(name, args, 2)?;
            let a = first_text(&args[0]);
            let b = first_text(&args[1]);
            Ok(Value::Bool(a == b))
        }
        "REPT" => {
            require_arity(name, args, 2)?;
            let text = first_text(&args[0]);
            let times = args[1]
                .0
                .first()
                .map_or(0.0, |v| v.as_number().unwrap_or(0.0));
            if times < 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Text(text.repeat(times as usize)))
        }
        "SUBSTITUTE" => {
            require_arity(name, args, 3)?;
            let text = first_text(&args[0]);
            let from = first_text(&args[1]);
            let to = first_text(&args[2]);
            // An empty needle would loop forever; a spreadsheet leaves the text
            // alone rather than hanging, and so does this.
            if from.is_empty() {
                return Ok(Value::Text(text));
            }
            Ok(Value::Text(text.replace(&from, &to)))
        }
        "LEN" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            Ok(Value::Number(text.chars().count() as f64))
        }
        "UPPER" => unary_text(name, args, str::to_uppercase),
        "LOWER" => unary_text(name, args, str::to_lowercase),
        "TRIM" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            Ok(Value::Text(
                text.split_whitespace().collect::<Vec<_>>().join(" "),
            ))
        }
        "CONCAT" | "CONCATENATE" => {
            require_at_least(name, args, 1)?;
            let mut out = String::new();
            for arg in args {
                for v in &arg.0 {
                    out.push_str(&v.to_string());
                }
            }
            Ok(Value::Text(out))
        }
        "LEFT" => {
            require_arity(name, args, 2)?;
            let text = first_text(&args[0]);
            let n = first_number(name, &args[1])? as usize;
            Ok(Value::Text(text.chars().take(n).collect()))
        }
        "RIGHT" => {
            require_arity(name, args, 2)?;
            let text = first_text(&args[0]);
            let n = first_number(name, &args[1])? as usize;
            let chars: Vec<char> = text.chars().collect();
            let start = chars.len().saturating_sub(n);
            Ok(Value::Text(chars[start..].iter().collect()))
        }
        "MID" => {
            require_arity(name, args, 3)?;
            let text = first_text(&args[0]);
            let start = first_number(name, &args[1])? as usize;
            let len = first_number(name, &args[2])? as usize;
            // MID is one-based, like a spreadsheet.
            let chars: Vec<char> = text.chars().collect();
            let from = start.saturating_sub(1).min(chars.len());
            let to = from.saturating_add(len).min(chars.len());
            Ok(Value::Text(chars[from..to].iter().collect()))
        }
        "AND" => {
            require_at_least(name, args, 1)?;
            Ok(Value::Bool(
                args.iter().flat_map(|a| a.0.iter()).all(Value::as_bool),
            ))
        }
        "OR" => {
            require_at_least(name, args, 1)?;
            Ok(Value::Bool(
                args.iter().flat_map(|a| a.0.iter()).any(Value::as_bool),
            ))
        }
        "NOT" => {
            require_arity(name, args, 1)?;
            Ok(Value::Bool(
                !args[0].0.first().unwrap_or(&Value::Empty).as_bool(),
            ))
        }
        other => Err(Error::UnknownFunction(other.to_string())),
    }
}

enum RoundMode {
    Half,
    Up,
    Down,
}

fn round_to(value: f64, digits: i32, mode: RoundMode) -> f64 {
    let factor = 10f64.powi(digits);
    let scaled = value * factor;
    let rounded = match mode {
        RoundMode::Half => scaled.round(),
        RoundMode::Up => {
            if scaled >= 0.0 {
                scaled.ceil()
            } else {
                scaled.floor()
            }
        }
        RoundMode::Down => {
            if scaled >= 0.0 {
                scaled.floor()
            } else {
                scaled.ceil()
            }
        }
    };
    rounded / factor
}

fn min_max(args: &[Arg], want_min: bool) -> Value {
    let values: Vec<f64> = args.iter().flat_map(Arg::numbers).collect();
    if values.is_empty() {
        return Value::Empty;
    }
    let mut best = values[0];
    for v in &values[1..] {
        if (want_min && *v < best) || (!want_min && *v > best) {
            best = *v;
        }
    }
    Value::Number(best)
}

fn unary_number(name: &str, args: &[Arg], f: impl Fn(f64) -> f64) -> Result<Value, Error> {
    require_arity(name, args, 1)?;
    Ok(Value::Number(f(first_number(name, &args[0])?)))
}

fn unary_text(name: &str, args: &[Arg], f: impl Fn(&str) -> String) -> Result<Value, Error> {
    require_arity(name, args, 1)?;
    Ok(Value::Text(f(&first_text(&args[0]))))
}

fn number_and_digits(name: &str, args: &[Arg]) -> Result<(f64, i32), Error> {
    require_arity(name, args, 2)?;
    let value = first_number(name, &args[0])?;
    let digits = first_number(name, &args[1])?;
    Ok((value, digits as i32))
}

fn first_number(name: &str, arg: &Arg) -> Result<f64, Error> {
    let value = arg.0.first().cloned().unwrap_or(Value::Empty);
    value.as_number().map_err(|_| Error::TypeError {
        operation: name.to_string(),
        found: value,
    })
}

/// The numbers `SUMIF` and `AVERAGEIF` should act on.
///
/// The first argument is tested against the criterion and the third holds the
/// numbers, and the two are aligned by position: the value summed for a match is
/// the one at the same index in the sum range. When the third argument is left
/// out the tested range is summed, which is the spreadsheet default and not the
/// same as summing the criterion.
fn matching_values(args: &[Arg]) -> Vec<f64> {
    let tested = &args[0];
    let sum_range = args.get(2).unwrap_or(&args[0]);
    tested
        .0
        .iter()
        .enumerate()
        .filter(|(_, value)| criterion(value, &args[1]))
        .filter_map(|(index, _)| sum_range.0.get(index).and_then(|v| v.as_number().ok()))
        .collect()
}

/// Requires an argument count within an inclusive range.
fn require_between(name: &str, args: &[Arg], minimum: usize, maximum: usize) -> Result<(), Error> {
    if args.len() < minimum || args.len() > maximum {
        return Err(Error::WrongArity {
            name: name.to_string(),
            given: args.len(),
            expected: "2 or 3",
        });
    }
    Ok(())
}

/// The first value of an argument, or `Empty`.
fn first_value(arg: &Arg) -> Value {
    arg.0.first().cloned().unwrap_or(Value::Empty)
}

/// Whether a value satisfies a criterion that may be text, a number, or a
/// comparison such as `">3"`.
///
/// The operators are the spreadsheet ones: `=`, `<>`, `<`, `<=`, `>`, `>=`. A
/// bare value means equality. Comparison of two numbers is numeric, and anything
/// else compares as text case-insensitively, which is what a spreadsheet does.
fn criterion(value: &Value, criterion_arg: &Arg) -> bool {
    let raw = first_text(criterion_arg);
    let (operator, rest) = match raw.as_bytes().first() {
        Some(b'<') if raw.starts_with("<=") => ("<=", &raw[2..]),
        Some(b'>') if raw.starts_with(">=") => (">=", &raw[2..]),
        Some(b'<') if raw.starts_with("<>") => ("<>", &raw[2..]),
        Some(b'<') => ("<", &raw[1..]),
        Some(b'>') => (">", &raw[1..]),
        Some(b'=') => ("=", &raw[1..]),
        _ => ("=", raw.as_str()),
    };
    let rest = rest.trim();

    // Two numbers compare numerically; anything else as text.
    let (left_number, right_number) = (value.as_number(), rest.parse::<f64>());
    let ordering = match (left_number, right_number) {
        (Ok(left), Ok(right)) => left.partial_cmp(&right),
        _ => {
            let left = value.to_string().to_lowercase();
            let right = rest.to_lowercase();
            Some(left.cmp(&right))
        }
    };

    match operator {
        "=" => match (value.as_number(), rest.parse::<f64>()) {
            (Ok(left), Ok(right)) => left == right,
            _ => value.to_string().eq_ignore_ascii_case(rest),
        },
        "<>" => match (value.as_number(), rest.parse::<f64>()) {
            (Ok(left), Ok(right)) => left != right,
            _ => !value.to_string().eq_ignore_ascii_case(rest),
        },
        "<" => ordering == Some(std::cmp::Ordering::Less),
        "<=" => ordering != Some(std::cmp::Ordering::Greater),
        ">" => ordering == Some(std::cmp::Ordering::Greater),
        ">=" => ordering != Some(std::cmp::Ordering::Less),
        _ => false,
    }
}

fn first_text(arg: &Arg) -> String {
    arg.0.first().cloned().unwrap_or(Value::Empty).to_string()
}

fn require_arity(name: &str, args: &[Arg], expected: usize) -> Result<(), Error> {
    if args.len() != expected {
        return Err(Error::WrongArity {
            name: name.to_string(),
            given: args.len(),
            expected: match expected {
                0 => "no",
                1 => "1",
                2 => "2",
                3 => "3",
                _ => "that many",
            },
        });
    }
    Ok(())
}

fn require_at_least(name: &str, args: &[Arg], minimum: usize) -> Result<(), Error> {
    if args.len() < minimum {
        return Err(Error::WrongArity {
            name: name.to_string(),
            given: args.len(),
            expected: "1 or more",
        });
    }
    Ok(())
}

/// Names of every function the crate implements.
///
/// This is public so that a caller can offer completion or validate a formula
/// against the supported set.
pub const FUNCTION_NAMES: &[&str] = &[
    "ABS",
    "AND",
    "AVERAGE",
    "AVERAGEIF",
    "CONCAT",
    "CONCATENATE",
    "COUNT",
    "COUNTA",
    "COUNTBLANK",
    "COUNTIF",
    "EXACT",
    "IFS",
    "INT",
    "LEFT",
    "LEN",
    "LOWER",
    "MAX",
    "MEDIAN",
    "MID",
    "MIN",
    "MOD",
    "NOT",
    "OR",
    "PI",
    "POWER",
    "PRODUCT",
    "REPT",
    "RIGHT",
    "ROUND",
    "ROUNDDOWN",
    "ROUNDUP",
    "SIGN",
    "SQRT",
    "SUBSTITUTE",
    "SUM",
    "SUMIF",
    "TRIM",
    "UPPER",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: f64) -> Arg {
        Arg(vec![Value::Number(v)])
    }

    fn nums(vs: &[f64]) -> Arg {
        Arg(vs.iter().map(|v| Value::Number(*v)).collect())
    }

    fn t(s: &str) -> Arg {
        Arg(vec![Value::Text(s.into())])
    }

    #[test]
    fn every_registered_name_has_an_implementation() {
        // `MID` was implemented but missing from `FUNCTION_NAMES`, so a caller
        // validating a formula against the published list rejected a function
        // that worked. This catches the next one.
        for name in FUNCTION_NAMES {
            let result = call(name, &[n(1.0)]);
            assert!(
                !matches!(result, Err(Error::UnknownFunction(_))),
                "{name} is advertised but not implemented"
            );
        }
    }

    #[test]
    fn countif_accepts_a_bare_value_and_a_comparison() {
        let range = Arg(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
            Value::Number(4.0),
        ]);
        assert_eq!(
            call("COUNTIF", &[range.clone(), n(3.0)]).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call("COUNTIF", &[range.clone(), t(">2")]).unwrap(),
            Value::Number(2.0)
        );
        assert_eq!(
            call("COUNTIF", &[range, t("<=2")]).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn countif_is_case_insensitive_over_text() {
        let range = Arg(vec![
            Value::Text("Yes".into()),
            Value::Text("yes".into()),
            Value::Text("no".into()),
        ]);
        assert_eq!(
            call("COUNTIF", &[range, t("YES")]).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn countblank_counts_empty_cells_and_empty_text() {
        let range = Arg(vec![
            Value::Empty,
            Value::Text(String::new()),
            Value::Number(0.0),
        ]);
        assert_eq!(call("COUNTBLANK", &[range]).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn sumif_uses_the_third_argument_as_the_sum_range() {
        // The bug this pins: summing the tested range instead of the sum range.
        // The criteria match the first two cells, whose values in the sum range
        // are 100 and 200, not 1 and 2.
        let tested = nums(&[1.0, 2.0, 3.0]);
        let sum_range = nums(&[100.0, 200.0, 300.0]);
        assert_eq!(
            call("SUMIF", &[tested.clone(), t("<=2"), sum_range.clone()]).unwrap(),
            Value::Number(300.0)
        );
        assert_eq!(
            call("AVERAGEIF", &[tested.clone(), t("<=2"), sum_range]).unwrap(),
            Value::Number(150.0)
        );
        // Without the third argument the tested range is summed.
        assert_eq!(
            call("SUMIF", &[tested, t("<=2")]).unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn ifs_returns_the_first_holding_condition() {
        let arg = |v: Value| Arg(vec![v]);
        assert_eq!(
            call(
                "IFS",
                &[
                    arg(Value::Bool(false)),
                    n(1.0),
                    arg(Value::Bool(true)),
                    n(2.0)
                ]
            )
            .unwrap(),
            Value::Number(2.0)
        );
        // With no condition holding, an odd count means the last argument is the
        // fallback and is returned.
        assert_eq!(
            call("IFS", &[arg(Value::Bool(false)), n(1.0), n(99.0)]).unwrap(),
            Value::Number(99.0)
        );
        // Without a fallback there is nothing to return, which is an error.
        assert_eq!(
            call("IFS", &[arg(Value::Bool(false)), n(1.0)]),
            Err(Error::NoMatchingCondition)
        );
    }

    #[test]
    fn exact_is_case_sensitive_where_a_comparison_is_not() {
        assert_eq!(
            call("EXACT", &[t("abc"), t("abc")]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            call("EXACT", &[t("abc"), t("ABC")]).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn mid_counts_from_one() {
        assert_eq!(
            call("MID", &[t("hello"), n(2.0), n(3.0)]).unwrap(),
            Value::Text("ell".into())
        );
        // Past the end is empty, not an error.
        assert_eq!(
            call("MID", &[t("hello"), n(10.0), n(3.0)]).unwrap(),
            Value::Text(String::new())
        );
    }

    #[test]
    fn rept_and_substitute_behave_on_the_edges() {
        assert_eq!(
            call("REPT", &[t("ab"), n(3.0)]).unwrap(),
            Value::Text("ababab".into())
        );
        // A zero count is empty rather than an error.
        assert_eq!(
            call("REPT", &[t("ab"), n(0.0)]).unwrap(),
            Value::Text(String::new())
        );
        assert_eq!(
            call("SUBSTITUTE", &[t("a-b-c"), t("-"), t("+")]).unwrap(),
            Value::Text("a+b+c".into())
        );
        // An empty needle must not hang.
        assert_eq!(
            call("SUBSTITUTE", &[t("abc"), t(""), t("x")]).unwrap(),
            Value::Text("abc".into())
        );
    }

    #[test]
    fn sum_over_scalars_and_ranges() {
        assert_eq!(call("SUM", &[n(1.0), n(2.0)]).unwrap(), Value::Number(3.0));
        assert_eq!(
            call("SUM", &[nums(&[1.0, 2.0, 3.0])]).unwrap(),
            Value::Number(6.0)
        );
        assert_eq!(
            call("SUM", &[nums(&[1.0, 2.0]), n(3.0)]).unwrap(),
            Value::Number(6.0)
        );
    }

    #[test]
    fn sum_ignores_text_and_empty() {
        let arg = Arg(vec![
            Value::Number(1.0),
            Value::Text("x".into()),
            Value::Empty,
        ]);
        assert_eq!(call("SUM", &[arg]).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn average_median_min_max() {
        let arg = nums(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(
            call("AVERAGE", std::slice::from_ref(&arg)).unwrap(),
            Value::Number(2.5)
        );
        assert_eq!(
            call("MEDIAN", std::slice::from_ref(&arg)).unwrap(),
            Value::Number(2.5)
        );
        assert_eq!(
            call("MIN", std::slice::from_ref(&arg)).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call("MAX", std::slice::from_ref(&arg)).unwrap(),
            Value::Number(4.0)
        );
        let odd = nums(&[3.0, 1.0, 2.0]);
        assert_eq!(call("MEDIAN", &[odd]).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn average_of_no_numbers_is_empty() {
        assert_eq!(call("AVERAGE", &[t("x")]).unwrap(), Value::Empty);
        assert_eq!(call("MIN", &[t("x")]).unwrap(), Value::Empty);
    }

    #[test]
    fn counting_functions_differ() {
        let arg = Arg(vec![
            Value::Number(1.0),
            Value::Text("x".into()),
            Value::Empty,
        ]);
        assert_eq!(
            call("COUNT", std::slice::from_ref(&arg)).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(call("COUNTA", &[arg]).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn rounding_modes() {
        assert_eq!(
            call("ROUND", &[n(2.5), n(0.0)]).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            call("ROUND", &[n(-2.5), n(0.0)]).unwrap(),
            Value::Number(-3.0)
        );
        assert_eq!(
            call("ROUND", &[n(2.345), n(2.0)]).unwrap(),
            Value::Number(2.35)
        );
        assert_eq!(
            call("ROUNDUP", &[n(2.1), n(0.0)]).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            call("ROUNDDOWN", &[n(2.9), n(0.0)]).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn mod_follows_divisor_sign() {
        assert_eq!(call("MOD", &[n(7.0), n(3.0)]).unwrap(), Value::Number(1.0));
        assert_eq!(call("MOD", &[n(-7.0), n(3.0)]).unwrap(), Value::Number(2.0));
        assert!(matches!(
            call("MOD", &[n(1.0), n(0.0)]),
            Err(Error::Arithmetic(crate::value::Error::DivisionByZero))
        ));
    }

    #[test]
    fn text_functions() {
        assert_eq!(call("LEN", &[t("hello")]).unwrap(), Value::Number(5.0));
        assert_eq!(
            call("UPPER", &[t("abc")]).unwrap(),
            Value::Text("ABC".into())
        );
        assert_eq!(
            call("LOWER", &[t("ABC")]).unwrap(),
            Value::Text("abc".into())
        );
        assert_eq!(
            call("TRIM", &[t("  a   b  ")]).unwrap(),
            Value::Text("a b".into())
        );
        assert_eq!(
            call("LEFT", &[t("hello"), n(2.0)]).unwrap(),
            Value::Text("he".into())
        );
        assert_eq!(
            call("RIGHT", &[t("hello"), n(2.0)]).unwrap(),
            Value::Text("lo".into())
        );
        assert_eq!(
            call("MID", &[t("hello"), n(2.0), n(3.0)]).unwrap(),
            Value::Text("ell".into())
        );
    }

    #[test]
    fn len_counts_characters_not_bytes() {
        assert_eq!(call("LEN", &[t("éé")]).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn concat_converts_numbers() {
        let arg = Arg(vec![Value::Text("n=".into()), Value::Number(18.0)]);
        assert_eq!(call("CONCAT", &[arg]).unwrap(), Value::Text("n=18".into()));
    }

    #[test]
    fn logical_functions() {
        let arg = Arg(vec![Value::Bool(true), Value::Bool(true)]);
        assert_eq!(
            call("AND", std::slice::from_ref(&arg)).unwrap(),
            Value::Bool(true)
        );
        let mixed = Arg(vec![Value::Bool(true), Value::Bool(false)]);
        assert_eq!(call("AND", &[mixed]).unwrap(), Value::Bool(false));
        let mixed = Arg(vec![Value::Bool(true), Value::Bool(false)]);
        assert_eq!(call("OR", &[mixed]).unwrap(), Value::Bool(true));
        assert_eq!(
            call("NOT", &[Arg(vec![Value::Bool(false)])]).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn arity_errors_are_reported() {
        assert!(matches!(call("ABS", &[]), Err(Error::WrongArity { .. })));
        assert!(matches!(
            call("ABS", &[n(1.0), n(2.0)]),
            Err(Error::WrongArity { .. })
        ));
        assert!(matches!(call("SUM", &[]), Err(Error::WrongArity { .. })));
        assert!(matches!(call("NOPE", &[]), Err(Error::UnknownFunction(_))));
    }

    #[test]
    fn every_advertised_function_is_callable() {
        for name in FUNCTION_NAMES {
            // A wrong-arity or type error still proves the name is dispatched;
            // only an unknown-function error would mean the list is wrong.
            if let Err(Error::UnknownFunction(n)) = call(name, &[]) {
                panic!("{n} is listed in FUNCTION_NAMES but not implemented");
            }
        }
    }

    #[test]
    fn pi_and_sqrt_and_power() {
        assert_eq!(
            call("PI", &[]).unwrap(),
            Value::Number(std::f64::consts::PI)
        );
        assert_eq!(call("SQRT", &[n(9.0)]).unwrap(), Value::Number(3.0));
        assert_eq!(
            call("POWER", &[n(2.0), n(10.0)]).unwrap(),
            Value::Number(1024.0)
        );
        assert_eq!(
            call("PRODUCT", &[n(2.0), n(3.0), n(4.0)]).unwrap(),
            Value::Number(24.0)
        );
    }
}
