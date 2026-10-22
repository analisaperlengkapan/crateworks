import json, time, urllib.request, urllib.parse, urllib.error
UA={'User-Agent':'gap-research'}
def get(url):
    for a in range(3):
        try:
            return json.load(urllib.request.urlopen(urllib.request.Request(url,headers=UA),timeout=25))
        except urllib.error.HTTPError as e:
            if e.code==404: return None
            time.sleep(3)
        except Exception: time.sleep(2)
    return None

def revdeps(name):
    d=get(f"https://crates.io/api/v1/crates/{urllib.parse.quote(name)}/reverse_dependencies?per_page=1")
    if not d: return None
    return d.get('meta',{}).get('total')

def gh_slug(repo):
    """The `owner/name` part of a GitHub URL, or None if it is not GitHub.

    Parses the URL and compares the host exactly. Testing for the substring
    'github.com' would also accept 'github.com.evil.example' or a query string
    that merely mentions github.com.
    """
    parts = urllib.parse.urlsplit(repo)
    if parts.hostname != 'github.com':
        return None
    return parts.path.strip('/').removesuffix('.git') or None

def gh_activity(repo):
    if not repo: return None
    r=repo.rstrip('/').replace('.git','')
    for host in ["https://api.github.com/repos/"]:
        try:
            slug=gh_slug(r)
            if not slug: return None
            d=get(host+slug)
            if not d: return None
            return {'stars':d.get('stargazers_count'),'pushed':(d.get('pushed_at') or '')[:10],
                    'archived':d.get('archived'),'open_issues':d.get('open_issues_count')}
        except Exception: return None
    return None

TARGETS = ["holidays","public_holidays","holiday","py-holidays-rs",
 "wsdl","soap","rsoap","soap-server",
 "stdnum","idsmith","vat_id_validator","ssn","br_tax_id",
 "formula","xlformula_engine","formula-lang","xlstream",
 "en16931-formats","zugferd","puml","plantuml-little","musicxml",
 "subparse","srt","ass_parser","srt_parser",
 "libdav","caldav","carddav","sepa","mt940","camt053","iso20022"]
def main():
    out={}
    for n in TARGETS:
        c=get(f"https://crates.io/api/v1/crates/{urllib.parse.quote(n)}")
        if not c or not c.get('crate'):
            out[n]={'exists':False}; print(f"  {n:<18} (none)"); time.sleep(0.3); continue
        cr=c['crate']
        rd=revdeps(n); time.sleep(0.35)
        repo=cr.get('repository')
        gh=gh_activity(repo); time.sleep(0.6)
        out[n]={'exists':True,'recent':cr.get('recent_downloads') or 0,'upd':(cr.get('updated_at') or '')[:10],
                'revdeps':rd,'repo':repo,'gh':gh}
        g = f" stars={gh['stars']} pushed={gh['pushed']} archived={gh['archived']} issues={gh['open_issues']}" if gh else " (repo not found)"
        print(f"  {n:<18} recent={out[n]['recent']:>9,} pub={out[n]['upd']} revdeps={rd}{g}")
        time.sleep(0.3)
    json.dump(out,open('revdeps.json','w'),indent=1)


if __name__ == "__main__":
    main()
