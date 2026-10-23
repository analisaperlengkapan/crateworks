# Gap scan round 14: the survivors re-measured, and the Islamic domain (2026-10-21)

Rounds 11, 12 and 13 each closed a headline before it was acted on. MRZ went
from an empty registry to served between rounds 10 and 11; round 12's "largest
opening in twelve rounds", the abandoned `serde_yaml`, turned out to have three
maintained successors by round 13. The lesson recorded in round 13 was that a
gap in a document is not a gap today, so this round does two things: it measures
every surviving candidate again, and it sweeps one domain no earlier round
touched.

Every figure was fetched on 2026-10-21 from `crates.io/api/v1`, `index.crates.io`
(the sparse index, for a real release date), `api.npmjs.org`, `pypistats.org`
and `api.github.com`. A gap is real when the Rust incumbent has under about
20,000 recent downloads and near-zero reverse dependencies, while the source
counterpart has real volume. Recent downloads are the rolling ninety-day figure,
so a crate that was downloaded once in 2019 does not count as in use.

## The surviving candidates, re-measured

The three round-13 survivors are all still open, but the successor columns show
why only one of them is worth entering.

| crate | recent | last release | repo | verdict |
| --- | --- | --- | --- | --- |
| `im` | 4,741,036 | 2022-04-29 | **archived**, 1,583 stars | see below |
| `fuzzy-matcher` | 9,454,303 | 2020-10-04 | **archived**, 295 stars | see below |
| `smartstring` | 13,074,627 | 2022-03-24 | **archived**, 543 stars | **closed in practice** |

### `smartstring` is closed, and round 13 missed it

`compact_str` did 66,232,948 recent downloads and shipped 2026-07-13. That is
five times `smartstring`'s audience, actively maintained, and it covers the same
capability (a small-string-optimised owned string). Round 13 listed `smartstring`
as "still open" without noticing the incumbent that had already taken the
audience. Do not build a small-string crate.

### `im` is contested

`rpds` does 4,764,247 recent downloads, shipped 2026-05-15, has 1,781 stars and
79 reverse dependencies. It covers persistent immutable collections, which is
exactly `im`'s use case, and it is maintained. `im` still carries 238 reverse
dependencies and a bigger audience, so the migration has not finished — but the
same reading that closed YAML in round 13 applies: an archived crate with a
maintained successor is a migration in progress, not an empty space. `im` is a
*watch*, not a build.

### `fuzzy-matcher` is the one that is genuinely open

`fuzzy-matcher` is archived with 295 stars, 9.45M recent downloads and **469
reverse dependencies**, and no release since October 2020. The neighbours do not
cover it:

| crate | recent | last release | note |
| --- | --- | --- | --- |
| `nucleo-matcher` | 2,096,750 | 2024-02-20 | matcher for the Helix editor; a different API, not a drop-in |
| `simsearch` | 53,278 | 2026-05-06 | small; 11 reverse dependencies |
| `fst` | 8,057,537 | 2021-06-06 | finite-state sets, not fuzzy scoring |
| `aho-corasick` | 296,245,918 | 2026-08-03 | exact multi-pattern search, not fuzzy |

The distinct thing `fuzzy-matcher` provides is a **relevance score** over
candidates (`fuzzywuzzy`-style, in Rust terms), which `aho-corasick` and `fst`
deliberately do not. The Python side is large and fragmented — `fuzzywuzzy`
10,907,322/month and its maintained fork `thefuzz` 8,008,774/month — which is
the same "abandoned leader plus a successor nobody standardised on" shape.

This is the strongest candidate this round: a general capability, not a
regulated standard, so the oracle problem does not apply. The correctness bar is
the scoring semantics, which the crate defines itself, and the test method is a
transcribed corpus of match/score pairs, which is the method this workspace
already uses.

## The Islamic and Indonesian domain: mostly claimed, and by this workspace

No earlier round searched a Muslim-majority market. The demand is real:
`hijri-converter` 2,835,293/month and `hijridate` 3,326,940/month on PyPI,
`arabic-reshaper` 4,806,276/month, `python-bidi` 8,274,765/month, `adhan` 36,100
and `moment-hijri` 14,274 weekly on npm.

The Rust registry looks empty for this domain — `adhan` 21 recent downloads,
`hijri` 11, `arabic` 18, `salah` 845. **But the emptiness is because this
workspace already ships the crates:**

| crate | in this workspace | covers |
| --- | --- | --- |
| `adhan` | yes, 1,507 LOC | prayer times by calculation method |
| `hijridate` | yes, 613 LOC | Umm al-Qura Hijri/Gregorian conversion |
| `arabic` | yes, 1,206 LOC | shaping, ligatures, harakat, normalisation |
| `holidays-rs` | yes | Indonesia among 19 calendars |
| `stdnum-rs` | yes | NPWP and NIK validation |

So the honest finding is the opposite of a gap: the domain is served *here*, and
the registry numbers are low because these crates are young, not because nobody
needs them. The names that remain genuinely free (`umm-al-qura`, `tashkeel`,
`arabic-text`, `salat`, `namaz`, `prayer-times`, `zakat`) are free because the
capability is already covered, not because there is an opening.

### What is genuinely missing in the domain

Three capabilities have no crate here and no incumbent anywhere:

| capability | demand | free name | note |
| --- | --- | --- | --- |
| **Islamic inheritance (faraid / mirath)** | not measurable on PyPI or npm | `faraid`, `mirath` | The share rules (2:1 male:female, the fixed shares, `awl` and `radd` adjustment) are a closed, well-specified algorithm. No package on either registry. The demand is institutional rather than in these registries, so this is a *judgement call*, not a measured gap. |
| **Zakat calculation** | `zakat` 72/month PyPI | `zakah`, `zakat` (taken, 3 recent) | Thin everywhere. Not a gap. |
| **Quran text and metadata** | `quran` 608/month PyPI, `quran-json` 2,827 weekly npm | `quran-rs`, `quran-text`, `tanzil` | The demand is real but small in these registries, and the capability is data-set-bound (the text plus per-verse metadata), which is the shape round 13 rejected for postcodes. |

`faraid` is the only one worth naming as a candidate, and it is the weakest kind:
a real capability with no registry signal. Round 13's method cannot confirm it,
and this round says so rather than dressing a guess as a measurement.

## New candidates outside the domain

Four areas were swept that no earlier round covered. Three produced a verdict.

### Candidate: subtitle formats (SRT / WebVTT / ASS / SSA)

| | |
| --- | --- |
| Demand | PyPI `srt` 969,672/month, `webvtt-py` **1,794,901/month**, `pysubs2` 193,478/month. npm `srt-parser-2` 86,911/week, `webvtt-parser` 44,314/week. |
| Best Rust crate | `subtp` 20,024 recent, 5 stars, 3 reverse dependencies, last release 2024-02-20. `subparse` 4,839 recent, **9 reverse dependencies, dormant since 2021-04-11**. `subtitler` 1,948 recent, 0 reverse dependencies. `vtt-rs` 177 recent. |
| Shape | Fragmented and thin, and the WebVTT demand is the largest number measured this round. |
| Names free | `subkit`, `subparse2`, `subtitle-rs`, `webvtt-rs` |

Round 13 raised subtitles and did not decide. This round decides: **build it,
because the demand is large and split across two formats with different specs.**
`webvtt-py` alone is 1.79M/month, and no Rust crate covers WebVTT and SRT
together with a real test corpus. The risk round 13 named is correct — subtitle
formats are forgiving, so a parser that handles 95% of real files looks
finished — which is why the corpus is the product. That is this workspace's
existing method.

### Candidate: OFX / QIF bank statements

| | |
| --- | --- |
| Demand | PyPI `ofxparse` 412,568/month, `qifparse` 33,848/month. npm `ofx-js` 28,550/week. |
| Best Rust crate | `ofx-rs` 2,951 recent, 4 stars, 1 reverse dependency. `ofxy` 1,274 recent, 2 stars, 0 reverse dependencies. `qif_parser` 363 recent. |
| Shape | Three single-author crates, none covering both formats, none with dependents. |
| Names free | `ofx-parse`, `ofxkit`, `ofx-qif`, `ofxparser` |

Still open, unchanged from round 13, and it extends the workspace's finance line
(`iso20022-rs`, `ebics`, `iso8583-msg`, `mt940`). Moderate demand — the same
order as `pyiso8583`, which this workspace already shipped a crate for. The
better of the two new candidates on fit; the subtitle candidate is better on
volume.

### Rejected this round

- **iCal / RFC 5545 recurrence.** npm `rrule` 1,767,336/week, PyPI `icalendar`
  7,563,624/month. **Served and active**: `rrule` 644,004 recent (released
  2025-04-20), `icalendar` 398,904 (released 2026-10-14), `ical` 331,621. Do not
  enter.
- **vCard.** PyPI `vobject` 1,418,872/month, npm `vcf` 159,574/week. Served:
  `vcard` 1,453 recent, `vcard-rs` 1,858 recent, `vobject` 1,139 recent — thin
  but three live crates for a small format. Not worth displacing.
- **EPUB.** PyPI `ebooklib` 903,541/month. Served: `epub` 103,344 recent
  (released 2025-10-29), `epub-builder` 32,504.
- **EXIF.** Served: `kamadak-exif` 3,956,350, `rexif` 1,417,558 (released
  2026-10-13).
- **ODS / ODF spreadsheets.** PyPI `odfpy` 5,197,259/month. `ods` is 4 recent
  and `odf` is free — but the capability is served by the workspace's own
  `spreadsheet-formula` and `pdfmake` line plus `calamine`-class readers, and
  the Python volume is mostly `odfpy` as a pandas dependency rather than a
  standalone need.
- **OpenTimestamps.** PyPI `opentimestamps` 166,938/month, npm 11,037/week.
  Served: `opentimestamps` 60,646 recent. A small, single-crate niche.
- **ZATCA / Indonesian e-invoice (`efaktur`, `coretax`).** Names free, no
  registry demand, and the workspace already covers e-invoicing with
  `factur-x`, `iso20022-rs` and `eu-vat`. Adding an Indonesian e-invoice profile
  is a product decision, not a measured gap.

## What this round concludes

1. **`fuzzy-matcher` is the strongest opening measured.** 469 reverse
   dependencies, 9.45M recent downloads, archived repository, no release since
   2020, and no successor covering relevance scoring. Build this first.
2. **`smartstring` is closed and round 13 was wrong to call it open.**
   `compact_str` has 66M recent downloads and a 2026 release. This is the third
   time a round's survivor list was stale on re-measurement, which is now the
   most reproducible finding in fourteen rounds: the correction rate on
   "still open" claims is high enough that the re-measurement is the work.
3. **`im` is contested by `rpds`** (4.76M recent, released 2026-05-15, 79
   reverse dependencies). Watch, do not build.
4. **Subtitles are worth building** — 1.79M/month for WebVTT alone, and the
   Rust side is fragmented with one dormant crate carrying nine dependents.
   **OFX/QIF is the smaller but better-fitting candidate**, extending the
   finance line.
5. **The Islamic domain is not a gap; it is this workspace's existing
   portfolio.** `adhan`, `hijridate` and `arabic` already ship here, and
   `holidays-rs` and `stdnum-rs` cover Indonesian calendars and NPWP/NIK. The
   low registry numbers are youth, not absence. The only unserved capability is
   `faraid` (Islamic inheritance), which has no registry signal and is therefore
   a judgement call rather than a measured gap — name the free names
   (`faraid`, `mirath`) and treat it as an institutional decision.
6. **Do not re-scan iCal, vCard, EPUB, EXIF, ODS, OpenTimestamps or small-string
   for "is there a crate".** The answer is recorded row by row above.

## Method note

The script is `tools/round14.py`:

```sh
python3 round14.py candidates   # re-measure the open survivors
python3 round14.py islamic      # sweep the Islamic/Indonesian domain
python3 round14.py names <name> # is a proposed name still free?
```

Two corrections to the earlier tooling are worth recording. The sparse index
exposes the publish date as `pubtime`, not `pubdate`, and a wrong field name
fails silently as an empty string. The index path for a two-character name is
`2/<name>`, and for a one- or three-character name it is `1/<name>` and
`3/<first>/<name>`. The API's `updated_at` is not a release date — it moves on a
yank or an owner edit — so `release_date` reads the index instead, which is what
makes "dormant since 2021" a measurement rather than a guess.

The rate-limit warning in `tools/README.md` applies in full: crates.io returns
HTTP 429 under load and a naive fetch reads that as "no such crate". Every
`FREE` in this document was re-checked, and the script sleeps 0.7s between
crates.io requests and retries a 429 with a growing backoff.
