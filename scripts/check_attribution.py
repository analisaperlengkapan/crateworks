#!/usr/bin/env python3
"""Fail if a crate credits an upstream project in its code but not in NOTICE.

Several crates here are independent reimplementations of an existing library,
and the licence requires that library to be named in NOTICE. The credit is easy
to add to a source comment and easy to forget in NOTICE, which is how
`stdnum-rs` ended up citing python-stdnum in `src/id/mod.rs` while its NOTICE
said nothing.

The list of upstream projects lives in `attribution.toml` next to this script,
so adding a crate means editing data rather than code. Each entry has a marker,
usually a repository path, chosen so a crate cannot match its own name
(`hijridate` matches itself; `dralshehri` does not).

Exits 1 and prints the offending crate when a marker is missing from NOTICE.
"""

import sys
import tomllib
from pathlib import Path

# Where a credit is allowed to appear. NOTICE is the file being checked.
CREDIT_SOURCES = ("src", "tests", "README.md")


def load_markers(path: Path) -> dict[str, str]:
    data = tomllib.loads(path.read_text())
    return {e["marker"]: e["project"] for e in data["upstream"]}


def texts(crate: Path):
    for rel in CREDIT_SOURCES:
        p = crate / rel
        if p.is_file():
            yield p
        elif p.is_dir():
            yield from sorted(p.rglob("*.rs"))


def check(root: Path, markers: dict[str, str]):
    failures = []
    for crate in sorted((root / "crates").iterdir()):
        if not crate.is_dir():
            continue
        notice = crate / "NOTICE"
        notice_text = notice.read_text() if notice.is_file() else ""
        credited = {m for m in markers if any(m in f.read_text() for f in texts(crate))}
        for marker in sorted(credited):
            if marker not in notice_text:
                failures.append((crate.name, marker, markers[marker]))
    return failures


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    markers = load_markers(root / "scripts" / "attribution.toml")
    failures = check(root, markers)
    for crate, marker, project in failures:
        print(
            f"{crate}: NOTICE does not mention {project!r} "
            f"(found {marker!r} in the crate's own files)",
            file=sys.stderr,
        )
    if failures:
        print(
            f"\n{len(failures)} crate(s) credit an upstream project without "
            "naming it in NOTICE. Add the attribution there.",
            file=sys.stderr,
        )
        return 1
    print(f"attribution: {len(markers)} upstream projects tracked, all named in NOTICE")
    return 0


if __name__ == "__main__":
    sys.exit(main())
