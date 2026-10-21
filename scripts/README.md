# scripts

Development tools for the workspace. Nothing here is compiled into a crate, run
by CI, or needed to build or test anything.

| Script | What it does |
| --- | --- |
| `publish.py` | Works out the order the crates can be published in, and checks or runs it |
| `fuzz/` | Generators for the fuzz corpora in `crates/*/tests/fixtures/` |

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
