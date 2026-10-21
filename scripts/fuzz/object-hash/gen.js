//! Builds the object-hash fuzz corpus: random values, each with the reference's
//! canonical byte stream and digest. Run with the `object-hash` npm package on
//! the module path. See scripts/fuzz/README.md.

const objectHash = require('object-hash');

const NUMS = [
  0, 1, -1, 42, 0.5, 1e21, 1e-7, 1e20, 1 / 3, 9007199254740992, -0.0,
  NaN, Infinity, -Infinity, 123.456, 1e-6, 5e-324, 1.7976931348623157e308,
  1.23e22, -1e21, 1e100,
];
const STRS = ['', 'a', 'ab', 'café', '中文', '😀', 'a:b', '42', '\u0000x', '\uFFFFz'];

// A seeded PRNG, so the corpus is the same every run. Math.random would make it
// impossible to rebuild a fixture from the committed script.
let state = 0x2f6e2b1;
function random() {
  // xorshift32
  state ^= state << 13;
  state ^= state >>> 17;
  state ^= state << 5;
  state >>>= 0;
  return state / 0x100000000;
}

function pick(items) {
  return items[Math.floor(random() * items.length)];
}

function randomValue(depth) {
  if (depth <= 0 || random() < 0.35) {
    const kind = Math.floor(random() * 4);
    if (kind === 0) return null;
    if (kind === 1) return random() < 0.5;
    if (kind === 2) return pick(NUMS);
    return pick(STRS);
  }
  const kind = Math.floor(random() * 3);
  const length = Math.floor(random() * 4);
  if (kind === 0) {
    return Array.from({ length }, () => randomValue(depth - 1));
  }
  if (kind === 1) {
    const set = new Set();
    for (let i = 0; i < length; i++) set.add(randomValue(depth - 1));
    return set;
  }
  const keys = ['a', 'b', 'café', '', 'k1', 'z'];
  const map = new Map();
  for (let i = 0; i < length; i++) map.set(pick(keys), randomValue(depth - 1));
  return map;
}

// The reference writes its canonical form to a stream. A passthrough algorithm
// hands back exactly the bytes the digest is taken over.
function canonical(value) {
  let bytes = '';
  const stream = { write: (x) => (bytes += x), update: (x) => (bytes += x) };
  objectHash.writeToStream(value, { algorithm: 'passthrough' }, stream);
  return bytes;
}

// The value as a tagged string the Rust side can rebuild: `{"n":..}` numbers,
// `{"a":..}` arrays, `{"s":..}` sets, `{"m":..}` maps. Numbers go through
// `String` so NaN, the infinities and the exponent forms survive.
function tagged(value) {
  if (value === null) return 'null';
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'number') {
    if (Number.isNaN(value)) return '{"n":"NaN"}';
    if (value === Infinity) return '{"n":"Infinity"}';
    if (value === -Infinity) return '{"n":"-Infinity"}';
    return '{"n":"' + String(value) + '"}';
  }
  if (typeof value === 'string') return JSON.stringify(value);
  if (Array.isArray(value)) return '{"a":[' + value.map(tagged).join(',') + ']}';
  if (value instanceof Set) return '{"s":[' + [...value].map(tagged).join(',') + ']}';
  const parts = [];
  for (const [key, item] of value) parts.push('[' + JSON.stringify(key) + ',' + tagged(item) + ']');
  return '{"m":[' + parts.join(',') + ']}';
}

const rows = [];
for (let i = 0; i < 4000; i++) {
  const value = randomValue(3);
  rows.push(tagged(value) + '\t' + canonical(value) + '\t' + objectHash(value));
}

const target = process.argv[2];
if (!target) {
  console.error('usage: node gen.js <path to fuzz.tsv>');
  process.exit(1);
}
require('fs').writeFileSync(target, rows.join('\n') + '\n');
console.log('wrote', rows.length, 'cases to', target);
