# PROGRESS — AeroWorship

Status pengerjaan seluruh item yang diturunkan dari [`docs/PRD.md`](docs/PRD.md).
Dikelola oleh agent `project-lead`. **Jangan diedit manual saat siklus agent sedang berjalan.**

| Field | Nilai |
| --- | --- |
| Dibuat | 2026-08-07 |
| Terakhir diperbarui | 2026-08-08 |
| Sumber requirement | `docs/PRD.md` v1.0 (MVP) |
| Total item | 6 setup + 67 functional requirement + 10 release gate = **83** |
| Selesai | 1 (SETUP-01) |

## Legenda

| Status | Arti |
| --- | --- |
| `todo` | Belum dimulai. |
| `in-progress` | Sudah didelegasikan ke implementer, belum lulus verifikasi. |
| `blocked` | Terhenti. Wajib disertai alasan di kolom Catatan. Dua kegagalan berturut-turut pada satu item otomatis menjadikannya `blocked`. |
| `done` | Acceptance criteria terpenuhi, `tester` lulus, `security-auditor` nol temuan Critical. Tidak ada jalan lain menuju status ini. |

**Prioritas** mengikuti PRD §4: `M` = Must (blocker MVP) · `S` = Should · `C` = Could (kandidat pasca-MVP).

**Kolom Keputusan** menunjuk entri di [`decisions.md`](decisions.md) yang mengikat item tersebut.

Detail tiap item — teks requirement, acceptance criteria, dan user story yang ditrace — ada di `docs/PRD.md`. Cari dengan ID-nya.

---

## Fase 0 — Scaffolding

Belum ada satu baris kode pun di repo ini. Item berikut mendahului seluruh FR.

| ID | Judul | Pri | Status | Diperbarui | Keputusan | Catatan |
| --- | --- | --- | --- | --- | --- | --- |
| SETUP-01 | Inisialisasi shell Tauri 2.x + Cargo workspace `src-tauri/`, sesuai layout PRD §6.13 | M | `done` | 2026-08-08 | [ADR-0001](decisions.md#adr-0001) · [0008](decisions.md#adr-0008) · [0009](decisions.md#adr-0009) · [0010](decisions.md#adr-0010) · [0012](decisions.md#adr-0012) · [0013](decisions.md#adr-0013) · [0014](decisions.md#adr-0014) | 3 siklus. Tester LULUS, auditor 0 Critical. Workspace 2 crate; `panic="unwind"`; capabilities = `core:event:default` saja; `dynamic-acl` mati. **Terbawa ke SETUP-02:** aplikasi belum pernah dijalankan — butuh `dist/`. **Terbuka (Warning, ditunda sadar):** [ADR-0011](decisions.md#adr-0011) `webviewInstallMode`, [ADR-0014](decisions.md#adr-0014) sisa jalur ACL runtime. |
| SETUP-02 | Vite dua entry point: `index.html` (Control Panel, bundle penuh) dan `output.html` (Projector, bundle minimal) | M | `in-progress` | 2026-08-08 | — | Pemisahan bundle adalah syarat NFR-01, bukan optimasi. **Membawa utang SETUP-01:** setelah `dist/` ada, aplikasi wajib dijalankan sekali secara manual oleh pengguna — konfirmasi window terbuka, tidak ada panic startup, dan console bersih dari pelanggaran CSP. Ini satu-satunya verifikasi SETUP-01 yang tidak dapat diselesaikan agent (`tauri dev` membuka GUI). |
| SETUP-03 | Toolchain kualitas: `clippy -D warnings`, `rustfmt`, ESLint, Prettier, `vue-tsc`, Vitest | M | `todo` | — | [ADR-0015](decisions.md#adr-0015) | Sampai item ini selesai, sebagian perintah verifikasi belum tersedia. **Tiga persyaratan tambahan dari audit SETUP-02:** (a) assertion build-time atas `dist/*.html` — tolak `<style`, `style="`, `<script>` ber-isi; ini satu-satunya penjaga `style-src` ([ADR-0015](decisions.md#adr-0015)); (b) aturan ESLint `no-restricted-imports` melarang `src/output/**` mengimpor `src/main/**` — auditor menyatakan bila SETUP-03 selesai tanpa ini, temuannya naik jadi Warning; (c) wajibkan `npm ci`, bukan `npm install`. |
| SETUP-04 | Skema SQLite + migrasi dari PRD Appendix A: WAL, `foreign_keys=ON`, `synchronous=NORMAL`, `cache_size=-8000`, FTS5 | M | `todo` | — | — | Blocking untuk seluruh FR-2xx |
| SETUP-05 | Pembangkitan tipe Rust→TS (`ts-rs`/`specta`) + kerangka `tests/{unit,integration,perf}` | M | `todo` | — | — | NFR-33: kontrak command/event tidak boleh ditulis ganda |
| SETUP-06 | Lengkapi `.gitignore`: `.env*`, `*.db` / `*-wal` / `*-shm` / `*.sqlite*`, `*.aero`, `*.aerotpl`, `*.log` | M | `todo` | — | — | Temuan auditor S5 (siklus 1, masih berdiri). Berhenti jadi Suggestion dan mulai jadi risiko kebocoran data pengguna begitu SETUP-04 membuat database — database dev berisi lirik dan isi ibadah nyata. Jauh lebih murah dikerjakan **sebelum** berkas-berkas itu ada. Kerjakan sebelum SETUP-04. |

---

## M1 — Foundation · FR-1xx Dual-Monitor System

*Exit criteria PRD §9: dua window di dua display; gate G1 mulai diukur dan dilacak sejak titik ini.*

| ID | Judul | Pri | Status | Diperbarui | Keputusan | Catatan |
| --- | --- | --- | --- | --- | --- | --- |
| FR-101 | Enumerasi display saat startup via backend Rust (`list_monitors`) | M | `todo` | — | — | — |
| FR-102 | Auto-placement: Control Panel di display primer, Projector Output fullscreen borderless di display non-primer pertama | M | `todo` | — | — | — |
| FR-103 | Reassign display output manual, persisten per mesin | M | `todo` | — | — | — |
| FR-104 | Deteksi connect/disconnect runtime + event `monitor:changed`; state slide dipertahankan, output dipulihkan otomatis | M | `todo` | — | — | Terkait NFR-08; jalur ini rawan under-tested (PRD R4) |
| FR-105 | Window output tanpa chrome, title bar, scrollbar, context menu, seleksi teks, dan devtools di release build | M | `todo` | — | — | — |
| FR-106 | Fallback satu display: preview berjendela, tanpa fullscreen output | M | `todo` | — | — | — |
| FR-107 | State output `content` / `black` / `logo` / `clear`, masing-masing dapat dialamatkan independen | M | `todo` | — | — | — |
| FR-108 | Penanganan DPI campuran: output render pada resolusi piksel native display output | S | `todo` | — | — | — |
| FR-109 | Output always-on-top hanya di display-nya sendiri, tidak mencuri fokus keyboard | M | `todo` | — | — | — |

---

## M2 — Content core · FR-2xx Library + FR-4xx renderer inti

*Exit criteria PRD §9: gate G3 lulus; sebuah lagu tampil di projector melalui renderer sungguhan.*

### FR-2xx — Library Management

| ID | Judul | Pri | Status | Diperbarui | Keputusan | Catatan |
| --- | --- | --- | --- | --- | --- | --- |
| FR-201 | Pencarian full-text inkremental via SQLite FTS5 (judul, penulis, isi lirik) | M | `todo` | — | — | Target NFR-03: p95 < 150 ms atas 10.000 lagu |
| FR-202 | CRUD lagu lengkap dengan soft-delete dan jendela pemulihan 30 hari | M | `todo` | — | — | — |
| FR-203 | Lagu disimpan sebagai kumpulan **section** berlabel, bukan blob teks tunggal | M | `todo` | — | — | Prasyarat FR-204 |
| FR-204 | **Arrangement** bernama: urutan referensi ke section; arrangement default dibuat otomatis | M | `todo` | — | — | — |
| FR-205 | Parser referensi kitab: nama penuh & singkatan ID/EN, pasal:ayat, rentang, rentang lintas pasal | M | `todo` | — | — | Fungsi murni di Rust, wajib unit-test (PRD §6.1) |
| FR-206 | Gagal parse referensi → jatuh diam-diam ke full-text search atas teks ayat | M | `todo` | — | — | Jalur gagal ini wajib punya test tersendiri |
| FR-207 | Dukungan banyak versi Alkitab; ganti versi tanpa mengubah referensi | S | `todo` | — | — | — |
| FR-208 | Impor versi Alkitab dari format interchange; kirimkan minimal satu versi public-domain | M | `todo` | — | [ADR-0005](decisions.md#adr-0005) | 31.102 ayat < 60 detik + cek integritas jumlah ayat |
| FR-209 | Tag dan filter lintas tipe konten | C | `todo` | — | — | — |
| FR-210 | Registrasi aset media di library dengan dimensi, ukuran, dan content hash | M | `todo` | — | — | BLAKE3 |
| FR-211 | Penanda tipe konten pada hasil pencarian, tidak bergantung warna | S | `todo` | — | — | Aksesibilitas, terkait NFR-35 |

### FR-4xx — Template inti (sisanya di M5)

| ID | Judul | Pri | Status | Diperbarui | Keputusan | Catatan |
| --- | --- | --- | --- | --- | --- | --- |
| FR-401 | Template sebagai JSON deklaratif berversi; tanpa kode yang dapat dieksekusi | M | `todo` | — | — | Input tidak tepercaya (NFR-28); node script ditolak saat load |
| FR-402 | Layer background: warna solid, gradasi linear, atau gambar (cover/contain/stretch/tile) | M | `todo` | — | — | — |
| FR-403 | Layer shape: rect, rounded rect, path SVG arbitrer — fill, opacity, stroke, blur | M | `todo` | — | — | Path SVG divalidasi terhadap allowlist grammar (PRD §6.7) |
| FR-404 | Slot teks terikat role `primary`/`secondary`/`reference`/`attribution` + properti tipografi | M | `todo` | — | — | — |
| FR-405 | Live preview < 200 ms, memakai renderer yang sama persis dengan output | M | `todo` | — | — | Klaim pixel-identity hanya benar bila renderer tunggal (PRD §6.7) |
| FR-406 | Seluruh geometri dalam satuan ternormalisasi 0–1 relatif kanvas | M | `todo` | — | — | Dasar independensi resolusi |
| FR-410 | Sertakan minimal tiga template bawaan yang tidak dapat dihapus | M | `todo` | — | — | — |

---

## M3 — Workflow · FR-3xx Session & Live Presentation

*Exit criteria PRD §9: gate G2 lulus; satu ibadah penuh dapat dijalankan ujung ke ujung.*

| ID | Judul | Pri | Status | Diperbarui | Keputusan | Catatan |
| --- | --- | --- | --- | --- | --- | --- |
| FR-301 | Aplikasi terbuka dengan Session panel kosong + hint inline; tidak ada auto-load | M | `todo` | — | — | — |
| FR-302 | Daftar "Recent sessions" sebagai aksi opt-in eksplisit | S | `todo` | — | — | — |
| FR-303 | Drag-and-drop Library → Session dengan indikator posisi sisip | M | `todo` | — | — | Drop-to-render ≤ 50 ms; duplikat diizinkan |
| FR-304 | Reorder via drag; hapus via `Delete` dengan undo 8 detik | M | `todo` | — | — | — |
| FR-305 | Override template per item, fallback ke default tipe lalu default global | S | `todo` | — | — | — |
| FR-306 | Navigasi slide: next, previous, first, last, jump-to-index — seluruhnya keyboard | M | `todo` | — | — | Tunduk pada NFR-02 |
| FR-307 | Panic control: blank (`B`), logo (`L`), clear (`C`) + indikator status persisten | M | `todo` | — | — | ≤ 100 ms; indikator harus terbaca sekilas di ruang gelap |
| FR-308 | Pemisahan Preparation Mode / Live Mode; Live Mode tanpa afordans penyuntingan sama sekali | M | `todo` | — | — | Diferensiator D3; tidak boleh dilonggarkan |
| FR-309 | Beralih Live → Preparation saat konten live memerlukan konfirmasi eksplisit | M | `todo` | — | — | — |
| FR-310 | Pemecahan slide otomatis pada batas baris, deterministik dan identik di preview dan output | M | `todo` | — | — | Fungsi murni Rust, wajib unit-test |
| FR-311 | Control Panel menampilkan preview slide aktif dan slide berikutnya bersamaan | S | `todo` | — | — | Harus muat di 1366×768 tanpa scroll (NFR-19) |
| FR-312 | Item session dapat berupa lagu, ayat, deck impor, gambar, atau penanda kosong | M | `todo` | — | — | — |
| FR-313 | Navigasi live melintasi batas item tanpa dead-end | M | `todo` | — | — | — |

---

## M4 — Portability · FR-7xx `.aero` Session Files

*Exit criteria PRD §9: session yang dibangun di mesin A tampil di mesin B.*

| ID | Judul | Pri | Status | Diperbarui | Keputusan | Catatan |
| --- | --- | --- | --- | --- | --- | --- |
| FR-701 | Ekspor session ke `.aero`: JSON UTF-8 sesuai Appendix C, dengan `schema_version` | M | `todo` | — | — | Berkas ibadah tipikal < 100 KB |
| FR-702 | Referensi entitas via UUID + field fallback human-readable terdenormalisasi | M | `todo` | — | — | — |
| FR-703 | Impor `.aero`; item yang tak terselesaikan **tidak** menggagalkan pemuatan | M | `todo` | — | — | Jalur gagal, wajib punya test |
| FR-704 | Aksi resolusi eksplisit untuk item tak terselesaikan: cari, buat dari fallback, hapus | M | `todo` | — | — | — |
| FR-705 | Path media relatif + fallback absolut + hash BLAKE3; relink-by-hash saat gagal | M | `todo` | — | — | — |
| FR-706 | Portable export opsional: sematkan teks lirik untuk mesin dengan library kosong | S | `todo` | — | — | 6 lagu tetap < 200 KB |
| FR-707 | Autosave tiap 30 detik dan tiap mutasi; tawarkan pemulihan setelah terminasi abnormal | S | `todo` | — | — | NFR-09 |
| FR-708 | Tolak `schema_version` masa depan dengan pesan spesifik; migrasi versi lama ke depan | M | `todo` | — | — | Tidak boleh muncul sebagai parse error |
| FR-709 | Registrasi asosiasi berkas `.aero` saat instalasi | C | `todo` | — | — | — |

---

## M5 — Import & polish · FR-5xx, FR-4xx sisa, FR-6xx

*Exit criteria PRD §9: seluruh FR prioritas Must selesai.*

### FR-5xx — Presentation Import

Komponen berisiko tertinggi di MVP (PRD R1). Jangan mulai sebelum [ADR-0004](decisions.md#adr-0004) diputuskan.

| ID | Judul | Pri | Status | Diperbarui | Keputusan | Catatan |
| --- | --- | --- | --- | --- | --- | --- |
| FR-501 | Impor `.ppt` / `.pptx` via file picker atau drag-and-drop ke Library | M | `todo` | — | [ADR-0004](decisions.md#adr-0004) | — |
| FR-502 | Konversi berjalan penuh di worker thread Rust, tidak memblokir UI | M | `todo` | — | — | Frame timing Control Panel < 16,7 ms p95 selama konversi |
| FR-503 | Rasterisasi tiap slide ke WebP q90 pada resolusi output, fallback PNG | M | `todo` | — | — | — |
| FR-504 | Event `import:progress` + progress bar determinate + pembatalan | M | `todo` | — | — | Pembatalan tidak boleh meninggalkan baris parsial atau berkas yatim |
| FR-505 | Thumbnail 320 px per slide untuk session grid | M | `todo` | — | — | — |
| FR-506 | Berkas sumber tidak diperlukan setelah impor; catat path, hash, tanggal | M | `todo` | — | — | Properti yang membuat alur flash-drive aman |
| FR-507 | Deteksi deck usang (hash sumber berubah) dan tawarkan impor ulang | C | `todo` | — | — | — |
| FR-508 | Toolchain konversi tidak tersedia → pesan spesifik menyebut dependensi dan solusinya | M | `todo` | — | [ADR-0004](decisions.md#adr-0004) | Tidak boleh berupa kegagalan generik |
| FR-509 | Impor PDF melalui jalur rasterisasi yang sama | S | `todo` | — | [ADR-0004](decisions.md#adr-0004) | Jalur tanpa dependensi eksternal sama sekali |

### FR-4xx — Template Builder (sisa)

| ID | Judul | Pri | Status | Diperbarui | Keputusan | Catatan |
| --- | --- | --- | --- | --- | --- | --- |
| FR-407 | Safe-area guide + indikator overflow saat konten tidak muat pada ukuran font minimum | S | `todo` | — | — | — |
| FR-408 | Resolusi template global → tipe konten → item; read-only di Live Mode | M | `todo` | — | — | — |
| FR-409 | Duplicate, rename, delete, export, import template `.aerotpl` | S | `todo` | — | — | Gambar hilang memicu alur relink |

### FR-6xx — Online Lyric Search

Seluruh blok ini bergantung pada [ADR-0006](decisions.md#adr-0006). Bila provider tidak lolos tinjauan ToS, blok ini ditunda pasca-MVP — kecuali FR-606, yang tetap wajib.

| ID | Judul | Pri | Status | Diperbarui | Keputusan | Catatan |
| --- | --- | --- | --- | --- | --- | --- |
| FR-601 | Pencarian online sebagai tab sekunder terpisah; hasil lokal dan online tidak pernah dicampur | S | `todo` | — | [ADR-0006](decisions.md#adr-0006) | — |
| FR-602 | Deteksi konektivitas via request nyata, lazy, tidak pernah memblokir startup atau pencarian lokal | S | `todo` | — | [ADR-0006](decisions.md#adr-0006) | — |
| FR-603 | Provider di balik trait Rust; timeout 5 detik dan dapat dibatalkan | S | `todo` | — | [ADR-0006](decisions.md#adr-0006) | Provider macet tidak boleh menggantung UI |
| FR-604 | Pratinjau sebelum impor; konfirmasi eksplisit; label section dapat disunting | S | `todo` | — | [ADR-0006](decisions.md#adr-0006) | — |
| FR-605 | Persistensi provenance: source URL, nama provider, timestamp; field CCLI tersedia | S | `todo` | — | [ADR-0006](decisions.md#adr-0006) | Terkait risiko hak cipta PRD R3 |
| FR-606 | Akses jaringan opt-in per query; nol request keluar tanpa inisiasi pengguna | M | `todo` | — | — | **Tetap wajib meski FR-601…605 ditunda.** Diverifikasi dengan packet capture |

---

## M6 — RC & Validation · Release Gates

Dari PRD §7.1. Gate berlabel *blocker* menahan rilis MVP.

| ID | Gate | Target | Blocker | Status | Diperbarui | Catatan |
| --- | --- | --- | --- | --- | --- | --- |
| GATE-G1 | RAM standby seluruh proses | ≤ 100 MB | Ya | `todo` | — | Diukur dan dilacak sejak M1 selesai |
| GATE-G2 | Latensi transisi slide p95 | < 100 ms | Ya | `todo` | — | Divalidasi silang sekali dengan kamera 240 fps |
| GATE-G3 | Latensi pencarian p95, 10k lagu | < 150 ms | Ya | `todo` | — | — |
| GATE-G4 | Cold start (HDD) | < 3 s | Ya | `todo` | — | — |
| GATE-G5 | Ukuran installer | < 15 MB | Ya | `todo` | 2026-08-08 | Baseline biner pasca-SETUP-01: `aeroworship.exe` **5.078.016 byte (4,84 MB)**, stripped, `panic="unwind"`, tanpa `dynamic-acl`. Turun 603.648 byte (−10,6%) dari 5.681.664 — mematikan `dynamic-acl` ([ADR-0013](decisions.md#adr-0013)) justru menguntungkan gate ini. Lihat [ADR-0012](decisions.md#adr-0012). Ini **lantai, bukan hasil**: NFR-16 mengukur installer, dan angkanya butuh `cargo tauri build` yang butuh `dist/` dari SETUP-02. Tegangan dengan [ADR-0004](decisions.md#adr-0004) dan [ADR-0011](decisions.md#adr-0011). |
| GATE-G6 | Request keluar selama ibadah offline | 0 | Ya | `todo` | — | Packet capture |
| GATE-G7 | Fault window output tidak memengaruhi Control Panel | Lulus | Ya | `todo` | — | Fault injection |
| GATE-G8 | Soak 2 jam, pertumbuhan RSS | < 5% | Ya | `todo` | — | — |
| GATE-G9 | CPU idle | < 1% | Tidak | `todo` | — | Menuntut nol animation loop dan nol polling timer saat idle |
| GATE-G10 | Cakupan unit-test core Rust | ≥ 80% | Tidak | `todo` | — | — |

Validasi usabilitas (NFR-22 SUS > 75, NFR-23 UEQ) memerlukan studi dengan n ≥ 12 operator dan tidak dapat diotomatisasi. Lihat PRD §7.3.

---

## Riwayat perubahan status

`project-lead` menambahkan satu baris di sini setiap kali sebuah item berpindah status. Append-only.

| Tanggal | Item | Dari | Ke | Catatan |
| --- | --- | --- | --- | --- |
| 2026-08-07 | — | — | — | PROGRESS.md dibuat dari docs/PRD.md v1.0. Seluruh item `todo`. |
| 2026-08-07 | SETUP-01 | `todo` | `in-progress` | Prasyarat toolchain diverifikasi. Didelegasikan ke implementer. |
| 2026-08-07 | SETUP-01 | `in-progress` | `in-progress` | Siklus 1: implementer → tester (LULUS) → auditor (0 Critical). Tetap `in-progress`, bukan kegagalan. Biaya siklus edit-test terukur: cold 37m12s, warm 13s, inkremental 1m22s pada crate 26 baris. |
| 2026-08-08 | SETUP-01 | `in-progress` | `in-progress` | Siklus 2: workspace 2 crate ([ADR-0008](decisions.md#adr-0008)), `panic="unwind"` ([ADR-0009](decisions.md#adr-0009)), `core:window:default` dihapus. Tester LULUS — isolasi crate core terbukti (target dir kosong → 4,78 s, nol artefak Tauri), `panic="unwind"` terbukti empiris via kontrol A/B, biner release 5,42 MB. Auditor **mencabut W4** (premis keliru, `reqwest` target-gated Android/iOS) dan menemukan Warning baru: feature `dynamic-acl` aktif → [ADR-0013](decisions.md#adr-0013). Angka ADR-0008 dikoreksi di [ADR-0012](decisions.md#adr-0012). |
| 2026-08-08 | SETUP-01 | `in-progress` | **`done`** | Siklus 3: `dynamic-acl` dimatikan ([ADR-0013](decisions.md#adr-0013)). Tester LULUS — daftar feature = default minus tepat satu, `dynamic-acl` terbukti configured out lewat diagnostik rustc, biner turun 603.648 byte (A/B byte-per-byte). Auditor 0 Critical; Warning baru [ADR-0014](decisions.md#adr-0014) (sisa `__allow_command`/`runtime_authority_mut`, tidak dapat ditutup lewat feature flag) ditunda sadar. Syarat penutupan auditor dipenuhi: ADR-0014 ditulis, SETUP-06 ditambahkan. |
| 2026-08-08 | SETUP-02 | `todo` | `in-progress` | Didelegasikan ke implementer. Membawa utang verifikasi runtime SETUP-01. |
