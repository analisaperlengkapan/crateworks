//! Every number asserted in `README.md`, checked against the crate.
//!
//! A README is documentation, but its numbers are claims about behaviour, and
//! they rot. This test is what keeps them true: if a score in the README stops
//! matching, this fails rather than the README quietly lying.

use fuzzy_score::{Scorer, dedupe, extract, extract_one};
use fuzzy_score::{partial_ratio, ratio, token_set_ratio, token_sort_ratio, w_ratio};

#[test]
fn readme() {
    assert_eq!(ratio("this is a test", "this is a test!"), 97);
    assert_eq!(partial_ratio("hello world", "hello"), 100);
    assert_eq!(ratio("hello world", "hello"), 62);
    assert_eq!(
        token_set_ratio(
            "new york mets vs atlanta braves",
            "atlanta braves vs new york mets"
        ),
        100
    );
    assert_eq!(w_ratio("new york mets", "the wonderful new york mets"), 90);

    let a = "mariners vs angels";
    let b = "los angeles angels at seattle mariners";
    assert_eq!(ratio(a, b), 29);
    assert_eq!(token_sort_ratio(a, b), 61);
    assert_eq!(token_set_ratio(a, b), 91);
    assert_eq!(w_ratio(a, b), 86);

    let teams = [
        "new york mets vs chicago cubs",
        "chicago cubs vs chicago white sox",
        "philadelphia phillies vs atlanta braves",
        "braves vs mets",
    ];
    let best = extract_one("new york mets at atlanta braves", teams, 0, Scorer::WRatio);
    assert_eq!(best.unwrap().0, "braves vs mets");
    let all = extract(
        "new york mets at atlanta braves",
        teams,
        Some(2),
        Scorer::WRatio,
    );
    assert_eq!(all.len(), 2);
    assert!(all[0].1 >= all[1].1);
    let unrelated = [
        "new york mets vs chicago cubs",
        "chicago cubs at new york mets",
        "atlanta braves vs pittsbugh pirates",
        "new york yankees vs boston red sox",
    ];
    let query = "los angeles dodgers vs san francisco giants";
    assert!(extract_one(query, unrelated, 50, Scorer::WRatio).is_none());

    let names = [
        "Frodo Baggin",
        "Frodo Baggins",
        "F. Baggins",
        "Samwise G.",
        "Gandalf",
    ];
    let unique = dedupe(names, 70);
    assert!(unique.contains(&"Frodo Baggins".to_string()));

    assert_eq!(ratio("a_b", "a b"), 67);
    assert_eq!(ratio("Cães", "Caes"), 75);
}
