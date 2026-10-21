#!/usr/bin/env python3
"""Check whether crate names are free on crates.io before writing any code.

A crate name cannot be changed after publication, so this runs first, not last.
The check that mattered in practice: `iso20022` looked like an obvious gap, but
the name was already taken by a crate abandoned in 2020, so it could never be
published under that name.

Usage:
    python3 namecheck.py iso20022 iso20022-rs xmlkit

Exit code is 0 always; read the table. 404 means the name is free.
"""

import json
import sys
import time
import urllib.error
import urllib.request

API = "https://crates.io/api/v1/crates/{}"
HEADERS = {"User-Agent": "crateworks-namecheck"}


def probe(name: str) -> tuple[str, str]:
    """Return (verdict, detail) for one crate name."""
    req = urllib.request.Request(API.format(name), headers=HEADERS)
    try:
        with urllib.request.urlopen(req, timeout=15) as response:
            crate = json.load(response)["crate"]
            recent = crate.get("recent_downloads") or 0
            created = (crate.get("created_at") or "")[:10]
            updated = (crate.get("updated_at") or "")[:10]
            # A crate that has not been touched in years is dead but still
            # occupies the name, which is the case that matters most.
            stale = "DEAD " if updated < "2025-01-01" else "     "
            return "TAKEN", f"{stale}{recent:>9} recent  {created}..{updated}"
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return "FREE", ""
        return f"HTTP{error.code}", ""
    except Exception as error:  # noqa: BLE001 - report and continue
        return "ERR", str(error)[:40]


def main() -> None:
    names = sys.argv[1:]
    if not names:
        print(__doc__)
        return
    free = []
    for name in names:
        verdict, detail = probe(name)
        print(f"  {name:24} {verdict:10} {detail}")
        if verdict == "FREE":
            free.append(name)
        time.sleep(0.4)  # be polite to the registry
    print()
    print("free:", ", ".join(free) if free else "(none)")


if __name__ == "__main__":
    main()
