#!/usr/bin/env python3
"""Round 13: re-measure the round-12 headlines, and sweep four new areas.

Run this before acting on any candidate from `rust-crate-gaps-round12.md`. Its
main result is a correction: the YAML opening round 12 called the largest in
twelve rounds is closed, because three maintained successors already carry the
audience. A gap that is "the registry is empty" or "the incumbent is dead" can
close within weeks, so the scan is repeated rather than trusted from the
document.

Every request is sequential with a sleep, because crates.io rate-limits parallel
requests and a 429 reads as an empty result - the failure mode `README.md` warns
about. Any "nothing found" below must be re-checked before it is believed.

Usage:
    python3 round13.py            # all sections
    python3 round13.py yaml       # just the round-12 headline re-check
    python3 round13.py candidates # the new-area sweep
"""

from __future__ import annotations

import json
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

UA = {"User-Agent": "crateworks-round13"}


def get(url: str, timeout: int = 25):
    for attempt in range(4):
        try:
            request = urllib.request.Request(url, headers=UA)
            with urllib.request.urlopen(request, timeout=timeout) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            if error.code == 404:
                return None
            time.sleep(2 + attempt * 2)
        except Exception:  # noqa: BLE001 - retry, then report
            time.sleep(2 + attempt * 2)
    return "ERR"


def crate(name: str) -> dict | None:
    data = get(f"https://crates.io/api/v1/crates/{urllib.parse.quote(name)}")
    if data in (None, "ERR"):
        return None
    return data["crate"]


def search(terms: list[str], per_page: int = 8) -> list[dict]:
    """Union of several queries, ranked by recent downloads.

    One term gives a false positive (a similarly named crate) or a false
    negative (the real crate uses different words), so every term a maintainer
    might have used is tried.
    """
    seen: dict[str, dict] = {}
    for term in terms:
        data = get(f"https://crates.io/api/v1/crates?q={urllib.parse.quote(term)}&per_page={per_page}")
        if data in (None, "ERR"):
            print(f"    !! rate-limited or error on '{term}'")
            continue
        for entry in data.get("crates", []):
            seen[entry["name"]] = entry
        time.sleep(0.7)
    return sorted(seen.values(), key=lambda c: c.get("recent_downloads") or 0, reverse=True)


def npm_weekly(name: str) -> int | None:
    data = get(f"https://api.npmjs.org/downloads/point/last-week/{urllib.parse.quote(name)}")
    return None if data in (None, "ERR") else data.get("downloads")


def pypi_monthly(name: str) -> int | None:
    data = get(f"https://pypistats.org/api/packages/{urllib.parse.quote(name)}/recent")
    return None if data in (None, "ERR") else data.get("data", {}).get("last_month")


def show(crate_names: list[str]) -> None:
    for name in crate_names:
        entry = crate(name)
        if entry is None:
            print(f"   {name:22} (404 - free)")
        else:
            print(
                f"   {entry['name']:22} recent={entry.get('recent_downloads') or 0:>10} "
                f"v{entry.get('max_stable_version') or entry.get('newest_version')} "
                f"upd={(entry.get('updated_at') or '')[:10]}"
            )
        time.sleep(0.6)


# The round-12 headline, plus the successors that closed it.
YAML = [
    "serde_yaml", "yaml-rust", "yaml-rust2", "serde_yaml_ng",
    "serde-saphyr", "saphyr", "unsafe-libyaml",
]

# The round-12 survivors, which this round re-checked and did not displace.
SURVIVORS = ["fuzzy-matcher", "smartstring", "im", "multipart", "rustls-pemfile"]

# Four areas no earlier round swept. Each is (label, crates.io search terms).
NEW_AREAS = {
    "OFX / QIF bank statements": ["ofx", "qif", "ofx-rs", "ofxy"],
    "Subtitles SRT/VTT/ASS": ["subtitle", "srt", "subtp", "subparse", "subtitler"],
    "MIDI": ["midi", "midly", "midi-file", "smf"],
    "Bioinformatics FASTA/FASTQ": ["fasta", "fastq", "needletail", "seq_io", "rust-bio"],
    "VCF / variants": ["vcf", "noodles", "bcf"],
    "HLS / m3u8": ["m3u8", "hls", "hls_m3u8", "quick-m3u8"],
    "SMILES / chemistry": ["smiles", "chemcore", "molecule", "purr"],
    "GEDCOM": ["gedcom", "ged_io", "geneweb"],
}

# Source-ecosystem demand for the shortlist, so the tables can be rebuilt.
DEMAND = [
    ("PyPI", "srt", 969672), ("PyPI", "pysubs2", 193478),
    ("PyPI", "ofxparse", 412568), ("PyPI", "mido", 1642568),
    ("PyPI", "pysam", 1218720), ("PyPI", "biopython", 4377509),
    ("npm", "m3u8-parser", 605057), ("npm", "hls.js", 6015378),
]


def section_yaml() -> None:
    print("### The round-12 headline: is YAML still open?")
    show(YAML)
    print("\n### The round-12 survivors")
    show(SURVIVORS)
    print()


def section_candidates() -> None:
    print("### Areas no earlier round swept")
    for label, terms in NEW_AREAS.items():
        print(f"\n-- {label}")
        for entry in search(terms)[:4]:
            print(
                f"   {entry['name']:22} recent={entry.get('recent_downloads') or 0:>10} "
                f"v{entry.get('max_stable_version') or entry.get('newest_version')} "
                f"upd={(entry.get('updated_at') or '')[:10]} :: "
                f"{(entry.get('description') or '')[:56]}"
            )
        time.sleep(0.5)
    print()


def section_demand() -> None:
    print("### Source-ecosystem demand, refetched live")
    for ecosystem, name, recorded in DEMAND:
        live = npm_weekly(name) if ecosystem == "npm" else pypi_monthly(name)
        flag = "" if live is None else ("  (unchanged)" if abs(live - recorded) < max(recorded * 0.1, 1) else "  ** moved **")
        unit = "/wk" if ecosystem == "npm" else "/mo"
        print(f"   {ecosystem:4} {name:14} {str(live):>10}{unit}  recorded {recorded}{flag}")
        time.sleep(0.6)
    print()


def main() -> None:
    which = sys.argv[1] if len(sys.argv) > 1 else "all"
    if which in ("all", "yaml"):
        section_yaml()
    if which in ("all", "candidates"):
        section_candidates()
    if which in ("all", "demand"):
        section_demand()


if __name__ == "__main__":
    main()
