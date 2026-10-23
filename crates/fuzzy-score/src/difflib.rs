//! A port of the part of CPython's `difflib.SequenceMatcher` that fuzzy
//! string scores are built on.
//!
//! `fuzzywuzzy.ratio` is `int(round(100 * SequenceMatcher(None, a, b).ratio()))`,
//! so a score is only reproducible if the matching blocks are. `difflib` uses
//! Ratcliff-Obershelp with an "autojunk" heuristic, and the details matter:
//!
//! * the popular-element cutoff only applies once `b` is at least 200 elements
//!   long, and elements are dropped when they occur more than `len(b)/100 + 1`
//!   times;
//! * the two extension passes re-admit those popular characters at the edges of
//!   a match, so dropping them from the index does not drop them from the score.
//!
//! Both are reproduced here. Elements are `char`, matching Python, where
//! iterating a `str` yields codepoints.

use std::collections::HashMap;

/// One matching run: `a[..] == b[..]` for `size` elements starting at `a` and `b`.
///
/// Field order is deliberate. Python sorts matching blocks as `(i, j, n)`
/// tuples, and the derived ordering here compares the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Match {
    /// Start index in the first sequence.
    pub(crate) a: usize,
    /// Start index in the second sequence.
    pub(crate) b: usize,
    /// Number of matching elements.
    pub(crate) size: usize,
}

/// The `difflib.SequenceMatcher` state `fuzzywuzzy` uses.
///
/// Constructed with `isjunk = None`, so no element is ever treated as junk.
#[derive(Debug)]
pub(crate) struct SequenceMatcher {
    a: Vec<char>,
    b: Vec<char>,
    /// For each element of `b`, the indices at which it occurs, excluding
    /// elements dropped by the autojunk heuristic.
    b2j: HashMap<char, Vec<usize>>,
}

impl SequenceMatcher {
    /// Builds the matcher for `a` and `b`, applying the autojunk heuristic
    /// when `autojunk` is set.
    pub(crate) fn new(a: &str, b: &str, autojunk: bool) -> Self {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();

        let mut b2j: HashMap<char, Vec<usize>> = HashMap::new();
        for (i, &c) in b.iter().enumerate() {
            b2j.entry(c).or_default().push(i);
        }

        if autojunk && b.len() >= 200 {
            let ntest = b.len() / 100 + 1;
            b2j.retain(|_, idxs| idxs.len() <= ntest);
        }

        Self { a, b, b2j }
    }

    /// `difflib.SequenceMatcher.find_longest_match`.
    fn find_longest_match(&self, alo: usize, ahi: usize, blo: usize, bhi: usize) -> Match {
        let (a, b) = (&self.a, &self.b);
        let (mut besti, mut bestj, mut bestsize) = (alo, blo, 0usize);

        // `j2len[j]` is the length of the longest match ending at `a[i - 1]`
        // and `b[j]`. Only the previous row is needed, so it is rebuilt each
        // iteration rather than stored in full.
        let mut j2len: HashMap<usize, usize> = HashMap::new();
        for (offset, &elem) in a[alo..ahi].iter().enumerate() {
            let i = alo + offset;
            let mut newj2len: HashMap<usize, usize> = HashMap::new();
            if let Some(positions) = self.b2j.get(&elem) {
                for &j in positions {
                    if j < blo {
                        continue;
                    }
                    if j >= bhi {
                        break;
                    }
                    let previous = j.checked_sub(1).and_then(|k| j2len.get(&k)).copied();
                    let k = previous.unwrap_or(0) + 1;
                    newj2len.insert(j, k);
                    if k > bestsize {
                        besti = i + 1 - k;
                        bestj = j + 1 - k;
                        bestsize = k;
                    }
                }
            }
            j2len = newj2len;
        }

        // Grow the match over characters the index skipped: popular elements
        // are absent from `b2j`, and the ends are the only place they can be
        // recovered cheaply.
        while besti > alo && bestj > blo && a[besti - 1] == b[bestj - 1] {
            besti -= 1;
            bestj -= 1;
            bestsize += 1;
        }
        while besti + bestsize < ahi
            && bestj + bestsize < bhi
            && a[besti + bestsize] == b[bestj + bestsize]
        {
            bestsize += 1;
        }

        Match {
            a: besti,
            b: bestj,
            size: bestsize,
        }
    }

    /// `difflib.SequenceMatcher.get_matching_blocks`.
    ///
    /// The returned blocks are ascending and non-overlapping, and the last one
    /// is the sentinel `(len(a), len(b), 0)`.
    pub(crate) fn get_matching_blocks(&self) -> Vec<Match> {
        let (la, lb) = (self.a.len(), self.b.len());

        // The recursion is flattened into a worklist, as CPython does, so that
        // a long input cannot exhaust the stack.
        let mut queue = vec![(0usize, la, 0usize, lb)];
        let mut blocks: Vec<Match> = Vec::new();
        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            let m = self.find_longest_match(alo, ahi, blo, bhi);
            if m.size > 0 {
                blocks.push(m);
                if alo < m.a && blo < m.b {
                    queue.push((alo, m.a, blo, m.b));
                }
                if m.a + m.size < ahi && m.b + m.size < bhi {
                    queue.push((m.a + m.size, ahi, m.b + m.size, bhi));
                }
            }
        }
        blocks.sort();

        // Collapse runs that ended up adjacent after sorting.
        let mut non_adjacent: Vec<Match> = Vec::new();
        let (mut i1, mut j1, mut k1) = (0usize, 0usize, 0usize);
        for m in blocks {
            if i1 + k1 == m.a && j1 + k1 == m.b {
                k1 += m.size;
            } else {
                if k1 > 0 {
                    non_adjacent.push(Match {
                        a: i1,
                        b: j1,
                        size: k1,
                    });
                }
                i1 = m.a;
                j1 = m.b;
                k1 = m.size;
            }
        }
        if k1 > 0 {
            non_adjacent.push(Match {
                a: i1,
                b: j1,
                size: k1,
            });
        }

        non_adjacent.push(Match {
            a: la,
            b: lb,
            size: 0,
        });
        non_adjacent
    }

    /// `difflib.SequenceMatcher.ratio`, a float in `0.0..=1.0`.
    ///
    /// Two empty sequences score `1.0`, matching `difflib`.
    pub(crate) fn ratio(&self) -> f64 {
        let matches: usize = self.get_matching_blocks().iter().map(|m| m.size).sum();
        let length = self.a.len() + self.b.len();
        if length > 0 {
            2.0 * matches as f64 / length as f64
        } else {
            1.0
        }
    }
}
