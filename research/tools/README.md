# Tools: riset celah crate Rust

Skrip yang dipakai untuk riset di `../rust-crate-5-final.md`. Semua memakai API
publik tanpa kunci. Butuh Python 3, tanpa dependensi eksternal.

| Skrip | Kegunaan |
|---|---|
| `crates.py` | Pembungkus API crates.io (cache + retry). Bisa dipakai langsung: `python3 crates.py semver glob` |
| `npmfetch.py` | Ambil unduhan mingguan + metadata npm. Batching aman untuk nama ber-scope |
| `demand.py` | Angka permintaan npm + PyPI untuk daftar kandidat terkurasi |
| `pipeline.py` | Tahap `pypi`, `crates-npm`, `crates-pypi` |
| `probe.py` | Probe ~263 kapabilitas terkurasi ke crates.io |
| `revdeps.py` | **Yang paling penting**: reverse dependencies + aktivitas repo incumbent |
| `incumbents.py` | Detail pesaing untuk tiap kandidat |
| `verify2.py` | Verifikasi 30 area tambahan (transit, kesehatan, keuangan, dll) |
| `namecheck.py` | **Cek dulu sebelum menulis kode**: apakah nama crate masih bebas. `python3 namecheck.py iso20022 xmlkit` |

## Peringatan operasional

- **Jangan jalankan dua skrip sekaligus.** crates.io membatasi laju permintaan;
  menjalankannya paralel memicu HTTP 429 dan hasilnya kosong palsu — kelihatan
  seperti celah padahal bukan.
- Endpoint bulk npm menolak campuran nama ber-scope dan tanpa scope (HTTP 400).
  Pisahkan keduanya, maksimal 32 per panggilan.
- `revdeps.py` adalah pengukur permintaan Rust yang sebenarnya. Angka unduhan
  PyPI/npm mengukur permintaan di Python/JS, bukan di Rust.
- **Jalankan `namecheck.py` sebelum menulis kode, bukan sesudah.** Nama crate
  tidak bisa diganti setelah publish. `iso20022` tampak seperti celah yang jelas
  sampai ketahuan namanya sudah dipakai crate yang mati sejak 2020.
