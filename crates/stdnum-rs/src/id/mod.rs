//! National identification numbers.
//!
//! Every country defines its own structure and check, so each is implemented
//! separately and reached through [`IdNumber::parse`]. Coverage starts with the
//! numbers this crate's authors could verify against published rules; adding a
//! country means adding one arm and its tests, which is the intended way to
//! extend the crate.
//!
//! # What is checked
//!
//! Only structure and checksum. A number that passes may still be unissued or
//! belong to someone else. Every algorithm here was cross-checked against the
//! independent `python-stdnum` implementation.

use crate::{Error, Result};

mod asia;
mod brazil;
mod europe;
mod nordics;
mod oceania;
mod southeast_asia;

use asia::{china, india, korea, mexico, turkey};
use brazil::{cnpj, cpf};
use europe::{belgium, france, germany, italy, netherlands, poland, portugal, romania, spain};
use nordics::{denmark, finland, norway, sweden};
use oceania::{australia, canada, new_zealand};
use southeast_asia::{indonesia, philippines};

pub use asia::verhoeff_check_digit;

/// A country whose identifiers this crate can check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Country {
    /// Indonesia: NPWP tax number, 15 digits.
    IndonesiaNpwp,
    /// Indonesia: NIK identity number, 16 digits.
    IndonesiaNik,
    /// Philippines: PhilSys number, 12 digits with a Luhn check.
    PhilippinesPhilSys,
    /// Brazil: CPF, 11 digits with two check digits.
    BrazilCpf,
    /// Brazil: CNPJ company number, 14 digits.
    BrazilCnpj,
    /// Spain: DNI, NIE or NIF.
    SpainDni,
    /// Germany: Steuerliche Identifikationsnummer, 11 digits.
    GermanyTaxId,
    /// Poland: PESEL, 11 digits.
    PolandPesel,
    /// India: Aadhaar, 12 digits with a Verhoeff check.
    IndiaAadhaar,
    /// Netherlands: BSN, 9 digits with the eleven-test.
    NetherlandsBsn,
    /// Belgium: National Number, 11 digits with a mod-97 check.
    BelgiumNationalNumber,
    /// Sweden: personnummer, 10 or 12 digits with a Luhn check.
    SwedenPersonnummer,
    /// Norway: fødselsnummer, 11 digits with two mod-11 check digits.
    NorwayBirthNumber,
    /// Norway: organisasjonsnummer, 9 digits with a mod-11 check digit.
    NorwayOrgNumber,
    /// Denmark: CPR, 10 digits with a mod-11 check.
    DenmarkCpr,
    /// Finland: HETU, 11 characters with a mod-31 check character.
    FinlandHetu,
    /// Portugal: NIF, 9 digits with a mod-11 check digit.
    PortugalNif,
    /// France: SIREN, 9 digits with a Luhn check.
    FranceSiren,
    /// France: SIRET, 14 digits with a Luhn check.
    FranceSiret,
    /// Italy: codice fiscale, 16 characters or an 11-digit VAT number.
    ItalyCodiceFiscale,
    /// Romania: CNP, 13 digits with a mod-11 check digit.
    RomaniaCnp,
    /// Turkey: T.C. Kimlik No, 11 digits with two check digits.
    TurkeyTckn,
    /// Canada: SIN, 9 digits with a Luhn check.
    CanadaSin,
    /// Australia: ABN, 11 digits with a mod-89 check.
    AustraliaAbn,
    /// Australia: TFN, 8 or 9 digits with a mod-11 check.
    AustraliaTfn,
    /// New Zealand: IRD number, 8 or 9 digits with a mod-11 check.
    NewZealandIrd,
    /// China: resident identity card, 18 characters.
    ChinaRic,
    /// South Korea: resident registration number, 13 digits.
    KoreaRrn,
    /// Mexico: RFC tax number, 12 or 13 characters.
    MexicoRfc,
}

impl Country {
    /// Every identifier this crate knows.
    pub const ALL: &'static [Self] = &[
        Self::IndonesiaNpwp,
        Self::IndonesiaNik,
        Self::PhilippinesPhilSys,
        Self::BrazilCpf,
        Self::BrazilCnpj,
        Self::SpainDni,
        Self::GermanyTaxId,
        Self::PolandPesel,
        Self::IndiaAadhaar,
        Self::NetherlandsBsn,
        Self::BelgiumNationalNumber,
        Self::SwedenPersonnummer,
        Self::NorwayBirthNumber,
        Self::NorwayOrgNumber,
        Self::DenmarkCpr,
        Self::FinlandHetu,
        Self::PortugalNif,
        Self::FranceSiren,
        Self::FranceSiret,
        Self::ItalyCodiceFiscale,
        Self::RomaniaCnp,
        Self::TurkeyTckn,
        Self::CanadaSin,
        Self::AustraliaAbn,
        Self::AustraliaTfn,
        Self::NewZealandIrd,
        Self::ChinaRic,
        Self::KoreaRrn,
        Self::MexicoRfc,
    ];

    /// A short English description of the identifier.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::IndonesiaNpwp => "Indonesia NPWP tax number",
            Self::IndonesiaNik => "Indonesia NIK identity number",
            Self::PhilippinesPhilSys => "Philippines PhilSys number",
            Self::BrazilCpf => "Brazil CPF",
            Self::BrazilCnpj => "Brazil CNPJ",
            Self::SpainDni => "Spain DNI/NIE/NIF",
            Self::GermanyTaxId => "Germany tax identification number",
            Self::PolandPesel => "Poland PESEL",
            Self::IndiaAadhaar => "India Aadhaar",
            Self::NetherlandsBsn => "Netherlands BSN",
            Self::BelgiumNationalNumber => "Belgium national number",
            Self::SwedenPersonnummer => "Sweden personnummer",
            Self::NorwayBirthNumber => "Norway fødselsnummer",
            Self::NorwayOrgNumber => "Norway organisasjonsnummer",
            Self::DenmarkCpr => "Denmark CPR",
            Self::FinlandHetu => "Finland HETU",
            Self::PortugalNif => "Portugal NIF",
            Self::FranceSiren => "France SIREN",
            Self::FranceSiret => "France SIRET",
            Self::ItalyCodiceFiscale => "Italy codice fiscale",
            Self::RomaniaCnp => "Romania CNP",
            Self::TurkeyTckn => "Turkey T.C. Kimlik No",
            Self::CanadaSin => "Canada SIN",
            Self::AustraliaAbn => "Australia ABN",
            Self::AustraliaTfn => "Australia TFN",
            Self::NewZealandIrd => "New Zealand IRD number",
            Self::ChinaRic => "China resident identity card",
            Self::KoreaRrn => "South Korea resident registration number",
            Self::MexicoRfc => "Mexico RFC",
        }
    }
}

/// A validated national identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdNumber {
    country: Country,
    normalized: String,
}

impl IdNumber {
    /// Validates `input` as an identifier of the given country.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] describing the first problem found.
    pub fn parse(country: Country, input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        // Some identifiers use a hyphen as a significant character: the Finnish
        // HETU century sign and the Swedish personnummer separator. Those get a
        // lighter normalisation that only trims and upper-cases, because the
        // full one would delete the character the check depends on.
        let kept = input
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_uppercase)
            .collect::<String>();
        let subject = match country {
            Country::FinlandHetu
            | Country::SwedenPersonnummer
            | Country::ItalyCodiceFiscale
            | Country::MexicoRfc
            | Country::ChinaRic => &kept,
            _ => &normalized,
        };
        match country {
            Country::IndonesiaNpwp => indonesia::validate_npwp(subject)?,
            Country::IndonesiaNik => indonesia::validate_nik(subject)?,
            Country::PhilippinesPhilSys => philippines::validate_philsys(subject)?,
            Country::BrazilCpf => cpf::validate(subject)?,
            Country::BrazilCnpj => cnpj::validate(subject)?,
            Country::SpainDni => spain::validate_dni(subject)?,
            Country::GermanyTaxId => germany::validate_tax_id(subject)?,
            Country::PolandPesel => poland::validate_pesel(subject)?,
            Country::IndiaAadhaar => india::validate_aadhaar(subject)?,
            Country::NetherlandsBsn => netherlands::validate_bsn(subject)?,
            Country::BelgiumNationalNumber => belgium::validate(subject)?,
            Country::SwedenPersonnummer => sweden::validate_personnummer(subject)?,
            Country::NorwayBirthNumber => norway::validate_birth_number(subject)?,
            Country::NorwayOrgNumber => norway::validate_org_number(subject)?,
            Country::DenmarkCpr => denmark::validate_cpr(subject)?,
            Country::FinlandHetu => finland::validate_hetu(subject)?,
            Country::PortugalNif => portugal::validate_nif(subject)?,
            Country::FranceSiren => france::validate_siren(subject)?,
            Country::FranceSiret => france::validate_siret(subject)?,
            Country::ItalyCodiceFiscale => italy::validate_codice_fiscale(subject)?,
            Country::RomaniaCnp => romania::validate_cnp(subject)?,
            Country::TurkeyTckn => turkey::validate_tckn(subject)?,
            Country::CanadaSin => canada::validate_sin(subject)?,
            Country::AustraliaAbn => australia::validate_abn(subject)?,
            Country::AustraliaTfn => australia::validate_tfn(subject)?,
            Country::NewZealandIrd => new_zealand::validate_ird(subject)?,
            Country::ChinaRic => china::validate_ric(subject)?,
            Country::KoreaRrn => korea::validate_rrn(subject)?,
            Country::MexicoRfc => mexico::validate_rfc(subject)?,
        }
        Ok(Self {
            country,
            normalized,
        })
    }

    /// The country this identifier belongs to.
    #[must_use]
    pub const fn country(&self) -> Country {
        self.country
    }

    /// The normalised identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_rejected_for_every_country() {
        for &country in Country::ALL {
            assert_eq!(
                IdNumber::parse(country, "   "),
                Err(Error::Empty),
                "{country:?}"
            );
        }
    }

    #[test]
    fn every_country_has_a_description() {
        for &country in Country::ALL {
            assert!(!country.description().is_empty(), "{country:?}");
        }
    }

    #[test]
    fn a_one_character_input_is_never_valid() {
        // Catches a validator that returns early without checking length.
        for &country in Country::ALL {
            assert!(
                IdNumber::parse(country, "1").is_err(),
                "{country:?} accepted a one-character input"
            );
        }
    }

    #[test]
    fn accessors_and_normalisation() {
        let n = IdNumber::parse(Country::BrazilCpf, "390.533.447-05").unwrap();
        assert_eq!(n.country(), Country::BrazilCpf);
        assert_eq!(n.as_str(), "39053344705");
    }
}
