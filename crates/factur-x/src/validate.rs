//! Business rules.
//!
//! EN 16931 defines a long list of rules with names like `BR-CO-15`, and this
//! module implements the arithmetic ones. They are the rules that catch a
//! genuinely wrong invoice: a total that does not equal the sum of its parts, a
//! tax amount that does not follow from the rate, a missing field.
//!
//! The rules here are the ones a generator must satisfy to be worth anything.
//! They are not the full standard: rules about code lists, about country
//! specific profiles and about the PDF container are out of scope, and
//! [`validate`] says so rather than implying a document that passes is
//! conformant in every respect.

use crate::amount::Amount;
use crate::model::Invoice;

/// A rule that was broken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The rule's identifier, as the standard names it.
    pub rule: &'static str,
    /// What went wrong, in terms a user of the crate can act on.
    pub message: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.rule, self.message)
    }
}

/// The result of checking an invoice.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// Every rule that was broken, in the order the checks ran.
    pub violations: Vec<Violation>,
}

impl Report {
    /// Whether no rule was broken.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    /// The rule identifiers that were broken.
    #[must_use]
    pub fn rules(&self) -> Vec<&'static str> {
        self.violations.iter().map(|v| v.rule).collect()
    }

    fn fail(&mut self, rule: &'static str, message: impl Into<String>) {
        self.violations.push(Violation {
            rule,
            message: message.into(),
        });
    }
}

/// Checks the arithmetic rules of EN 16931.
///
/// The rules implemented are:
///
/// * `BR-01` to `BR-05`: the fields every invoice must carry.
/// * `BR-06` and `BR-09`: a party has a name and a two-letter country.
/// * `BR-16`: at least one invoice line.
/// * `BR-CO-10`: the sum of the line totals equals the document line total.
/// * `BR-CO-13`: with no allowances or charges, the total excluding tax equals
///   the line total.
/// * `BR-CO-14`: the tax total equals the sum of the breakdown.
/// * `BR-CO-15`: the total including tax equals the total excluding tax plus the
///   tax.
/// * `BR-CO-16`: the amount payable equals the total including tax minus what
///   was already paid.
/// * `BR-CO-17`: each breakdown's tax amount equals its taxable amount times its
///   rate, rounded to two decimals.
/// * `BR-CO-18`: at least one VAT breakdown entry.
/// * `BR-S-08`: the taxable amount of a standard-rated entry equals the sum of
///   the lines at that rate.
///
/// The rules that depend on data this model does not carry, such as allowances
/// and charges on the document level, are not checked.
#[must_use]
pub fn validate(invoice: &Invoice) -> Report {
    let mut report = Report::default();

    required(
        &mut report,
        "BR-01",
        "specification identifier",
        &invoice.guideline,
    );
    required(&mut report, "BR-02", "invoice number", &invoice.number);
    required(&mut report, "BR-03", "issue date", &invoice.issue_date);
    required(
        &mut report,
        "BR-04",
        "invoice type code",
        &invoice.type_code,
    );
    required(&mut report, "BR-05", "currency", invoice.currency.as_str());

    if invoice.lines.is_empty() {
        report.fail("BR-16", "an invoice must have at least one line");
    }
    if invoice.tax_breakdown.is_empty() {
        report.fail(
            "BR-CO-18",
            "an invoice must have at least one VAT breakdown entry",
        );
    }

    for party in [&invoice.seller, &invoice.buyer] {
        if party.name.trim().is_empty() {
            report.fail("BR-06", "a party must have a name");
        }
        if party.address.country.len() != 2 {
            report.fail(
                "BR-09",
                format!(
                    "country code {:?} must be two letters",
                    party.address.country
                ),
            );
        }
    }

    // BR-CO-10: the line totals must add up.
    let line_sum: i64 = invoice.lines.iter().map(|l| l.line_total.cents()).sum();
    if line_sum != invoice.totals.line_total.cents() {
        report.fail(
            "BR-CO-10",
            format!(
                "line totals add up to {} but the document says {}",
                Amount::from_cents(line_sum),
                invoice.totals.line_total
            ),
        );
    }

    // BR-CO-13: with no allowances or charges the total excluding tax is the
    // line total.
    if invoice.totals.tax_exclusive.cents() != invoice.totals.line_total.cents() {
        report.fail(
            "BR-CO-13",
            format!(
                "total excluding tax is {} but the line total is {}",
                invoice.totals.tax_exclusive, invoice.totals.line_total
            ),
        );
    }

    // BR-CO-15: the total including tax.
    let expected_inclusive = invoice
        .totals
        .tax_exclusive
        .cents()
        .saturating_add(invoice.totals.tax_total.cents());
    if expected_inclusive != invoice.totals.tax_inclusive.cents() {
        report.fail(
            "BR-CO-15",
            format!(
                "total including tax should be {} but is {}",
                Amount::from_cents(expected_inclusive),
                invoice.totals.tax_inclusive
            ),
        );
    }

    // BR-CO-16: what is payable after what was already paid.
    let prepaid = invoice.totals.prepaid.as_ref().map_or(0, Amount::cents);
    let expected_payable = invoice.totals.tax_inclusive.cents().saturating_sub(prepaid);
    if expected_payable != invoice.totals.due_payable.cents() {
        report.fail(
            "BR-CO-16",
            format!(
                "amount payable should be {} but is {}",
                Amount::from_cents(expected_payable),
                invoice.totals.due_payable
            ),
        );
    }

    // BR-CO-14: the tax total is the sum of the breakdown.
    let tax_sum: i64 = invoice
        .tax_breakdown
        .iter()
        .map(|t| t.tax_amount.cents())
        .sum();
    if tax_sum != invoice.totals.tax_total.cents() {
        report.fail(
            "BR-CO-14",
            format!(
                "breakdown tax adds up to {} but the document says {}",
                Amount::from_cents(tax_sum),
                invoice.totals.tax_total
            ),
        );
    }

    // BR-CO-17: each breakdown's tax follows from its rate.
    for entry in &invoice.tax_breakdown {
        let Some(rate) = entry.rate.as_deref().and_then(parse_rate) else {
            continue;
        };
        let expected = round_half_up(entry.taxable_amount.cents(), rate);
        if expected != entry.tax_amount.cents() {
            report.fail(
                "BR-CO-17",
                format!(
                    "category {} at rate {}: tax should be {} but is {}",
                    entry.category,
                    entry.rate.as_deref().unwrap_or(""),
                    Amount::from_cents(expected),
                    entry.tax_amount
                ),
            );
        }
    }

    // BR-S-08: the taxable amount of a standard-rated entry is the sum of the
    // lines charged at that rate.
    for entry in &invoice.tax_breakdown {
        if entry.category != "S" {
            continue;
        }
        let matching: i64 = invoice
            .lines
            .iter()
            .filter(|line| {
                line.vat_category == entry.category
                    && parse_rate(line.vat_rate.as_deref().unwrap_or(""))
                        == parse_rate(entry.rate.as_deref().unwrap_or(""))
            })
            .map(|line| line.line_total.cents())
            .sum();
        if matching != entry.taxable_amount.cents() {
            report.fail(
                "BR-S-08",
                format!(
                    "lines at category S rate {} add up to {} but the breakdown says {}",
                    entry.rate.as_deref().unwrap_or(""),
                    Amount::from_cents(matching),
                    entry.taxable_amount
                ),
            );
        }
    }

    report
}

/// A required text field must not be blank.
fn required(report: &mut Report, rule: &'static str, what: &str, value: &str) {
    if value.trim().is_empty() {
        report.fail(rule, format!("{what} is required"));
    }
}

/// Parses a percentage into hundredths of a percent, so `19` and `19.00` agree.
fn parse_rate(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    let mut parts = trimmed.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some() || whole.is_empty() {
        return None;
    }
    if !whole.bytes().all(|b| b.is_ascii_digit()) || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let whole_value: i64 = whole.parse().ok()?;
    let fraction_value: i64 = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().ok()? * 10,
        2 => fraction.parse().ok()?,
        _ => return None,
    };
    Some(whole_value * 100 + fraction_value)
}

/// Computes tax on an amount, rounding half away from zero to two decimals.
///
/// The standard fixes the result at two decimals, and rounding half up is what
/// the reference implementations do, so a generator that rounds differently
/// produces a document that fails the rule it was trying to satisfy.
fn round_half_up(amount_cents: i64, rate_hundredths: i64) -> i64 {
    let product = i128::from(amount_cents) * i128::from(rate_hundredths);
    let divisor = 10_000i128;
    let quotient = product / divisor;
    let remainder = product % divisor;
    let rounded = if remainder.abs() * 2 >= divisor {
        quotient + product.signum()
    } else {
        quotient
    };
    rounded as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{rich_invoice, sample_invoice};

    #[test]
    fn the_sample_invoice_is_valid() {
        let report = validate(&sample_invoice());
        assert!(report.is_valid(), "{:?}", report.violations);
    }

    #[test]
    fn the_rich_invoice_is_valid() {
        let report = validate(&rich_invoice());
        assert!(report.is_valid(), "{:?}", report.violations);
    }

    #[test]
    fn a_wrong_tax_amount_is_caught() {
        let mut invoice = sample_invoice();
        invoice.tax_breakdown[0].tax_amount = Amount::parse("180.00").unwrap();
        let report = validate(&invoice);
        assert!(report.rules().contains(&"BR-CO-17"), "{:?}", report.rules());
    }

    #[test]
    fn a_wrong_line_total_is_caught() {
        let mut invoice = sample_invoice();
        invoice.totals.line_total = Amount::parse("999.00").unwrap();
        let report = validate(&invoice);
        assert!(report.rules().contains(&"BR-CO-10"), "{:?}", report.rules());
    }

    #[test]
    fn a_wrong_payable_amount_is_caught() {
        let mut invoice = sample_invoice();
        invoice.totals.due_payable = Amount::parse("1.00").unwrap();
        let report = validate(&invoice);
        assert!(report.rules().contains(&"BR-CO-16"), "{:?}", report.rules());
    }

    #[test]
    fn a_wrong_tax_total_is_caught() {
        let mut invoice = sample_invoice();
        invoice.totals.tax_total = Amount::parse("180.00").unwrap();
        let report = validate(&invoice);
        assert!(report.rules().contains(&"BR-CO-14"), "{:?}", report.rules());
    }

    #[test]
    fn an_invoice_with_no_lines_is_caught() {
        let mut invoice = sample_invoice();
        invoice.lines.clear();
        let report = validate(&invoice);
        assert!(report.rules().contains(&"BR-16"));
    }

    #[test]
    fn a_blank_invoice_number_is_caught() {
        let mut invoice = sample_invoice();
        invoice.number = "  ".to_string();
        let report = validate(&invoice);
        assert!(report.rules().contains(&"BR-02"));
    }

    #[test]
    fn a_one_digit_country_code_is_caught() {
        let mut invoice = sample_invoice();
        invoice.seller.address.country = "D".to_string();
        let report = validate(&invoice);
        assert!(report.rules().contains(&"BR-09"));
    }

    #[test]
    fn tax_rounds_half_away_from_zero() {
        // 5% of 0.10 is 0.005, which rounds up to 0.01.
        assert_eq!(round_half_up(10, 500), 1);
        // 5% of 0.30 is 0.015, which rounds up to 0.02.
        assert_eq!(round_half_up(30, 500), 2);
        // 19% of 1000.00 is 190.00 exactly.
        assert_eq!(round_half_up(100_000, 1900), 19_000);
    }

    #[test]
    fn rates_are_compared_by_value_not_by_spelling() {
        assert_eq!(parse_rate("19"), parse_rate("19.00"));
        assert_eq!(parse_rate("7.5"), parse_rate("7.50"));
        assert_eq!(parse_rate("19.00"), Some(1900));
        assert_eq!(parse_rate(""), None);
    }

    #[test]
    fn a_line_at_a_different_rate_is_not_counted_in_a_standard_breakdown() {
        let mut invoice = sample_invoice();
        invoice.lines.push(crate::model::Line {
            id: "2".to_string(),
            name: "Book".to_string(),
            description: None,
            quantity: "1".to_string(),
            unit: "C62".to_string(),
            unit_price: Amount::parse("100.00").unwrap(),
            line_total: Amount::parse("100.00").unwrap(),
            vat_category: "S".to_string(),
            vat_rate: Some("7.00".to_string()),
        });
        invoice.totals.line_total = Amount::parse("1100.00").unwrap();
        invoice.totals.tax_exclusive = Amount::parse("1100.00").unwrap();
        invoice.totals.tax_total = Amount::parse("197.00").unwrap();
        invoice.totals.tax_inclusive = Amount::parse("1297.00").unwrap();
        invoice.totals.due_payable = Amount::parse("1297.00").unwrap();
        invoice.tax_breakdown.push(crate::model::TaxBreakdown {
            taxable_amount: Amount::parse("100.00").unwrap(),
            tax_amount: Amount::parse("7.00").unwrap(),
            category: "S".to_string(),
            rate: Some("7.00".to_string()),
            exemption_reason: None,
        });
        let report = validate(&invoice);
        assert!(report.is_valid(), "{:?}", report.violations);
    }
}
