# Gap scan round 13: the YAML opening closes, and two new candidates (2026-10-21)

Rounds 1-12 all converged on one shape: the npm and PyPI popularity lists are
fully served, and a gap is a *standard* a regulated industry is obliged to
implement. Round 12 added the "abandoned incumbent" as a second shape and named
`serde_yaml` — 97M recent downloads, archived repository, no release since March
2024 — the largest opening measured in twelve rounds.

This round re-measures that headline against the registry as it stands today,
and sweeps four areas no earlier round touched: interchange formats for finance,
media, music and text.

Every figure was fetched on 2026-10-21 from `crates.io/api/v1`, `api.npmjs.org`,
`pypistats.org` and `api.github.com`. A gap is real when the Rust incumbent has
under about 20,000 recent downloads and near-zero reverse dependencies, while the
source counterpart has real volume.

## Correction: the YAML opening is closed, and round 12's headline is wrong

Round 12 called `serde_yaml` the biggest opening in twelve rounds. Re-measured
today, the space behind it has three live successors, and the largest is not
young:

| crate | recent downloads | last release | repository | note |
| --- | --- | --- | --- | --- |
| `serde_yaml` | 97,228,781 | 2024-03-25 | **archived** | the abandoned original |
| `yaml-rust` | 36,996,503 | 2021-01-03 | dormant | the old parser |
| `yaml-rust2` | **19,652,829** | 2026-09-11 | active | the maintained fork |
| `serde_yaml_ng` | 11,648,856 | 2024-05-26 | active | drop-in fork of `serde_yaml` |
| `serde-saphyr` | **9,284,020** | 2026-10-13 | active, 235 stars | new serde layer on `saphyr` |
| `saphyr` | 1,044,513 | 2026-10-15 | active | pure-Rust YAML 1.2 parser |

The reading is unambiguous. `yaml-rust2` alone carries 19.6M recent downloads and
was released six weeks ago. `serde-saphyr` is a 2026 crate with 9.3M recent
downloads and a commit from yesterday. The `unsafe-libyaml` concern round 12
raised — that a replacement would have to carry a parser as well — is already
answered by `saphyr`, a pure-Rust YAML 1.2 parser.

**The abandoned original is not the same thing as an unserved audience.** Users
of `serde_yaml` have migrated, are migrating, or have three maintained crates to
migrate to. A fourth entrant would compete with `serde-saphyr` on its own ground,
which is the case round 5 warned about and round 11 saw happen to MRZ.

This is the second time a round-11/12 headline has closed before it was acted on
(MRZ went from empty to served between rounds 10 and 11). Recorded as a
correction, not as a rejection: **do not build a YAML crate.**

## The other round-12 candidates, re-measured

| crate | recent downloads | last release | repository | verdict |
| --- | --- | --- | --- | --- |
| `fuzzy-matcher` | 9,428,270 | 2020-10-04 | **archived**, 295 stars | **still open.** No successor covers the scoring use case. |
| `smartstring` | 13,069,177 | 2022-03-24 | **archived**, 543 stars, 23 issues | **still open.** |
| `im` | 4,753,789 | 2022-04-29 | **archived**, 1,583 stars, 53 issues | **still open**, and the largest audience of the three. |
| `multipart` | 5,674,786 | 2021-05-29 | archived | figures from round 12, not re-measured |
| `rustls-pemfile` | 77,627,787 | 2024-09-30 | archived | figures from round 12; superseded inside the rustls org |

`im` is the one worth naming: 1,583 stars, 53 open issues and an archived
repository is a large, engaged audience with nobody home. Immutable collections
are a general capability rather than a standard, so the oracle problem does not
apply — the correctness bar is the data-structure invariants, which are the
crate's own.

## New candidates this round

Four areas were swept that no earlier round covered. Two produced a candidate.

### Candidate 1: OFX / QIF bank statements

Every accounting and personal-finance tool imports a bank statement, and the two
formats for it are OFX (the SGML dialect most banks export) and QIF (Quicken).

| | |
| --- | --- |
| Demand | PyPI `ofxparse` **412,568/month**. npm `ofx-js` 28,550/week. |
| Best Rust crate | `ofx-rs` 2,938 recent, v0.2.0, **4 stars**, 1 dependent. `ofxy` 1,272, **2 stars**, 0 dependents. `qif_parser` 363. |
| Shape | Three tiny, single-author, unadopted crates. None covers both formats; none has a corpus; none is a library anything builds on. |
| Names free | `ofx-parse`, `ofxkit`, `ofx-qif` — all **FREE**. |
| Names taken | `ofx-rs`, `ofxy`, `ofx` (an OpenFX *video* binding, unrelated), `qif`, `qif_parser` |

The workspace is already here: `iso20022-rs` (camt), `ebics`, `iso8583-msg` and
`mt940`-adjacent work are all "a bank sends a fixed-format statement and an
application has to read it". OFX is the consumer-banking member of that family,
and it is the one with no Rust crate that anyone depends on.

The demand is moderate, not large: 412k/month on PyPI is an order of magnitude
below `pyhanko` (5M/month) and two below `iso8583-msg`'s `pyiso8583` (400k/month
— comparable, in fact). That places it in the same band as a crate this
workspace already shipped.

### Candidate 2: Subtitles (SRT / WebVTT / ASS / SSA)

| | |
| --- | --- |
| Demand | PyPI `srt` **969,672/month**, `pysubs2` 193,478/month. npm `srt-parser-2` 86,911/week. |
| Best Rust crate | `subtp` 19,896 recent, **5 stars**, 3 dependents, last release 2024-02-20. `subparse` 4,857, **dormant since 2021 but 9 dependents**. `subtitler` 1,948, **3 stars**, 0 dependents. |
| Shape | Fragmented and thin: the leader has 5 stars and has not shipped in twenty months. `subparse` is the interesting one — dormant for four years with nine crates still depending on it, which is the "dead crate with dependents" shape rounds 4 and 11 called the best opportunity. |
| Names free | `subkit` **FREE**. |
| Names taken | `subtitles`, `subtitle`, `subtp`, `subparse`, `subtitler` |

The demand is the largest of the two new candidates (969k/month) and the Rust side
is genuinely fragmented, with one dormant crate carrying nine dependents. The risk
is that subtitle formats are forgiving — a parser that handles 95% of real files
looks finished — which makes a test corpus the whole product, and that is the
method this workspace already uses. Note that earlier notes rejected subtitles
because `subtitler` is active; that is true but incomplete, since `subtitler` has
zero dependents and `subparse`'s nine dependents have had no release since 2021.
Worth a second look, not an automatic build.

## Rejections this round, with the number that rejected them

- **HLS / m3u8.** npm `hls.js` 6,015,378/week, `m3u8-parser` 605,057/week — the
  largest npm demand measured this round. **Served**: `hls_m3u8` 111,860 recent,
  `quick-m3u8` 13,568, `m3u8-rs` 313,907. Do not enter.
- **MIDI.** PyPI `mido` 1,642,568/month. Served by `midly` (181,753 recent, 200
  stars) and `wmidi` (29,331). Dormant but complete; a format with no remaining
  ambiguity.
- **Bioinformatics FASTA/FASTQ.** PyPI `biopython` 4,377,509/month. Served by the
  `bio` / `rust-bio` family (115,511), `needletail` (85,068), `seq_io` (54,995).
  A mature, coordinated ecosystem.
- **VCF / BCF variants.** PyPI `pysam` 1,218,720/month. **Served and active**:
  the `noodles` family — `noodles-vcf` 457,496, `noodles-bgzf` 581,327,
  `noodles-bcf` 31,581 — all released 2026-10-15. Do not enter.
- **SMILES / cheminformatics.** PyPI `rdkit` 4,679,992/month. `chematic-smiles`
  (19,130 recent, released 2026-10-20) and `purr` (13,573) are present; RDKit is
  a C++ library with Python bindings and cannot be matched by a single-purpose
  crate.
- **GEDCOM genealogy.** PyPI demand unmeasured and small; npm 508/week. Three
  tiny crates (`ged_io`, `geneweb`, `gedcomkit`). Empty-and-unwanted.
- **Postcode / address parsing.** PyPI `usaddress` 5,029,779/month, but the
  capability is data-set-bound (a country's address register), and Rust has no
  incumbent for the same reason `stdnum-rs` refuses registry lookups. Not a
  format gap.
- **Modbus / CAN.** npm `modbus-serial` 42,475/week. Served: `modbus` 34,638,
  `socketcan` 2,183,860. Industrial-protocol demand is small in the crate
  audience.
- **bencode / torrent.** Served by `bendy` (153,609) and `torrust-bencode`.
- **Pluralize / inflect.** npm `pluralize` 23,522,439/week, PyPI `inflect`
  22,422,041/month. Served by `Inflector` (19,150,643) and `pluralizer`
  (1,835,133, released 2026-10-19).
- **Humanize duration.** PyPI `humanize` 46,399,361/month. Served by `humantime`
  (71,149,912) for durations and `human-repr` for bytes.
- **Color science.** Served by `palette` (19,447,746) and `color` (6,133,495).
- **3D mesh STL/OBJ/PLY.** Served: `stl_io` 992,162, `tobj` 721,989, `ply-rs-bw`
  660,458.
- **Geospatial GeoJSON.** Served: `geo` 4,922,575, `geojson` 2,082,463,
  `geozero` 736,407.
- **Audio tags.** Served: `id3` 1,726,937, `lofty` 531,755.
- **MT940 / NACHA / SEPA.** `mt940` 2,706 recent, `sepa` 1,599 — thin, but PyPI
  demand is also thin (`mt940` 11,886/month, `nacha` 444/month) except SEPA
  (`sepaxml` 311,525/month), which this workspace's `iso20022-rs` already
  addresses on the XML side. Watch MT940; do not build it alone.

## What this round concludes

1. **The YAML opening is closed.** Round 12's headline was measured correctly but
   read wrongly: an archived incumbent with 97M downloads and three active
   successors is a *migration in progress*, not an unserved audience. Do not
   build a YAML crate. This is the second headline to close before it was acted
   on, which is itself the lesson — a gap that is "the registry is empty" or
   "the incumbent is dead" can close within weeks, so the scan has to be repeated
   before the work starts, not trusted from the document.
2. **The abandoned incumbent that is still open is `im`.** 1,583 stars, 53 open
   issues, archived repository, 4.75M recent downloads. `fuzzy-matcher` (9.4M,
   archived) and `smartstring` (13M, archived) remain open too. These are the
   three round-12 survivors; nothing this round displaced them.
3. **Two new candidates, both in the workspace's own idiom: OFX/QIF bank
   statements and subtitle formats.** OFX is the better fit — it extends the
   finance line (`iso20022-rs`, `ebics`, `iso8583-msg`) with a format whose
   Python incumbent does 412k/month against a Rust leader of 2,938 downloads and
   4 stars. Subtitles have the larger demand (969k/month) and a dormant crate
   with nine dependents, but a looser spec.
4. **Do not re-scan finance interchange or subtitle formats for "is there a
   crate".** The answer is recorded row by row above.

## Method note

The scripts for this round are `tools/round13*.py`-style ad-hoc probes run in
sequence with sleeps between requests; the permanent tool remains `tools/stale.py`
from round 12, which is what should be run before any candidate is acted on. The
warning in `tools/README.md` applies with full force here: crates.io rate-limits,
and a 429 returns as an empty result, so a "nothing found" is a rate limit until
it is re-checked. Every "nothing found" above was re-checked.
