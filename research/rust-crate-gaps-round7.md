# Gap scan round 7: what is left after six rounds (2026-10-22)

A continuation of `rust-crate-gaps-round6.md`. Every figure was measured today
through the crates.io, npm and PyPI APIs.

The question this round answers is narrower than the earlier ones. Rounds 1-6
scanned by *popularity* (the npm top-400, the PyPI top list, curated shortlists)
and found that everything popular is served. This round asks the opposite
question: **where is the incumbent absent entirely, regardless of how loud the
demand is?**

## Method, and the correction it forces

1. Demand in the source ecosystem (npm/week, PyPI/month).
2. `recent_downloads` (90 days) and `reverse_dependencies` of the best Rust
   crate in the area.
3. Exact-name availability on crates.io (`404` means free).

A gap is real when the Rust side is empty or has `recent_downloads` under about
20,000 with near-zero reverse dependencies.

**A space round 5 and 6 never checked: office-document conversion.** Round 5
covered DOCX *writing* (`docx-rs`). Reading a DOCX and converting it to HTML was
not measured until now, and it turns out to be served: `office_oxide` is at
719,643 recent, `anydoc` at 849,714 and `docx-lite` at 56,988. Recorded so it is
not proposed as a gap later.

## The finding that matters: the npm top-400 is fully served, and so is SaaS

A fresh sweep of the obvious "Rust is behind" candidates found an incumbent
every time:

| Area | npm / PyPI counterpart | Best Rust crate | recent |
|---|---|---|---|
| PDF generation | `pdfkit`, `pdf-lib` | `typst-as-lib`, `oxidize-pdf`, many | 767,000+ |
| XLSX | `exceljs` | `calamine` / `rust_xlsxwriter` | 5,775,596 / 2,473,268 |
| DOCX | `docx` | `docx-rs`, `office_oxide` | 791,317 / 719,643 |
| Email | `nodemailer` | `lettre` | 7,958,782 |
| Cron | `node-cron` | `cron` / `croner` | 9,340,696 / 4,728,846 |
| Templating | `handlebars`, `nunjucks` | `askama` | 15,058,659 |
| JWT | `jsonwebtoken` | `jsonwebtoken` | 61,838,929 |
| OIDC | `openid-client` | `openidconnect` | 4,984,939 |
| WebAuthn / FIDO2 | `@simplewebauthn` | `webauthn-rs`, `passkey-types` | 3,234,302 |
| SAML | `passport-saml` | `samael` / `uppsala` | 254,682 / 236,019 |
| Image | `sharp` | `image` | 65,836,658 |
| Zip | `jszip` | `zip` | 88,979,022 |
| QR | `qrcode` | `qrcode` / `qrcodegen` | 6,229,143 |
| Diff | `diff` | `similar` / `dissimilar` | 13,029,162 |
| S3 | `@aws-sdk/client-s3` | `aws-sdk-s3` | - |
| Salesforce / Shopify / Twilio | their SDKs | `tree-sitter-sfapex` / `shopify_function` / `twilio` | 109,857 / 155,206 / 17,114 |
| QuickBooks / Plaid | their SDKs | `quickbooks` / `plaid` | 4,549 / 7,043 |

This is now a stable result across four rounds. **Do not scan by popularity
again.** The Rust ecosystem has closed the npm top-400.

## What is genuinely absent: specialist data, not general capability

The only places an incumbent is missing are narrow standards and domain data
sets where the audience is small. Two are worth recording.

### Candidate A: SWIFT MT / ISO 15022 (FIN) messages

| | |
|---|---|
| Demand | PyPI `mt-940` **499,079/month**, `mt940` **11,886/month**. Every bank statement that is not ISO 20022 is MT940/MT942. |
| Best Rust crate | `swift-mt-message` (3,974 recent, **0 revdeps**), plus `mt940` (2,623 recent, 0 revdeps). |
| Shape of the incumbents | `swift-mt-message` is a **serializer** built on a `serde` derive, at v3.1.5: it parses fields, not message semantics. `mt940` describes itself as "a strict MT940 bank statement parser", so MT940 *parsing* is genuinely taken. What is missing is the rest of FIN: MT101/MT103/MT202 payment messages, field-format validation, and the `:60F:`/`:62F:` balance chain. |
| Names free | `swift-mt`, `swiftmt`, `mt-parser`, `iso15022`, `mt940-rs` - all **FREE**. |
| Names taken | `swift-mt-message`, `mt940`, `camt053`, `mx-message` |

This is the strongest *demand* signal of the round, and it sits in the space this
workspace already occupies (`iso20022-rs`, `ebics`, `iso8583-msg`). The honest
framing is that it is **not empty**: MT940 parsing is served by a young crate
with zero dependents, and `swift-mt-message` covers field parsing. The opening
is the payment-message side (MT101/103/202) with validation and a round-trip
corpus, using the published FIN field grammar as the oracle.

### Candidate B: Quran text, tajweed and recitation data

| | |
|---|---|
| Demand | npm `quran-json` **2,827/week**, `quran` 122/week; PyPI `pyquran` 802/month. Modest but real, and stable. |
| Best Rust crate | Effectively none. `quran` is v0.0.1 from 2022 at **2 recent**; `dhad` 86, `nizaam-core` 124, `ilm_core` 53, `widequran` 5. |
| The field | `crates.io?q=tajweed` and `?q=tahfidz` return **total 0**. |
| Names free | `quran-rs`, `quran-data`, `tajweed`, `tahfidz` - all **FREE** |

Round 6 rejected the "empty but no demand" cases (MICR, PCL). This one is
different in one specific way: it is empty *and* the demand is measurable, and
the correctness oracle is a fixed, well-published data set (the Uthmani text and
the tajweed rules are public, and the JS/Python packages above expose the same
data). It is also the one area where this workspace has unusual domain
authority.

The caveat is the same as FatturaPA's: a few thousand npm downloads a week is a
small Rust audience, and the value is in the data being correct and complete
rather than in clever code.

## Rejected this round, with the number that rejected them

- **Office documents (DOCX/PPTX/XLSX to HTML).** Served: `office_oxide` 719,643,
  `anydoc` 849,714, `docx-lite` 56,988.
- **SaaS SDKs (QuickBooks, Xero, Plaid, Salesforce, HubSpot, Shopify, Twilio).**
  Each has a Rust crate; the thin ones are thin because the Rust audience is
  thin, and a generated REST client is not a durable crate.
- **Auth stack (OAuth2, OIDC, SAML, WebAuthn, FIDO2, JWT, TOTP, LDAP).** Fully
  served, several at tens of millions of recent downloads.
- **RDF / JSON-LD / SPARQL.** Served by the Oxigraph family (`oxttl` 723,396,
  `oxjsonld` 492,053, `oxrdfio` 460,073).
- **EPUB / MOBI.** `lychee-lib` 82,632 for EPUB parsing; MOBI is niche and
  already has `boko` (2,784) and `kindling-mobi` (831).

## Recommendation

Neither candidate is the clean win that EBICS was in round 6, because neither is
empty. Ranked by what is defensible:

1. **SWIFT MT payment messages (MT101/103/202) with validation.** Largest
   measured demand (499k/month on PyPI for statement data), a published FIN field
   grammar to port, and it extends the workspace's existing financial-message
   line. MT940 parsing itself is taken, so scope this to the payment messages and
   to semantic validation rather than to a statement parser.
2. **Quran / tajweed data crate.** Zero competition and real (if modest) demand,
   plus unusual domain authority for this workspace. The risk is audience size,
   not technical difficulty.

Everything else measured this round has an active incumbent and should be left
alone.

## Note on the pattern across rounds

Rounds 4-7 agree on one thing: in 2026 there is no popular npm or PyPI package
that Rust lacks. The remaining openings are all *standards and data sets* whose
audience is a few thousand users, where the value of a crate is correctness and
completeness against a published specification rather than novelty. A new crate
should be chosen for whether its oracle is unambiguous, not for how many people
download the JavaScript version.
