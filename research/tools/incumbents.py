import json, time, urllib.request, urllib.parse, urllib.error
UA={'User-Agent':'gap-research'}
def crate(n):
    for a in range(3):
        try:
            d=json.load(urllib.request.urlopen(urllib.request.Request(
                "https://crates.io/api/v1/crates/"+urllib.parse.quote(n),headers=UA),timeout=25))
            c=d.get('crate')
            if not c: return None
            return {'name':c['name'],'recent':c.get('recent_downloads') or 0,
                    'dl':c.get('downloads',0),'ver':c.get('max_stable_version') or c.get('newest_version'),
                    'upd':(c.get('updated_at') or '')[:10],'repo':c.get('repository'),
                    'desc':(c.get('description') or '')[:90]}
        except urllib.error.HTTPError as e:
            if e.code==404: return None
            time.sleep(3)
        except Exception: time.sleep(2)
    return None
AREAS={
 "1 holidays":["holidays","public-holidays","holiday","py-holidays-rs","holidays_jp"],
 "2 soap":["wsdl","soap","rsoap","soap-server","xml-rpc"],
 "3 stdnum":["stdnum","idsmith","vat_id_validator","ssn","br_tax_id"],
 "4 formula":["formula","xlformula_engine","formula-lang","xlstream"],
 "5 en16931":["en16931-formats","zugferd","en16931"],
 "6 plantuml":["puml","plantuml-little"],
 "7 musicxml":["musicxml"],
 "8 subtitles":["subparse","srt","ass-parser","srt-parser"],
 "9 caldav":["libdav","caldav","carddav"],
 "10 iso20022":["sepa","mt940","camt053","iso20022"],
}
out={}
for a,names in AREAS.items():
    out[a]={}
    print(f"### {a}")
    for n in names:
        r=crate(n); out[a][n]=r
        if r: print(f"    {r['name']:<18} rec={r['recent']:>9,} dl={r['dl']:>10,} v{str(r['ver'])[:8]:<8} {r['upd']}  repo={r['repo']}")
        else:  print(f"    {n:<18} (none)")
        time.sleep(0.3)
json.dump(out,open('incumbents.json','w'),indent=1)
