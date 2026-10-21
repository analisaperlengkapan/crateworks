import json, urllib.request, urllib.parse, time, sys

UA = {'User-Agent': 'gap-research (rust gap analysis)'}
_cache = {}

def api(path, **params):
    url = "https://crates.io/api/v1/" + path
    if params:
        url += "?" + urllib.parse.urlencode(params)
    if url in _cache:
        return _cache[url]
    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers=UA)
            data = json.load(urllib.request.urlopen(req, timeout=30))
            _cache[url] = data
            return data
        except Exception as e:
            if attempt == 3:
                print(f"  !! {url} -> {e}", file=sys.stderr)
                return None
            time.sleep(2 + attempt * 2)

def search(q, per_page=5):
    d = api("crates", q=q, per_page=per_page)
    if not d:
        return []
    return d.get("crates", [])

def best_match(name_or_query, per_page=6):
    """Return the top crate by downloads for a query."""
    cr = search(name_or_query, per_page)
    if not cr:
        return None
    cr.sort(key=lambda c: c.get("downloads", 0), reverse=True)
    return cr[0]

def probe(names, query=None):
    """Check whether a crate named `names` exists (exact), else fall back to search."""
    out = {}
    for n in names:
        d = api(f"crates/{urllib.parse.quote(n)}")
        if d and d.get("crate"):
            c = d["crate"]
            out[n] = {"exact": True, "downloads": c.get("downloads", 0),
                      "recent": c.get("recent_downloads", 0),
                      "max_version": c.get("max_stable_version") or c.get("newest_version"),
                      "updated": (c.get("updated_at") or "")[:10],
                      "desc": (c.get("description") or "")[:110],
                      "repo": c.get("repository")}
        else:
            out[n] = {"exact": False}
        time.sleep(0.3)
    return out

if __name__ == "__main__":
    for a in sys.argv[1:]:
        m = best_match(a, 5)
        print(f"--- query '{a}'")
        for c in search(a, 5):
            print(f"   {c['name']:<28} dl={c.get('downloads',0):>10} recent={c.get('recent_downloads') or 0:>9} "
                  f"v{c.get('max_stable_version') or c.get('newest_version')} upd={c.get('updated_at','')[:10]} :: {(c.get('description') or '')[:80]}")
