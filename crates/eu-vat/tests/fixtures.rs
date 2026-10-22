//! Fixtures generated from the MIT-licensed `python-stdnum` library, which is
//! an independent implementation of the same national specifications. A number
//! appears here only if the oracle and this crate agree about it, so a failure
//! means one of the two was transcribed wrongly.

use eu_vat::{Country, Vat};

/// `(prefix, input, canonical)` for every number the oracle accepts.
const VALID: &[(&str, &str, &str)] = &[
    ("AT", "AT U13585627", "ATU13585627"),
    ("BE", "BE 428759497", "BE0428759497"),
    ("CY", "CY-10259033P ", "CY10259033P"),
    ("DE", "DE136695976", "DE136695976"),
    ("DK", "DK 13585628", "DK13585628"),
    ("FI", "FI 20774740", "FI20774740"),
    ("EL", "EL 094259216 ", "EL094259216"),
    ("HR", "HR 33392005961", "HR33392005961"),
    ("HU", "HU-12892312", "HU12892312"),
    ("IE", "IE 6433435F", "IE6433435F"),
    ("IE", "IE 6433435OA", "IE6433435OA"),
    ("IT", "IT 00743110157", "IT00743110157"),
    ("LT", "LT 100001919017", "LT100001919017"),
    ("LU", "LU 150 274 42", "LU15027442"),
    ("LV", "LV 4000 3521 600", "LV40003521600"),
    ("MT", "MT 1167-9112", "MT11679112"),
    ("NL", "NL4495445B01", "NL004495445B01"),
    ("NL", "NL002455799B11", "NL002455799B11"),
    ("PL", "PL 8567346215", "PL8567346215"),
    ("PT", "PT 501 964 843", "PT501964843"),
    ("RO", "RO 185 472 90", "RO18547290"),
    ("SE", "SE 123456789701", "SE123456789701"),
    ("SI", "SI 5022 3054", "SI50223054"),
    ("SK", "SK 202 274 96 19", "SK2022749619"),
    ("BG", "BG175074752", "BG175074752"),
    ("CZ", "CZ25123891", "CZ25123891"),
    ("CZ", "CZ7103192745", "CZ7103192745"),
    ("CZ", "CZ640903926", "CZ640903926"),
    ("EE", "EE100594102", "EE100594102"),
    ("ES", "ESB64717838", "ESB64717838"),
    ("ES", "ES54362315K", "ES54362315K"),
    ("ES", "ESX-5253868-R", "ESX5253868R"),
    ("ES", "ESM-1234567-L", "ESM1234567L"),
    ("FR", "FR23334175221", "FR23334175221"),
    ("FR", "FRK7399859412", "FRK7399859412"),
    ("FR", "FR4Z123456782", "FR4Z123456782"),
    ("XI", "XI 980 7806 84", "XI980780684"),
    ("XI", "XI 980 7806 84 123", "XI980780684123"),
];

/// `(prefix, input)` for every number the oracle rejects.
const INVALID: &[(&str, &str)] = &[
    ("AT", "ATU13585620"),
    ("AT", "ATU135856279"),
    ("BE", "BE0428759490"),
    ("BE", "BE04287594979"),
    ("CY", "CY102590330"),
    ("CY", "CY10259033P9"),
    ("DE", "DE136695970"),
    ("DE", "DE1366959769"),
    ("DK", "DK13585620"),
    ("DK", "DK135856289"),
    ("FI", "FI20774741"),
    ("FI", "FI207747409"),
    ("EL", "EL094259210"),
    ("EL", "EL0942592169"),
    ("HR", "HR33392005960"),
    ("HR", "HR333920059619"),
    ("HU", "HU12892310"),
    ("HU", "HU128923129"),
    ("IE", "IE64334350"),
    ("IE", "IE6433435F9"),
    ("IE", "IE6433435O0"),
    ("IE", "IE6433435OA9"),
    ("IT", "IT00743110150"),
    ("IT", "IT007431101579"),
    ("LT", "LT100001919010"),
    ("LT", "LT1000019190179"),
    ("LU", "LU15027440"),
    ("LU", "LU150274429"),
    ("LV", "LV40003521601"),
    ("LV", "LV400035216009"),
    ("MT", "MT11679110"),
    ("MT", "MT116791129"),
    ("NL", "NL004495445B00"),
    ("NL", "NL004495445B019"),
    ("NL", "NL002455799B10"),
    ("NL", "NL002455799B119"),
    ("PL", "PL8567346210"),
    ("PL", "PL85673462159"),
    ("PT", "PT501964840"),
    ("PT", "PT5019648439"),
    ("RO", "RO18547291"),
    ("RO", "RO185472909"),
    ("SE", "SE123456789700"),
    ("SE", "SE1234567897019"),
    ("SI", "SI50223050"),
    ("SI", "SI502230549"),
    ("SK", "SK2022749610"),
    ("SK", "SK20227496199"),
    ("BG", "BG175074750"),
    ("BG", "BG1750747529"),
    ("CZ", "CZ25123890"),
    ("CZ", "CZ251238919"),
    ("CZ", "CZ7103192740"),
    ("CZ", "CZ71031927459"),
    ("CZ", "CZ640903920"),
    ("CZ", "CZ6409039269"),
    ("EE", "EE100594100"),
    ("EE", "EE1005941029"),
    ("ES", "ESB64717830"),
    ("ES", "ESB647178389"),
    ("ES", "ES543623150"),
    ("ES", "ES54362315K9"),
    ("ES", "ESX52538680"),
    ("ES", "ESX5253868R9"),
    ("ES", "ESM12345670"),
    ("ES", "ESM1234567L9"),
    ("FR", "FR23334175220"),
    ("FR", "FR233341752219"),
    ("FR", "FRK7399859410"),
    ("FR", "FRK73998594129"),
    ("FR", "FR4Z123456780"),
    ("FR", "FR4Z1234567829"),
    ("XI", "XI980780680"),
    ("XI", "XI9807806849"),
    ("XI", "XI9807806841239"),
];

#[test]
fn accepted_numbers_parse_and_reach_the_canonical_form() {
    for (prefix, input, canonical) in VALID {
        let vat = Vat::parse(input).unwrap_or_else(|e| panic!("{input:?} rejected: {e}"));
        assert_eq!(vat.to_string(), *canonical, "{input:?}");
        assert_eq!(vat.country().code(), *prefix, "{input:?}");
    }
}

#[test]
fn every_jurisdiction_has_at_least_one_accepted_number() {
    for country in Country::ALL {
        assert!(
            VALID.iter().any(|(p, _, _)| *p == country.code()),
            "no fixture for {country}"
        );
    }
}

#[test]
fn rejected_numbers_are_rejected() {
    for (prefix, input) in INVALID {
        assert!(
            Vat::parse(input).is_err(),
            "{input:?} ({prefix}) was accepted but the oracle rejects it"
        );
    }
}

#[test]
fn the_canonical_form_reparses_to_itself() {
    for (_, _, canonical) in VALID {
        let again = Vat::parse(canonical).unwrap_or_else(|e| panic!("{canonical:?}: {e}"));
        assert_eq!(again.to_string(), *canonical);
    }
}

#[test]
fn lowercase_and_separators_do_not_change_the_result() {
    for (_, input, canonical) in VALID {
        let lowered = input.to_lowercase();
        let vat = Vat::parse(&lowered).unwrap_or_else(|e| panic!("{lowered:?}: {e}"));
        assert_eq!(vat.to_string(), *canonical);
    }
}

#[test]
fn a_wrong_character_count_is_reported_as_such() {
    use eu_vat::Error;
    assert_eq!(
        Vat::parse("DE13669597"),
        Err(Error::WrongLength {
            expected: 9,
            found: 8
        })
    );
}

/// The cases below were found by comparing against `python-stdnum` 2.2, which
/// is an independent implementation of the same national rules.
mod regressions {
    use eu_vat::Vat;

    /// A Slovenian weighted sum that leaves a remainder of one gives a check
    /// digit of ten, and the authority writes that as a zero. Rejecting it, as
    /// an earlier version did, refused numbers that are in use.
    #[test]
    fn a_slovenian_check_digit_of_ten_is_written_as_zero() {
        for number in ["SI 48673820", "SI 56633360", "SI 68297530"] {
            assert!(Vat::parse(number).is_ok(), "{number} was rejected");
        }
    }

    /// Greece dropped its eight-digit form in 2014 and writes it with a leading
    /// zero today, so an eight-digit input is the same number.
    #[test]
    fn an_eight_digit_greek_number_is_padded() {
        let vat = Vat::parse("EL 94051189").expect("the eight-digit form is valid");
        assert_eq!(vat.to_string(), "EL094051189");
    }

    /// The Dutch eleven-digit form omits the leading zero of the nine-digit
    /// citizen service number and keeps the three-character suffix.
    #[test]
    fn an_eleven_digit_dutch_number_is_padded() {
        for (input, canonical) in [
            ("NL00449544B01", "NL000449544B01"),
            ("NL68357370B01", "NL068357370B01"),
        ] {
            let vat = Vat::parse(input).expect("the eleven-digit form is valid");
            assert_eq!(vat.to_string(), canonical, "{input}");
        }
    }

    /// Belgium writes the old leading zero inside parentheses, and that is a
    /// separator rather than part of the number.
    #[test]
    fn a_bracketed_belgian_zero_is_a_separator() {
        let vat = Vat::parse("BE (0)468.561.072").expect("the bracketed zero is a separator");
        assert_eq!(vat.to_string(), "BE0468561072");
    }

    /// The prefix is stripped once, so a second one means the input is not a
    /// number the authority issues. Accepting `RO RO ...` would report a valid
    /// number for a string no authority would issue.
    #[test]
    fn a_repeated_romanian_prefix_is_rejected() {
        assert!(Vat::parse("RO RO 16621241").is_err());
        assert!(Vat::parse("RO16621241").is_ok());
    }

    /// A comma is not one of the separators the authorities tolerate.
    #[test]
    fn a_comma_is_not_a_separator() {
        assert!(Vat::parse("BE 0220,764.971").is_err());
        assert!(Vat::parse("BE 0220.764.971").is_ok());
    }
}
