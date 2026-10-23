"""Round 14: re-measure the survivors, and sweep the Islamic/Indonesian domain.

Two jobs.

First, re-measurement. Rounds 11-13 each closed a headline before it was acted
on (MRZ, then YAML), so the standing lesson is that a gap recorded in a document
is not a gap today. Every candidate that earlier rounds left open is measured
again here, with the release date read from the sparse index rather than the
API's `updated_at`, because a yank or an owner edit is not a release.

Second, a domain no earlier round swept. Rounds 1-13 searched the npm and PyPI
popularity lists, which are dominated by Western web and data-science packages.
They never asked what a Muslim-majority market needs, and the demand for that is
visible in the same two registries: `adhan`, `hijri`, `moment-hijri` and the
Arabic text tools all have real volume, and the Rust side is close to empty.

Usage:

    python3 round14.py candidates     # re-measure the open survivors
    python3 round14.py islamic        # sweep the Islamic/Indonesian domain
    python3 round14.py names          # is a proposed name still free?
"""

import json
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

UA = {"User-Agent": "gap-research (rust gap analysis)"}
SLEEP = 0.7  # crates.io rate-limits; a 429 comes back as an empty result


def get(url, headers=None):
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers=headers or UA)
            return json.load(urllib.request.urlopen(req, timeout=30))
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return None
            if e.code == 429:
                time.sleep(5 + attempt * 5)
                continue
            return None
        except Exception:
            time.sleep(2 + attempt * 2)
    return None


def get_text(url, headers=None):
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers=headers or UA)
            return urllib.request.urlopen(req, timeout=30).read().decode("utf-8")
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return None
            if e.code == 429:
                time.sleep(5 + attempt * 5)
                continue
            return None
        except Exception:
            time.sleep(2 + attempt * 2)
    return None


def crate(name):
    """Exact crate lookup. Returns None when the name is free."""
    d = get("https://crates.io/api/v1/crates/" + urllib.parse.quote(name))
    if not d or not d.get("crate"):
        return None
    c = d["crate"]
    return {
        "name": c["name"],
        "recent": c.get("recent_downloads") or 0,
        "all_time": c.get("downloads") or 0,
        "version": c.get("max_stable_version") or c.get("newest_version"),
        "api_updated": (c.get("updated_at") or "")[:10],
        "desc": (c.get("description") or "")[:100],
        "repo": c.get("repository"),
    }


def release_date(name):
    """Publish date of the newest non-yanked version, from the sparse index.

    `updated_at` in the API moves on a yank or an owner edit, so it is not a
    release date. The index is.
    """
    q = urllib.parse.quote(name.lower())
    if len(name) == 1:
        path = f"1/{q}"
    elif len(name) == 2:
        path = f"2/{q}"
    elif len(name) == 3:
        path = f"3/{q[0]}/{q}"
    else:
        path = f"{q[:2]}/{q[2:4]}/{q}"
    raw = get_text(f"https://index.crates.io/{path}")
    if raw is None:
        return None, None
    latest = None
    for line in raw.splitlines():
        try:
            v = json.loads(line)
        except Exception:
            continue
        if v.get("yanked"):
            continue
        latest = v
    return (latest or {}).get("vers"), ((latest or {}).get("pubtime") or "")[:10]


def reverse_deps(name):
    d = get(f"https://crates.io/api/v1/crates/{urllib.parse.quote(name)}/reverse_dependencies")
    if not d:
        return None
    return d.get("meta", {}).get("total")


def github(repo):
    if not repo or "github.com" not in repo:
        return None
    slug = repo.rstrip("/").split("github.com/")[-1]
    d = get(f"https://api.github.com/repos/{slug}")
    if not d:
        return None
    return {"stars": d.get("stargazers_count"), "archived": d.get("archived"),
            "open_issues": d.get("open_issues_count"),
            "pushed": (d.get("pushed_at") or "")[:10]}


def npm(names):
    out = {}
    for n in names:
        d = get(f"https://api.npmjs.org/downloads/point/last-week/{urllib.parse.quote(n)}")
        out[n] = (d or {}).get("downloads")
        time.sleep(0.3)
    return out


def pypi(name):
    d = get(f"https://pypistats.org/api/packages/{urllib.parse.quote(name)}/recent")
    if not d:
        return None
    return d.get("data", {}).get("last_month")


def pypi_many(names):
    return {n: pypi(n) for n in names}


def fmt(n):
    if n is None:
        return "-"
    return f"{n:,}"


# ---------------------------------------------------------------- candidates

# The survivors rounds 12 and 13 left open, plus their neighbours so a successor
# that appeared since would be visible in the same output.
CANDIDATES = {
    "immutable collections": ["im", "im-rc", "rpds", "rpds-py", "ordermap"],
    "fuzzy matching": ["fuzzy-matcher", "nucleo-matcher", "fuzzy-matcher-simsearch",
                       "simsearch", "fst", "aho-corasick"],
    "small string": ["smartstring", "compact_str", "ecow", "smallvec"],
    "multipart": ["multipart", "multer", "multipart-stream", "formdata"],
    "pem": ["rustls-pemfile", "pem", "pem-rfc7468", "rustls-pki-types"],
    "OFX/QIF bank statements": ["ofx-rs", "ofxy", "qif_parser", "ofx", "ofx-parse",
                                "ofxkit", "ofx-qif", "mt940"],
    "subtitles": ["subtp", "subparse", "subtitler", "subkit", "srt", "ass"],
}

# The Islamic/Indonesian domain. Names on the right are the Rust side.
ISLAMIC = {
    "prayer times": ["adhan", "adhan-rs", "praytimes", "salah", "namaz", "salat",
                     "prayer-times", "prayertime"],
    "hijri calendar": ["hijri", "hijridate", "hijri-converter", "islamic-calendar",
                       "umm-al-qura", "hijri-date"],
    "quran": ["quran", "quran-rs", "quran-text", "tanzil", "quran-metadata"],
    "arabic text": ["arabic", "arabic-text", "tashkeel", "tashkeel-rs",
                    "arabic-reshaper", "arabic_reshaper", "bidi", "unicode-bidi",
                    "arabic-normalization"],
    "islamic finance": ["murabaha", "islamic-finance", "zakah", "zakat",
                        "riba", "takaful", "mudarabah", "musharakah"],
    "halal": ["halal", "halal-check", "halal-food"],
    "indonesian gov": ["npwp", "nik-rs", "ktp", "wilayah", "dapodik", "emis",
                       "bpjs", "kemenag", "pesantren", "tahfidz", "madrasah",
                       "kurikulum-merdeka", "raport"],
}


def show_candidates():
    for area, names in CANDIDATES.items():
        print(f"\n### {area}")
        for n in names:
            c = crate(n)
            if c is None:
                print(f"  {n:24s} FREE")
            else:
                ver, pub = release_date(n)
                rd = reverse_deps(n)
                gh = github(c["repo"])
                star = f"{gh['stars']}*" if gh and gh.get("stars") is not None else "-"
                arch = " ARCHIVED" if gh and gh.get("archived") else ""
                print(f"  {n:24s} recent={fmt(c['recent']):>12s} v{ver or '?':10s} "
                      f"pub={pub or '?':10s} revdeps={rd if rd is not None else '?':>4} "
                      f"stars={star}{arch}")
            time.sleep(SLEEP)


def show_islamic():
    for area, names in ISLAMIC.items():
        print(f"\n### {area}")
        for n in names:
            c = crate(n)
            if c is None:
                print(f"  {n:24s} FREE")
            else:
                ver, pub = release_date(n)
                rd = reverse_deps(n)
                gh = github(c["repo"])
                star = f"{gh['stars']}*" if gh and gh.get("stars") is not None else "-"
                arch = " ARCHIVED" if gh and gh.get("archived") else ""
                print(f"  {n:24s} recent={fmt(c['recent']):>12s} v{ver or '?':10s} "
                      f"pub={pub or '?':10s} revdeps={rd if rd is not None else '?':>4} "
                      f"stars={star}{arch}")
            time.sleep(SLEEP)


def show_names():
    for n in sys.argv[2:]:
        c = crate(n)
        print(f"  {n:24s} {'FREE' if c is None else 'TAKEN recent=' + fmt(c['recent'])}")
        time.sleep(SLEEP)


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "candidates"
    if cmd == "candidates":
        show_candidates()
    elif cmd == "islamic":
        show_islamic()
    elif cmd == "names":
        show_names()
    else:
        raise SystemExit(f"unknown command {cmd!r}")
