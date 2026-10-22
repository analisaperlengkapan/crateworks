//! Runs the README example, so the documentation cannot drift from the API.

use stdnum_rs::{Country, IdNumber};

#[test]
fn readme_examples() {
    use stdnum_rs::{Iban, Luhn};

    assert!(Iban::parse("GB82 WEST 1234 5698 7654 32").is_ok());
    assert!(Luhn::parse("4539148803436467").is_ok());
}

/// Every number in `stdnum_vectors.tsv` was labelled by python-stdnum, the
/// reference implementation, in the same mode this crate implements (its strict
/// check-digit mode for the Mexican RFC, which is the only place the two modes
/// differ).
///
/// The file holds numbers copied out of the reference's own documentation and
/// mutations of them: every single-digit substitution of a valid number, the
/// last character dropped and a character appended. A mutation is what a typo
/// produces, and a check-digit routine that accepts one is not doing its job.
///
/// This is the test that found the Luhn check digit missing from the NPWP, the
/// two-shape RFC, the alphanumeric CNPJ and the province field of the NIK.
#[test]
fn every_labelled_number_agrees_with_python_stdnum() {
    let fixture = include_str!("fixtures/stdnum_vectors.tsv");
    let mut agree = 0u32;
    let mut disagreements = Vec::new();
    for line in fixture.lines() {
        let mut fields = line.split('\t');
        let (Some(name), Some(value), Some(expected)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let Some(country) = country_named(name) else {
            continue;
        };
        let ours = IdNumber::parse(country, value).is_ok();
        if ours == (expected == "1") {
            agree += 1;
        } else {
            disagreements.push(format!("{name} {value:?}"));
        }
    }
    assert!(agree > 8000, "only {agree} vectors were compared");
    assert!(
        disagreements.is_empty(),
        "{} numbers disagree with python-stdnum: {:?}",
        disagreements.len(),
        &disagreements[..disagreements.len().min(20)]
    );
}

fn country_named(name: &str) -> Option<Country> {
    Some(match name {
        "IndonesiaNpwp" => Country::IndonesiaNpwp,
        "IndonesiaNik" => Country::IndonesiaNik,
        "BrazilCpf" => Country::BrazilCpf,
        "BrazilCnpj" => Country::BrazilCnpj,
        "SpainDni" => Country::SpainDni,
        "GermanyTaxId" => Country::GermanyTaxId,
        "PolandPesel" => Country::PolandPesel,
        "NetherlandsBsn" => Country::NetherlandsBsn,
        "BelgiumNationalNumber" => Country::BelgiumNationalNumber,
        "SwedenPersonnummer" => Country::SwedenPersonnummer,
        "NorwayBirthNumber" => Country::NorwayBirthNumber,
        "NorwayOrgNumber" => Country::NorwayOrgNumber,
        "DenmarkCpr" => Country::DenmarkCpr,
        "FinlandHetu" => Country::FinlandHetu,
        "PortugalNif" => Country::PortugalNif,
        "FranceSiren" => Country::FranceSiren,
        "FranceSiret" => Country::FranceSiret,
        "ItalyCodiceFiscale" => Country::ItalyCodiceFiscale,
        "RomaniaCnp" => Country::RomaniaCnp,
        "TurkeyTckn" => Country::TurkeyTckn,
        "CanadaSin" => Country::CanadaSin,
        "AustraliaAbn" => Country::AustraliaAbn,
        "AustraliaTfn" => Country::AustraliaTfn,
        "NewZealandIrd" => Country::NewZealandIrd,
        "ChinaRic" => Country::ChinaRic,
        "KoreaRrn" => Country::KoreaRrn,
        "MexicoRfc" => Country::MexicoRfc,
        _ => return None,
    })
}
