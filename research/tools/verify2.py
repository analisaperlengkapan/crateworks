import json, time, sys, urllib.request, urllib.parse, urllib.error
sys.path.insert(0,'.')
from crates import search
UA={'User-Agent':'gap-research'}

def exact(n):
    url="https://crates.io/api/v1/crates/"+urllib.parse.quote(n)
    for a in range(3):
        try:
            d=json.load(urllib.request.urlopen(urllib.request.Request(url,headers=UA),timeout=25))
            c=d.get('crate')
            if not c: return None
            return {'name':c['name'],'recent':c.get('recent_downloads') or 0,'dl':c.get('downloads',0),
                    'ver':c.get('max_stable_version') or c.get('newest_version'),
                    'upd':(c.get('updated_at') or '')[:10],'desc':(c.get('description') or '')[:100],
                    'repo':c.get('repository')}
        except urllib.error.HTTPError as e:
            if e.code==404: return None
            time.sleep(3)
        except Exception: time.sleep(2)
    return None

AREAS = {
 "national holidays":        ["holidays-rs","holiday-calendar","public-holidays","multicalendar","holidays"],
 "pdf table extraction":     ["pdf-table","pdf-tables","pdfplumber-rs","table-extract","camelot-rs"],
 "spreadsheet formula":      ["formula","xlformula","spreadsheet-formula","excel-formula","calceval"],
 "EN16931 e-invoice":        ["en16931","zugferd","factur-x","ubl","peppol","cii"],
 "hunspell pure rust":       ["hunspell-rs","spellbook","zspell","spellbook-core","hunspell-pure"],
 "GTFS transit":             ["gtfs","gtfs-rt","gtfs-realtime","transitland","gtfs-structures"],
 "SAML":                     ["saml","saml2","saml-rs","rust-saml","samael"],
 "SWIFT / ISO20022 / SEPA":  ["swift-mt","iso20022","sepa","pain001","mt940","camtrs"],
 "HL7 FHIR / DICOM":         ["fhir","hl7","dicom","dicom-rs","fhirpath"],
 "EDI EDIFACT / X12":        ["edifact","x12","edi","edifact-parser"],
 "BibTeX / CSL citations":   ["biblatex","bibtex","csl","citation-style","hayagriva"],
 "subtitles SRT/ASS":        ["srt","ass","subtitles","subtitle-parser","subparse"],
 "hyphenation/justify":      ["hyphenation","hyphenate","text-justify","justify"],
 "address parsing":          ["address-parser","libpostal-rs","postal-parser","addrparse"],
 "MRZ / national id":        ["mrz","mrz-parser","emrtd","icao-9303","td3"],
 "DKIM/SPF/DMARC":           ["dkim","spf","dmarc","mail-auth","mailchecker"],
 "IMAP client":              ["imap","imap-client","imap-codec","async-imap"],
 "CalDAV":                   ["caldav","carddav","dav-client","webdav"],
 "OSM PBF / vector tiles":   ["osmpbf","osm-pbf","mvt","vector-tile","tilejson"],
 "HAR / JSON-LD":            ["har","json-ld","jsonld","microdata"],
 "SBOM CycloneDX/SPDX":      ["cyclonedx","spdx","sbom","cargo-sbom"],
 "Mermaid / PlantUML":       ["mermaid","plantuml","mermaid-rs","plantuml-rs"],
 "HCL / Terraform":          ["hcl","hcl-rs","hcl2","terraform-hcl"],
 "ISO 8583 / EMV":           ["iso8583","emv","apdu","emvqrcode","nfc-apdu"],
 "MusicXML / MIDI":          ["musicxml","midi","midly","musicxml-parser"],
 "RTF read":                 ["rtf","rtf-parser","rtf-reader"],
 "PDF/A + redaction":        ["pdfa","pdf-redact","pdf-redaction","pdf-sanitize"],
 "fixed-width parse":        ["fixed-width","fixedwidth","fixed-width-parser"],
 "CUE / Jsonnet / Dhall":    ["cue","jsonnet","dhall","jrsonnet"],
 "Liquid / Django templates":["liquid","django-template","tera","minijinja"],
}
out={}
for area, names in AREAS.items():
    res={}
    for n in names:
        res[n]=exact(n); time.sleep(0.25)
    out[area]=res
    found=[(v['recent'],k,v) for k,v in res.items() if v]
    found.sort(reverse=True)
    print(f"### {area}")
    if not found: print("    (semua nama BEBAS / tidak ada crate)")
    for rec,k,v in found:
        print(f"    {k:<22} rec={rec:>10,} dl={v['dl']:>11,} v{str(v['ver'])[:8]:<8} {v['upd']} :: {v['desc'][:60]}")
    free=[k for k,v in res.items() if not v]
    if free: print(f"    NAMA BEBAS: {', '.join(free)}")
    sys.stdout.flush()
json.dump(out, open('verify2.json','w'), indent=1)
