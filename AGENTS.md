# AGENTS.md

Instructions for anyone, human or agent, working in this repository.

## What this is

`crateworks` is a Cargo workspace of small, single-purpose Rust crates. Each one
fills a gap where npm or PyPI has a mature library and Rust does not, and each is
published to crates.io independently. There is no umbrella crate.

Crates are grouped by domain in the root `README.md`. `CONTRIBUTING.md` states
the rules a contribution must keep — read it before changing behaviour.
`PLAN.md` records the build history and per-crate scope, but it was written early
and covers only the first seven crates; for anything later, the crate's own
`README.md` is current and `PLAN.md` is not. Fixing that is a worthwhile change.

## Commands

Everything is checked by CI (`.github/workflows/ci.yml`) and must pass locally:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Single crate, which is much faster while iterating:

```sh
cargo test -p adhan
cargo clippy -p adhan --all-targets -- -D warnings
```

Publishing order and readiness, without uploading anything:

```sh
python3 scripts/publish.py
```

Cargo lives at `$HOME/.cargo/bin` in this environment. `rust-toolchain.toml`
pins `stable`; edition is 2024 and `rust-version` is 1.88.

`rust-version` is a **floor**, not the toolchain CI builds with. CI runs two
jobs: one on current stable, one that installs 1.88 and runs `cargo check
--workspace --all-targets`, clippy with `-D warnings`, and the doctests. The
second job exists because the floor was once wrong for months: it read 1.85 while
`pdfa`, `iso20022-rs` and `soap-client` used let-chains, which need 1.88. Nothing
caught it because no job ever compiled against the declared minimum. If you raise
the floor deliberately, raise it in `Cargo.toml` and in the `msrv` job's
`toolchain:` input together, or the job will pass while the claim is false.

## Non-negotiable standards

A change that breaks one of these is not mergeable, however good it is otherwise.

- License is **Apache-2.0 only**. Every crate ships `LICENSE` and `NOTICE`.
- **No dependency beyond the standard library** unless it is genuinely
  necessary. `xmlkit` is the one crate other crates may depend on; it is part of
  this workspace. No network access anywhere.
- **Failures carry a dedicated error type with a `Display` implementation**,
  never a bare `String`. Most crates use an enum of variants; `xmlkit` uses a
  struct carrying a byte offset and a message, because where the parse failed is
  the useful part. Crates whose functions cannot fail (`adhan`, `arabic`,
  `holidays-rs`, `object-hash`) have no error type at all, deliberately.
- **`unsafe` is forbidden** at the workspace lint level. Do not add an `allow`.
- **No `#[allow(dead_code)]`.** If something is unused, delete it. More
  generally, do not silence a lint to make it pass: fix the cause. The only
  allows in the repository are five narrowly scoped clippy exceptions
  (`too_many_arguments` on functions mirroring a reference's parameter list,
  `similar_names` in tests); treat that as the ceiling, not a precedent.
- Every crate has a **doctest on its crate root** (`# Example` in `src/lib.rs`)
  and a **`tests/readme.rs`** that executes the README example. Both must exist;
  the README test is what stops the documentation drifting from the API.
- `-D warnings` means a clippy warning fails the build. The workspace runs
  `clippy::all` plus `missing_debug_implementations`.

## How correctness is established here

This is the part that is easy to get wrong.

**Never invent an expectation. Take it from a reference implementation.** Where a
crate reproduces behaviour another ecosystem already defines, install that
implementation, ask it for the answer, and record the answer as a fixture. The
oracles used so far: `python-stdnum` (`eu-vat`), `pyiso8583` (`iso8583-msg`),
`arabic-reshaper` (`arabic`), the official `factur-x` package, the JavaScript
`adhan` package, `object-hash` npm v3, and Python `javaproperties`. An oracle is
reference material for development; it is **never** a dependency of a crate and
is never installed by CI.

**When a port disagrees with a fixture, read the oracle source before "fixing"
the port.** Most failures turn out to be a wrong expectation, not a bug in the
crate. Diff them.

**Hand-picked cases are not enough when the output must match a reference.**
Generate a few thousand random inputs, run them through the oracle, commit the
result as a corpus under `crates/<name>/tests/fixtures/`, and **commit the
generator too**, under `scripts/fuzz/<name>/`. A fixture without its generator
cannot be rebuilt or extended. Two of the three corpora in the repository found a
real divergence the curated cases had passed straight over; the third found
nothing, which is also worth having, because it is what turns "the cases we
thought of are right" into "the cases we checked agree".

**A generator is code, and its bugs look exactly like crate bugs.** Before
concluding the crate is wrong, print the failing case and check that the fixture
means what you think it means. This has cost two separate debugging rounds.

## Known traps

- **A newer clippy can fail an untouched crate.** `-D warnings` turns every new
  lint into a build failure, so a green build on one toolchain says nothing about
  the next. A clippy bump to 1.99 produced 17 failures across nine crates, none of
  which had changed: `manual_is_multiple_of` (`x % 2 == 0` becomes
  `x.is_multiple_of(2)`), `chunks_exact_to_as_chunks` (a constant chunk size
  becomes `as_chunks::<64>().0`), and `collapsible_if` (a nested `if let` becomes
  a let-chain). The fix is to satisfy the lint, not to add an `allow`; the repo's
  five existing allows are the ceiling, not a precedent.
- **The lints differ in BOTH directions, so the floor must be linted too.**
  `uninlined_format_args` is warn-by-default in 1.88 but moved to `pedantic` by
  1.99, so `format!("{:?}", x)` is an error on the floor and fine on stable.
  Fixing those five sites in favour of 1.88 keeps both green; an `allow` would
  have papered over it. Lint the floor, not just stable, or the floor's clippy
  stays red forever.
- **A credit in the code must also be in `NOTICE`.** Several crates are
  independent reimplementations of an existing library, and the upstream licence
  requires that library to be named in `NOTICE`. `stdnum-rs` cited
  `python-stdnum` in `src/id/mod.rs` while its `NOTICE` listed nothing, even
  though `eu-vat`, which shares that lineage, did name it. No compiler catches
  this, so `scripts/check_attribution.py` does, in CI. When you add a crate that
  reimplements something, add an entry to `scripts/attribution.toml` (the data
  file the script reads) and the attribution to the crate's `NOTICE` in the same
  change. The marker must be distinctive to the upstream project, usually a
  repository path; a generic marker such as `xmlkit` matches every dependent.
- **The README guard is a real test, not ceremony.** Adding the `ReadmeDoctests`
  marker to `iso20022-rs` immediately failed: its README said
  `use iso20022-rs::camt::BankStatement;`, and a hyphen is not legal in a Rust
  path, so the documented import could never have compiled. The marker belongs in
  every crate's `src/lib.rs`; a crate without one has an unverified README.
- **`rust-toolchain.toml` outranks the toolchain you select.** `rustup` reads the
  file in the working directory, so `rustup run 1.88.0` or a CI job whose action
  installs 1.88 still compiles with `stable` unless `RUSTUP_TOOLCHAIN` is set
  (that variable beats the file) or the action uses `override: true`. This is why
  the MSRV job sets both: without them the job would pass while testing the wrong
  compiler, which is exactly how a `rust-version` of 1.85 survived in a tree that
  needed 1.88.
- **The toolchain is wiped when the sandbox restarts.** `$HOME/.cargo` and
  `$HOME/.rustup` disappear; `target/` survives. Reinstall rustup, or install to a
  directory that is not under `$HOME` (for example
  `CARGO_HOME=/workspace/.toolchain/cargo RUSTUP_HOME=/workspace/.toolchain/rustup`)
  to keep it across restarts. `pip` packages are wiped too.
- **`cargo clippy` prints nothing on a cached build.** If a lint seems to have
  vanished, `touch crates/*/src/lib.rs` (or `find crates -name '*.rs' -exec touch {} +`)
  to force a rebuild before believing the result.
- **`xmlkit` name matching.** `Element::child` and `children_named` match the
  full name including the prefix, so `child(el, "IssueDateTime")` does **not**
  find `ram:IssueDateTime`. Match on the local name. `Element::text()`
  concatenates only direct text children, so a value nested in a sub-element
  comes back empty.
- **`cargo publish` resolves path dependencies against the registry.** A crate
  that depends on `xmlkit` cannot be packaged until `xmlkit` is *published*, and
  the error ("no matching package named `xmlkit` found") reads like a broken
  manifest but is only the order. **`--no-verify` does not help**: the failure is
  in dependency resolution, not in building.
- **Cargo's uncommitted-files check is per package.** A dirty file at the
  repository root does not stop `cargo publish -p xmlkit`; one under
  `crates/xmlkit/` does. `scripts/publish.py` reports this per crate.
- **A published version number cannot be reused or deleted.** If `0.1.0` is
  wrong, the fix ships as `0.1.1`. Dry-run before uploading.
- **A crate name cannot be changed after publication.** Check with
  `research/tools/namecheck.py` before writing code, not after.
- **crates.io category names must be in the live list**, which is not the same as
  an intuitive guess. Fetch
  `https://crates.io/api/v1/categories?page=N&per_page=100` (it paginates at 10
  per page) rather than trusting a hardcoded set.

## Boundaries

- Branch `main` only. Do not create long-lived branches without being asked.
- **Never commit `.openhands/`** — it holds agent memory and is not intended to
  be public. Stage with `git add -A -- ':!.openhands'`.
- Do not push and do not open a pull request unless asked to.
- `git push` prompts and then hangs, because `origin` is plain HTTPS. Use
  `GIT_TERMINAL_PROMPT=0 git push "https://x-access-token:${GITHUB_TOKEN}@github.com/cipansor/crateworks.git" main`.
- If a change deleted something that was never committed, `git checkout -- <path>`
  recovers it. Do not reach for destructive git commands to undo work; ask first.

## Where things live

```
crates/<name>/          one crate: src/, tests/, README.md, LICENSE, NOTICE
scripts/publish.py      derive and run the publish order (dry-run by default)
scripts/fuzz/<name>/    corpus generators (dev tools, never built by CI)
research/               the measurements that identified the gaps
research/tools/         the scripts that produced those numbers
PLAN.md                 build history and per-crate scope
```

## Documentation language

Everything in this repository is written in **English**: crate and root READMEs,
doc comments, `AGENTS.md`, `CONTRIBUTING.md`, `PLAN.md`, `research/`, commit
messages, and the messages and status strings printed by the scripts. The owner's
first language is Indonesian, but the audience for a published crate is
international, so the documents are English and the chat can be Indonesian.

Comments explain why, not what: state a non-obvious invariant, a workaround, or a
deliberate trade-off. Do not restate the code or narrate the change history.
