# Contributing

Thank you for helping. This workspace is maintained by Yayasan Pesantren
Cipansor and contributors, and contributions of every size are welcome: a new
country calendar, one more identifier validator, a documentation fix, or a bug
report.

## Ground rules

Every crate in this workspace follows the same rules, and a change is expected
to keep them:

- **No dependency beyond the standard library** unless it is genuinely
  necessary. Each crate must be droppable into any project.
- **No network access.** Nothing here contacts a service or a registry.
  Validation is checksum and structure only.
- **Rules over tables.** A calendar or a validator is expressed as a rule that
  holds for any year or any input, not as a data table that goes stale. A table
  is acceptable when the standard itself is a table, such as an ISO 7064
  weight list, but prefer a rule when one exists.
- **Typed errors.** Errors are enums with a `Display` implementation, never a
  string.
- **Documented public API**, with a doctest on every crate root.
- **No `unsafe`.** The workspace forbids it at the lint level.

## Before you open a pull request

Run the same checks CI runs:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All three must pass. `-D warnings` means a warning is a build failure, so a
clean run locally is required.

## What makes a good contribution

### A new country or identifier

Add one country or one identifier per pull request. For a validator, the
algorithm must be verifiable against a source outside this repository, such as
the standard text, a government specification, or an established reference
implementation. A test that only restates your own implementation proves
nothing.

Include real example numbers. Where a source provides one, add it as a test
case and cite the source in a comment.

### Tests

Test the real code path. Do not reach for a mock unless there is no other way,
and say why in a comment when you do.

For a validator, prefer a table of known-good and known-bad inputs transcribed
from an authority. For a calendar, assert against dates published by the
country concerned, not against whatever the code currently returns.

Where a crate has to match a reference implementation exactly, hand-picked cases
are not enough on their own. The `object-hash`, `adhan` and `javaproperties`
crates each carry a random corpus generated against their reference, and each one
found a real divergence that the curated cases had passed. If you add such a
corpus, commit the generator with it, under `scripts/fuzz/`, so the corpus can be
rebuilt rather than guessed at. The generators are development tools: they are not
built by CI and are never a dependency of a crate.

### Documentation

If you change behaviour, update the crate's `README.md`. The README of each
crate is compiled as a doctest, so a documented example that no longer compiles
will fail the test suite.

## Reporting a bug

Open an issue with the input that triggers it, what you expected, and what
happened. If the input is a document you cannot share, reduce it to the
smallest input that still shows the problem.

## Security

Do not open a public issue for a security problem. See [`SECURITY.md`](SECURITY.md).

## License

By contributing you agree that your contribution is licensed under the Apache
License, Version 2.0 ([`LICENSE`](LICENSE)), matching the rest of
the workspace. There is no contributor licence agreement to sign. Add yourself
to the contributors list in the pull request if you would like to be credited.
