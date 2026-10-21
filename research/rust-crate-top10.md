# Top 10 kandidat crate Rust (verifikasi ulang, 2026-10-21)

Putaran kedua. Semua angka diambil dari API registry hari ini. Metrik pasokan
tetap `recent_downloads` crates.io (90 hari), tetapi kali ini setiap kandidat
juga dicek di GitHub (`api.github.com/search/repositories`) untuk menemukan
proyek WIP, dan angka permintaan diambil ulang dari pypistats + npm.

## Koreksi penting terhadap analisis putaran pertama

Tiga klaim putaran pertama saya ternyata salah setelah diperiksa ulang:

1. **Ekstraksi tabel PDF BUKAN celah.** Saya menyebutnya "tidak ada sama
   sekali" karena pencarian crates.io kosong. Padahal di GitHub ada minimal 5
   proyek aktif: `pdfsink-rs` (24 bintang, update 2026-10-13),
   `tablers` (15 bintang, 2026-10-19), `bolivar`, `distillpdf`, `pdfni`.
   Di crates.io juga ada `pdf-text-extract` (34,6 rb recent) dan
   `pdfsink-rs` (6,9 rb recent). Kandidat ini saya keluarkan dari daftar.
2. **CycloneDX SBOM bukan celah.** `cyclonedx-bom` punya 1,34 juta recent
   downloads dan aktif (2026-03). Saya hampir salah menilai karena hanya
   melihat crate `cyclonedx` yang kosong.
3. **XML Signature bukan celah.** `bergshamra-dsig` 186 rb recent dan
   `xml-sec` 43 rb recent (pure Rust, 2026-10). Sudah terisi.

Pelajaran metodologis: `recent_downloads` rendah TIDAK sama dengan tidak ada
pesaing. Crate baru yang belum dipublikasikan atau yang dipakai lewat path
dependency tidak terlihat di registry. GitHub search wajib untuk kandidat
teratas.

## Daftar 10

Diurutkan berdasarkan kekuatan kombinasi (permintaan nyata x kekosongan Rust x
kelayakan dikerjakan). "Nama bebas" sudah diverifikasi lewat API crates.io.

### 1. `holidays-rs` - kalender hari libur nasional
- Permintaan: PyPI `holidays` 26,9 juta/bulan, `workalendar` 957 rb/bulan.
- Rust sekarang: `holidays` 7,8 rb recent (v0.1.0, 2023), `public-holidays`
  v0.0.0. GitHub: `mapbox-jp/holidays-rs` 7 bintang (2024).
- Kenapa layak: datanya sudah terbuka (python-holidays, workalendar,
  date-holidays semuanya open source), jadi ini pekerjaan pemetaan data, bukan
  riset algoritma. Nama `holidays-rs`, `holiday-calendar`, `multicalendar`,
  `world-holidays` semuanya bebas.
- Kesulitan: rendah. Ini pilihan paling aman.

### 2. `soap-client` - klien SOAP/WSDL
- Permintaan: PyPI `zeep` 29,4 juta/bulan, `suds-py3` 771 rb/bulan. npm
  `soap` 453 rb/minggu.
- Rust sekarang: praktis kosong. `wsdl` 8 recent (2023), `soap` 3 recent,
  `rsoap` 46 recent, `soap-server` 195 recent. Tidak ada klien matang.
- Kenapa layak: bank, pemerintah, dan sistem ERP masih penuh SOAP. Tidak ada
  alternatif sama sekali, jadi ini celah paling lebar di daftar.
- Kesulitan: sedang-tinggi (perlu generasi kode dari WSDL, WS-Security, MTOM).
- Nama bebas: `soap-client`, `wsdl-client`, `soap-rs`, `rustsoap`.

### 3. `stdnum-rs` - validasi nomor identitas/VAT/pajak multi-negara
- Permintaan: PyPI `python-stdnum` 5,08 juta/bulan.
- Rust sekarang: `stdnum` cuma 16 recent. Terpecah-pecah: `dni` (Spanyol),
  `idnr` (Jerman), `ssn` (AS), `amka` (Yunani), `br_tax_id` (Brasil),
  `idsmith`, `vat_id_validator` 5,3 rb. Tidak ada payung yang menyatukan.
- Kenapa layak: setiap negara butuh validasi NIK/NPWP/VAT dan sekarang setiap
  proyek menulis sendiri. Sangat cocok untuk kontribusi bertahap, satu negara
  per pull request.
- Kesulitan: sedang, tapi sangat terukur dan mudah diuji.
- Nama bebas: `stdnum-rs`, `checkdigits`, `national-id`, `vatcheck`.

### 4. `spreadsheet-formula` - evaluator formula spreadsheet
- Permintaan: npm `exceljs` 10,8 juta/minggu, `xlsx` 7,2 juta/minggu; PyPI
  `openpyxl` 283 juta/bulan.
- Rust sekarang: `formula` 20 recent (mati 2022), `xlformula_engine` 2,4 rb
  (mati 2023), `formula-lang` 190 recent, `xlstream` 47 recent. Semua mati atau
  belum matang. Tidak ada yang mengevaluasi formula lengkap.
- Kenapa layak: `calamine` dan `rust_xlsxwriter` sudah bagus untuk baca/tulis
  berkas, tetapi keduanya berhenti di situ. Yang hilang adalah mesin
  evaluasinya.
- Kesulitan: sedang-tinggi. Mulai dari subset fungsi dulu.
- Nama bebas: `spreadsheet-formula`, `excel-formula`, `formula-eval`,
  `calcengine`.

### 5. `en16931-rs` - e-invoice Eropa (ZUGFeRD/Factur-X, UBL, PEPPOL)
- Permintaan: PyPI `factur-x` 287 rb/bulan. Regulasi: XRechnung sudah wajib
  untuk faktur B2G di Jerman, dan negara Uni Eropa lain menyusul.
- Rust sekarang: `en16931-formats` 5,5 rb recent (WIP, repo cuma 3 bintang),
  `zugferd` 159 recent.
- Kenapa layak: permintaannya didorong kewajiban hukum, bukan selera. Pasar
  yang tumbuh sendiri tanpa perlu meyakinkan siapa pun.
- Kesulitan: sedang (pemodelan data + aturan bisnis, bukan algoritma sulit).
- Nama bebas: `en16931-rs`, `e-invoice`, `facturx`, `zugferd-rs`.

### 6. `plantuml-rs` - renderer PlantUML
- Permintaan: PyPI `plantuml` 333 rb/bulan, npm `plantuml-encoder` 133
  rb/minggu.
- Rust sekarang: `puml` 97 recent, `plantuml-little` 671 recent. Perhatikan:
  Mermaid di Rust sudah terisi (`mermaid-rs-renderer` 160 rb recent, `merman`
  137 rb recent) — jadi PlantUML adalah satu-satunya yang kosong.
- Kesulitan: sedang-tinggi (butuh mesin tata letak diagram).
- Nama bebas: `plantuml-rs`, `plantuml-render`.

### 7. `musicxml-rs` - MusicXML
- Permintaan: PyPI `music21` 522 rb/bulan.
- Rust sekarang: `musicxml` 1,8 rb recent (2024). Untuk MIDI sudah ada
  `midly` (176 rb recent), tetapi MusicXML kosong.
- Kesulitan: sedang. Skemanya besar tetapi jelas.
- Nama bebas: `musicxml-rs`, `music-notation`.

### 8. `subtitles` - toolkit subtitle SRT/ASS/VTT
- Permintaan: PyPI `srt` 970 rb/bulan, `pysubs2` 193 rb/bulan; npm
  `srt-parser-2` 87 rb/minggu.
- Rust sekarang: semuanya mati. `subparse` 4,8 rb recent (terakhir 2021),
  `srt` 599 recent (2017), `ass-parser` 134 recent, `srt-parser` 269 recent.
- Kenapa layak: kebutuhan sehari-hari (media, pipeline video, alat AI), dan
  semuanya ditinggalkan. Ini kandidat dengan usaha terendah per nilai.
- Kesulitan: rendah-sedang. Cocok untuk rilis pertama yang cepat.
- Nama bebas: `subtitles`, `subtitle-parser`.

### 9. `libdav`-alternatif / `caldav-client` - klien CalDAV + CardDAV
- Permintaan: PyPI `caldav` 542 rb/bulan, `vdirsyncer` 27 rb/bulan.
- Rust sekarang: `libdav` 14,5 rb recent adalah satu-satunya pemain serius,
  dan itu batasnya. `caldav` 47 recent (2018), `carddav` 9 recent (2018).
- Kesulitan: sedang.
- Nama bebas: `dav-client`.

### 10. `iso20022` - pesan pembayaran ISO 20022 (pain/pacs/camt)
- Permintaan: PyPI `sepaxml` 312 rb/bulan; MT940/MT103 masih dipakai luas di
  perbankan. Migrasi SWIFT ke ISO 20022 menambah dorongan.
- Rust sekarang: `sepa` 1,6 rb recent, `mt940` 2,4 rb recent, `camt053` 220
  recent, `iso20022` 21 recent (mati 2020). WIP: `mx20022` (7 bintang),
  `quackiso` (6 bintang).
- Kesulitan: sedang. Skema XSD-nya publik dan bisa di-generate.
- Nama bebas: `iso20022-rs`, `sepa-rs`, `pain-rs`, `camt`.

## Yang sengaja saya keluarkan

- Ekstraksi tabel PDF - ramai WIP di GitHub (lihat koreksi di atas).
- CycloneDX SBOM, XML Signature, JSON-LD, HAR, Mermaid, XLSX, DOCX, PDF
  generik, OCR, browser automation, job queue, state machine, OAuth - sudah
  ada pemain aktif dengan puluhan ribu sampai jutaan recent downloads.
- Hunspell murni-Rust - `spellbook` sudah 140 rb recent, jadi bukan celah
  kosong meski masih bisa diperbaiki.
- PII redaction, embeddings/vector DB - sudah terisi.

## Saran langkah berikutnya

Mulai dari nomor 1 atau 8 untuk membangun reputasi cepat dengan risiko rendah,
lalu naik ke nomor 2 atau 4 yang nilainya paling besar tetapi paling berat.

Sebelum mulai, lakukan dua hal untuk kandidat yang dipilih:
1. `cargo search <nama>` dan cek GitHub `search?q=<topik>+language:rust` lagi,
   karena ruang ini bergerak cepat.
2. Cek apakah ada crate yang sudah di-`yank` atau repo yang diarsipkan tetapi
   belum digantikan; itu peluang adopsi tercepat karena pengguna sudah ada dan
   sedang mencari.
