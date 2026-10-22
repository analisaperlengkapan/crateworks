# scripts

Development tools for the workspace. None of them is needed to build or test a
crate. `check_attribution.py` runs in CI, on the `test` job; the rest are run by
hand.

| Script | What it does |
| --- | --- |
| `check_attribution.py` | Fails when a crate credits an upstream project in its code but not in `NOTICE` (run in CI) |
| `publish.py` | Works out the order the crates can be published in, and checks or runs it |
| `fuzz/` | Generators for the fuzz corpora in `crates/*/tests/fixtures/` |

## check_attribution.py

Crates here are independent reimplementations of existing libraries, and the
upstream licence requires that library to be named in `NOTICE`. The credit is
easy to put in a source comment and forget in `NOTICE`: `stdnum-rs` cited
`python-stdnum` in `src/id/mod.rs` while its `NOTICE` said nothing, even though
`eu-vat`, from the same lineage, did name it. No compiler catches that, so the
script does.

It looks for a fixed list of upstream markers (repository paths such as
`python-stdnum` or `dralshehri`, chosen so a crate does not match its own name)
in each crate's `src`, `tests` and `README.md`, and requires the same marker in
that crate's `NOTICE`.

```sh
python3 scripts/check_attribution.py   # exit 1 and name the crate if a credit is missing
```

## publish.py

`cargo publish` resolves a `path` dependency against the registry, so a crate
that depends on another crate in this workspace cannot be packaged until that
dependency is published. Get the order wrong and the error is

```
error: failed to prepare local package for uploading
Caused by:
  no matching package named `xmlkit` found
```

which reads like a broken manifest but is only the order. `--no-verify` does not
help, because the failure is in resolving the dependency rather than in building
it.

The script derives the order from the manifests, so it cannot go stale the way a
hand-written list does: if a crate gains a workspace dependency, it moves.

```sh
python3 scripts/publish.py            # dry-run every crate, upload nothing
python3 scripts/publish.py --strict   # same, but exit 1 if anything is blocked
python3 scripts/publish.py --publish  # upload, in order, stopping at the first failure
python3 scripts/publish.py --publish --delay 65   # space uploads out, one per minute
```

The default mode uploads nothing and always exits 0 unless a crate genuinely
fails to build or package. A crate reported as `blocked` is the expected state
before its dependency is published, not an error; `--strict` is the flag to use
after publishing a dependency, to confirm the next group is now clear.

`--publish` prints the order and waits for the word `publish` before doing
anything. It stops at the first failure rather than carrying on, because a
published version number cannot be reused: if `0.1.0` of a crate is wrong, the
fix has to ship as `0.1.1`. Run the dry run first.

Each line reports one of four states:

| State | Meaning |
| --- | --- |
| `ready` | `cargo publish --dry-run` succeeded; this crate can be uploaded now |
| `blocked` | A workspace dependency is not on crates.io yet. Publish that first |
| `uncommitted` | Files in the crate are changed or untracked, so cargo refuses to package it |
| `FAILED` | Something else is wrong. Fix it before publishing anything |

The `uncommitted` check is scoped to each crate because cargo's is: a changed
file at the repository root does not stop `cargo publish -p xmlkit`, but one
under `crates/xmlkit/` does.

## Rate limiting

crates.io accepts one upload per minute with a burst of 30, so this workspace
fits inside the burst. A refused upload comes back as HTTP 429, which cargo
reports as a plain failure; the script waits 70 seconds and retries instead of
calling the crate broken. Pass `--delay 65` to stay under the limit rather than
relying on the retry.

`cargo publish --workspace` also orders by dependency and is the command to use
when cargo's own output is wanted. It cannot check without attempting an upload,
which is why this script exists for the dry run.
