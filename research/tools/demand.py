import json, urllib.request, urllib.parse, time, sys
sys.path.insert(0,'.')
from npmfetch import bulk, get_json

NPM = ["xstate","json-rules-engine","bpmn-js","bpmn-moddle","@temporalio/client","exceljs","docx",
 "pptxgenjs","pdfkit","pdf-lib","zugferd","ibantools","libphonenumber-js","currency.js",
 "node-cron","cron-parser","bull","bullmq","hunspell-spellchecker","nspell",
 "intl-messageformat","@formatjs/intl-messageformat","adhan","moment-hijri","googleapis",
 "google-auth-library","playwright","cheerio","puppeteer","ua-parser-js","useragent","geoip-lite",
 "jsbarcode","qrcode","svg-captcha","nodemailer","mailparser","ical.js","rrule","vcard-parser",
 "sql-formatter","deep-object-diff","jsondiffpatch","fast-json-patch","slugify","pluralize",
 "transliteration","humanize-duration","date-fns","luxon","bytes","pretty-bytes","filesize",
 "chokidar","inquirer","ora","boxen","cli-table3","ical","@apidevtools/swagger-parser",
 "swagger-ui-express","js-yaml","yaml","xml2js","fast-xml-parser","html-to-pdfmake","pdfmake",
 "bcryptjs","argon2","jsonwebtoken","jose","passport","openid-client","node-forge",
 "zxcvbn","otplib","speakeasy","csv-parse","papaparse","excel4node","write-excel-file",
 "sheetjs","xlsx","node-xlsx","mammoth","textract","tesseract.js","sharp","jimp",
 "fluent-ffmpeg","music-metadata","node-taglib-sharp","id3-writer","@breejs/later","later",
 "agenda","node-schedule","tarn","ssh2","socks","http-proxy-agent","undici","got","node-fetch"]

PYPI = ["holidays","workalendar","pandas","openpyxl","xlsxwriter","python-docx","python-pptx",
 "reportlab","weasyprint","fpdf2","pdfplumber","camelot-py","pyhanko","endesive","factur-x",
 "schwifty","phonenumbers","forex-python","CurrencyConverter","croniter","APScheduler","celery",
 "rq","dramatiq","pyspellchecker","hunspell","babel","quran","adhan","hijri-converter",
 "hijridate","zakat","google-api-python-client","google-auth","scrapy","beautifulsoup4",
 "yt-dlp","pytube","streamlit","gradio","tiktoken","openai","anthropic","langchain","chromadb",
 "faiss-cpu","opencv-python","nltk","spacy","faker","pytz","PyYAML","lxml","html5lib",
 "chardet","python-dateutil","arrow","jsonschema","cryptography","paramiko","dnspython",
 "netaddr","geoip2","python-barcode","qrcode","captcha","pywebpush","pyfcm","apns2",
 "icalendar","recurring-ical-events","vobject","xmltodict","sqlparse","presidio-analyzer",
 "deepdiff","jsonpatch","python-slugify","inflection","Unidecode","humanize","watchdog",
 "psutil","tabulate","prompt_toolkit","rich","tqdm","click","colorama","defusedxml",
 "pikepdf","pypdf","python-magic","langdetect","validators","email-validator","twine",
 "wheel","setuptools","pip","requests","urllib3","boto3","pydantic","sqlalchemy","alembic",
 "redis","psycopg2-binary","pymongo","elasticsearch","kafka-python","paho-mqtt","stripe",
 "twilio","slack-sdk","discord.py","python-telegram-bot","praw","tweepy","plotly","matplotlib",
 "seaborn","sympy","networkx","gensim","sentence-transformers","transformers","torch",
 "scikit-learn","scipy","numpy","pillow","Pillow","moviepy","easyocr","pytesseract",
 "textblob","wordcloud","openpyxl","pyexcel","tabula-py","pdf2image","img2pdf","docx2pdf",
 "python-pptx","pptx","aspose-words","aspose-cells","aspose-pdf","aspose-slides"]

def npm_dl(names):
    out = {}
    for i in range(0, len(names), 25):
        ch = names[i:i+25]
        d = bulk(ch)
        for k, v in d.items():
            if isinstance(v, dict): out[k] = v.get('downloads')
        time.sleep(0.2)
    return out

def pypi_dl(names):
    out = {}
    for n in names:
        try:
            req = urllib.request.Request(f"https://pypistats.org/api/packages/{urllib.parse.quote(n)}/recent",
                                         headers={'User-Agent':'gap-research'})
            d = json.load(urllib.request.urlopen(req, timeout=20))
            out[n] = d.get('data', {})
        except Exception as e:
            out[n] = {'error': str(e)[:40]}
        time.sleep(0.15)
    return out

if __name__ == '__main__':
    nd = npm_dl(NPM)
    json.dump(nd, open('demand-npm.json','w'))
    print("npm done", len(nd))
    pd_ = pypi_dl(PYPI)
    json.dump(pd_, open('demand-pypi.json','w'))
    print("pypi done", len(pd_))
