// Generates the navigation corpus: for a location, a date and a clock reading,
// which prayer has begun and which comes next, as the reference library answers.
//
// The reference compares whole instants. This crate returns a clock reading with
// no day attached, and it reduces a time into the day, so the two agree only when
// no time has rolled over midnight. A case where any of the six times lands on a
// neighbouring UTC date is therefore dropped rather than written with a guess -
// that is what keeps this a comparison against the reference and not against our
// own arithmetic.
//
// Run from this directory:  node gen_nav.js > ../../../crates/adhan/tests/fixtures/navigation.tsv
const adhan = require('adhan');
const { Coordinates, PrayerTimes, CalculationMethod, Madhab, HighLatitudeRule, Rounding, Prayer } = adhan;

const METHODS = {
  MuslimWorldLeague: CalculationMethod.MuslimWorldLeague,
  Egyptian: CalculationMethod.Egyptian,
  Karachi: CalculationMethod.Karachi,
  UmmAlQura: CalculationMethod.UmmAlQura,
  Dubai: CalculationMethod.Dubai,
  MoonsightingCommittee: CalculationMethod.MoonsightingCommittee,
  NorthAmerica: CalculationMethod.NorthAmerica,
  Kuwait: CalculationMethod.Kuwait,
  Qatar: CalculationMethod.Qatar,
  Singapore: CalculationMethod.Singapore,
  Tehran: CalculationMethod.Tehran,
  Turkey: CalculationMethod.Turkey,
  Other: CalculationMethod.Other,
};
const MADHABS = { Standard: Madhab.Shafi, Hanafi: Madhab.Hanafi };
const RULES = {
  MiddleOfTheNight: HighLatitudeRule.MiddleOfTheNight,
  SeventhOfTheNight: HighLatitudeRule.SeventhOfTheNight,
  TwilightAngle: HighLatitudeRule.TwilightAngle,
};
const ROUNDINGS = { nearest: Rounding.Nearest, up: Rounding.Up, none: Rounding.None };

// A small deterministic PRNG so the corpus can be rebuilt exactly.
let seed = 20261021;
function rnd() {
  seed = (seed * 1103515245 + 12345) & 0x7fffffff;
  return seed / 0x7fffffff;
}
function pick(obj) {
  const keys = Object.keys(obj);
  return keys[Math.floor(rnd() * keys.length)];
}

// Cities chosen so the day rolls over in neither direction: the times must land
// inside the same UTC date the answer is asked for.
const CITIES = [
  ['Jakarta', -6.2, 106.8],
  ['Makkah', 21.4225, 39.8262],
  ['Cairo', 30.0444, 31.2357],
  ['Istanbul', 41.0082, 28.9784],
  ['London', 51.5074, -0.1278],
  ['New York', 40.7128, -74.006],
  ['Karachi', 24.8607, 67.0011],
  ['Tokyo', 35.6762, 139.6503],
  ['Sydney', -33.8688, 151.2093],
  ['Sao Paulo', -23.5505, -46.6333],
  ['Nairobi', -1.2921, 36.8219],
  ['Reykjavik', 64.1466, -21.9426],
  ['Cape Town', -33.9249, 18.4241],
  ['Delhi', 28.6139, 77.209],
  ['Los Angeles', 34.0522, -118.2437],
  ['Dubai', 25.2048, 55.2708],
];

function prayerName(prayer) {
  switch (prayer) {
    case Prayer.Fajr: return 'Fajr';
    case Prayer.Sunrise: return 'Sunrise';
    case Prayer.Dhuhr: return 'Dhuhr';
    case Prayer.Asr: return 'Asr';
    case Prayer.Maghrib: return 'Maghrib';
    case Prayer.Isha: return 'Isha';
    default: return 'None';
  }
}

function clock(date) {
  return [
    String(date.getUTCHours()).padStart(2, '0'),
    String(date.getUTCMinutes()).padStart(2, '0'),
    String(date.getUTCSeconds()).padStart(2, '0'),
  ].join(':');
}

// Every time must be on the target date, and finite.
function onSameDay(times, date) {
  const y = date.getUTCFullYear();
  const m = date.getUTCMonth();
  const d = date.getUTCDate();
  return times.every(
    (t) => t instanceof Date && !Number.isNaN(t.getTime()) && t.getUTCFullYear() === y && t.getUTCMonth() === m && t.getUTCDate() === d,
  );
}

const rows = ['city\tlat\tlon\tyear\tmonth\tday\tmethod\tmadhab\trule\trounding\tnow\tcurrent\tnext\tfajr\tsunrise\tdhuhr\tasr\tmaghrib\tisha'];

let written = 0;
let dropped = 0;
const target = 4000;

while (written < target) {
  const [city, lat, lon] = CITIES[Math.floor(rnd() * CITIES.length)];
  const method = pick(METHODS);
  const madhab = pick(MADHABS);
  const rule = pick(RULES);
  const rounding = pick(ROUNDINGS);

  const year = 2024 + Math.floor(rnd() * 4);
  const month = 1 + Math.floor(rnd() * 12);
  const day = 1 + Math.floor(rnd() * 28);
  const date = new Date(Date.UTC(year, month - 1, day));

  const params = METHODS[method]();
  params.madhab = MADHABS[madhab];
  params.highLatitudeRule = RULES[rule];
  params.rounding = ROUNDINGS[rounding];

  const times = new PrayerTimes(new Coordinates(lat, lon), date, params);
  const six = [times.fajr, times.sunrise, times.dhuhr, times.asr, times.maghrib, times.isha];
  if (!onSameDay(six, date)) {
    dropped += 1;
    continue;
  }

  // A clock reading anywhere in the day, so the answers land on both sides of
  // every boundary.
  const now = new Date(Date.UTC(year, month - 1, day, Math.floor(rnd() * 24), Math.floor(rnd() * 60), Math.floor(rnd() * 60)));

  rows.push(
    [
      city, lat, lon, year, month, day, method, madhab, rule, rounding,
      clock(now), prayerName(times.currentPrayer(now)), prayerName(times.nextPrayer(now)),
      ...six.map(clock),
    ].join('\t'),
  );
  written += 1;
}

process.stderr.write(`wrote ${written} cases, dropped ${dropped} that rolled over\n`);
process.stdout.write(rows.join('\n') + '\n');
