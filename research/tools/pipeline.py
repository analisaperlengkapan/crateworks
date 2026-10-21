import json, os, sys, time, urllib.request, urllib.parse

sys.path.insert(0, '.')
from crates import api, search

# ---------- stage 1: PyPI metadata for top N ----------
def pypi_meta(names, path='pypi-meta.json'):
    out = json.load(open(path)) if os.path.exists(path) else {}
    todo = [n for n in names if n not in out]
    for i, n in enumerate(todo):
        try:
            req = urllib.request.Request(f"https://pypi.org/pypi/{urllib.parse.quote(n)}/json",
                                         headers={'User-Agent': 'gap-research'})
            d = json.load(urllib.request.urlopen(req, timeout=20))
            info = d.get('info', {})
            out[n] = {
                'summary': (info.get('summary') or '')[:200],
                'requires_dist': len(info.get('requires_dist') or []),
                'home': info.get('home_page') or info.get('project_url'),
                'classifiers': [c for c in (info.get('classifiers') or []) if 'Topic' in c or 'Programming Language' in c][:6],
            }
        except Exception as e:
            out[n] = {'summary': '', 'error': str(e)[:60]}
        if i % 100 == 0:
            json.dump(out, open(path, 'w'))
            print(f"  pypi-meta {i}/{len(todo)}", flush=True)
        time.sleep(0.05)
    json.dump(out, open(path, 'w'))
    return out

# ---------- stage 2: crates.io lookup ----------
def crate_lookup(names, path='crates-lookup.json'):
    out = json.load(open(path)) if os.path.exists(path) else {}
    todo = [n for n in names if n not in out]
    for i, n in enumerate(todo):
        res = search(n, 8)
        rec = {'results': []}
        for c in res:
            rec['results'].append({
                'name': c['name'],
                'dl': c.get('downloads', 0),
                'recent': c.get('recent_downloads') or 0,
                'ver': c.get('max_stable_version') or c.get('newest_version'),
                'upd': (c.get('updated_at') or '')[:10],
                'desc': (c.get('description') or '')[:120],
            })
        rec['exact'] = any(r['name'].lower() == n.lower() for r in rec['results'])
        rec['best_recent'] = max([r['recent'] for r in rec['results']] or [0])
        out[n] = rec
        if i % 25 == 0:
            json.dump(out, open(path, 'w'))
            print(f"  crate-lookup {i}/{len(todo)}", flush=True)
        time.sleep(0.35)
    json.dump(out, open(path, 'w'))
    return out

if __name__ == '__main__':
    stage = sys.argv[1]
    if stage == 'pypi':
        d = json.load(open('top-pypi.json'))['rows']
        names = [r['project'] for r in d[:600]]
        json.dump(names, open('pypi-candidates.json', 'w'))
        pypi_meta(names)
    elif stage == 'crates-npm':
        dl = json.load(open('npm-downloads.json'))
        names = [n for n, _ in sorted(dl.items(), key=lambda kv: -kv[1])][:450]
        json.dump(names, open('crates-npm-names.json', 'w'))
        crate_lookup(names)
    elif stage == 'crates-pypi':
        names = json.load(open('pypi-candidates.json'))
        crate_lookup(names)
