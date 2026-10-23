//! Choosing the best match from a list, matching Python's `fuzzywuzzy.process`.

use crate::fuzz;
use crate::utils::full_process;

/// Which score to rank choices by.
///
/// This mirrors `fuzzywuzzy`'s ability to pass any `fuzz` function as the
/// `scorer` argument, and preserves its optimisation of preprocessing the query
/// and each choice only once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scorer {
    /// [`fuzz::w_ratio`]. The default, and the best general choice.
    #[default]
    WRatio,
    /// [`fuzz::q_ratio`].
    QRatio,
    /// [`fuzz::token_set_ratio`].
    TokenSetRatio,
    /// [`fuzz::token_sort_ratio`].
    TokenSortRatio,
    /// [`fuzz::partial_token_set_ratio`].
    PartialTokenSetRatio,
    /// [`fuzz::partial_token_sort_ratio`].
    PartialTokenSortRatio,
    /// [`fuzz::uw_ratio`].
    UWRatio,
    /// [`fuzz::uq_ratio`].
    UQRatio,
    /// [`fuzz::ratio`].
    Ratio,
    /// [`fuzz::partial_ratio`].
    PartialRatio,
}

impl Scorer {
    /// Scores a pair that has already been through the processor.
    ///
    /// The token and weighted scorers must not preprocess again, so they are
    /// called in their raw form. `ratio` and `partial_ratio` never preprocess
    /// on their own, so they are called directly.
    fn score_processed(self, a: &str, b: &str) -> u8 {
        match self {
            Scorer::WRatio => fuzz::w_ratio_body(a, b, fuzz::Options::RAW),
            Scorer::QRatio => fuzz::q_ratio_body(a, b, fuzz::Options::RAW),
            Scorer::TokenSetRatio => fuzz::token_set_ratio_with_options(a, b, fuzz::Options::RAW),
            Scorer::TokenSortRatio => fuzz::token_sort_ratio_with_options(a, b, fuzz::Options::RAW),
            Scorer::PartialTokenSetRatio => {
                fuzz::partial_token_set_ratio_with_options(a, b, fuzz::Options::RAW)
            }
            Scorer::PartialTokenSortRatio => {
                fuzz::partial_token_sort_ratio_with_options(a, b, fuzz::Options::RAW)
            }
            Scorer::UWRatio => fuzz::w_ratio_body(a, b, fuzz::Options::RAW),
            Scorer::UQRatio => fuzz::q_ratio_body(a, b, fuzz::Options::RAW),
            Scorer::Ratio => fuzz::ratio(a, b),
            Scorer::PartialRatio => fuzz::partial_ratio(a, b),
        }
    }

    /// Whether the processor folds to ASCII.
    ///
    /// This follows `fuzzywuzzy.process`, which preprocesses with
    /// `force_ascii=True` for the ASCII-named scorers and `False` for the
    /// `u`-prefixed ones. `ratio` and `partial_ratio` are not on that list, so
    /// they go through the default processor, which also uses `False`.
    fn processor_force_ascii(self) -> bool {
        matches!(
            self,
            Scorer::WRatio
                | Scorer::QRatio
                | Scorer::TokenSetRatio
                | Scorer::TokenSortRatio
                | Scorer::PartialTokenSetRatio
                | Scorer::PartialTokenSortRatio
        )
    }

    /// Scores two raw strings the way `fuzzywuzzy.process` does.
    ///
    /// Both inputs are run through [`full_process`] first, because that is the
    /// default `processor`. Note that this means
    /// `Scorer::Ratio.score(a, b)` is *not* the same as [`fuzz::ratio`]:
    /// the former compares the processed forms, the latter the raw ones.
    pub fn score(self, a: &str, b: &str) -> u8 {
        let ascii = self.processor_force_ascii();
        self.score_processed(&full_process(a, ascii), &full_process(b, ascii))
    }
}

/// Port of `fuzzywuzzy.process.extract`.
///
/// Returns every choice with a score at or above `score_cutoff`, best first,
/// as `(choice, score)` pairs. Ties keep the order the choices were given in.
/// With `limit` set to `Some(n)` only the best `n` are returned.
///
/// ```
/// use fuzzy_score::{extract, Scorer};
///
/// let choices = ["chicago cubs vs new york mets", "braves vs mets"];
/// let best = extract("new york mets at chicago cubs", choices, Some(1), Scorer::WRatio);
/// assert_eq!(best[0].0, "chicago cubs vs new york mets");
/// ```
pub fn extract<S: AsRef<str>>(
    query: &str,
    choices: impl IntoIterator<Item = S>,
    limit: Option<usize>,
    scorer: Scorer,
) -> Vec<(String, u8)> {
    let mut scored = extract_without_order(query, choices, 0, scorer);
    // A stable sort keeps equal scores in the original order, which is what
    // Python's `heapq.nlargest` does with its decorated tuples.
    scored.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    if let Some(n) = limit {
        scored.truncate(n);
    }
    scored
}

/// Port of `fuzzywuzzy.process.extractOne`.
///
/// The single best choice, or `None` when nothing reaches `score_cutoff`. On a
/// tie the earlier choice wins, matching Python's `max`.
///
/// ```
/// use fuzzy_score::{extract_one, Scorer};
///
/// let choices = ["new york mets vs chicago cubs", "braves vs mets"];
/// let best = extract_one("new york mets at atlanta braves", choices, 0, Scorer::WRatio);
/// assert_eq!(best.unwrap().0, "braves vs mets");
/// ```
pub fn extract_one<S: AsRef<str>>(
    query: &str,
    choices: impl IntoIterator<Item = S>,
    score_cutoff: u8,
    scorer: Scorer,
) -> Option<(String, u8)> {
    extract_without_order(query, choices, score_cutoff, scorer)
        .into_iter()
        .fold(None, |best, candidate| match best {
            Some((_, s)) if s >= candidate.1 => best,
            _ => Some(candidate),
        })
}

/// `fuzzywuzzy.process.extractWithoutOrder`.
fn extract_without_order<S: AsRef<str>>(
    query: &str,
    choices: impl IntoIterator<Item = S>,
    score_cutoff: u8,
    scorer: Scorer,
) -> Vec<(String, u8)> {
    let ascii = scorer.processor_force_ascii();
    let processed_query = full_process(query, ascii);

    let mut results = Vec::new();
    for choice in choices {
        let choice = choice.as_ref();
        let processed = full_process(choice, ascii);
        let score = scorer.score_processed(&processed_query, &processed);
        if score >= score_cutoff {
            results.push((choice.to_string(), score));
        }
    }
    results
}

/// Port of `fuzzywuzzy.process.dedupe`.
///
/// Removes near-duplicates from a list. Any two entries scoring above
/// `threshold` are treated as the same, and the longest spelling is kept, on
/// the grounds that it carries the most information. Length ties are broken
/// alphabetically.
///
/// ```
/// use fuzzy_score::dedupe;
///
/// let names = ["Frodo Baggin", "Frodo Baggins", "Samwise G.", "Gandalf"];
/// let unique = dedupe(names, 70);
/// assert!(unique.contains(&"Frodo Baggins".to_string()));
/// assert!(!unique.contains(&"Frodo Baggin".to_string()));
/// ```
pub fn dedupe<S: AsRef<str>>(choices: impl IntoIterator<Item = S>, threshold: u8) -> Vec<String> {
    let choices: Vec<String> = choices
        .into_iter()
        .map(|c| c.as_ref().to_string())
        .collect();

    let mut kept: Vec<String> = Vec::new();
    for item in &choices {
        let matches = extract(item, choices.iter(), None, Scorer::TokenSetRatio);
        let mut filtered: Vec<(String, u8)> = matches
            .into_iter()
            .filter(|(_, score)| *score > threshold)
            .collect();

        if filtered.len() == 1 {
            kept.push(filtered[0].0.clone());
        } else {
            filtered.sort_by(|a, b| a.0.cmp(&b.0));
            filtered.sort_by_key(|(s, _)| std::cmp::Reverse(s.chars().count()));
            kept.push(filtered[0].0.clone());
        }
    }

    // Preserve first-seen order while removing repeats.
    let mut seen = std::collections::HashSet::new();
    kept.retain(|s| seen.insert(s.clone()));

    if kept.len() == choices.len() {
        choices
    } else {
        kept
    }
}
