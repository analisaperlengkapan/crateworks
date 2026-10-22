"""Round 12: the abandoned incumbent, measured systematically.

Round 11 found `fuzzy-matcher` (9.4M recent downloads, last release 2020,
repository archived) by hand and called it the strongest opening it had seen.
This script asks the same question for every crate in the top of the registry,
so the answer stops depending on someone noticing one example.

The signal is a crate with real recent downloads whose newest *live* release is
years old. That is a different opening from an empty registry: the users, the
name and the reverse dependencies already exist, and nobody is maintaining them.

Two data sources, both public and keyless:

* `crates.io/api/v1/crates?sort=recent-downloads` for the ranked list, paged
  with the `seek` cursor the API returns. `recent_downloads` is a rolling
  ninety-day figure, which is what makes "still in use" separable from
  "downloaded once in 2019".
* `index.crates.io` (the sparse index) for the publish date of the newest
  non-yanked version. The API's `updated_at` is the last time *anything* about
  the crate changed, including a yank or an owner edit, so it is not a release
  date; the index is.

Usage:

    python3 stale.py                 # top 3000, at least 730 days old
    python3 stale.py --min-downloads 1000000 --min-age 1095
    python3 stale.py --json out.json

The GitHub check is separate and optional: `--github` reads each candidate's
repository URL and asks `api.github.com` whether the repository is archived.
Without `GITHUB_TOKEN` that is 60 requests an hour, so it is off by default.
"""

import argparse
import concurrent.futures
import datetime
import json
import os
import pathlib
import sys
import time
import urllib.error
import urllib.request

UA = {"User-Agent": "gap-research (rust gap analysis)"}
CACHE = pathlib.Path(__file__).with_name(".cache")
CACHE.mkdir(exist_ok=True)


def get_json(url, headers=None, timeout=30, retries=4):
    head = dict(UA)
    if headers:
        head.update(headers)
    last = None
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, headers=head)
            return json.load(urllib.request.urlopen(req, timeout=timeout))
        except urllib.error.HTTPError as error:
            if error.code == 404:
                return None
            last = error
        except Exception as error:  # noqa: BLE001 - a network error is retried
            last = error
        time.sleep(1.5 * (attempt + 1))
    print(f"  !! {url} -> {last}", file=sys.stderr)
    return None


def top_crates(min_recent, limit):
    """The ranked crates.io list, down to `min_recent` recent downloads."""
    out = []
    url = "https://crates.io/api/v1/crates?per_page=100&sort=recent-downloads"
    while url and len(out) < limit:
        page = get_json(url)
        if not page:
            break
        for crate in page.get("crates", []):
            if (crate.get("recent_downloads") or 0) < min_recent:
                return out
            out.append(crate)
        nxt = (page.get("meta") or {}).get("next_page")
        url = "https://crates.io/api/v1/crates" + nxt if nxt else None
        time.sleep(0.3)
    return out


def index_path(name):
    """The sparse-index path for a crate name, by the index's own rule."""
    p = name.lower()
    if len(p) == 1:
        return f"1/{p}"
    if len(p) == 2:
        return f"2/{p}"
    if len(p) == 3:
        return f"3/{p[0]}/{p}"
    return f"{p[:2]}/{p[2:4]}/{p}"


def last_release(name):
    """`(publish time, version, live versions)` of the newest live version.

    A yank removes a version from what a new build can resolve, so a crate whose
    only recent release was yanked is treated as older than it looks.
    """
    cached = CACHE / f"{name}.json"
    if cached.exists():
        return json.loads(cached.read_text())
    body = None
    try:
        req = urllib.request.Request(
            "https://index.crates.io/" + index_path(name), headers=UA
        )
        body = urllib.request.urlopen(req, timeout=25).read().decode()
    except Exception:  # noqa: BLE001 - a missing crate is a result, not a crash
        body = None
    if body is None:
        return None
    rows = [json.loads(line) for line in body.strip().split("\n") if line.strip()]
    live = [row for row in rows if not row.get("yanked")]
    newest = (live or rows)[-1]
    result = {
        "published": newest.get("pubtime"),
        "version": newest.get("vers"),
        "versions": len(rows),
        "live_versions": len(live),
    }
    cached.write_text(json.dumps(result))
    return result


def age_days(published):
    if not published:
        return None
    when = datetime.datetime.fromisoformat(published.replace("Z", "+00:00"))
    now = datetime.datetime.now(datetime.timezone.utc)
    return (now - when).days


def repository_of(crate):
    return (crate.get("repository") or "").rstrip("/")


def archived(url):
    """Whether a GitHub repository is archived, or `None` when unknown."""
    if "github.com/" not in url:
        return None
    slug = url.split("github.com/", 1)[1]
    parts = [p for p in slug.split("/") if p]
    if len(parts) < 2:
        return None
    owner, repo = parts[0], parts[1].removesuffix(".git")
    token = os.environ.get("GITHUB_TOKEN")
    headers = {"Accept": "application/vnd.github+json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    data = get_json(f"https://api.github.com/repos/{owner}/{repo}", headers)
    if data is None:
        return None
    return {
        "archived": bool(data.get("archived")),
        "stars": data.get("stargazers_count"),
        "pushed_at": data.get("pushed_at"),
        "open_issues": data.get("open_issues_count"),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--min-downloads", type=int, default=200_000,
                        help="recent (90-day) downloads a crate must have")
    parser.add_argument("--min-age", type=int, default=730,
                        help="days since the newest live release")
    parser.add_argument("--limit", type=int, default=3000,
                        help="how far down the ranked list to look")
    parser.add_argument("--github", action="store_true",
                        help="ask api.github.com whether each repo is archived")
    parser.add_argument("--json", help="write the full result set here")
    args = parser.parse_args()

    # Walk the ranked list once, down to a tenth of the download floor, so the
    # age filter has candidates to reject rather than a list already cut short.
    crates = top_crates(args.min_downloads // 10, args.limit)
    print(f"ranked list: {len(crates)} crates at or above "
          f"{args.min_downloads // 10} recent downloads", flush=True)

    with concurrent.futures.ThreadPoolExecutor(8) as pool:
        releases = dict(zip([c["name"] for c in crates],
                            pool.map(last_release, [c["name"] for c in crates])))

    rows = []
    for crate in crates:
        release = releases.get(crate["name"])
        if not release:
            continue
        age = age_days(release["published"])
        if age is None or age < args.min_age:
            continue
        if (crate.get("recent_downloads") or 0) < args.min_downloads:
            continue
        rows.append({
            "name": crate["name"],
            "recent_downloads": crate["recent_downloads"],
            "downloads": crate.get("downloads"),
            "last_release": release["published"],
            "version": release["version"],
            "age_days": age,
            "versions": release["versions"],
            "live_versions": release["live_versions"],
            "description": (crate.get("description") or "").strip(),
            "repository": repository_of(crate),
            "homepage": crate.get("homepage"),
        })

    rows.sort(key=lambda r: r["recent_downloads"], reverse=True)

    if args.github:
        with concurrent.futures.ThreadPoolExecutor(4) as pool:
            statuses = list(pool.map(archived, [r["repository"] for r in rows]))
        for row, status in zip(rows, statuses):
            row["github"] = status
        time.sleep(0.2)

    print(f"\n{len(rows)} crates with >= {args.min_downloads} recent downloads "
          f"and no live release in {args.min_age} days\n")
    header = f"{'crate':32} {'recent':>12} {'last release':12} {'days':>6} {'vers':>5}  repo"
    print(header)
    print("-" * len(header))
    for row in rows:
        flag = ""
        gh = row.get("github")
        if gh and gh.get("archived"):
            flag = "  ARCHIVED"
        print(f"{row['name'][:32]:32} {row['recent_downloads']:12,} "
              f"{row['last_release'][:10]:12} {row['age_days']:6} "
              f"{row['live_versions']:5}  {row['repository'][:44]}{flag}")

    if args.json:
        pathlib.Path(args.json).write_text(json.dumps(rows, indent=2) + "\n")
        print(f"\nwrote {args.json}")


if __name__ == "__main__":
    main()
