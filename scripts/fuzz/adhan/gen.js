const adhan = require('adhan');
const { Coordinates, PrayerTimes, CalculationMethod, Madhab, HighLatitudeRule, Rounding } = adhan;

const METHODS = {
  MuslimWorldLeague: CalculationMethod.MuslimWorldLeague, Egyptian: CalculationMethod.Egyptian,
  Karachi: CalculationMethod.Karachi, UmmAlQura: CalculationMethod.UmmAlQura,
  Dubai: CalculationMethod.Dubai, MoonsightingCommittee: CalculationMethod.MoonsightingCommittee,
  NorthAmerica: CalculationMethod.NorthAmerica, Kuwait: CalculationMethod.Kuwait,
  Qatar: CalculationMethod.Qatar, Singapore: CalculationMethod.Singapore,
  Tehran: CalculationMethod.Tehran, Turkey: CalculationMethod.Turkey,
  Other: CalculationMethod.Other,
};
const MADHABS = { Standard: Madhab.Shafi, Hanafi: Madhab.Hanafi };
const RULES = { MiddleOfTheNight: HighLatitudeRule.MiddleOfTheNight, SeventhOfTheNight: HighLatitudeRule.SeventhOfTheNight, TwilightAngle: HighLatitudeRule.TwilightAngle };
const ROUNDINGS = { nearest: Rounding.Nearest, up: Rounding.Up, none: Rounding.None };
const KEYS = ['fajr','sunrise','dhuhr','asr','maghrib','isha'];

function pick(obj, rnd) { const ks = Object.keys(obj); return ks[Math.floor(rnd() * ks.length)]; }

// A small deterministic PRNG so the corpus can be regenerated exactly.
let seed = 20261021;
function rnd() { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x7fffffff; }

const rows = ['lat\tlon\tdate\tmethod\tmadhab\trule\trounding\tfajr_adj\tsunrise_adj\tdhuhr_adj\tasr_adj\tmaghrib_adj\tisha_adj\tfajr\tsunrise\tdhuhr\tasr\tmaghrib\tisha'];

// Coordinates: a broad sweep, weighted to the interesting latitudes.
function coordinate() {
  const r = rnd();
  let lat;
  if (r < 0.25) lat = -90 + 180 * rnd();          // anywhere
  else if (r < 0.5) lat = 48 + 42 * rnd();        // where the rules start
  else if (r < 0.65) lat = -48 - 42 * rnd();      // the southern equivalent
  else lat = -40 + 80 * rnd();                    // temperate
  lat = Math.max(-89.9, Math.min(89.9, lat));
  const lon = -180 + 360 * rnd();
  return [lat, lon];
}
function date() {
  const y = 2020 + Math.floor(rnd() * 12);
  const m = 1 + Math.floor(rnd() * 12);
  const d = 1 + Math.floor(rnd() * 28);
  return [y, m, d];
}

let made = 0, attempts = 0;
while (made < 6000 && attempts < 60000) {
  attempts++;
  const [lat, lon] = coordinate();
  const [y, m, d] = date();
  const mname = pick(METHODS, rnd), hname = pick(MADHABS, rnd);
  const rname = pick(RULES, rnd), oname = pick(ROUNDINGS, rnd);
  const adj = {};
  for (const k of KEYS) adj[k] = [0,0,0,0,1,-1,2,-2,5,-5][Math.floor(rnd() * 10)];

  const params = METHODS[mname]();
  params.madhab = MADHABS[hname];
  params.highLatitudeRule = RULES[rname];
  params.rounding = ROUNDINGS[oname];
  params.adjustments = adj;

  let pt;
  try { pt = new PrayerTimes(new Coordinates(lat, lon), new Date(Date.UTC(y, m - 1, d)), params); }
  catch (e) { continue; }

  const iso = x => (x && !isNaN(x) ? x.toISOString() : 'null');
  rows.push([lat, lon, `${y}-${m}-${d}`, mname, hname, rname, oname,
    adj.fajr, adj.sunrise, adj.dhuhr, adj.asr, adj.maghrib, adj.isha,
    iso(pt.fajr), iso(pt.sunrise), iso(pt.dhuhr), iso(pt.asr), iso(pt.maghrib), iso(pt.isha)].join('\t'));
  made++;
}
const target = process.argv[2] || 'crates/adhan/tests/fixtures/fuzz.tsv';
require('fs').writeFileSync(target, rows.join('\n') + '\n');
console.log('wrote', made, 'cases to', target, 'in', attempts, 'attempts');
