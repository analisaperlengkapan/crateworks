//! Country-specific VAT rules.
//!
//! Every function here is a transcription of the matching `validate()` in the
//! MIT-licensed `python-stdnum` library, which in turn follows the national
//! authority's published specification. The tests in `tests/fixtures.rs`
//! compare against numbers generated from python-stdnum itself, so a
//! transcription mistake shows up as a failing fixture rather than as a silent
//! difference in behaviour.

use crate::checksum::{expect_len, is_digits, parse_int, require_digits};
use crate::{Country, Error, Result, checksum};

/// Digits and letters, used by France for its two prefix characters.
const FRENCH_ALPHABET: &str = "0123456789ABCDEFGHJKLMNPQRSTUVWXYZ";
/// The letters a Spanish DNI check character can take, indexed by `n % 23`.
const DNI_ALPHABET: &str = "TRWAGMYFPDXBNJZSQVHLCKE";
/// The letters an Irish check character can take.
const IRISH_ALPHABET: &str = "WABCDEFGHIJKLMNOPQRSTUV";

/// Applies any length normalisation a jurisdiction needs before validation.
///
/// Belgium used a nine-digit enterprise number before 2008 and the current
/// form pads it with a leading zero, so a nine-digit input is expanded here
/// rather than rejected. The padded form is what the canonical string holds.
pub(crate) fn canonicalize(country: Country, number: String) -> String {
    match country {
        // The enterprise number was nine digits before 2008 and is written with
        // a leading zero today.
        Country::Belgium if number.len() == 9 => format!("0{number}"),
        // The Dutch number embeds a citizen service number, which is written
        // without its leading zeros and padded back to nine for checking.
        Country::Netherlands if number.len() == 10 => {
            let (bsn, tail) = number.split_at(7);
            format!("{bsn:0>9}{tail}")
        }
        _ => number,
    }
}

/// Dispatches to the rule for one jurisdiction.
pub(crate) fn validate(country: Country, number: &str) -> Result<()> {
    match country {
        Country::Austria => austria(number),
        Country::Belgium => belgium(number),
        Country::Bulgaria => bulgaria(number),
        Country::Cyprus => cyprus(number),
        Country::Czechia => czechia(number),
        Country::Germany => germany(number),
        Country::Denmark => denmark(number),
        Country::Estonia => estonia(number),
        Country::Greece => greece(number),
        Country::Spain => spain(number),
        Country::Finland => finland(number),
        Country::France => france(number),
        Country::Croatia => croatia(number),
        Country::Hungary => hungary(number),
        Country::Ireland => ireland(number),
        Country::Italy => italy(number),
        Country::Lithuania => lithuania(number),
        Country::Luxembourg => luxembourg(number),
        Country::Latvia => latvia(number),
        Country::Malta => malta(number),
        Country::Netherlands => netherlands(number),
        Country::Poland => poland(number),
        Country::Portugal => portugal(number),
        Country::Romania => romania(number),
        Country::Sweden => sweden(number),
        Country::Slovenia => slovenia(number),
        Country::Slovakia => slovakia(number),
        Country::NorthernIreland => united_kingdom(number),
    }
}

/// The digit that completes `payload` under the Luhn scheme.
fn luhn_check_digit(payload: &str) -> u32 {
    let mut probe = String::with_capacity(payload.len() + 1);
    probe.push_str(payload);
    probe.push('0');
    (10 - checksum::luhn(&probe)) % 10
}

/// Whether every character is an ASCII upper-case letter.
fn all_upper(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_uppercase())
}

/// Austria: `U` followed by eight digits, the last a Luhn-derived check.
///
/// `stdnum.at.uid`.
fn austria(n: &str) -> Result<()> {
    expect_len(n, 9)?;
    if !n.starts_with('U') {
        return Err(Error::InvalidFormat);
    }
    require_digits(&n[1..])?;
    // Signed arithmetic, because the Luhn checksum can exceed six.
    let expected = (6 - i64::from(checksum::luhn(&n[1..8]))).rem_euclid(10);
    if i64::from(n.as_bytes()[8] - b'0') != expected {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Belgium: ten digits whose first two are `0` or `1`, mod-97 over the halves.
///
/// `stdnum.be.vat`.
fn belgium(n: &str) -> Result<()> {
    expect_len(n, 10)?;
    require_digits(n)?;
    if !n.starts_with('0') && !n.starts_with('1') {
        return Err(Error::OutOfRange);
    }
    if (parse_int(&n[..8]) + parse_int(&n[8..])) % 97 != 0 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Bulgaria: nine digits for legal entities, ten for others.
///
/// The ten-digit form is a personal number, a foreigner number, or a third
/// weighting. `stdnum.bg.vat`.
fn bulgaria(n: &str) -> Result<()> {
    require_digits(n)?;
    match n.len() {
        9 => {
            let mut expected =
                checksum::weighted(&n[..8], &[1, 2, 3, 4, 5, 6, 7, 8]).rem_euclid(11);
            if expected == 10 {
                expected =
                    checksum::weighted(&n[..8], &[3, 4, 5, 6, 7, 8, 9, 10]).rem_euclid(11) % 10;
            }
            if i64::from(n.as_bytes()[8] - b'0') != expected {
                return Err(Error::BadChecksum);
            }
        }
        10 => {
            if bulgarian_egn(n).is_ok() || bulgarian_pnf(n).is_ok() {
                return Ok(());
            }
            let expected =
                (11 - checksum::weighted(&n[..9], &[4, 3, 2, 7, 6, 5, 4, 3, 2])).rem_euclid(11);
            if i64::from(n.as_bytes()[9] - b'0') != expected {
                return Err(Error::BadChecksum);
            }
        }
        found => {
            return Err(Error::WrongLength { expected: 9, found });
        }
    }
    Ok(())
}

/// Bulgarian personal number: the first six digits encode a birth date, with
/// the month offset by 20 or 40 to record the century.
///
/// `stdnum.bg.egn`.
fn bulgarian_egn(n: &str) -> Result<()> {
    let base_year = parse_int(&n[0..2]) + 1900;
    let mut month = parse_int(&n[2..4]) as u32;
    let day = parse_int(&n[4..6]) as u32;
    let year = if month > 40 {
        month -= 40;
        base_year + 100
    } else if month > 20 {
        month -= 20;
        base_year - 100
    } else {
        base_year
    };
    if !checksum::is_real_date(year, month, day) {
        return Err(Error::OutOfRange);
    }
    let expected = checksum::weighted(&n[..9], &[2, 4, 8, 5, 10, 9, 7, 3, 6]).rem_euclid(11) % 10;
    if i64::from(n.as_bytes()[9] - b'0') != expected {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Bulgarian foreigner number. `stdnum.bg.pnf`.
fn bulgarian_pnf(n: &str) -> Result<()> {
    let expected = checksum::weighted(&n[..9], &[21, 19, 17, 13, 11, 9, 7, 3, 1]).rem_euclid(10);
    if i64::from(n.as_bytes()[9] - b'0') != expected {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Cyprus: eight digits and a trailing letter. `stdnum.cy.vat`.
fn cyprus(n: &str) -> Result<()> {
    expect_len(n, 9)?;
    require_digits(&n[..8])?;
    if n.starts_with("12") {
        return Err(Error::OutOfRange);
    }
    if n.as_bytes()[8] != cypriot_check_digit(&n[..8]) as u8 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// The Cypriot check letter for eight digits.
///
/// Digits in even positions contribute a mapped value, those in odd positions
/// their own.
fn cypriot_check_digit(body: &str) -> char {
    const MAPPED: [u32; 10] = [1, 0, 5, 7, 9, 13, 15, 17, 19, 21];
    let mut sum = 0u32;
    for (i, b) in body.bytes().enumerate() {
        let digit = u32::from(b - b'0');
        sum += if i % 2 == 0 {
            MAPPED[digit as usize]
        } else {
            digit
        };
    }
    char::from(b'A' + (sum % 26) as u8)
}

/// Czechia: eight digits for legal entities, otherwise a birth number.
///
/// `stdnum.cz.dic`.
fn czechia(n: &str) -> Result<()> {
    require_digits(n)?;
    match n.len() {
        8 => {
            if n.starts_with('9') {
                return Err(Error::OutOfRange);
            }
            if n.as_bytes()[7] != czech_legal_check_digit(&n[..7]) as u8 {
                return Err(Error::BadChecksum);
            }
        }
        9 if n.starts_with('6') => {
            if n.as_bytes()[8] != czech_special_check_digit(&n[1..8]) as u8 {
                return Err(Error::BadChecksum);
            }
        }
        9 | 10 => czech_birth_number(n)?,
        found => {
            return Err(Error::WrongLength { expected: 8, found });
        }
    }
    Ok(())
}

/// Check digit for an eight-digit Czech legal entity. `stdnum.cz.dic`.
fn czech_legal_check_digit(body: &str) -> char {
    let sum = descending_weight_sum(body);
    let check = (11 - sum).rem_euclid(11);
    let check = if check == 0 { 1 } else { check };
    char::from(b'0' + (check % 10) as u8)
}

/// Check digit for a nine-digit Czech number starting with six, where the
/// leading six is not part of the sum.
fn czech_special_check_digit(body: &str) -> char {
    let check = descending_weight_sum(body).rem_euclid(11);
    char::from(b'0' + ((8 - (10 - check).rem_euclid(11)).rem_euclid(10)) as u8)
}

/// Sums `body` with weights running 8, 7, 6, ... left to right.
fn descending_weight_sum(body: &str) -> i64 {
    body.bytes()
        .enumerate()
        .map(|(i, b)| (8 - i as i64) * i64::from(b - b'0'))
        .sum()
}

/// A Czech or Slovak birth number, nine or ten digits. `stdnum.cz.rc`.
fn czech_birth_number(n: &str) -> Result<()> {
    let mut year = 1900 + parse_int(&n[0..2]);
    // Women have fifty added to the month, and twenty more when the serial
    // number overflows, which has been possible since 2004.
    let month = (parse_int(&n[2..4]) % 50 % 20) as u32;
    let day = parse_int(&n[4..6]) as u32;
    if n.len() == 9 {
        if year >= 1980 {
            year -= 100;
        }
        if year > 1953 {
            return Err(Error::OutOfRange);
        }
    } else if year < 1954 {
        year += 100;
    }
    if !checksum::is_real_date(year, month, day) {
        return Err(Error::OutOfRange);
    }
    if n.len() == 10 && parse_int(&n[9..]) != parse_int(&n[..9]) % 11 % 10 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Germany: nine digits under ISO 7064 Mod 11, 10. `stdnum.de.vat`.
fn germany(n: &str) -> Result<()> {
    expect_len(n, 9)?;
    require_digits(n)?;
    if n.starts_with('0') {
        return Err(Error::InvalidFormat);
    }
    if !checksum::mod_11_10_ok(n) {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Denmark: eight digits under a mod-11 weighting. `stdnum.dk.cvr`.
fn denmark(n: &str) -> Result<()> {
    expect_len(n, 8)?;
    require_digits(n)?;
    if n.starts_with('0') {
        return Err(Error::InvalidFormat);
    }
    if checksum::weighted(n, &[2, 7, 6, 5, 4, 3, 2, 1]).rem_euclid(11) != 0 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Estonia: nine digits under a repeating weighting. `stdnum.ee.kmkr`.
fn estonia(n: &str) -> Result<()> {
    expect_len(n, 9)?;
    require_digits(n)?;
    if checksum::weighted(n, &[3, 7, 1, 3, 7, 1, 3, 7, 1]).rem_euclid(10) != 0 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Greece: nine digits, check digit from a doubling recurrence.
///
/// `stdnum.gr.vat`.
fn greece(n: &str) -> Result<()> {
    expect_len(n, 9)?;
    require_digits(n)?;
    let mut running = 0u64;
    for b in n[..8].bytes() {
        running = running * 2 + u64::from(b - b'0');
    }
    if u64::from(n.as_bytes()[8] - b'0') != (running * 2) % 11 % 10 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Spain: a DNI, an NIE or a company number. `stdnum.es.nif`.
fn spain(n: &str) -> Result<()> {
    expect_len(n, 9)?;
    require_digits(&n[1..8])?;
    let first = n.as_bytes()[0] as char;
    let last = n.as_bytes()[8] as char;
    if matches!(first, 'K' | 'L' | 'M') {
        // Spaniards under fourteen, Spaniards abroad, and foreigners without an
        // NIE all use the DNI check on the eight middle digits.
        if last != dni_check_digit(&n[1..8]) {
            return Err(Error::BadChecksum);
        }
    } else if first.is_ascii_digit() {
        if last != dni_check_digit(&n[..8]) {
            return Err(Error::BadChecksum);
        }
    } else if matches!(first, 'X' | 'Y' | 'Z') {
        if last != nie_check_digit(n) {
            return Err(Error::BadChecksum);
        }
    } else {
        // Everything else has to be a company number, and may carry either the
        // numeric or the alphabetic form of its check character.
        if !matches!(
            first,
            'A' | 'B'
                | 'C'
                | 'D'
                | 'E'
                | 'F'
                | 'G'
                | 'H'
                | 'J'
                | 'N'
                | 'P'
                | 'Q'
                | 'R'
                | 'S'
                | 'U'
                | 'V'
                | 'W'
        ) {
            return Err(Error::InvalidFormat);
        }
        let check = luhn_check_digit(&n[1..8]);
        let letter = char::from(b"JABCDEFGHI"[check as usize]);
        if last != char::from(b'0' + check as u8) && last != letter {
            return Err(Error::BadChecksum);
        }
    }
    Ok(())
}

/// The Spanish DNI check letter for eight digits.
fn dni_check_digit(digits: &str) -> char {
    let index = (parse_int(digits) % 23) as usize;
    char::from(DNI_ALPHABET.as_bytes()[index])
}

/// The Spanish NIE check letter, where `X`, `Y` and `Z` stand for 0, 1 and 2.
fn nie_check_digit(n: &str) -> char {
    let leading = match n.as_bytes()[0] as char {
        'X' => '0',
        'Y' => '1',
        _ => '2',
    };
    let mut body = String::with_capacity(8);
    body.push(leading);
    body.push_str(&n[1..8]);
    dni_check_digit(&body)
}

/// Finland: eight digits under a weighted checksum. `stdnum.fi.alv`.
fn finland(n: &str) -> Result<()> {
    expect_len(n, 8)?;
    require_digits(n)?;
    if checksum::weighted(n, &[7, 9, 10, 5, 8, 4, 2, 1]).rem_euclid(11) != 0 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// France: two prefix characters then a SIREN. `stdnum.fr.tva`.
fn france(n: &str) -> Result<()> {
    expect_len(n, 11)?;
    if !n[..2].chars().all(|c| FRENCH_ALPHABET.contains(c)) {
        return Err(Error::InvalidFormat);
    }
    require_digits(&n[2..])?;
    // Numbers beginning 000 are Monacan and are not SIRENs, so they skip the
    // SIREN check but still carry the VAT check.
    if &n[2..5] != "000" && !checksum::luhn_ok(&n[2..]) {
        return Err(Error::BadChecksum);
    }
    if is_digits(n) {
        let shifted = format!("{}12", &n[2..]);
        if parse_int(&n[..2]) != parse_int(&shifted) % 97 {
            return Err(Error::BadChecksum);
        }
    } else {
        let index = |c: char| FRENCH_ALPHABET.find(c).unwrap_or(0) as i64;
        let (first, second) = (
            n.chars().next().unwrap_or('0'),
            n.chars().nth(1).unwrap_or('0'),
        );
        let check = if n.as_bytes()[0].is_ascii_digit() {
            index(first) * 24 + index(second) - 10
        } else {
            index(first) * 34 + index(second) - 100
        };
        let body = parse_int(&n[2..]);
        if (body + 1 + check.div_euclid(11)) % 11 != check.rem_euclid(11) {
            return Err(Error::BadChecksum);
        }
    }
    Ok(())
}

/// Croatia: eleven digits under ISO 7064 Mod 11, 10. `stdnum.hr.oib`.
fn croatia(n: &str) -> Result<()> {
    expect_len(n, 11)?;
    require_digits(n)?;
    if !checksum::mod_11_10_ok(n) {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Hungary: eight digits under a weighted checksum. `stdnum.hu.anum`.
fn hungary(n: &str) -> Result<()> {
    expect_len(n, 8)?;
    require_digits(n)?;
    if checksum::weighted(n, &[9, 7, 3, 1, 9, 7, 3, 1]).rem_euclid(10) != 0 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Ireland: seven digits and one or two letters. `stdnum.ie.vat`.
fn ireland(n: &str) -> Result<()> {
    if !matches!(n.len(), 8 | 9) {
        return Err(Error::WrongLength {
            expected: 8,
            found: n.len(),
        });
    }
    let first = n.as_bytes()[0] as char;
    require_digits(&n[2..7])?;
    if !all_upper(&n[7..]) {
        return Err(Error::InvalidFormat);
    }
    if is_digits(&n[..7]) {
        // Modern form: seven digits, then one or two letters.
        let mut probe = String::with_capacity(8);
        probe.push_str(&n[..7]);
        probe.push_str(&n[8..]);
        if n.as_bytes()[7] != irish_check_digit(&probe) as u8 {
            return Err(Error::BadChecksum);
        }
    } else if matches!(n.as_bytes()[1] as char, 'A'..='Z' | '+') {
        // Old form: the second character is a letter or a symbol, and it moves
        // to the end of the probe.
        let mut probe = String::with_capacity(8);
        probe.push_str(&n[2..7]);
        probe.push(first);
        if n.as_bytes()[7] != irish_check_digit(&probe) as u8 {
            return Err(Error::BadChecksum);
        }
    } else {
        return Err(Error::InvalidFormat);
    }
    Ok(())
}

/// The Irish check character.
///
/// `value` is up to seven digits optionally followed by one letter; a missing
/// digit counts as a leading zero and a missing letter contributes nothing.
fn irish_check_digit(value: &str) -> char {
    let (digits, suffix) = value.split_at(value.len().min(7));
    let mut sum: i64 = 0;
    let padding = 7 - digits.len();
    for (i, b) in digits.bytes().enumerate() {
        sum += (8 - (i + padding) as i64) * i64::from(b - b'0');
    }
    if let Some(c) = suffix.chars().next() {
        sum += 9 * IRISH_ALPHABET.find(c).unwrap_or(0) as i64;
    }
    char::from(IRISH_ALPHABET.as_bytes()[sum.rem_euclid(23) as usize])
}

/// Italy: eleven digits, the last a Luhn check. `stdnum.it.iva`.
fn italy(n: &str) -> Result<()> {
    expect_len(n, 11)?;
    require_digits(n)?;
    if parse_int(&n[..7]) == 0 {
        return Err(Error::InvalidFormat);
    }
    let province = &n[7..10];
    if !(("001"..="100").contains(&province) || matches!(province, "120" | "121" | "888" | "999")) {
        return Err(Error::OutOfRange);
    }
    if !checksum::luhn_ok(n) {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Lithuania: nine digits, or twelve for temporary registrations.
///
/// `stdnum.lt.pvm`.
fn lithuania(n: &str) -> Result<()> {
    require_digits(n)?;
    match n.len() {
        9 => {
            if n.as_bytes()[7] != b'1' {
                return Err(Error::OutOfRange);
            }
        }
        12 => {
            if n.as_bytes()[10] != b'1' {
                return Err(Error::OutOfRange);
            }
        }
        found => {
            return Err(Error::WrongLength { expected: 9, found });
        }
    }
    let last = n.len() - 1;
    let body = &n[..last];
    let mut check = body
        .bytes()
        .enumerate()
        .map(|(i, b)| (1 + i as i64 % 9) * i64::from(b - b'0'))
        .sum::<i64>()
        .rem_euclid(11);
    if check == 10 {
        check = body
            .bytes()
            .enumerate()
            .map(|(i, b)| (1 + (i as i64 + 2) % 9) * i64::from(b - b'0'))
            .sum();
    }
    if i64::from(n.as_bytes()[last] - b'0') != check.rem_euclid(11) % 10 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Luxembourg: eight digits whose last two are the first six mod 89.
///
/// `stdnum.lu.tva`.
fn luxembourg(n: &str) -> Result<()> {
    expect_len(n, 8)?;
    require_digits(n)?;
    if parse_int(&n[..6]) % 89 != parse_int(&n[6..]) {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Latvia: eleven digits, either a company or a personal code.
///
/// `stdnum.lv.pvn`.
fn latvia(n: &str) -> Result<()> {
    expect_len(n, 11)?;
    require_digits(n)?;
    if n.as_bytes()[0] > b'3' {
        if checksum::weighted(n, &[9, 1, 4, 8, 3, 10, 2, 5, 7, 6, 1]).rem_euclid(11) != 3 {
            return Err(Error::BadChecksum);
        }
        return Ok(());
    }
    // Codes starting 32 carry no birth date; every other personal code does.
    if !n.starts_with("32") {
        let day = parse_int(&n[0..2]) as u32;
        let month = parse_int(&n[2..4]) as u32;
        let year = parse_int(&n[4..6]) + 1800 + i64::from(n.as_bytes()[6] - b'0') * 100;
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
    }
    if i64::from(n.as_bytes()[10] - b'0') != latvian_personal_check_digit(&n[..10]) {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// The Latvian personal-code check digit. `stdnum.lv.pvn`.
fn latvian_personal_check_digit(body: &str) -> i64 {
    let weights = [10, 5, 8, 4, 2, 1, 6, 3, 7, 9];
    (1 + checksum::weighted(body, &weights)).rem_euclid(11) % 10
}

/// Malta: eight digits under a mod-37 weighting. `stdnum.mt.vat`.
fn malta(n: &str) -> Result<()> {
    expect_len(n, 8)?;
    require_digits(n)?;
    if n.starts_with('0') {
        return Err(Error::InvalidFormat);
    }
    if checksum::weighted(n, &[3, 4, 6, 7, 8, 9, 10, 1]).rem_euclid(37) != 0 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Netherlands: nine digits, `B`, then two digits. `stdnum.nl.btw`.
fn netherlands(n: &str) -> Result<()> {
    expect_len(n, 12)?;
    require_digits(&n[..9])?;
    if parse_int(&n[..9]) == 0 {
        return Err(Error::InvalidFormat);
    }
    if n.as_bytes()[9] != b'B' {
        return Err(Error::InvalidFormat);
    }
    require_digits(&n[10..])?;
    if parse_int(&n[10..]) == 0 {
        return Err(Error::InvalidFormat);
    }
    // Either the old citizen-service-number check or the newer Mod 97, 10 check
    // is accepted, because registrations issued before 2020 use the former.
    if !dutch_bsn_ok(&n[..9]) && !checksum::mod_97_10_ok(&format!("NL{n}")) {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// The Dutch citizen service number check, where the last digit is subtracted
/// rather than added. `stdnum.nl.bsn`.
fn dutch_bsn_ok(body: &str) -> bool {
    let sum: i64 = body
        .bytes()
        .enumerate()
        .map(|(i, b)| {
            let weight = if i < 8 { 9 - i as i64 } else { -1 };
            weight * i64::from(b - b'0')
        })
        .sum();
    sum.rem_euclid(11) == 0
}

/// Poland: ten digits under a weighted checksum whose last weight is negative.
///
/// `stdnum.pl.nip`.
fn poland(n: &str) -> Result<()> {
    expect_len(n, 10)?;
    require_digits(n)?;
    if checksum::weighted(n, &[6, 5, 7, 2, 3, 4, 5, 6, 7, -1]).rem_euclid(11) != 0 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Portugal: nine digits, check digit from a descending weighting.
///
/// `stdnum.pt.nif`.
fn portugal(n: &str) -> Result<()> {
    expect_len(n, 9)?;
    require_digits(n)?;
    if n.starts_with('0') {
        return Err(Error::InvalidFormat);
    }
    let sum: i64 = n[..8]
        .bytes()
        .enumerate()
        .map(|(i, b)| (9 - i as i64) * i64::from(b - b'0'))
        .sum();
    if i64::from(n.as_bytes()[8] - b'0') != (11 - sum).rem_euclid(11) % 10 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Romania: a company number of two to ten digits, or a thirteen-digit
/// personal number. `stdnum.ro.cf`.
fn romania(n: &str) -> Result<()> {
    let digits = n.strip_prefix("RO").unwrap_or(n);
    match digits.len() {
        13 => romanian_personal_number(digits),
        2..=10 => {
            require_digits(digits)?;
            if digits.starts_with('0') {
                return Err(Error::InvalidFormat);
            }
            let last = digits.len() - 1;
            let body = &digits[..last];
            let mut padded = String::with_capacity(9);
            for _ in body.len()..9 {
                padded.push('0');
            }
            padded.push_str(body);
            let expected = (10 * checksum::weighted(&padded, &[7, 5, 3, 2, 1, 7, 5, 3, 2]))
                .rem_euclid(11)
                % 10;
            if i64::from(digits.as_bytes()[last] - b'0') != expected {
                return Err(Error::BadChecksum);
            }
            Ok(())
        }
        found => Err(Error::WrongLength { expected: 2, found }),
    }
}

/// The Romanian personal numeric code. `stdnum.ro.cnp`.
fn romanian_personal_number(n: &str) -> Result<()> {
    require_digits(n)?;
    if n.starts_with('0') {
        return Err(Error::OutOfRange);
    }
    // The first digit records the century; 7, 8 and 9 are foreign residents,
    // for whom the century is assumed to be the twentieth.
    let century = match n.as_bytes()[0] as char {
        '1' | '2' => 1900,
        '3' | '4' => 1800,
        '5' | '6' => 2000,
        _ => 1900,
    };
    let year = parse_int(&n[1..3]) + century;
    let month = parse_int(&n[3..5]) as u32;
    let day = parse_int(&n[5..7]) as u32;
    if !checksum::is_real_date(year, month, day) || !is_romanian_county(&n[7..9]) {
        return Err(Error::OutOfRange);
    }
    let check = checksum::weighted(&n[..12], &[2, 7, 9, 1, 4, 6, 3, 5, 8, 2, 7, 9]).rem_euclid(11);
    let expected = if check == 10 { 1 } else { check };
    if i64::from(n.as_bytes()[12] - b'0') != expected {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Whether `code` is a Romanian county or Bucharest sector code.
///
/// `stdnum.ro.cnp`. The codes are not contiguous: 49 and 50 are unused, and the
/// foreign-resident and sector ranges sit apart from the counties.
fn is_romanian_county(code: &str) -> bool {
    matches!(
        code,
        "01" | "02"
            | "03"
            | "04"
            | "05"
            | "06"
            | "07"
            | "08"
            | "09"
            | "10"
            | "11"
            | "12"
            | "13"
            | "14"
            | "15"
            | "16"
            | "17"
            | "18"
            | "19"
            | "20"
            | "21"
            | "22"
            | "23"
            | "24"
            | "25"
            | "26"
            | "27"
            | "28"
            | "29"
            | "30"
            | "31"
            | "32"
            | "33"
            | "34"
            | "35"
            | "36"
            | "37"
            | "38"
            | "39"
            | "40"
            | "41"
            | "42"
            | "43"
            | "44"
            | "45"
            | "46"
            | "47"
            | "48"
            | "51"
            | "52"
            | "70"
            | "80"
            | "81"
            | "82"
            | "83"
    )
}

/// Sweden: ten digits under Luhn, then the literal `01`. `stdnum.se.vat`.
fn sweden(n: &str) -> Result<()> {
    expect_len(n, 12)?;
    require_digits(n)?;
    if !n.ends_with("01") {
        return Err(Error::InvalidFormat);
    }
    if !checksum::luhn_ok(&n[..10]) {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Slovenia: eight digits, the last a check digit. `stdnum.si.ddv`.
fn slovenia(n: &str) -> Result<()> {
    expect_len(n, 8)?;
    require_digits(n)?;
    if n.starts_with('0') {
        return Err(Error::InvalidFormat);
    }
    let check = 11 - descending_weight_sum(&n[..7]).rem_euclid(11);
    // A weighted sum divisible by eleven gives a two-digit check, which can
    // never match the single character present, so that case is always a
    // mismatch.
    if n.as_bytes()[7] != char::from(b'0' + check as u8) as u8 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Slovakia: ten digits, either a birth number or a weighted checksum.
///
/// `stdnum.sk.dph`.
fn slovakia(n: &str) -> Result<()> {
    expect_len(n, 10)?;
    require_digits(n)?;
    if czech_birth_number(n).is_ok() {
        return Ok(());
    }
    if n.starts_with('0') || !matches!(n.as_bytes()[2] as char, '2' | '3' | '4' | '7' | '8' | '9') {
        return Err(Error::InvalidFormat);
    }
    if parse_int(n) % 11 != 0 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Northern Ireland: the United Kingdom numbering scheme. `stdnum.gb.vat`.
fn united_kingdom(n: &str) -> Result<()> {
    match n.len() {
        5 => {
            require_digits(&n[2..])?;
            let value = parse_int(&n[2..]);
            if !((n.starts_with("GD") && value < 500) || (n.starts_with("HA") && value >= 500)) {
                return Err(Error::OutOfRange);
            }
        }
        11 => {
            if !matches!(&n[..6], "GD8888" | "HA8888") {
                return Err(Error::OutOfRange);
            }
            require_digits(&n[6..])?;
            let value = parse_int(&n[6..9]);
            if !((n.starts_with("GD") && value < 500) || (n.starts_with("HA") && value >= 500)) {
                return Err(Error::OutOfRange);
            }
            if value % 97 != parse_int(&n[9..11]) {
                return Err(Error::BadChecksum);
            }
        }
        9 | 12 => {
            require_digits(n)?;
            let check = checksum::weighted(&n[..9], &[8, 7, 6, 5, 4, 3, 2, 10, 1]).rem_euclid(97);
            // Numbers from 100 upwards may also check out at 42 or 55, which is
            // how restarted registrations are distinguished.
            let ok = if parse_int(&n[..3]) >= 100 {
                matches!(check, 0 | 42 | 55)
            } else {
                check == 0
            };
            if !ok {
                return Err(Error::BadChecksum);
            }
        }
        found => {
            return Err(Error::WrongLength { expected: 9, found });
        }
    }
    Ok(())
}
