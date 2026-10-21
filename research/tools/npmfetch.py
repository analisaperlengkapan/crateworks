import json, urllib.request, urllib.parse, time, os, sys

def get_json(url, headers=None, timeout=45):
    h = {'User-Agent': 'gap-research'}
    if headers: h.update(headers)
    req = urllib.request.Request(url, headers=h)
    return json.load(urllib.request.urlopen(req, timeout=timeout))

def enc(name):
    return '%40' + name[1:] if name.startswith('@') else name

def bulk(chunk, retries=3):
    url = "https://api.npmjs.org/downloads/point/last-week/" + ",".join(enc(c) for c in chunk)
    for a in range(retries):
        try:
            return get_json(url)
        except Exception as e:
            if a == retries - 1:
                print(f"  bulk fail ({len(chunk)}): {e}", file=sys.stderr); return {}
            time.sleep(2 * (a + 1))

def fetch_downloads(names, batch=32):
    out = {}
    unscoped = [n for n in names if not n.startswith('@')]
    scoped = [n for n in names if n.startswith('@')]
    for group, label in ((unscoped, 'unscoped'), (scoped, 'scoped')):
        for i in range(0, len(group), batch):
            ch = group[i:i+batch]
            d = bulk(ch)
            for k, v in d.items():
                if isinstance(v, dict) and 'downloads' in v:
                    out[k] = v['downloads']
            print(f"  {label} {i+len(ch)}/{len(group)}", flush=True)
            time.sleep(0.25)
    return out

def fetch_meta(names):
    out = {}
    path = 'npm-meta.json'
    if os.path.exists(path):
        out = json.load(open(path))
    todo = [n for n in names if n not in out]
    for i, n in enumerate(todo):
        try:
            d = get_json("https://registry.npmjs.org/" + urllib.parse.quote(n, safe='@/'),
                         headers={'Accept': 'application/vnd.npm.install-v1+json'}, timeout=25)
            latest = d.get('dist-tags', {}).get('latest')
            v = (d.get('versions') or {}).get(latest, {})
            out[n] = {
                'description': (d.get('description') or '')[:200],
                'latest': latest,
                'deprecated': bool(v.get('deprecated')),
                'license': v.get('license') if isinstance(v.get('license'), str) else None,
                'repo': ((v.get('repository') or {}).get('url') if isinstance(v.get('repository'), dict) else v.get('repository')),
                'homepage': v.get('homepage'),
                'deps': len(v.get('dependencies') or {}),
            }
        except Exception as e:
            out[n] = {'description': '', 'error': str(e)[:80]}
        if i % 50 == 0:
            json.dump(out, open(path, 'w'))
            print(f"  meta {i}/{len(todo)}", flush=True)
        time.sleep(0.12)
    json.dump(out, open(path, 'w'))
    return out

if __name__ == '__main__':
    names = json.load(open('npm-candidates.json'))
    if not os.path.exists('npm-downloads.json'):
        dl = fetch_downloads(names)
        json.dump(dl, open('npm-downloads.json', 'w'))
    else:
        dl = json.load(open('npm-downloads.json'))
    print("downloads:", len(dl))
    meta = fetch_meta(names)
    print("meta:", len(meta))
