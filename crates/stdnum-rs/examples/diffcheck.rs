//! Reads "Country<TAB>value" lines and prints the verdict for each, so the
//! crate can be compared against an independent implementation.
use std::io::{self, BufRead};

use stdnum_rs::{Country, IdNumber};

fn country(name: &str) -> Option<Country> {
    Some(match name {
        "BrazilCpf" => Country::BrazilCpf,
        "BrazilCnpj" => Country::BrazilCnpj,
        "SpainDni" => Country::SpainDni,
        "GermanyTaxId" => Country::GermanyTaxId,
        "PolandPesel" => Country::PolandPesel,
        "IndonesiaNpwp" => Country::IndonesiaNpwp,
        "IndonesiaNik" => Country::IndonesiaNik,
        "IndiaAadhaar" => Country::IndiaAadhaar,
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

fn main() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let Some((name, value)) = line.split_once('\t') else {
            continue;
        };
        let Some(c) = country(name) else { continue };
        let verdict = if IdNumber::parse(c, value).is_ok() {
            "accept"
        } else {
            "reject"
        };
        println!("{name}\t{value}\t{verdict}");
    }
}
