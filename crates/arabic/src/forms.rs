//! The Arabic presentation forms.
//!
//! Each entry maps a letter to its isolated, initial, medial and final shapes.
//! A letter that cannot join on a side has no shape for that side, which is
//! exactly how the joining behaviour is recorded: a letter with no initial form
//! cannot join to the following letter, and one with no final form cannot join
//! to the preceding letter.
//!
//! The table is the one used by the MIT-licensed `arabic-reshaper` library,
//! which in turn follows the Unicode Arabic Presentation Forms-A and -B blocks.

/// The four contextual shapes of a letter, in the order isolated, initial,
/// medial, final.
pub(crate) type Shapes = [Option<char>; 4];

/// The index of each shape inside a [`Shapes`] array.
pub(crate) const ISOLATED: usize = 0;
/// Index of the initial shape.
pub(crate) const INITIAL: usize = 1;
/// Index of the medial shape.
pub(crate) const MEDIAL: usize = 2;
/// Index of the final shape.
pub(crate) const FINAL: usize = 3;

/// The letters this crate can shape.
const LETTERS: [(char, Shapes); 78] = [
    ('\u{0621}', [Some('\u{fe80}'), None, None, None]),
    ('\u{0622}', [Some('\u{fe81}'), None, None, Some('\u{fe82}')]),
    ('\u{0623}', [Some('\u{fe83}'), None, None, Some('\u{fe84}')]),
    ('\u{0624}', [Some('\u{fe85}'), None, None, Some('\u{fe86}')]),
    ('\u{0625}', [Some('\u{fe87}'), None, None, Some('\u{fe88}')]),
    (
        '\u{0626}',
        [
            Some('\u{fe89}'),
            Some('\u{fe8b}'),
            Some('\u{fe8c}'),
            Some('\u{fe8a}'),
        ],
    ),
    ('\u{0627}', [Some('\u{fe8d}'), None, None, Some('\u{fe8e}')]),
    (
        '\u{0628}',
        [
            Some('\u{fe8f}'),
            Some('\u{fe91}'),
            Some('\u{fe92}'),
            Some('\u{fe90}'),
        ],
    ),
    ('\u{0629}', [Some('\u{fe93}'), None, None, Some('\u{fe94}')]),
    (
        '\u{062a}',
        [
            Some('\u{fe95}'),
            Some('\u{fe97}'),
            Some('\u{fe98}'),
            Some('\u{fe96}'),
        ],
    ),
    (
        '\u{062b}',
        [
            Some('\u{fe99}'),
            Some('\u{fe9b}'),
            Some('\u{fe9c}'),
            Some('\u{fe9a}'),
        ],
    ),
    (
        '\u{062c}',
        [
            Some('\u{fe9d}'),
            Some('\u{fe9f}'),
            Some('\u{fea0}'),
            Some('\u{fe9e}'),
        ],
    ),
    (
        '\u{062d}',
        [
            Some('\u{fea1}'),
            Some('\u{fea3}'),
            Some('\u{fea4}'),
            Some('\u{fea2}'),
        ],
    ),
    (
        '\u{062e}',
        [
            Some('\u{fea5}'),
            Some('\u{fea7}'),
            Some('\u{fea8}'),
            Some('\u{fea6}'),
        ],
    ),
    ('\u{062f}', [Some('\u{fea9}'), None, None, Some('\u{feaa}')]),
    ('\u{0630}', [Some('\u{feab}'), None, None, Some('\u{feac}')]),
    ('\u{0631}', [Some('\u{fead}'), None, None, Some('\u{feae}')]),
    ('\u{0632}', [Some('\u{feaf}'), None, None, Some('\u{feb0}')]),
    (
        '\u{0633}',
        [
            Some('\u{feb1}'),
            Some('\u{feb3}'),
            Some('\u{feb4}'),
            Some('\u{feb2}'),
        ],
    ),
    (
        '\u{0634}',
        [
            Some('\u{feb5}'),
            Some('\u{feb7}'),
            Some('\u{feb8}'),
            Some('\u{feb6}'),
        ],
    ),
    (
        '\u{0635}',
        [
            Some('\u{feb9}'),
            Some('\u{febb}'),
            Some('\u{febc}'),
            Some('\u{feba}'),
        ],
    ),
    (
        '\u{0636}',
        [
            Some('\u{febd}'),
            Some('\u{febf}'),
            Some('\u{fec0}'),
            Some('\u{febe}'),
        ],
    ),
    (
        '\u{0637}',
        [
            Some('\u{fec1}'),
            Some('\u{fec3}'),
            Some('\u{fec4}'),
            Some('\u{fec2}'),
        ],
    ),
    (
        '\u{0638}',
        [
            Some('\u{fec5}'),
            Some('\u{fec7}'),
            Some('\u{fec8}'),
            Some('\u{fec6}'),
        ],
    ),
    (
        '\u{0639}',
        [
            Some('\u{fec9}'),
            Some('\u{fecb}'),
            Some('\u{fecc}'),
            Some('\u{feca}'),
        ],
    ),
    (
        '\u{063a}',
        [
            Some('\u{fecd}'),
            Some('\u{fecf}'),
            Some('\u{fed0}'),
            Some('\u{fece}'),
        ],
    ),
    (
        '\u{0640}',
        [
            Some('\u{0640}'),
            Some('\u{0640}'),
            Some('\u{0640}'),
            Some('\u{0640}'),
        ],
    ),
    (
        '\u{0641}',
        [
            Some('\u{fed1}'),
            Some('\u{fed3}'),
            Some('\u{fed4}'),
            Some('\u{fed2}'),
        ],
    ),
    (
        '\u{0642}',
        [
            Some('\u{fed5}'),
            Some('\u{fed7}'),
            Some('\u{fed8}'),
            Some('\u{fed6}'),
        ],
    ),
    (
        '\u{0643}',
        [
            Some('\u{fed9}'),
            Some('\u{fedb}'),
            Some('\u{fedc}'),
            Some('\u{feda}'),
        ],
    ),
    (
        '\u{0644}',
        [
            Some('\u{fedd}'),
            Some('\u{fedf}'),
            Some('\u{fee0}'),
            Some('\u{fede}'),
        ],
    ),
    (
        '\u{0645}',
        [
            Some('\u{fee1}'),
            Some('\u{fee3}'),
            Some('\u{fee4}'),
            Some('\u{fee2}'),
        ],
    ),
    (
        '\u{0646}',
        [
            Some('\u{fee5}'),
            Some('\u{fee7}'),
            Some('\u{fee8}'),
            Some('\u{fee6}'),
        ],
    ),
    (
        '\u{0647}',
        [
            Some('\u{fee9}'),
            Some('\u{feeb}'),
            Some('\u{feec}'),
            Some('\u{feea}'),
        ],
    ),
    ('\u{0648}', [Some('\u{feed}'), None, None, Some('\u{feee}')]),
    (
        '\u{0649}',
        [
            Some('\u{feef}'),
            Some('\u{fbe8}'),
            Some('\u{fbe9}'),
            Some('\u{fef0}'),
        ],
    ),
    (
        '\u{064a}',
        [
            Some('\u{fef1}'),
            Some('\u{fef3}'),
            Some('\u{fef4}'),
            Some('\u{fef2}'),
        ],
    ),
    ('\u{0671}', [Some('\u{fb50}'), None, None, Some('\u{fb51}')]),
    ('\u{0677}', [Some('\u{fbdd}'), None, None, None]),
    (
        '\u{0679}',
        [
            Some('\u{fb66}'),
            Some('\u{fb68}'),
            Some('\u{fb69}'),
            Some('\u{fb67}'),
        ],
    ),
    (
        '\u{067a}',
        [
            Some('\u{fb5e}'),
            Some('\u{fb60}'),
            Some('\u{fb61}'),
            Some('\u{fb5f}'),
        ],
    ),
    (
        '\u{067b}',
        [
            Some('\u{fb52}'),
            Some('\u{fb54}'),
            Some('\u{fb55}'),
            Some('\u{fb53}'),
        ],
    ),
    (
        '\u{067e}',
        [
            Some('\u{fb56}'),
            Some('\u{fb58}'),
            Some('\u{fb59}'),
            Some('\u{fb57}'),
        ],
    ),
    (
        '\u{067f}',
        [
            Some('\u{fb62}'),
            Some('\u{fb64}'),
            Some('\u{fb65}'),
            Some('\u{fb63}'),
        ],
    ),
    (
        '\u{0680}',
        [
            Some('\u{fb5a}'),
            Some('\u{fb5c}'),
            Some('\u{fb5d}'),
            Some('\u{fb5b}'),
        ],
    ),
    (
        '\u{0683}',
        [
            Some('\u{fb76}'),
            Some('\u{fb78}'),
            Some('\u{fb79}'),
            Some('\u{fb77}'),
        ],
    ),
    (
        '\u{0684}',
        [
            Some('\u{fb72}'),
            Some('\u{fb74}'),
            Some('\u{fb75}'),
            Some('\u{fb73}'),
        ],
    ),
    (
        '\u{0686}',
        [
            Some('\u{fb7a}'),
            Some('\u{fb7c}'),
            Some('\u{fb7d}'),
            Some('\u{fb7b}'),
        ],
    ),
    (
        '\u{0687}',
        [
            Some('\u{fb7e}'),
            Some('\u{fb80}'),
            Some('\u{fb81}'),
            Some('\u{fb7f}'),
        ],
    ),
    ('\u{0688}', [Some('\u{fb88}'), None, None, Some('\u{fb89}')]),
    ('\u{068c}', [Some('\u{fb84}'), None, None, Some('\u{fb85}')]),
    ('\u{068d}', [Some('\u{fb82}'), None, None, Some('\u{fb83}')]),
    ('\u{068e}', [Some('\u{fb86}'), None, None, Some('\u{fb87}')]),
    ('\u{0691}', [Some('\u{fb8c}'), None, None, Some('\u{fb8d}')]),
    ('\u{0698}', [Some('\u{fb8a}'), None, None, Some('\u{fb8b}')]),
    (
        '\u{06a4}',
        [
            Some('\u{fb6a}'),
            Some('\u{fb6c}'),
            Some('\u{fb6d}'),
            Some('\u{fb6b}'),
        ],
    ),
    (
        '\u{06a6}',
        [
            Some('\u{fb6e}'),
            Some('\u{fb70}'),
            Some('\u{fb71}'),
            Some('\u{fb6f}'),
        ],
    ),
    (
        '\u{06a9}',
        [
            Some('\u{fb8e}'),
            Some('\u{fb90}'),
            Some('\u{fb91}'),
            Some('\u{fb8f}'),
        ],
    ),
    (
        '\u{06ad}',
        [
            Some('\u{fbd3}'),
            Some('\u{fbd5}'),
            Some('\u{fbd6}'),
            Some('\u{fbd4}'),
        ],
    ),
    (
        '\u{06af}',
        [
            Some('\u{fb92}'),
            Some('\u{fb94}'),
            Some('\u{fb95}'),
            Some('\u{fb93}'),
        ],
    ),
    (
        '\u{06b1}',
        [
            Some('\u{fb9a}'),
            Some('\u{fb9c}'),
            Some('\u{fb9d}'),
            Some('\u{fb9b}'),
        ],
    ),
    (
        '\u{06b3}',
        [
            Some('\u{fb96}'),
            Some('\u{fb98}'),
            Some('\u{fb99}'),
            Some('\u{fb97}'),
        ],
    ),
    ('\u{06ba}', [Some('\u{fb9e}'), None, None, Some('\u{fb9f}')]),
    (
        '\u{06bb}',
        [
            Some('\u{fba0}'),
            Some('\u{fba2}'),
            Some('\u{fba3}'),
            Some('\u{fba1}'),
        ],
    ),
    (
        '\u{06be}',
        [
            Some('\u{fbaa}'),
            Some('\u{fbac}'),
            Some('\u{fbad}'),
            Some('\u{fbab}'),
        ],
    ),
    ('\u{06c0}', [Some('\u{fba4}'), None, None, Some('\u{fba5}')]),
    (
        '\u{06c1}',
        [
            Some('\u{fba6}'),
            Some('\u{fba8}'),
            Some('\u{fba9}'),
            Some('\u{fba7}'),
        ],
    ),
    ('\u{06c5}', [Some('\u{fbe0}'), None, None, Some('\u{fbe1}')]),
    ('\u{06c6}', [Some('\u{fbd9}'), None, None, Some('\u{fbda}')]),
    ('\u{06c7}', [Some('\u{fbd7}'), None, None, Some('\u{fbd8}')]),
    ('\u{06c8}', [Some('\u{fbdb}'), None, None, Some('\u{fbdc}')]),
    ('\u{06c9}', [Some('\u{fbe2}'), None, None, Some('\u{fbe3}')]),
    ('\u{06cb}', [Some('\u{fbde}'), None, None, Some('\u{fbdf}')]),
    (
        '\u{06cc}',
        [
            Some('\u{fbfc}'),
            Some('\u{fbfe}'),
            Some('\u{fbff}'),
            Some('\u{fbfd}'),
        ],
    ),
    (
        '\u{06d0}',
        [
            Some('\u{fbe4}'),
            Some('\u{fbe6}'),
            Some('\u{fbe7}'),
            Some('\u{fbe5}'),
        ],
    ),
    ('\u{06d2}', [Some('\u{fbae}'), None, None, Some('\u{fbaf}')]),
    ('\u{06d3}', [Some('\u{fbb0}'), None, None, Some('\u{fbb1}')]),
    (
        '\u{200d}',
        [
            Some('\u{200d}'),
            Some('\u{200d}'),
            Some('\u{200d}'),
            Some('\u{200d}'),
        ],
    ),
];

/// The shapes of a letter, or `None` if it is not an Arabic letter.
pub(crate) fn lookup(c: char) -> Option<Shapes> {
    LETTERS
        .binary_search_by_key(&c, |(letter, _)| *letter)
        .ok()
        .map(|index| LETTERS[index].1)
}

/// Whether `c` can join to the letter before it, which needs a final or medial
/// shape.
pub(crate) fn joins_before(c: char) -> bool {
    lookup(c).is_some_and(|s| s[FINAL].is_some() || s[MEDIAL].is_some())
}

/// Whether `c` can join to the letter after it, which needs an initial or
/// medial shape.
pub(crate) fn joins_after(c: char) -> bool {
    lookup(c).is_some_and(|s| s[INITIAL].is_some() || s[MEDIAL].is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sorted_so_binary_search_works() {
        let mut previous = '\0';
        for (letter, _) in LETTERS {
            assert!(letter > previous, "table is out of order at {letter:?}");
            previous = letter;
        }
    }

    #[test]
    fn alef_joins_only_to_the_letter_before_it() {
        // Alef never joins to what follows, which is why it has no initial or
        // medial shape.
        assert!(joins_before('\u{0627}'));
        assert!(!joins_after('\u{0627}'));
    }

    #[test]
    fn beh_joins_on_both_sides() {
        assert!(joins_before('\u{0628}'));
        assert!(joins_after('\u{0628}'));
    }

    #[test]
    fn hamza_joins_on_neither_side() {
        assert!(!joins_before('\u{0621}'));
        assert!(!joins_after('\u{0621}'));
    }

    #[test]
    fn every_entry_has_at_least_one_shape() {
        for (letter, shapes) in LETTERS {
            assert!(
                shapes.iter().any(Option::is_some),
                "{letter:?} has no shape"
            );
        }
    }
}
