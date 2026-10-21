# 5 kandidat final (putaran 3, 2026-10-21)

## Kesalahan yang saya perbaiki di putaran ini

Dua hal fundamental yang membuat daftar sebelumnya meleset:

**1. Angka permintaan saya bukan permintaan Rust.**
PyPI/npm mengukur permintaan di Python/JS, bukan di Rust. Sinyal Rust yang
sebenarnya adalah **reverse dependencies** crate incumbent (berapa crate lain
yang bergantung padanya) dan apakah repo-nya benar-benar mati. Saya ukur itu
sekarang untuk semua kandidat.

**2. Nomor 8 (subtitles) di daftar sebelumnya SALAH.**
Saya menyimpulkan "semua mati" karena `subparse`/`srt` yang muncul di
pencarian. Ternyata ada **`subtitler` v2.9.0**: 25 versi, rilis terakhir
2026-10-02, mendukung 17 format (SRT, VTT, ASS/SSA, TTML, MicroDVD, EBU STL,
Whisper JSON). Itu crate matang dan aktif. Selain itu ada `subtp` (18,9 rb
recent) dan `revelo-parsers-text`. Bidang subtitle sudah terisi dengan baik.
Kandidat ini saya buang.

Pelajaran: crate yang mendominasi hasil pencarian bukan berarti crate terbaik.
Harus cek juga crate yang lebih baru yang belum populer.

## Data pendukung (diukur hari ini)

| Crate | recent | revdeps | rilis terakhir | status repo |
|---|---|---|---|---|
| `holidays` | 7.866 | 1 | 2023-02, **hanya 1 versi** | mati (push 2024-07), 6 issue |
| `py-holidays-rs` | 911 | 0 | 2025-09 | aktif, 1 bintang |
| `stdnum` | 16 | 0 | 2026-07 | baru, 0 bintang |
| `ssn` / `br_tax_id` / `idsmith` | 138 / 294 / 186 | 0 | 2026 | terpecah per negara |
| `formula` | 20 | 0 | 2022-11 | mati |
| `xlformula_engine` | 2.428 | 1 | 2023-05 | mati |
| `formula-lang` | 190 | 1 | 2026-10 | baru, 0 bintang |
| `iso20022` | 21 | 0 | 2020-03 | mati |
| `sepa` / `mt940` / `camt053` | 1.595 / 2.422 / 220 | 1 / 2 / 0 | 2026 | aktif tapi sempit |
| `wsdl` / `soap` / `rsoap` | 8 / 3 / 46 | **0 / 0 / 0** | 2023 / 2025 / 2026 | coba-coba |
| `en16931-formats` | 5.404 | 1 | 2026-10 | **sangat aktif** (11 versi / 3 bulan) |
| `musicxml` | 1.834 | **7** | 2024-11 | berhenti |
| `subtitler` | 1.905 | 0 | 2026-10 | **aktif, 25 versi** |
| `libdav` | 14.557 | 4 | 2026-09 | aktif, sehat |

Catatan penting soal SOAP: **0 reverse dependencies di seluruh ruang SOAP**.
Artinya permintaan besar itu ada di Python/JS, tetapi di Rust belum terbukti
siapa pun memakainya. Celah terlebar sekaligus permintaan Rust yang paling
belum terbukti.

## 5 kandidat

### 1. `holidays-rs` - kalender hari libur nasional
- **Kegunaan:** menghitung hari kerja. HR menghitung cuti dan gaji, bank
  menghitung jatuh tempo T+2, sistem tiket menghitung SLA. Butuh tahu tanggal
  libur per negara, dan data itu berubah tiap tahun.
- **Bukti gap:** `holidays` hanya punya 1 versi (2023), repo mati sejak 2024,
  6 issue terbuka. Penantang aktif cuma `py-holidays-rs` (911 recent, 1
  bintang). Permintaan: PyPI `holidays` 26,9 juta/bulan.
- **Kesulitan:** rendah. Ini pemetaan data, bukan riset algoritma. Sumber
  terbuka: python-holidays, workalendar, date-holidays.
- **Langkah pertama:** port python-holidays untuk 10 negara dulu, lengkapi
  dengan aturan "hari kerja berikutnya" dan "hari kerja ke-N".

### 2. `stdnum-rs` - validasi nomor identitas resmi
- **Kegunaan:** validasi NPWP, NIK, VAT Eropa, SSN, CPF, IBAN. Dipakai di form
  registrasi, onboarding KYC, validasi faktur pajak. Tiap negara punya rumus
  checksum sendiri.
- **Bukti gap:** tidak ada crate payung. Yang ada terpecah: `ssn` (AS),
  `br_tax_id` (Brasil), `dni` (Spanyol), `idnr` (Jerman), `amka` (Yunani),
  `vat_id_validator` (5,3 rb). `stdnum` sendiri baru lahir (16 recent, 0
  bintang). Permintaan: PyPI `python-stdnum` 5,08 juta/bulan.
- **Kesulitan:** sedang, tapi sangat terukur. Cocok untuk kontribusi bertahap:
  satu negara per pull request, satu modul per PR.
- **Langkah pertama:** API `validate(country, number)` + 15 negara pertama.

### 3. `spreadsheet-formula` - evaluator formula spreadsheet
- **Kegunaan:** membaca berkas Excel yang selnya berisi rumus lalu menghitung
  hasilnya, tanpa membuka Excel. Untuk laporan keuangan, impor data, server
  yang memproses spreadsheet.
- **Bukti gap:** setiap percobaan mati. `formula` (2022), `xlformula_engine`
  (2023), `formula-lang` (0 bintang). Sementara sisi baca/tulis sudah bagus
  (`calamine` 5,7 juta recent, `rust_xlsxwriter` 2,4 juta) — yang hilang persis
  mesin hitungnya. Permintaan: PyPI `openpyxl` 283 juta/bulan, npm `exceljs`
  10,8 juta/minggu.
- **Kesulitan:** sedang-tinggi. Kendalikan lingkup: mulai dari 50 fungsi
  (SUM, AVERAGE, IF, VLOOKUP, tanggal, teks), belum perlu pivot atau makro.
- **Langkah pertama:** parser formula + evaluator 20 fungsi, lalu uji terhadap
  berkas xlsx nyata dari `calamine`.

### 4. `iso20022` - pesan pembayaran perbankan
- **Kegunaan:** rekonsiliasi pembayaran, transfer otomatis, aplikasi akuntansi.
  MT940 = laporan rekening bank; SEPA = transfer euro; ISO 20022 =
  pain/pacs/camt, standar baru pengganti SWIFT MT.
- **Bukti gap:** semua pemain kecil dan terpisah. `iso20022` mati sejak 2020,
  `sepa` 1,6 rb, `mt940` 2,4 rb, `camt053` 220. Tidak ada yang menguasai.
  Permintaan didorong regulasi: SWIFT MT sedang dipensiunkan dan diganti
  ISO 20022.
- **Kesulitan:** sedang. Skema XSD-nya publik, jadi model tipe bisa
  di-generate, tidak perlu ditulis tangan.
- **Langkah pertama:** generate tipe dari XSD untuk camt.053 dan pain.001,
  lalu tambahkan parser + validator.

### 5. `soap-client` - klien SOAP/WSDL
- **Kegunaan:** menyambung ke web service gaya lama berbasis XML. Bank,
  instansi pemerintah, SAP, dan sistem warisan masih memakainya.
- **Bukti gap:** paling kosong di daftar. `wsdl` 8 recent (2023), `soap` 3
  recent, `rsoap` 46 recent, `soap-server` 195 recent. Tidak ada klien matang.
  Permintaan: PyPI `zeep` 29,4 juta/bulan, `suds-py3` 771 rb/bulan.
- **Risiko yang harus disadari:** 0 reverse dependencies di seluruh ruang SOAP.
  Permintaan itu nyata di Python, tetapi di Rust belum terbukti. Dan ini yang
  paling berat: butuh generasi kode dari WSDL, WS-Security, MTOM.
- **Langkah pertama:** parser WSDL + generasi tipe + klien dasar SOAP 1.1.
  Tunda WS-Security sampai ada pengguna nyata.

## Yang saya keluarkan dan alasannya

- **subtitles** - `subtitler` sudah matang (25 versi, 17 format). Salah nilai
  di putaran sebelumnya.
- **CalDAV** - `libdav` sehat dan aktif (4 revdeps). Sudah terisi.
- **en16931 e-invoice** - `en16931-formats` sedang digarap sangat agresif
  (11 versi dalam 3 bulan). Bersaing dengan penulis yang bermotivasi tinggi
  itu risiko besar.
- **MusicXML** - punya 7 revdeps (pengguna Rust nyata), tetapi sudah ada
  beberapa pendatang baru (`acorde-core` 8 revdeps, `music21-rs`, `guitarpro`,
  `croma-core`). Tidak lagi kosong.
- **PlantUML** - `puml` punya 23 issue terbuka (penulis tunggal kesulitan),
  tetapi mesin tata letak diagram itu pekerjaan berat untuk hasil yang sempit.
- **PDF table extraction, CycloneDX, XML Signature, Mermaid, JSON-LD, HAR,
  GTFS, SAML, FHIR, BibTeX, HCL, DICOM, IMAP, DKIM/SPF/DMARC** - semuanya
  sudah ada pemain aktif.

## Cara memilih di antara kelima ini

- Mau hasil cepat dan aman: **1 atau 2**.
- Mau dampak besar dan siap kerja lebih lama: **3 atau 5**.
- Mau pasar yang tumbuh sendiri karena regulasi: **4**.

Sebelum mulai, ulangi dua pemeriksaan ini karena ruangnya bergerak cepat:
`cargo search <nama>` dan GitHub `search?q=<topik>+language:rust`, lalu cek
tab reverse dependencies di crates.io untuk incumbent yang Anda incar.

---

# Addendum (2026-10-21, pemeriksaan nama dan kompetitor)

Ditambahkan setelah kandidat di atas mulai dikerjakan. Bagian ini tidak
mengubah penilaian di atas, hanya menambahkan dua hal yang belum diperiksa
waktu itu: **ketersediaan nama di crates.io** dan **pergerakan kompetitor**.

## 1. Ketersediaan nama (belum pernah diperiksa sebelumnya)

Semua nama kandidat diukur lewat `GET https://crates.io/api/v1/crates/<nama>`.
404 berarti bebas. Hasil yang mengubah keputusan:

| Nama | Status | Catatan |
|---|---|---|
| `iso20022` | **TERPAKAI** | keithnoguchi/iso20022-rs, dibuat 2020-03-11, terakhir 2020-03-16, 21 recent. Mati, tapi nama tetap terpakai |
| `iso20022-rs` | bebas | dipilih |
| `crateworks-xml` | bebas | diganti demi alasan desain, bukan bentrok |
| `holidays-rs` | bebas | nama polos `holidays` terpakai mapbox-jp (2023) |
| `stdnum-rs` | bebas | nama polos `stdnum` terpakai hey-jj (2026-07) |
| `soap-client` | bebas | nama polos `soap`/`wsdl` terpakai |
| `spreadsheet-formula` | bebas | nama polos `spreadsheet`/`formula` terpakai |
| `sepa` | terpakai | hupe1980/sepa |

**Kesalahan yang ditemukan: kandidat 4 (`iso20022`) tidak bisa di-publish
dengan nama itu.** Nama sudah diambil crate mati sejak 2020. Ini seharusnya
masuk pemeriksaan sejak awal: **cek ketersediaan nama adalah langkah pertama,
bukan langkah terakhir.** Mengganti nama setelah publish tidak mungkin.

Pelajaran soal konvensi `-rs`: akhiran itu dipakai **ketika nama polosnya sudah
terpakai**. `holidays` dan `stdnum` keduanya terpakai, jadi `holidays-rs` dan
`stdnum-rs` adalah keputusan yang benar, bukan kekurangan nama. `iso20022-rs`
mengikuti pola yang sama.

`crateworks-xml` diganti jadi `xmlkit` bukan karena bentrok, tapi karena crate
fondasi akan muncul di manifest setiap pengguna. Prefiks merek di situ memaksa
orang menulis `crateworks` di daftar dependensi mereka tanpa manfaat.

## 2. Pergerakan kompetitor sejak pengukuran pertama

| Crate | Status terbaru | Dampak |
|---|---|---|
| `sepa` | v0.9.0, 1.595 recent, update **2026-10-13** (8 hari lalu) | **Naik kelas.** Deskripsinya sekarang "SEPA identifiers and ISO 20022 payment messages: builders, bank-report". Ini tepat sasaran kandidat 4 dan bergerak cepat |
| `stdnum` | v0.2.0, 16 recent, dibuat 2026-07-01 | masih baru, 0 bintang. Deskripsinya persis kandidat 2 |
| `iso20022` | 21 recent, mati sejak 2020 | tidak berubah |
| `calamine` | 5,67 juta recent, v0.37.0, update 2026-10-18 | sisi baca/tulis xlsx tetap kuat, mesin hitungnya tetap kosong |

**Yang berubah dari penilaian awal:** `sepa` sekarang bukan "kecil dan sempit"
lagi. Dengan 1.595 recent dan rilis 8 hari lalu, ia penantang serius di ruang
ISO 20022, bukan pemain pinggiran. Penilaian "tidak ada yang menguasai" masih
benar, tetapi tingkat kesulitannya naik.

`stdnum` (hey-jj) baru 3 bulan dan masih 0 bintang, jadi kandidat 2 masih
terbuka, tapi tidak lagi kosong.

## 3. Yang tidak berubah

- **SOAP tetap paling kosong.** `wsdl` 8 recent, `soap` 3 recent, `rsoap` 46
  recent, dan **0 reverse dependencies** di seluruh ruang. Celah terlebar,
  permintaan Rust paling belum terbukti.
- **Mesin formula spreadsheet tetap kosong.** `formula` (2022, mati),
  `xlformula_engine` (2023, mati), `formula-lang` (0 bintang). Sisi
  baca/tulis sudah bagus, yang hilang tetap mesin hitungnya.
- **`holidays` tetap 1 versi dari 2023** dengan repo mati.

## 4. Catatan metode

- Reverse dependencies tetap sinyal terbaik. Unduhan mentah menyesatkan: crate
  mati seperti `spreadsheet` (2016) masih punya 5.725 unduhan total.
- Pemeriksaan ketersediaan nama sebaiknya dijalankan **sebelum** menulis kode,
  untuk semua kandidat sekaligus. Hasilnya jarang berubah, jadi sekali periksa
  cukup sampai ada kandidat baru.
- Ruang ini bergerak cepat. `sepa` naik dari "sempit" ke penantang aktif dalam
  hitungan bulan, jadi pengukuran perlu diulang sebelum mulai mengerjakan
  kandidat berikutnya.

---

## Addendum 2026-10-21 (kedua): putaran probe kandidat baru

Setelah 6 crate pertama, saya probe 19 kapabilitas lagi dengan cara yang sama
(permintaan dari npm/PyPI, pasokan dari crates.io). Tujuannya mencari yang layak
dibangun berikutnya. Hasilnya mengecewakan dalam arti yang berguna: sebagian
besar kandidat ternyata **sudah terisi**, dan saya hampir membangun duplikat.

### Yang ternyata sudah terisi (jangan dibangun)

| Kapabilitas | Yang saya kira celah | Kenyataannya |
|---|---|---|
| Angka ke kata (multi-bahasa) | `num2words` (PyPI 7,8 jt/bln), `inflect` (22 jt/bln) | `num2words` Rust sudah ada dan **sudah multi-bahasa** (French, dll), 94 rb recent. Bukan celah. |
| Format & parse SQL | `sqlparse` (107 jt/bln), `sqlglot` (72 jt/bln) | `sqlformat` 4,3 jt recent + `sqlparser` 14,8 jt recent, 435 revdeps. Bukan celah. |
| Ukuran byte ke teks | `bytes` (84 jt/mgg), `pretty-bytes` (19 jt/mgg) | `humansize` 12,5 jt, `bytesize` 21,9 jt, 500 revdeps. Bukan celah. |
| Durasi ISO 8601 | `iso8601-duration`, `tinyduration` | `iso8601` 6,4 jt, `duration-str` 3,7 jt. Bukan celah. |
| Fuzzy matching | `rapidfuzz` (116 jt/bln), `thefuzz` | `strsim` 249 jt recent, `fuzzy-matcher` 9,2 jt, `rapidfuzz` Rust 4,3 jt. Bukan celah. |
| Validasi nomor telepon | `libphonenumber-js` (16 jt/mgg), `phonenumbers` (34 jt/bln) | `phonenumber` 3,2 jt recent, 65 revdeps. Bukan celah. |
| Barcode | `jsbarcode`, `python-barcode` | `rxing` 212 rb, `barcoders` 162 rb, keduanya aktif. Bukan celah. |
| Kekuatan sandi | `zxcvbn` | `zxcvbn` Rust 1,8 jt recent. Sudah terisi. |
| Kartu kredit | `card-validator` | `card-validate` 439 rb recent. Sudah terisi. |
| Kalender Hijriah **via nama `hijri`** | PyPI `hijri-converter` 2,8 jt/bln | Crate `hijri` Rust cuma **CLI**, 11 recent. Ini yang menipu: namanya mirip, fungsinya bukan library. |

### Kandidat yang lolos, dan yang dipilih

**Dipilih dan sudah dibangun: `hijridate`.**

- Permintaan: PyPI `hijridate` 3,3 jt/bln, `hijri-converter` 2,8 jt/bln,
  `ummalqura` 1,2 jt/bln. Semua jutaan, bukan ribuan.
- Pasokan: crate `hijri` (CLI, 11 recent), `hijridate`/`hijri-converter`/
  `ummalqura`/`islamic-calendar` **semuanya bebas**. Nama `hijri-date` terpakai
  tapi mati sejak 2023 (857 recent).
- Kualitas celah: `holidays-rs` di workspace ini sendiri sudah menuliskan bahwa
  Idul Fitri dan Idul Adha Indonesia tidak dimasukkan karena "bergantung pada
  keputusan resmi, bukan rumus". Tabel Umm al-Qura adalah tempat tanggal itu
  dicari. Jadi celah ini sudah terdokumentasi sebelum saya mencarinya.
- Data tersedia terbuka dan MIT (`dralshehri/hijridate`), jadi tidak perlu
  menebak tanggal.

**Tetap terbuka, belum dikerjakan:**

- **Ekstraksi tabel dari PDF** (`pdfplumber` 40,7 jt/bln, `tabula-py` 1,2 jt/bln,
  `camelot-py` 722 rb/bln). Di Rust: `pdfplumber` 40 rb recent (satu pemain,
  alpha), `pdf-table`/`camelot` bebas. Celah terbesar yang tersisa, tapi
  algoritmanya berat.
- **Hunspell murni-Rust**: `spellbook` 140 rb recent sudah ada dan aktif, jadi
  ini bukan lagi celah bersih. `nspell` (npm) 147 rb/mgg.
- **vCard**: `vcard`, `vcard-rs`, `vobject`, `vcf` semuanya ada tapi kecil
  (1,1-1,5 rb recent). Setengah celah, permintaan Rust belum terbukti.

### Catatan metode (koreksi)

- **Nama crate yang mirip bisa menipu.** `hijri` terdengar seperti library
  kalender; ternyata CLI. Selalu baca deskripsi dan `crate_type`, bukan hanya
  nama dan angka unduhan.
- `MONTH_STARTS` di `hijridate` Python menyimpan **RJD** (JDN - 2.400.000), bukan
  JDN. Saya sempat salah asumsi dan tabelnya menghasilkan tahun -4647. Pelajaran
  yang sama seperti port Gitea: baca sumbernya, jangan menebak arti sebuah kolom.
- Regenerasi tabel bersifat idempoten, jadi diff pada berkas hasil generate
  berarti ada rilis upstream baru atau ada kesalahan — bukan perubahan gaya.

---

## Addendum 2026-10-21 (ketiga): putaran probe lanjutan, 40 kandidat

Saya probe 40 kapabilitas lagi dalam empat batch. Hasilnya memperkuat pola yang
sama: **sebagian besar kandidat ternyata sudah terisi**, dan beberapa nama yang
terdengar seperti celah ternyata crate yang aktif.

### PENTING: pesaing baru untuk crate yang sudah ada di workspace ini

Riset ini menemukan bahwa dua crate workspace ini sudah punya tetangga:

| Crate workspace | Pesaing yang ditemukan | Ukuran |
|---|---|---|
| `iso20022-rs` | `mx20022` (50 recent), `rust_iso20022` (55), `iso20022-common` (**2.362 recent**, 2024), `mx20022-translate` | Ada, tapi semua kecil kecuali `iso20022-common` |
| `soap-client` | (tidak berubah) `wsdl` 8, `soap` 3, 0 revdeps | Masih paling kosong |

`iso20022-common` 2.362 recent itu perlu diperhatikan: sebelumnya saya mencatat
"hanya ada satu crate dari 2020". Itu tidak lagi akurat. Kandidat ISO 20022
tidak lagi kosong, meski belum ada pemain dominan.

### Yang ternyata sudah terisi (jangan dibangun)

| Kapabilitas | Permintaan | Pasokan Rust | Vonis |
|---|---|---|---|
| **SWIFT MT103/MT202** | PyPI `mt-940` 499 rb/bln | **`swift-mt-message` 3.881 recent + derive macros, aktif 2026-02** | Terisi. Nama `mt103`/`swift-mt` bebas tapi tidak berguna. |
| JSONLogic rules engine | npm `json-logic-js` 1,5 jt/mgg, PyPI `json-logic` 2,2 jt/bln | `datalogic-rs` **85 rb recent**, `juspay_jsonlogic` 77 rb, `jsonlogic-rs` 14 rb | Terisi kuat. |
| Kata ke angka | PyPI `word2number` 2 jt/bln, `number-parser` 622 rb | `text2num` 21.655 recent (multi-bahasa), `words2num2-core` | Terisi. |
| Konversi satuan | PyPI `pint` 5,2 jt/bln, npm `convert-units` 116 rb/mgg | `uom` **2,6 jt recent** | Terisi kuat. |
| Transliterasi/ASCII folding | PyPI `text-unidecode` 65 jt/bln | `deunicode` **17 jt recent**, `unidecode` 1,4 jt, `any_ascii` 2,1 jt | Terisi kuat. |
| Slug URL | PyPI `python-slugify` 64 jt/bln, npm `slugify` 11 jt/mgg | `slug` **8 jt recent**, 157 revdeps | Terisi kuat. |
| Durasi human-readable | PyPI `humanize` 46 jt/bln | `humantime` **70 jt recent**, 1.359 revdeps | Terisi kuat. |
| Parsing tanggal natural | PyPI `python-dateutil` 899 jt/bln | `dateparser` 2,1 jt, `parse_datetime` 1,6 jt, `dtparse` 365 rb | Terisi. |
| Timezone dari koordinat | PyPI `timezonefinder` 5,1 jt/bln | `tzf-rs` 147 rb recent | Terisi. |
| DICOM medis | PyPI `pydicom` 5,8 jt/bln | `dicom` **307 rb recent**, 13 revdeps | Terisi. |
| NMEA GPS | PyPI `pynmea2` 112 rb/bln | `nmea-parser` 118 rb, `nmea` 43 rb | Terisi. |
| EDI X12 | PyPI `pyx12` 308 rb/bln | `edi` 58 rb recent | Terisi. |
| EDIFACT | PyPI `pydifact` 171 rb/bln | `edifact-rs` 1.931 recent (aktif 2026-10) | Terisi. |
| OFX | PyPI `ofxparse` 413 rb/bln | `ofx-rs` 2.819 recent | Terisi. |
| MRZ paspor | PyPI `mrz` 52 rb/bln | `mrz` 6.300 recent | Terisi. |
| MT940 rekening koran | PyPI `mt-940` 499 rb/bln | `mt940` 2.435 recent, aktif | Terisi. |
| HL7 v2 | PyPI `hl7apy` 758 rb/bln | `hl7-parser` 6.645 recent, `hl7` 172, `hl7v2` 120 | Ada, tapi lemah. |
| i18n angka/tanggal | PyPI `babel` 101 jt/bln | `icu` 703 rb recent (ICU4X), `num-format` 14 jt | Terisi. |
| Nama orang | PyPI `nameparser` 3,1 jt/bln | `human-name` 485 rb recent | Terisi. |
| Beancount/ledger | PyPI `beancount` 133 rb/bln | `beancount-rs` 12 recent (2026-07) | Kosong tapi permintaan kecil. |

### Celah yang masih nyata, diurutkan

1. **ISO 8583** (pesan transaksi kartu). PyPI `pyiso8583` **403.648/bln**. Di Rust:
   `iso8583` **62 recent (2017, mati)**, `iso8583-rs` **73 recent (2020, mati)**,
   `iso8583-codec-rs` 3 recent. Nama `iso8583-msg` dan `iso8583kit` **bebas**.
   Ini celah terkuat: dua percobaan mati, permintaan nyata di industri pembayaran.
2. **Teks Arab** (reshaping, harakat, normalisasi). PyPI `arabic-reshaper`
   **4,8 jt/bln**. Di Rust: `arabic-reshaper` 1.859 recent (2023), `ar-reshaper`
   9.241 recent (2024), `arabic_text_utils` 46 recent. Permintaan besar, pasokan
   tua dan terpecah. Nama `arabic` dan `tashkeel` bebas.
3. **EN 16931 e-invoice** (Factur-X/ZUGFeRD/UBL/XRechnung). PyPI `factur-x`
   287 rb/bln. Di Rust: `en16931-formats` 5.417 recent (1 revdep, aktif),
   `en16931` 5.586 recent, `zugferd` 159 recent, `xrechnung` 5 recent.
   Permintaan sedang dan tumbuh karena regulasi UE.
4. **Teks Quran** (lookup ayat, halaman mushaf, tajwid). Kecil di semua ekosistem
   (npm `quran-json` 2.827/mgg, PyPI `quran` 608/bln), Rust `quran` 2 recent
   (2022). Celah nyata tapi ceruk sempit. Nama `quran-rs`, `tajweed`, `mushaf`
   bebas.

### Catatan metode

- **Bahkan `mt103` yang terdengar pasti kosong ternyata sudah ada.** Crate
  `swift-mt-message` muncul hanya setelah saya mencari frasa "swift mt message",
  bukan nama crate. Mencari berdasarkan **kapabilitas**, bukan nama, itu wajib.
- Nama crate bebas bukan berarti celahnya nyata: `mt103` bebas, tapi kapabilitas
  MT103 sudah dikerjakan `swift-mt-message`. Ketersediaan nama adalah syarat
  perlu, bukan syarat cukup.
- Pola yang berulang di tiga putaran: **kapabilitas generik hampir selalu sudah
  terisi; yang kosong adalah kapabilitas yang terikat domain tertentu**
  (pembayaran kartu, regulasi fiskal UE, teks Arab/Quran). Itu arah yang benar
  untuk putaran berikutnya.

---

## Addendum 2026-10-21 (keempat): 5 kandidat final

Saya probe ~45 kapabilitas lagi dalam lima batch, dengan penekanan pada area
yang **belum pernah** saya sentuh: protokol keuangan, transit, bioinformatika,
kimia, otomotif, kode medis, dan pemrosesan aksara non-Latin.

### Yang gugur di putaran ini (jangan dibangun)

| Kapabilitas | Permintaan | Kenapa gugur |
|---|---|---|
| Transliterasi Kiril | PyPI `cyrtranslit` 103 rb/bln | **`transliteratio` ada** (236 recent, aktif 2026-10-19, sudah mencakup tabel ISO **dan** GOST), plus `translit` 2.651, `iuliia`, `cranberry`. Terisi. |
| Segmentasi kata Thai | PyPI `pythainlp` 658 rb/bln | `kham-core` (464 recent, murni Rust), `thaibreak` 125, `nlpo3` 5.155. Terisi. |
| QR pembayaran EMVCo | npm `qrcode-generator` 2,9 jt/mgg | `raast-qr` 227 recent (#![no_std]), `promptparse` 232, `brcode` 62, `pix-brcode-parser`. Terisi. |
| FIX protocol | PyPI `quickfix` 25 rb/bln | Keluarga `minifix*` (2026-10-08) + `quickfix` binding 162 rb recent. Ada. |
| ACH/NACHA | PyPI `ach` 11 rb/bln | Permintaan kecil (`nacha` PyPI 444/bln). Tidak sepadan. |
| GTFS transit | PyPI `gtfs-realtime-bindings` 647 rb/bln | `gtfs-structures` 24 rb recent + `gtfs-rt`. Terisi. |
| Shapefile GIS | PyPI `pyshp` 3,6 jt/bln | `shapefile` 160 rb recent, 37 revdeps. Terisi. |
| FASTA/FASTQ/VCF bio | PyPI `biopython` 4,4 jt/bln | `bio` 114 rb recent, `noodles` 113 rb, 141 revdeps. Terisi kuat. |
| SMILES kimia | PyPI `rdkit` 4,7 jt/bln | `purr` 13 rb, `chemcore`, `rdkit` binding. Setengah terisi, butuh FFI. |
| CAN DBC otomotif | PyPI `cantools` 881 rb/bln | `can-dbc` 175 rb recent, 13 revdeps. Terisi. |
| ICD-10/SNOMED/FHIR | PyPI `fhir.resources` 2,5 jt/bln | `snomed` 900, `loinc` 227, `fhir` 530 (semua aktif 2026-10). Terisi. |
| Pinyin Mandarin | PyPI `pypinyin` 1,6 jt/bln | `pinyin` 297 rb recent, 34 revdeps. Terisi. |
| Segmentasi kata Mandarin | PyPI `jieba` 4,8 jt/bln | `jieba-rs` **1,77 jt recent**, 59 revdeps. Terisi kuat. |
| Hanzi ringkas/tradisional | PyPI `opencc` 650 rb/bln | `zhconv` 20 rb recent + `opencc-rs`. Terisi. |
| Kana/romaji Jepang | PyPI `pykakasi` 2,2 jt/bln | `kakasi` 10,5 rb recent, `furigana`, `romaji`. Terisi. |
| Hangul Korea | PyPI `hangul-romanize` 93 rb/bln | `hangul` 4.750 recent. Terisi. |
| XBRL | PyPI `xbrl` 19 rb/bln | `xbrl-rs` 258 recent (aktif 2026-10). Terisi. |
| SMPP SMS | PyPI `smpplib` 55 rb/bln | `smpp34` 2.762 recent (murni Rust, aktif). Terisi. |
| RADIUS/Diameter | PyPI `pyrad` 256 rb/bln | `radius` 15 rb recent, `diameter` 512. Terisi. |

### 5 kandidat final

Diurutkan berdasarkan rasio permintaan terhadap pasokan, lalu kejelasan celah.

**1. ISO 8583 — pesan transaksi kartu (ATM/EDC/switching)**
- Permintaan: PyPI `pyiso8583` **403.648/bln**.
- Pasokan: `iso8583` 62 recent (2017, mati), `iso8583-rs` 73 recent (2020, mati),
  `iso8583-codec-rs` 3 recent, `iso8583-core` 12 recent (2026-02). **Empat
  percobaan, semuanya ≤73 recent.** Tidak ada yang dipakai.
- Nama: `iso8583-msg` **bebas**, `iso8583kit` **bebas**.
- Kenapa cocok: spesifikasi publik dan stabil, table-driven (bitmap + tipe data
  fixed/LLVAR/LLLVAR), tanpa masalah "otoritas mana". Paling mirip `hijridate`
  dalam hal bentuk kerja, tapi pasarnya jauh lebih besar.

**2. Teks Arab — reshaping, harakat, normalisasi**
- Permintaan: PyPI `arabic-reshaper` **4,8 jt/bln**; npm `arabic-persian-reshaper`
  42.084/mgg + `arabic-reshaper` 14.329/mgg.
- Pasokan: `ar-reshaper` 9.241 recent (2024), `arabic-reshaper` 1.859 (2023),
  `arabic_text_utils` 46, `dhad` 86, `arabic-normalize` 9. **Terpecah dan tua;
  tidak ada yang dominan.** Yang terbesar (9 rb) pun 25x lebih kecil dari pinyin.
- Nama: `arabic` **bebas**, `tashkeel` **bebas**.
- Kenapa cocok: permintaan terbesar di antara semua celah yang saya temukan, dan
  bersinergi dengan `hijridate` (satu domain: teks Arab + kalender Hijriah).

**3. EN 16931 e-invoice — Factur-X / ZUGFeRD / UBL / XRechnung**
- Permintaan: PyPI `factur-x` **286.877/bln**.
- Pasokan: `en16931-formats` 5.417 recent (1 revdep, aktif), `en16931` 5.586,
  `zugferd` 159, `xrechnung` 5. **Pemain tunggal dan masih kecil.**
- Nama: `factur-x` **bebas**.
- Kenapa cocok: regulasi UE (EN 16931 wajib untuk pengadaan publik) memaksa
  adopsi. Butuh `xmlkit` dari workspace ini, jadi sinergi internal.

**4. Validasi nomor PPN/PPN EU (VAT)**
- Permintaan: npm `jsvat` **50.847/mgg**; `python-stdnum` (yang memuatnya)
  **5,08 jt/bln**.
- Pasokan: crate `vat` **4 recent (2016, mati)**, tidak ada lagi. Nama `eu-vat`
  **bebas**, `vat-validator` **bebas**.
- Kenapa cocok: **sinergi langsung dengan `stdnum-rs`** yang sudah ada — ini
  penambahan modul, bukan crate dari nol. Checksum per negara + format, murni
  tabel aturan, std-only. Pekerjaan paling ringan dari lima ini.

**5. Transliterasi aksara Indic (Devanagari dkk.)**
- Permintaan: PyPI `indic-transliteration` **69.267/bln**; npm `sanscript`
  84/mgg.
- Pasokan: **NOL.** Tidak ada crate transliterasi Indic sama sekali di Rust
  (`indic-transliteration`, `devanagari`, `sanscript`, `indic` semua bebas dan
  tidak ada padanannya).
- Nama: `indic-transliteration` **bebas**, `devanagari` **bebas**, `sanscript`
  **bebas**.
- Kenapa cocok: celah paling bersih — benar-benar kosong, bukan "ada tapi tua".
  Permintaan sedang tapi nyata. Catatan penting: transliterasi Kiril **sudah
  terisi** (`transliteratio` mencakup ISO/GOST), jadi jangan generalisasi ke
  "semua transliterasi kosong"; yang kosong spesifik Indic.

### Catatan tambahan: celah Indonesia tanpa pesaing

`pph21`, `bpjs`, `pajak`, `ppn`, `efaktur` **semuanya bebas** dengan **nol**
pesaing di crates.io. Masalahnya: tidak ada paket npm/PyPI yang bisa dipakai
mengukur permintaan (semuanya `None`), jadi ini "bangun dulu, mudah-mudahan
dipakai" — bukan celah terukur. Tetap relevan untuk konteks cipansor/pesantren,
tapi saya tidak memasukkannya ke lima besar karena permintaannya belum terbukti.

### Catatan metode

- **"Tidak ada crate" lebih kuat daripada "ada crate tapi tua".** Kiril tampak
  kosong sampai saya mencari frasa "gost transliteration" dan menemukan
  `transliteratio` yang aktif. Indic tetap kosong bahkan setelah dicari.
- Pencarian harus memakai **frasa kapabilitas**, bukan nama crate. `iso8583-core`
  dan `swift-mt-message` tidak muncul dari pencarian nama.
- Pola tiga putaran ini konsisten: **generik terisi, terikat-domain kosong.**
  Lima kandidat di atas semuanya terikat domain (pembayaran kartu, aksara Arab,
  regulasi fiskal UE, aksara Indic).

---

## Addendum 2026-10-21 (kelima): dependensi kritis, dan tiga koreksi

Fokus putaran ini berbeda: bukan "kapabilitas apa yang dipakai pengguna akhir",
melainkan **dependensi kritis** — lapisan yang harus ada sebelum crate lain bisa
berfungsi. Caranya sama (permintaan dari PyPI/npm, pasokan dari crates.io dengan
`recent_downloads` + reverse dependencies), tetapi yang dicari adalah
infrastruktur, bukan alat.

### Tiga koreksi terhadap catatan sebelumnya

| Klaim lama | Kenyataannya (diukur hari ini) |
|---|---|
| **Indic transliteration: "NOL"** | **Salah.** `shlesha` **3.394 recent** (2026-07-06, "extensible transliteration library with hub"), `vidyut-lipi` 636, `lipilekhika` 31, `transliterator` 3. Jadi ada pemain, meski kecil. Nama `indic`, `devanagari`, `sanscript`, `itrans`, `velthuis`, `aksharamukha` tetap bebas. |
| **Package URL (purl) tampak kosong** | **Salah.** `purl` **1.623.677 recent**, `packageurl` 148.497, `purl_validator` 1.361. Sudah terisi kuat. (PyPI `packageurl-python` 32 jt/bln.) |
| **IPP/CUPS printing kosong** | **Salah.** `ipp` **25.800 recent** (2026-08-12, "asynchronous IPP print protocol"), `cups-rs` 9.105, `cups` 179. Sudah terisi. Yang masih kosong hanya `ipp-client` (2 recent). |

Pelajaran yang sama terulang: **nama crate yang mirip menipu, dan pencarian harus
memakai frasa kapabilitas.** `purl-rs` ternyata CLI HTTP, bukan Package URL.

### Yang ternyata sudah terisi (jangan dibangun)

| Kapabilitas | Permintaan | Pasokan Rust | Vonis |
|---|---|---|---|
| XML Signature + XML Encryption | `xmlsec` 8,5 jt/bln, `signxml` 2,3 jt/bln | `xml-sec` **43.682 recent** (2026-10-21), `bergshamra-dsig` 187 rb, `bergshamra-enc` 101 rb | Terisi kuat |
| PKCS#11 / HSM | - | `cryptoki` **1,43 jt recent**, 10 revdeps | Terisi kuat |
| Kerberos / GSSAPI | - | `krb5` 432 recent (2026-10-16, pure Rust), `kerberos-parser` 129 rb | Terisi |
| LDAP | - | `ldap3` **1,34 jt recent**, 10 revdeps | Terisi kuat |
| eBPF / pcap | - | `rbpf` 62 rb, `pcap-file` 2,88 jt, `pcap` 1,13 jt | Terisi |
| XSD / RELAX NG / Schematron | - | `uppsala` **230.774 recent** (XSD), `relax-ng` 2.327, `schematron-engine` 2.234 | Terisi |
| XSLT / XPath | - | `xrust` 13.923, `oxixml-xpath` 23.739 | Terisi |
| HDF5 / NetCDF | - | `hdf5-metno` 714 rb, `hdf5-pure` 121 rb, `netcdf` 54 rb | Terisi |
| COBOL copybook / EBCDIC | - | `copybook` 220, `copybook-charset` 596 (2026-09-17) | Ada, kecil |
| COBOL copybook / EBCDIC | - | keluarga `copybook-*` aktif 2026-09 | Ada |
| Modbus / OPC UA / DNP3 | - | `modbus` 34 rb, `async-opcua` 104 rb, `dnp3` 16.967 | Terisi |
| Unicode collation | - | `feruca` 14.615, `colligo` 7.782, `icu_collator` 5,36 jt | Terisi |
| DICOM + jaringan DIMSE | - | `dicom` 307 rb, `dicom-ul` 336 rb | Terisi |
| SM2/SM3/SM4 (Tiongkok) | `gmssl` 116 rb/bln | `gmcrypto-core` 1.889 (2026-10-09, pure Rust) | Ada, tipis |
| GOST (Rusia) | `gostcrypto` 12 rb/bln | `gost-crypto` 288, `kuznyechik` 15.423, `magma` 13.765 | Ada |

### Dua celah dependensi kritis yang nyata

**A. PDF/A + XMP conformance.** Ini **separuh yang hilang dari `factur-x`** yang
sudah ada di workspace ini. `factur-x` menghasilkan XML-nya; untuk benar-benar
mengirim faktur, XML itu harus ditanam di PDF/A-3 dengan metadata XMP yang benar.
Tanpa itu, crate kita hanya berguna separuh.

- Permintaan: PyPI `pikepdf` **9,48 jt/bln**, `pypdf` **126 jt/bln**,
  `reportlab` **66 jt/bln**, `ocrmypdf` 1,08 jt/bln.
- Pasokan: `lopdf` 12,55 jt recent dan `pdf-writer` 3,44 jt recent memang besar,
  **tetapi keduanya penulis PDF tingkat rendah tanpa konsep PDF/A.** Yang
  menangani kepatuhan: `pdf-compliance` **235 recent**, `pdfpurr` 1.669,
  `page_validation` 716, `pdfluent` 138. Tidak ada yang dipakai.
- Nama `pdfa` dan `pdf-a` **bebas**.
- Catatan: bagian *checker*-nya (aturan profil PDF/A di atas PDF yang sudah
  diurai) jauh lebih mudah daripada penulisnya, dan itu tepat yang disediakan
  veraPDF. Bisa dibangun di atas `lopdf` atau `pdf-writer`.

**B. WS-Security untuk SOAP.** Ini **separuh yang hilang dari `soap-client`** di
workspace ini. Setiap endpoint SOAP bank, pemerintah, dan SAP mewajibkan
WS-Security (UsernameToken, X.509 signing, timestamp). Tanpa itu `soap-client`
tidak bisa bicara dengan mereka.

- Permintaan: PyPI `zeep` **29,4 jt/bln**, `xmlsec` 8,5 jt/bln, `signxml`
  2,3 jt/bln.
- Pasokan: `xml-sec` 43.682 recent sudah menyediakan XMLDSig, XMLEnc, dan C14N
  sebagai fondasinya. **WS-Security sendiri kosong:** `parco-ws-security` **11
  recent** satu-satunya percobaan. Nama `wssec` **bebas**.
- Ini dependensi kritis yang paling jelas: fondasinya sudah ada, lapisan di
  atasnya belum.

### Tiga celah domain yang masih nyata

**C. HL7 v2 + MLLP.** Cara setiap rumah sakit dan laboratorium bertukar data.
- Permintaan: PyPI `hl7apy` **758 rb/bln**.
- Pasokan terpecah dan lemah: `hl7-parser` 6.645 recent (2025-02, deskripsinya
  mengaku "does not do" sebagian), `hl7v2-parser` 106 (3 revdeps), `hl7` 172,
  `hl7-v2` 62, `hl7-mllp` 76, `hl7v2-mllp` 166, `vyna-hl7-mllp` 249. **Tidak ada
  pemimpin**, dan lapisan transport MLLP tersebar di tiga crate kecil.
- Sinergi: format berdelimiter, sangat mirip `iso8583-msg` yang sudah ada.

**D. Transliterasi aksara Indic.** (Dikoreksi: `shlesha` 3.394 recent ada, jadi
ini bukan lagi kosong — tetapi 3,4 rb masih kecil.)
- Permintaan: PyPI `indic-transliteration` 69.267/bln.
- Pasokan: `shlesha` 3.394, `vidyut-lipi` 636, `lipilekhika` 31.
- Nama bebas: `indic`, `devanagari`, `sanscript`, `itrans`, `velthuis`.
- Sinergi: kerja tabel murni, seperti `arabic`.

**E. ZPL II (printer label Zebra).** Gudang dan logistik.
- Permintaan: PyPI `zpl` 46.313/bln, `zebra` 13.050/bln.
- Pasokan: `zpl_toolchain_core` 311, `zpl` 159, `zpl-builder` 154, semuanya
  kecil dan tidak ada yang dominan.
- Nama `epl2` **bebas**. Bahasa perintah berbasis teks, mudah.

### Catatan metode (kelima)

- Reverse dependencies tetap sinyal terbaik, tetapi **crate besar tidak sama
  dengan celah tertutup**: `lopdf` 12,55 jt recent tidak berarti PDF/A terisi,
  karena `lopdf` tidak mengurus kepatuhan sama sekali. Tanyakan *lapisan* mana
  yang terisi, bukan *kategori* mana.
- Pola empat putaran tetap: **generik terisi, terikat-domain kosong.** Yang baru
  di putaran ini: **lapisan kepatuhan/standar di atas fondasi yang sudah ada**
  juga kosong (`pdfa` di atas `lopdf`, `wssec` di atas `xml-sec`). Itu arah
  paling produktif untuk putaran berikutnya.
