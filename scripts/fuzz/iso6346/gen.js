// Builds the iso6346 corpus: (owner, category, serial) -> check digit, as the
// reference computes it. Run with the `container-validator` npm package on the
// module path. See scripts/fuzz/README.md.

const fs = require('fs');
const ContainerValidator = require('container-validator');

const OUT = process.argv[2] || 'crates/iso6346/tests/fixtures/oracle.tsv';

// A seeded PRNG, so the corpus is the same every run. Math.random would make it
// impossible to rebuild a fixture from the committed script.
let state = 0x2545f491;
function random() {
  // xorshift32
  state ^= state << 13;
  state ^= state >>> 17;
  state ^= state << 5;
  state >>>= 0;
  return state;
}
function pick(list) {
  return list[random() % list.length];
}

const LETTERS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('');
const CATEGORIES = ['U', 'J', 'Z'];

// The published example goes first, so the fixture always contains it.
const rows = [['CSQ', 'U', 305438]];
const seen = new Set(['CSQU305438']);

while (rows.length < 4000) {
  const owner = pick(LETTERS) + pick(LETTERS) + pick(LETTERS);
  const category = pick(CATEGORIES);
  const serial = random() % 1000000;
  const ten = owner + category + String(serial).padStart(6, '0');
  if (seen.has(ten)) continue;
  seen.add(ten);
  rows.push([owner, category, serial]);
}

const lines = [];
for (const [owner, category, serial] of rows) {
  const ten = owner + category + String(serial).padStart(6, '0');
  const validator = new ContainerValidator();
  const checkDigit = validator.createCheckDigit(ten);
  if (checkDigit < 0) {
    throw new Error(`reference rejected ${ten}: ${validator.getErrorMessages()}`);
  }
  lines.push([owner, category, String(serial).padStart(6, '0'), String(checkDigit), ten + checkDigit].join('\t'));
}

fs.writeFileSync(OUT, lines.join('\n') + '\n');
console.error(`${lines.length} cases -> ${OUT}`);
