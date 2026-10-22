# Gap scan round 5: what is left after the first 17 crates (2026-10-22)

A continuation of `rust-crate-gaps-round4.md`. Every figure below was measured
today through the crates.io, npm and PyPI APIs. Tooling: `research/tools/`.

This round asks a narrower question than the earlier ones. The first four rounds
looked for the *best* gaps and produced the 17 crates that now exist. This round
asks **what is still open**, now that those 17 are published.

## Method, and one correction to it

The three figures from round 4 still apply:

1. **Demand in the source ecosystem** (npm/week, PyPI/month). Shows the problem
   is real; does **not** show anyone wants it in Rust.
2. **`recent_downloads` of the best Rust crate in the area** (90 days).
3. **`reverse_dependencies` of that crate.** The only real Rust signal.

The verdict rule, unchanged: a gap is real when the best crate has
`recent_downloads` under about 20,000 **and** near-zero reverse dependencies,
while its npm or PyPI counterpart has millions.

**Correction this round:** `research/tools/revdeps.py` prints a summary table as
an **import side effect** (it has no `if __name__ == "__main__"` guard). Importing
it to reuse `revdeps()` therefore runs its whole scan and triggers HTTP 429,
which makes the *next* search return empty and a real capability look like a gap.
I lost one run to this and nearly recorded "Hashids: nothing found". Use
`namecheck.py` or a self-contained script instead, and treat any "nothing found"
as a rate limit until it is re-checked.

## What is already served (checked, so it is not re-proposed)

The famous npm names are all served. Measured today, best crate by `recent`:

| Capability | Best Rust crate | recent | revdeps |
|---|---|---|---|
| HTTP client (axios) | `reqwest` | 224.7M | 31,522 |
| Bidi algorithm | `unicode-bidi` | 93.8M | 184 |
| Image resize (sharp) | `image` | 65.2M | 7,323 |
| Charts (matplotlib) | `plotters` | 58.9M | 533 |
| Validation (zod) | `validator` | 14.6M | 750 |
| Slugify | `slug` | 8.0M | 157 |
| HTML scrape (cheerio) | `scraper` | 9.8M | 1,338 |
| Cron (cron-parser) | `cron` | 9.3M | 463 |
| GraphQL | `async-graphql` | 4.2M | 329 |
| MQTT | `rumqttc` | 2.2M | 221 |
| CRDT (yjs) | `yrs` | 2.2M | 71 |
| Cassandra | `scylla` | 2.0M | 83 |
| PDF generation | `printpdf` | 1.6M | 104 |
| Language detect | `whatlang` | 1.6M | 70 |
| Puppeteer | `chromiumoxide` | 2.4M | 18 |
| OCR (tesseract.js) | `ocrs` | 260K | 18 |
| DOCX write | `docx-rs` | 787K | 66 |
| Barcode | `rxing` | 213K | 33 |
| EPUB | `epub` | 104K | 34 |
| RTF | `rtf-parser` | 71K | 5 |
| SAML | `saml-rs` | 48K | 5 |
| GTFS | `gtfs-structures` | 24K | 24 |
| vCard | `calcard` | 32K | 10 |

Two that look thin in a search but are not, once the right crate is found:

- **Mermaid** (npm `mermaid` 10.97M/week) - `mermaid-rs-renderer` 162K recent,
  40 revdeps. Served.
- **PDF form filling** (npm `pdf-lib` + `pdfkit` 11.47M/week) - `pdfium-render`
  1.67M recent, 70 revdeps, plus `lopdf` 12.6M / 392 for the low level. Served.

## The one gap that survived: BPMN 2.0

| | |
|---|---|
| Demand | npm `bpmn-moddle` **250,445/week**, `bpmn-js` 173,487/week, `camunda-bpmn-moddle` 33,141/week |
| Best Rust crate | `orvanta-api` 566 recent (an OpenAPI-generated stub, not a BPMN crate) |
| Best real Rust crate | `ebi_bpmn` 313 recent, 1 revdep |
| The whole field | 19 crates, **none above 600 recent**, none above 1 revdep |
| Name `bpmn` on crates.io | **FREE** (`bpmn-engine`, `feel`, `dmn` are taken; `bpmn-model`, `dmn-engine`, `bpmn-rs` are free) |

The field is not empty, it is fragmented and tiny: `ebi_bpmn` (parser, writer and
executor), `snurr` (66 recent), `bpm-engine-bpmn` (82), `bpmn-engine` (10),
`bpmn-sdk` (4), `bmbp_bpmn_model` (3), `bpxe-bpmn-schema` (49, 2021). Not one has
a user base. `sutra-feel` (492 recent, 1 revdep) has the FEEL expression engine
that BPMN conditions need, and `sutra-dmn` (61) has DMN, so the *rules* half
exists; what is missing is the **model** half.

### Scope that would fit this workspace

The workspace rule is std-only where possible and rules over tables, so the
honest scope is the part that needs no engine and no vendor SDK:

- **BPMN 2.0 XML model**: parse and write `<definitions>`, `<process>`,
  `<task>` and its types, `<sequenceFlow>`, the gateways, `<startEvent>` and
  `<endEvent>`, `<subProcess>`. This is a schema, which is the same kind of work
  as `factur-x` and `iso20022-rs`.
- **Graph validation**: every flow has a source and a target that exist; every
  node is reachable from a start event; every path ends at an end event. This is
  pure graph arithmetic and it is the part a caller can use on its own.
- **A token-flow interpreter**: execute a *sequential* process - start, task,
  exclusive gateway, end - over a caller-supplied task handler. This is the part
  that makes the crate useful rather than decorative, and it is still a
  state machine, not an engine.

Leave out: timers, message events, boundary events, compensation, multi-instance,
and the Camunda/Zeebe vendor extensions. Those need a scheduler and a cluster.

### Why it is not a certain win

`bpmn-moddle`'s 250K/week is mostly `bpmn-js`, a *diagram editor* in a browser.
That demand is for drawing and editing BPMN in JavaScript, which a Rust crate
cannot serve. The Rust-side use case is narrower: reading a `.bpmn` file in a
service, validating it, and running a sequential flow. That is a real use case
(workflow services, audit tooling, form routing) but it is not the same size as
the npm number, and the same warning from rounds 1-4 applies: npm demand is not
Rust demand. Every BPMN crate on crates.io today has near-zero users, which is
also the evidence that this is unproven.

Verdict: the widest remaining gap, and an unproven one, exactly like SOAP was in
round 1. Reasonable to build; not reasonable to expect it to be adopted quickly.

## Rejected this round, and why

- **Hashids** (npm 261K/week) - the "best" crate in a search is an Oliphaunt
  extension artifact, but `hashids-rust` exists (59 recent) and the algorithm is
  a toy. Already served, thinly, and not worth a crate.
- **DHCP server** - `esp-hal-dhcp-server` 622, `leasehund` 588, `mini-dhcp` 127.
  The demand is embedded (`no_std`), which is a different design constraint.
- **Snowflake ID** - `snowflake` 94K recent, 25 revdeps. Served.
- **Gender inflection** (npm `inflections` 4.6M/week) - `inflections` exists and
  is served, however old. Not a gap, just an old crate.
- **Cassandra driver** - `scylla` is the maintained driver (2.0M, 83). The
  `cassandra` crate is dead but nobody needs it.
- **Collaborative editing** - `yrs` and `automerge` are healthy.

## Recommended next crate

If one more crate is wanted, **BPMN 2.0 model and sequential interpreter** is the
only gap found this round that passes all three checks: real demand, near-empty
supply, and a free name. It also fits the workspace's shape (a schema plus rules,
comparable against an oracle - `bpmn-moddle` for the XML, and the BPMN 2.0
specification for the semantics) and it can be written std-only if the XML is
handled by `xmlkit`.

Everything else measured this round is served well enough that a new crate would
be competing with an incumbent that already has users.
