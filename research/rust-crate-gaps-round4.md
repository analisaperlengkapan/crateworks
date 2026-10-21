# Celah paket Rust: verifikasi ulang + pemindaian baru (2026-10-21)

Lanjutan dari `rust-crate-5-final.md`. Semua angka di bawah **diukur hari ini**
lewat API crates.io, npmjs.org, dan PyPI — bukan dari ingatan. Tool: `research/tools/`.

## Metode yang dipakai

Tiga angka, bukan satu:

1. **Permintaan di ekosistem asal** (npm/minggu atau PyPI/bulan). Ini menunjukkan
   masalahnya nyata dan banyak orang mencarinya. **Bukan** bukti orang mencarinya di Rust.
2. **`recent_downloads` crate Rust incumbent.** Kalau ada incumbent dengan puluhan
   juta unduhan, celahnya sudah tertutup — sebagus apa pun idenya.
3. **`reverse_dependencies` incumbent.** Ini sinyal Rust yang sebenarnya. npm 40 juta
   per minggu di JS tidak berarti apa-apa kalau nol crate Rust yang bergantung pada
   padanannya.

Plus satu pemeriksaan kualitas: **apakah repo incumbent masih hidup.** Incumbent
mati dengan permintaan besar adalah peluang terbaik — permintaan sudah terbukti,
dan tidak ada yang melayani.

### Koreksi tooling

Endpoint bulk npm (`/downloads/point/last-week/a,b,c`) **mengembalikan 0 secara
senyap** kalau batch-nya terlalu besar atau memuat nama scoped. Pemeriksaan pertama
saya melaporkan `object-hash = 0/minggu`, yang salah; nilainya 40,75 juta. Ambil
per paket, atau batch kecil tanpa nama scoped.

## Hasil pemindaian

Diperiksa 130+ kategori. Sebagian besar **sudah terisi dengan baik** — ini penting
diketahui supaya tidak membangun yang sudah ada:

| Kategori | Crate Rust | recent | revdeps | vonis |
|---|---|---|---|---|
| JSON Schema | `jsonschema` | 31,7 jt | 1569 | terisi |
| Cron | `cron` | 9,3 jt | 463 | terisi |
| Phone (libphonenumber) | `phonenumber` | 3,2 jt | 65 | terisi |
| IBAN | `iban` | 121 rb | 2 | **stale** |
| vCard | `calcard` | 32 rb | 10 | terisi |
| RTF | `rtf-parser` | 70 rb | 5 | terisi |
| Hashids | `hashids-rust` | 59 | 0 | terisi tapi tipis |
| Canonical JSON (JCS) | `serde_jcs` | 3,8 jt | 157 | terisi |
| Durasi manusiawi | `humantime` | 70,1 jt | 1359 | terisi |
| User-agent | `uaparser` | 2,9 jt | 7 | terisi |
| Waktu shalat | `salah` | 851 | 4 | **terpecah & tipis** |

## 3 kandidat yang bertahan

### 1. `object-hash` — hashing struktural deterministik  ⭐ terkuat

- **Kegunaan:** memberi sidik jari yang stabil pada sebuah nilai terstruktur
  (objek/dict/struct bersarang), bukan pada string. Dipakai untuk cache key, ETag,
  dedup isi, dan content addressing. Ini masalah yang tidak ada hubungannya dengan
  bahasa: nilai yang sama harus menghasilkan hash yang sama, apa pun urutan field
  dan tipe numeriknya.
- **Bukti permintaan:** npm `object-hash` **40,75 juta/minggu**. Sekitarnya:
  `fast-json-stable-stringify` 96,8 jt, `safe-stable-stringify` 39,1 jt,
  `canonicalize` 3,0 jt.
- **Bukti celah di Rust:** `objecthash` (port cryptosphere) **ada** tapi **mati
  sejak 2017-01, 58 unduhan recent, 0 revdeps**. Nama `object-hash` **bebas** di
  crates.io. Pencarian "content addressable hash", "structural hash", "deep hash"
  tidak menemukan apa pun yang hidup.
- **Yang sudah ada dan bukan celah:** JCS (RFC 8785) **sudah terisi** —
  `serde_jcs` 3,8 jt recent / 157 revdeps, `serde_json_canonicalizer` 9,4 jt / 139.
  Jadi jangan membangun "canonical JSON" saja; itu sudah ada dan sehat. Yang tidak
  ada adalah **hashing nilai arbitrer lintas encoding** (JSON, CBOR, struct), yaitu
  kasus pakai object-hash.
- **Kesulitan:** rendah-sedang. Algoritma intinya: normalisasi nilai → encoding
  kanonik → digest. Bagian sulitnya adalah kesetaraan tipe (1 vs 1.0 vs "1",
  urutan map, -0.0, NaN) dan itu justru bagian yang bisa diuji keras.
- **Sumber acuan:** `object-hash` (npm, MIT), RFC 8785 (JCS), `cryptosphere/rust-objecthash`.
- **Risiko jujur:** nol revdeps untuk incumbent yang mati berarti permintaan Rust
  **belum terbukti**. Ini celah terlebar sekaligus yang paling belum terbukti —
  sama seperti catatan SOAP di riset sebelumnya.

### 2. `adhan` — waktu shalat  ⭐ paling cocok dengan workspace

- **Kegunaan:** menghitung waktu shalat dari koordinat, tanggal, dan mazhab.
  Dipakai aplikasi masjid, jadwal imsakiyah, pengingat waktu.
- **Bukti permintaan:** npm `adhan` 36,1 rb/minggu; `moment-hijri` 14,3 rb.
  PyPI `adhan` aktif.
- **Bukti celah di Rust:** nama `adhan` **bebas**. Ruangnya terpecah dan tipis:
  `salah` 851 recent / 4 revdeps, `adhaan` 137, `praytimes` 42 (2023, mati),
  `prayer-times` 6, `salati` 0 (2022), `praytime-rs` 7. Tidak ada yang dominan.
- **Kenapa cocok:** workspace sudah punya `hijridate` (konversi Hijriah) dan
  `arabic`. Waktu shalat adalah potongan domain yang sama, dan **tidak ada**
  yang mengisi ruang ini dengan baik. Ini pola "kosakata domain yang sepi" yang
  sudah terbukti di `cipansor/packages/shared`.
- **Kesulitan:** rendah. Ini aritmetika astronomi yang sudah ditentukan
  (posisi matahari, sudut fajar 18°/20°, Asr Syafi'i vs Hanafi, koreksi altitude).
  Bukan riset; hasilnya bisa dibandingkan angka-per-angka dengan `adhan` (npm/PyPI).
- **Sumber acuan:** `batoulapps/adhan-js`, `batoulapps/adhan-java`, `praytimes.org`.
- **Catatan:** `hijridate` **juga masih bebas** di crates.io — crate workspace ini
  belum pernah dipublikasikan. Kalau niatnya publikasi, nama-nama itu perlu diamankan.

### 3. `properties` — berkas .properties Java

- **Kegunaan:** membaca `application.properties`, pesan i18n Java, konfigurasi
  Spring. Format sederhana tapi punya aturan aneh: escape Unicode `\uXXXX`,
  line continuation dengan `\`, pemisah `=`/`:`/spasi, komentar `#` dan `!`.
- **Bukti permintaan:** npm `properties` 317 rb/minggu.
- **Bukti celah:** `java-properties` ada (868 rb recent, 38 revdeps) tapi
  **terakhir 2023-05**. Nama `properties` **bebas**.
- **Kesulitan:** rendah sekali. Pemetaan aturan, bukan algoritma.
- **Vonis:** layak tapi **kecil**. Incumbent-nya hidup-enough. Ini kandidat
  "kalau yang lain sudah selesai", bukan yang pertama.

## Yang saya buang, dan alasannya

- **`format-duration`** — nama bebas, npm `pretty-ms` 26,7 jt/minggu. Tapi
  `humantime` (70,1 jt recent, 1359 revdeps) dan `duration-str` (3,7 jt, 65)
  sudah melayani kasus pakai yang sama. Celahnya cuma bentuk API, bukan kemampuan.
- **`cuid2`** — sudah terisi (`cuid2` 321 rb recent, 2026-04).
- **`vcard`, `rtf`, `hashids`** — sudah terisi.
- **Canonical JSON / JCS** — sudah terisi dan sehat (lihat kandidat 1).
- **`iban`** — incumbent stale (121 rb recent, 2 revdeps) dan nama `iban` sudah
  dipakai. `eu-vat` di workspace sudah menyentuh wilayah ini.

## Rekomendasi urutan

1. **`object-hash`** — permintaan terbesar, nama bebas, hanya ada incumbent mati
   dari 2017. Kerjakan lebih dulu; kesulitannya sedang dan hasilnya mudah diuji
   (setiap aturan kesetaraan tipe adalah satu tes).
2. **`adhan`** — paling rendah risiko, paling cocok dengan workspace, dan
   melengkapi `hijridate` + `arabic` jadi satu set domain.
3. **`properties`** — kecil, cepat, kalau dua di atas sudah jalan.

Ketiganya memenuhi syarat workspace: nama bebas, bisa std-only, hasilnya bisa
dibandingkan dengan oracle yang ada (npm `object-hash`, npm `adhan`, Java
`Properties.load`), dan Apache-2.0 bisa dipakai.

## Peringatan yang jujur

Riset sebelumnya sudah mencatat ini dan tetap berlaku: **permintaan npm bukan
permintaan Rust.** Untuk `object-hash`, incumbent Rust yang mati berarti kita
tidak punya bukti siapa pun mencarinya di Rust — hanya bukti bahwa konsepnya
besar di JS. Sebelum berkomitmen, ada baiknya memeriksa: berapa banyak proyek
Rust yang meng-hack ini sendiri (mencari "cache key hash" di kode mereka), dan
apakah `serde_jcs` + `blake3` sudah cukup bagi mereka. Kalau ya, celahnya lebih
sempit daripada yang terlihat.
