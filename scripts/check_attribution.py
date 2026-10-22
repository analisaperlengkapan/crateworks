#!/usr/bin/env python3
"""Fail if a crate credits an upstream project in its code but not in NOTICE.

Several crates here are independent reimplementations of an existing library,
and the licence requires that library to be named in NOTICE. The credit is easy
to add to a source comment and easy to forget in NOTICE, which is how
`stdnum-rs` ended up citing python-stdnum in `src/id/mod.rs` while its NOTICE
said nothing. This script looks for a short list of upstream markers in each
crate's sources, tests and README, and requires NOTICE to carry the same marker.

Markers are distinctive fragments (usually a repository path) rather than bare
project names, so that a crate's own name does not match itself: `hijridate`
matches itself, `dralshehri` does not.

Exits 1 and prints the offending crate when a marker is missing from NOTICE.
"""

import sys
from pathlib import Path

# marker -> the project it stands for, for the error message
MARKERS = {
    "batoulapps/adhan-js": "adhan (JS) by Batoul Apps",
    "arabic-reshaper": "arabic-reshaper by Abdullah Diab",
    "python-stdnum": "python-stdnum by Arthur de Jong",
    "akretion/factur-x": "factur-x by Alexis de Lattre",
    "dralshehri": "hijridate by Mohammed Alshehri",
    "pyiso8583": "pyiso8583 by Konstantin Novichikhin",
    "jwodder/javaproperties": "javaproperties by John T. Wodder II",
    "puleos/object-hash": "object-hash (npm) by Stefan Judis and David Bittner",
    "python-zeep": "zeep by Mathieu Leplatre",
}

# Where a credit is allowed to appear. NOTICE is the file being checked.
CREDIT_SOURCES = ("src", "tests", "README.md")


def texts(crate: Path):
    for rel in CREDIT_SOURCES:
        p = crate / rel
        if p.is_file():
            yield p
        elif p.is_dir():
            yield from sorted(p.rglob("*.rs"))


def check(root: Path):
    failures = []
    for crate in sorted((root / "crates").iterdir()):
        if not crate.is_dir():
            continue
        notice = crate / "NOTICE"
        notice_text = notice.read_text() if notice.is_file() else ""
        credited = {m for m in MARKERS if any(m in f.read_text() for f in texts(crate))}
        for marker in sorted(credited):
            if marker not in notice_text:
                failures.append((crate.name, marker, MARKERS[marker]))
    return failures


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    failures = check(root)
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
    print("attribution: every credited upstream project is named in NOTICE")
    return 0


if __name__ == "__main__":
    sys.exit(main())
