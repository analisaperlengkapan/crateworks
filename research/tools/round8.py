"""Round 8 gap scan: the PyPI top list against crates.io.

Every call uses a public API without a key. crates.io rate-limits, so the
helpers return a dict carrying `__error__` instead of raising, and a caller that
sees an error must treat the result as unknown rather than as "free".

Run it as a script for the round 8 tables:

    python3 research/tools/round8.py
"""

from __future__ import annotations

import json
import urllib.error
import urllib.parse
import urllib.request

_UA = {"User-Agent": "crateworks-gap-research/1.0"}


def get(url: str, timeout: int = 25):
    """Fetch a JSON document, or a dict carrying `__error__` on failure.

    A 404 is not an error here: on crates.io, npm and PyPI it is how the API
    says the name is free. It is returned as `None` so callers can tell it apart
    from a rate limit, which is a real error and must not read as "free".
    """
    req = urllib.request.Request(url, headers=_UA)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            return json.loads(response.read().decode())
    except urllib.error.HTTPError as exc:
        if exc.code == 404:
            return None
        return {"__error__": f"HTTP {exc.code}"}
    except Exception as exc:  # noqa: BLE001 - reported to the caller, not raised
        return {"__error__": f"{type(exc).__name__}: {exc}"}


def crate(name: str) -> dict:
    """Exact-name lookup on crates.io. `status` is `taken` or `FREE`."""
    data = get(f"https://crates.io/api/v1/crates/{name}")
    if data is None:
        return {"name": name, "status": "FREE"}
    if "__error__" in data:
        return {"name": name, "status": f"ERR {data['__error__'][:40]}"}
    if "crate" not in data:
        return {"name": name, "status": "FREE"}
    found = data["crate"]
    return {
        "name": name,
        "status": "taken",
        "version": found.get("max_version"),
        "recent": found.get("recent_downloads"),
        "updated": (found.get("updated_at") or "")[:10],
        "desc": (found.get("description") or "")[:80],
    }


def crates_search(query: str, per_page: int = 8) -> list[dict]:
    """Search crates.io, sorted by recent downloads."""
    quoted = urllib.parse.quote(query)
    data = get(f"https://crates.io/api/v1/crates?q={quoted}&per_page={per_page}")
    if "__error__" in data:
        return []
    return [
        {
            "name": c["name"],
            "recent": c.get("recent_downloads"),
            "desc": (c.get("description") or "")[:70],
        }
        for c in data.get("crates", [])
    ]


def search_total(query: str) -> int | str:
    """Number of crates matching a query. Zero is the strongest gap signal."""
    quoted = urllib.parse.quote(query)
    data = get(f"https://crates.io/api/v1/crates?q={quoted}&per_page=1")
    if "__error__" in data:
        return data["__error__"][:30]
    return data.get("meta", {}).get("total")


def reverse_dependencies(name: str) -> int | str:
    """Dependents of a crate. The only real measure of Rust adoption."""
    data = get(f"https://crates.io/api/v1/crates/{name}/reverse_dependencies?per_page=1")
    if "__error__" in data:
        return data["__error__"][:30]
    return data.get("meta", {}).get("total")


def npm(name: str) -> dict:
    """Exact-name lookup on the npm registry."""
    data = get(f"https://registry.npmjs.org/{urllib.parse.quote(name, safe='@')}")
    if data is None or "error" in data:
        return {"pkg": name, "status": "FREE"}
    if "__error__" in data:
        return {"pkg": name, "status": f"ERR {data['__error__'][:40]}"}
    return {"pkg": name, "status": "taken", "version": data.get("dist-tags", {}).get("latest")}


def npm_downloads(name: str, period: str = "last-week") -> int | None:
    """npm downloads over a period."""
    quoted = urllib.parse.quote(name, safe="@")
    data = get(f"https://api.npmjs.org/downloads/point/{period}/{quoted}")
    if "__error__" in data:
        return None
    return data.get("downloads")


def pypi(name: str) -> dict:
    """Exact-name lookup on PyPI."""
    data = get(f"https://pypi.org/pypi/{name}/json")
    if data is None:
        return {"pkg": name, "status": "FREE"}
    if "__error__" in data:
        return {"pkg": name, "status": f"ERR {data['__error__'][:40]}"}
    if "info" not in data:
        return {"pkg": name, "status": "FREE"}
    return {
        "pkg": name,
        "status": "taken",
        "version": data["info"].get("version"),
        "summary": (data["info"].get("summary") or "")[:70],
    }


def pypi_downloads(name: str) -> int | None:
    """PyPI downloads over the last month."""
    data = get(f"https://pypistats.org/api/packages/{name}/recent")
    if "__error__" in data:
        return None
    return data.get("data", {}).get("last_month")


def github_repos(query: str) -> int | str:
    """Repository count for a GitHub search. Unauthenticated: expect 403s."""
    quoted = urllib.parse.quote(query)
    data = get(f"https://api.github.com/search/repositories?q={quoted}&per_page=1")
    if "__error__" in data:
        return data["__error__"][:30]
    return data.get("total_count")


def main() -> None:
    """Print the tables quoted in `research/rust-crate-gaps-round8.md`."""
    print("### PAdES / PDF signing")
    for name in ["pyhanko", "endesive", "pikepdf"]:
        print(f"  PyPI {name:<12} month={pypi_downloads(name)}")
    for name in ["@signpdf/signpdf", "node-signpdf"]:
        print(f"  npm  {name:<18} weekly={npm_downloads(name)}")
    for name in ["pdf_signer", "underskrift", "pdfluent-sign", "pdf_signing"]:
        found = crate(name)
        print(f"  rust {name:<16} recent={found.get('recent')} deps={reverse_dependencies(name)}")

    print("\n### X12 HIPAA EDI")
    print(f"  PyPI pyx12       month={pypi_downloads('pyx12')}")
    for name in ["x12-parser", "edi-parser"]:
        print(f"  npm  {name:<14} weekly={npm_downloads(name)}")
    for name in ["edi", "x12-types", "serde_x12"]:
        found = crate(name)
        print(f"  rust {name:<18} recent={found.get('recent')} deps={reverse_dependencies(name)}")

    print("\n### ODF authoring")
    for name in ["odfpy", "ezodf", "odfdo"]:
        print(f"  PyPI {name:<12} month={pypi_downloads(name)}")
    for name in ["spreadsheet-ods", "lo_odf", "ferrodoc-odt"]:
        found = crate(name)
        print(f"  rust {name:<16} recent={found.get('recent')} deps={reverse_dependencies(name)}")

    print("\n### national e-invoicing regimes (name availability)")
    for name in ["zatca", "fatoora", "myinvois", "ksef", "verifactu", "efaktur", "coretax"]:
        print(f"  {name:<12} {crate(name)['status']}")


if __name__ == "__main__":
    main()
