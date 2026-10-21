# Paket populer di npm/PyPI yang belum punya padanan matang di Rust

Tanggal riset: 2026-10-21. Semua angka diambil langsung dari API registry
(npm, PyPI, crates.io) pada hari yang sama, bukan dari ingatan.

## Metode

1. Sumber permintaan (demand):
   - npm: 1.000 paket "most depended-upon" (gist anvaka, daftar nama) + angka
     download mingguan yang diambil ulang dari `api.npmjs.org` (angka gist itu
     snapshot 2019, jadi hanya namanya yang dipakai).
   - PyPI: 15.000 paket teratas dari `top-pypi-packages-30-days.json`.
   - Angka bulanan PyPI dari `pypistats.org`.
2. Sumber pasokan (supply): crates.io `/api/v1/crates?q=...` dan
   `/api/v1/crates/<nama>`. Metrik yang dipakai adalah `recent_downloads`
   (unduhan 90 hari terakhir), bukan total unduhan, karena total unduhan
   menyembunyikan crate yang sudah mati.
3. Kandidat difilter: buang paket yang memang spesifik platform JS (React,
   Babel, ESLint, DOM, bundler). Sisanya adalah kapabilitas generik yang
   seharusnya bisa hidup di Rust.
4. Setiap kandidat diverifikasi manual dengan nama crate persis, karena
   pencarian crates.io sering memberi false negative.

Aturan penilaian: sebuah celah dianggap nyata kalau crate terbaik di kategori
itu punya `recent_downloads` di bawah ~20.000 (praktis tidak dipakai) atau
belum ada sama sekali, sementara padanan npm/PyPI-nya punya jutaan unduhan.

## Kandidat terkuat

| Kapabilitas | Padanan terkuat | Permintaan | Crate terbaik di Rust | recent | vonis |
|---|---|---|---|---|---|
| Hari libur nasional (kalender libur per negara) | PyPI `holidays` 26,9 jt/bln; `workalendar` 957 rb/bln | besar, stabil | `holidays` 20 rb total (2023), `public-holidays` v0.0.0 (2024) | <200 | **CELAH BESAR** |
| Ekstraksi tabel dari PDF | PyPI `tabula-py` 1,16 jt/bln; `camelot-py` 722 rb/bln | besar | tidak ada; hanya `lopdf`/`pdf-extract` untuk teks mentah | 0 | **CELAH BESAR** |
| Evaluator formula spreadsheet | npm `exceljs` 10,8 jt/mgg; `xlsx` 7,2 jt/mgg | besar | `formula` 20 total (2022), `xlformula_engine` mati (2023) | <100 | **CELAH BESAR** |
| E-invoice Eropa (ZUGFeRD/Factur-X, UBL, PEPPOL) | PyPI `factur-x` 287 rb/bln; npm `zugferd` | sedang, tumbuh | `en16931-formats` | 5,4 rb | **CELAH** |
| Kamus Hunspell murni-Rust | PyPI `hunspell`; npm `nspell` 147 rb/mgg | sedang | `hunspell-sys`/`hunspell-rs` (binding C, 2022), `spellbook` (parsial) | <30 rb | **CELAH** |
| Kekuatan sandi tipe zxcvbn (locale ID) | npm `zxcvbn` 783 rb/mgg | sedang | `zxcvbn` sudah ada | 1,8 jt | sudah terisi |
| Parser iCal + ekspansi RRULE | PyPI `icalendar` + `recurring-ical-events` | sedang | `rrule` (2025), `icalendar` 395 rb | 629 rb | sebagian terisi |
| Inflection/pluralisasi Inggris | npm `pluralize` 23,5 jt/mgg | besar | `inflector` mati (2019), `cruet` fork, `pluralizer` | 1,8 jt | **FORK-DAN-MENANG** |
| Deteksi user-agent | npm `ua-parser-js` 19 jt/mgg | besar | `uaparser` (data 2024) | 2,9 jt | fork-dan-menang |
| Harakat/tashkeel Arab + normalisasi teks Arab | - | niche | `ar-reshaper`, `arabic_text_utils`, `dengjen-tashkeel` | <10 rb | **CELAH NICHE** |
| Kalender Hijriah (Umm al-Qura) + waktu salat | PyPI `hijri-converter` 2,8 jt/bln | sedang | `hijri` (CLI), `salah` 848, `praytimes` (2023) | <1 rb | **CELAH NICHE** |

## Catatan penting: banyak yang tampak celah, ternyata sudah ada

Ini yang membedakan riset berbasis registry dengan tebakan:

- XLSX: `rust_xlsxwriter` (2,4 jt recent), `calamine` (5,7 jt recent),
  `umya-spreadsheet` sudah matang. Bukan celah.
- DOCX: `docx-rs` (790 rb recent) sudah aktif. Bukan celah.
- PDF generik: `pdf-writer`, `printpdf`, `typst` (1,6 jt recent) sudah kuat.
  Bukan celah.
- OCR: `ocrs` (254 rb recent), `tesseract-rs` (42 rb recent) sudah ada.
  Bukan celah.
- Pemrosesan gambar: `image` 64,6 jt recent + `imageproc`. Bukan celah.
- YAML: `serde_yaml` sudah deprecated tetapi `serde-yaml-ng` (11,3 jt recent)
  dan `saphyr` sudah menggantikan. Bukan celah.
- Browser automation: `chromiumoxide` 2,1 jt recent, `headless_chrome`,
  `thirtyfour`, `fantoccini` sudah ada. Bukan celah.
- Antrean job: `apalis` (394 rb recent) sudah ada. Bukan celah.
- State machine: `statig` (231 rb recent), `rust-fsm`, `sm` sudah ada.
  Bukan celah.
- Workflow engine: `dataflow-rs` (6,7 rb recent), `acts`, `aion-rs` sudah ada
  meski belum ada setara Temporal. Setengah celah.
- OAuth/OIDC: `oauth2` 17 jt recent, `openidconnect` 4,9 jt recent. Bukan celah.
- PII redaction: PyPI `presidio-analyzer` 5,2 jt/bln, tetapi di Rust sudah ada
  pemain aktif (`redactable` 16,6 rb recent, `redact-core`, `qubit-redact`).
  Bukan celah besar, meski pasarnya masih terbuka.
- Embeddings/vector DB: `fastembed` 2,3 jt recent, `pgvector` 5,2 jt recent.
  Bukan celah.
- NLP Indonesia: ternyata **sudah ada** `sastrawi-rs`, `rustrawi`, `harmorp`.
  Ini koreksi penting terhadap asumsi lama bahwa bidang ini kosong.
- Nama crate `temporal-rs` sudah dipakai `temporal_rs` (7,1 jt recent, kalender
  Temporal JS), jadi jangan pakai nama itu.

## Rekomendasi urutan pengerjaan

1. `holidays-rs` - permintaan paling jelas, pesaing praktis tidak ada, data
   tersedia terbuka (workalendar, date-holidays, python-holidays semuanya
   open source). Risiko rendah, nilai tinggi.
2. Ekstraksi tabel dari PDF - permintaan besar, tidak ada pesaing sama sekali,
   tetapi algoritmanya berat (deteksi garis, klaster kata, rekonstruksi sel).
3. Evaluator formula spreadsheet - permintaan besar, dua pesaing mati. Lingkup
   bisa dibatasi dulu ke subset formula yang umum.
4. E-invoice EN 16931 - permintaan sedang tetapi tumbuh karena regulasi Uni
   Eropa, dan `en16931-formats` masih terlalu kecil untuk dianggap pesaing.
5. Inflection Inggris - cara paling cepat menang adalah fork `inflector`,
   tambal bug yang lama, rilis ulang. Bukan menulis dari nol.

## Cara mereproduksi

Skrip ada di `/tmp/gapresearch` (tidak ikut di-commit):

- `crates.py` - pembungkus API crates.io dengan cache dan retry.
- `npmfetch.py` - ambil unduhan + metadata npm (endpoint bulk menolak campuran
  nama ber-scope dan tanpa scope, jadi harus dibatch terpisah).
- `pipeline.py` - tahap `pypi`, `crates-npm`, `crates-pypi`.
- `probe.py` - probe 263 kapabilitas terkurasi ke crates.io.
- `demand.py` - angka permintaan npm + PyPI untuk daftar kandidat terkurasi.

Catatan operasional: crates.io membatasi laju permintaan. Menjalankan dua skrip
sekaligus memicu HTTP 429 dan hasilnya jadi kosong palsu. Jalankan satu per satu.
