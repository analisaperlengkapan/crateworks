# Fuzz corpus generators

The fuzz corpora in `crates/*/tests/fixtures/fuzz.tsv` are not hand-written. Each
was produced by asking the reference implementation, in its own language, for the
answer to a few thousand inputs that a human would not think to try, and recording
it. This directory keeps the generator for each one, so a corpus can be rebuilt
and extended instead of guessed at.

They are development tools. Nothing here is compiled into a crate, run by CI, or
needed to build or test the workspace. A crate's dependency on the standard
library alone is unaffected.

## Why they are committed

A fixture without its generator is a dead end: when a case has to be added or a
seed changed, the only option left is to write a second generator that reproduces
the first, and it will not. Committing the generator makes the corpus
reproducible, and a corpus you can rebuild is one you can grow.

## What each one needs

Each generator drives a reference library that is not part of this repository, so
each needs that library installed before it will run. This is reference material,
never a dependency of any crate.

| Crate | Generator | Reference library | Run |
| --- | --- | --- | --- |
| `object-hash` | `object-hash/gen.js` | `object-hash` npm 3.0.0 | `NODE_PATH=<modules> node object-hash/gen.js [out]` |
| `adhan` | `adhan/gen.js` | `adhan` npm 4.4.6 | `NODE_PATH=<modules> node adhan/gen.js [out]` |
| `javaproperties` | `javaproperties/gen.py` | `javaproperties` PyPI 0.8.2 | `python3 javaproperties/gen.py [out]` |
| `iso6346` | `iso6346/gen.js` | `container-validator` npm 1.1.0 | `NODE_PATH=<modules> node iso6346/gen.js [out]` |

With no `out` argument each writes to the crate's own fixture, which is the path
you want when rebuilding it in place. The recorded versions are the ones the
committed fixtures were built against; a newer reference may legitimately change
an answer, and then the crate has to change with it or say why not.

`object-hash/gen.js` and `javaproperties/gen.py` take the output path from an
argument or a constant; `adhan/gen.js` writes straight to the crate fixture. Note
the one thing the npm scripts need: `NODE_PATH` has to point at a `node_modules`
holding the reference package, because the script is run from a directory where
it is not installed.

## Determinism

Every generator uses a fixed seed, so the same script produces the same corpus
every time. That is the point: a fixture that changes on each run cannot be said
to be checked. `object-hash/gen.js` uses a small xorshift, `adhan/gen.js` a
linear congruential generator, `javaproperties/gen.py` Python's `random` with a
set seed.

The first version of the object-hash corpus used `Math.random`, which made it
unreproducible; it has since been replaced. If you change a seed you are changing
the corpus, so regenerate the fixture in the same commit and let the crate's test
confirm the crate still agrees.

## The lesson each corpus taught

These are here because the curated fixtures, chosen by hand, all passed while the
crate was still wrong. The value of a corpus is exactly the cases nobody thought
to write down.

- `object-hash` — 4,000 values: numbers past 1e21 and below 1e-6, strings above
  U+FFFF, and the `Set`/`Map` deduplication rules. Found three divergences the 74
  curated cases missed.
- `adhan` — 6,000 cases: latitudes near the poles, the manual adjustments, and
  `Rounding::None`. Found three gaps the 37,440-time grid missed, because the grid
  never set an adjustment and only used the default rounding.
- `javaproperties` — 6,000 documents built from the format's own punctuation.
  Found nothing, which is the useful outcome too: it is what turns "the rules we
  thought of are right" into "the rules we checked agree".
- `iso6346` — 4,000 codes from every owner-prefix letter, all three categories and
  a serial across the whole range. Found nothing: the check digit is one page of
  arithmetic and the reference agrees with it on every case. The corpus is what
  makes that a measurement rather than a claim.
