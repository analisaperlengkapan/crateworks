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
