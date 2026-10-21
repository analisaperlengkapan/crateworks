//! Contextual shaping.

use crate::forms::{self, FINAL, INITIAL, ISOLATED, MEDIAL};
use crate::{TATWEEL, ZWJ, is_harakat};

/// Which order the shaped text should be written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Keep the logical order: the first letter of the word comes first in the
    /// string. Suitable when something else does the bidi reordering, such as a
    /// browser or a text layout engine.
    Logical,
    /// Reverse the shaped text so it reads correctly when drawn left to right.
    /// This is what a terminal, an image and a PDF generator need.
    RightToLeft,
}

/// How shaping should treat marks, tatweel and ligatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Keep combining marks and attach them to the letter they follow.
    pub keep_harakat: bool,
    /// Keep the tatweel rather than removing it.
    pub keep_tatweel: bool,
    /// Replace the standard ligatures, such as lam-alef.
    pub ligatures: bool,
    /// Treat the zero-width joiner as meaningful rather than removing it.
    pub support_zwj: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            keep_harakat: true,
            keep_tatweel: true,
            ligatures: true,
            support_zwj: false,
        }
    }
}

impl Options {
    /// Keep everything and use ligatures.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            keep_harakat: true,
            keep_tatweel: true,
            ligatures: true,
            support_zwj: false,
        }
    }

    /// Drop combining marks and tatweel, for a display that cannot position
    /// them.
    #[must_use]
    pub const fn plain() -> Self {
        Self {
            keep_harakat: false,
            keep_tatweel: false,
            ligatures: true,
            support_zwj: false,
        }
    }
}

/// A base character together with the marks that follow it.
#[derive(Debug, Clone)]
struct Cluster {
    base: Option<char>,
    marks: Vec<char>,
}

impl Cluster {
    fn text(&self) -> String {
        let mut out = String::new();
        if let Some(base) = self.base {
            out.push(base);
        }
        out.extend(self.marks.iter());
        out
    }
}

/// Shapes text with the default options.
///
/// See [`reshape_with`] for the configurable form and [`Direction`] for what the
/// direction argument does.
#[must_use]
pub fn reshape(text: &str, direction: Direction) -> String {
    reshape_with(text, direction, Options::default())
}

/// Shapes text, replacing each Arabic letter with the presentation form that
/// matches its neighbours.
///
/// Non-Arabic characters pass through unchanged, and a character with no Arabic
/// shape breaks the joining chain, which is why Latin text inside an Arabic
/// sentence does not join to it.
#[must_use]
pub fn reshape_with(text: &str, direction: Direction, options: Options) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut clusters: Vec<Cluster> = Vec::new();
    for c in text.chars() {
        if is_harakat(c) {
            if !options.keep_harakat {
                continue;
            }
            // A mark before any letter has nothing to attach to; keep it as its
            // own cluster rather than dropping it.
            match clusters.last_mut() {
                Some(last) => last.marks.push(c),
                None => clusters.push(Cluster {
                    base: None,
                    marks: vec![c],
                }),
            }
            continue;
        }
        if c == TATWEEL && !options.keep_tatweel {
            continue;
        }
        if c == ZWJ && !options.support_zwj {
            continue;
        }
        clusters.push(Cluster {
            base: Some(c),
            marks: Vec::new(),
        });
    }

    let bases: Vec<Option<char>> = clusters.iter().map(|c| c.base).collect();
    let mut shaped: Vec<String> = Vec::with_capacity(clusters.len());
    for (index, cluster) in clusters.iter().enumerate() {
        let Some(base) = cluster.base else {
            shaped.push(cluster.text());
            continue;
        };
        let Some(shapes) = forms::lookup(base) else {
            shaped.push(cluster.text());
            continue;
        };

        let joins_previous = matches!(
            neighbour_before(&bases, index, options),
            Some(previous) if previous == ZWJ || forms::joins_after(previous)
        );
        let joins_next = matches!(
            neighbour_after(&bases, index, options),
            Some(next) if next == ZWJ || forms::joins_before(next)
        );

        let mut rendered = String::new();
        rendered.push(select_shape(shapes, joins_previous, joins_next, base));
        rendered.extend(cluster.marks.iter());
        shaped.push(rendered);
    }

    if direction == Direction::RightToLeft {
        shaped.reverse();
    }

    let joined: String = shaped.concat();
    if options.ligatures {
        crate::ligatures::apply(&joined)
    } else {
        joined
    }
}

/// The base character before `index`, skipping clusters with no base.
fn neighbour_before(bases: &[Option<char>], index: usize, options: Options) -> Option<char> {
    for position in (0..index).rev() {
        match bases[position] {
            Some(c) if c == ZWJ && !options.support_zwj => continue,
            Some(c) => return Some(c),
            None => continue,
        }
    }
    None
}

/// The base character after `index`, skipping clusters with no base.
fn neighbour_after(bases: &[Option<char>], index: usize, options: Options) -> Option<char> {
    for candidate in &bases[index + 1..] {
        match candidate {
            Some(c) if *c == ZWJ && !options.support_zwj => continue,
            Some(c) => return Some(*c),
            None => continue,
        }
    }
    None
}

/// Picks the presentation form for a letter.
///
/// A letter that cannot take a medial shape still connects on whichever side it
/// can, which is why the medial case falls back to the final shape.
fn select_shape(
    shapes: forms::Shapes,
    joins_previous: bool,
    joins_next: bool,
    letter: char,
) -> char {
    let index = match (joins_previous, joins_next) {
        (true, true) if shapes[MEDIAL].is_some() => MEDIAL,
        (true, true) => FINAL,
        (true, false) => FINAL,
        (false, true) => INITIAL,
        (false, false) => ISOLATED,
    };
    shapes[index].or(shapes[ISOLATED]).unwrap_or(letter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin_text_is_untouched() {
        assert_eq!(reshape("hello world", Direction::Logical), "hello world");
        assert_eq!(reshape("", Direction::Logical), "");
    }

    #[test]
    fn a_single_letter_stays_isolated() {
        assert_eq!(reshape("\u{0628}", Direction::Logical), "\u{fe8f}");
    }

    #[test]
    fn two_joining_letters_become_initial_and_final() {
        assert_eq!(
            reshape("\u{0628}\u{0628}", Direction::Logical),
            "\u{fe91}\u{fe90}"
        );
    }

    #[test]
    fn three_joining_letters_take_a_medial_shape() {
        assert_eq!(
            reshape("\u{0628}\u{0628}\u{0628}", Direction::Logical),
            "\u{fe91}\u{fe92}\u{fe90}"
        );
    }

    #[test]
    fn a_letter_that_does_not_join_forward_breaks_the_chain() {
        // Alef is first, so it is isolated, and it does not join forward, so
        // the beh after it is isolated too.
        assert_eq!(
            reshape("\u{0627}\u{0628}", Direction::Logical),
            "\u{fe8d}\u{fe8f}"
        );
    }

    #[test]
    fn latin_inside_arabic_breaks_the_joining_chain() {
        assert_eq!(
            reshape("\u{0628}x\u{0628}", Direction::Logical),
            "\u{fe8f}x\u{fe8f}"
        );
    }

    #[test]
    fn right_to_left_reverses_the_clusters() {
        let logical = reshape("\u{0628}\u{0628}", Direction::Logical);
        let reversed = reshape("\u{0628}\u{0628}", Direction::RightToLeft);
        assert_eq!(reversed.chars().rev().collect::<String>(), logical);
    }

    #[test]
    fn marks_stay_with_their_letter_when_reversing() {
        // Beh with a fatha, then alef. Reversing swaps the two letters but the
        // fatha stays behind the beh it belongs to.
        let reversed = reshape("\u{0628}\u{064e}\u{0627}", Direction::RightToLeft);
        assert_eq!(reversed, "\u{fe8e}\u{fe91}\u{064e}");
    }

    #[test]
    fn marks_can_be_dropped() {
        assert_eq!(
            reshape_with("\u{0628}\u{064e}", Direction::Logical, Options::plain()),
            "\u{fe8f}"
        );
    }

    #[test]
    fn tatweel_can_be_kept_or_dropped() {
        let text = "\u{0628}\u{0640}\u{0628}";
        assert_eq!(reshape(text, Direction::Logical).chars().count(), 3);
        assert_eq!(
            reshape_with(text, Direction::Logical, Options::plain())
                .chars()
                .count(),
            2
        );
    }

    #[test]
    fn lam_alef_becomes_a_single_ligature() {
        assert_eq!(reshape("\u{0644}\u{0627}", Direction::Logical), "\u{fefb}");
    }

    #[test]
    fn a_word_containing_lam_alef_keeps_its_other_letters() {
        assert_eq!(
            reshape(
                "\u{0627}\u{0644}\u{0633}\u{0644}\u{0627}\u{0645}",
                Direction::Logical
            ),
            "\u{fe8d}\u{fedf}\u{feb4}\u{fefc}\u{fee1}"
        );
    }
}
