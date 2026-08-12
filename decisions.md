# DECISIONS — AeroWorship

Log keputusan arsitektur. **Append-only.**

Entri lama tidak pernah diedit dan tidak pernah dihapus. Bila sebuah keputusan
berubah, tulis entri baru yang menggantikannya, lalu tambahkan satu baris
`> Digantikan oleh ADR-xxxx` di bawah judul entri lama — itu satu-satunya
perubahan yang boleh menyentuh entri terdahulu.

Yang dicatat di sini: keputusan yang **mengikat pekerjaan berikutnya** dan yang
alasannya akan terlupakan dalam dua bulan. Pilihan penamaan variabel bukan
keputusan arsitektur. Memilih satu dari dua pustaka rasterisasi, iya.

Ditulis oleh `project-lead`. Item di [`PROGRESS.md`](PROGRESS.md) menunjuk ke
entri di sini lewat kolom Keputusan.

## Status yang dipakai

| Status | Arti |
| --- | --- |
| `Diterima` | Berlaku. Pekerjaan boleh bersandar padanya. |
| `Diusulkan` | Rekomendasi sudah ada, persetujuan pengguna belum. Item yang bergantung padanya **tidak boleh dimulai**. |
| `Digantikan` | Tidak berlaku lagi. Ada penunjuk ke entri penggantinya. |
| `Ditolak` | Dipertimbangkan dan tidak diambil. Tetap dicatat agar tidak diusulkan ulang. |

## Format entri

```
### ADR-nnnn — <judul singkat>

| Tanggal | YYYY-MM-DD |
| Status  | Diterima / Diusulkan / Digantikan / Ditolak |
| Terkait | item PROGRESS.md, section PRD, ADR lain |

**Keputusan.** Satu paragraf. Apa yang diputuskan, dalam kalimat aktif.

**Alasan.** Mengapa. Sebutkan batasan yang memaksa keputusan ini — angka
budget, target NFR, atau properti produk yang dipertahankan.

**Alternatif yang ditolak.**
- <alternatif> — mengapa tidak diambil.
- <alternatif> — mengapa tidak diambil.

**Konsekuensi yang diterima.** Apa yang menjadi lebih sulit atau hilang karena
keputusan ini. Bagian ini wajib diisi; keputusan tanpa biaya biasanya berarti
biayanya belum ditemukan.
```

---

### ADR-0001 — Sistem multi-agent orchestrator/worker

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | Diterima |
| Terkait | `.claude/agents/*`, seluruh item PROGRESS.md |

**Keputusan.** Pengerjaan AeroWorship dijalankan lewat satu coordinator
(`project-lead`) dan tiga worker (`implementer`, `tester`, `security-auditor`).
Coordinator tidak menulis kode aplikasi. Sebuah item hanya menjadi `done` bila
tester lulus **dan** security-auditor melaporkan nol temuan Critical. Dua
kegagalan berturut-turut pada satu item menghentikan pekerjaan dan
mengembalikannya ke pengguna.

**Alasan.** PRD berisi 67 functional requirement dengan acceptance criteria
eksplisit dan 36 NFR yang sebagian besar terukur. Beban ini cocok dibagi:
satu pihak menjaga urutan dan verifikasi, satu pihak menulis kode sekecil
mungkin, satu pihak menguji, satu pihak mengaudit tanpa kepentingan pada kode
yang sudah ditulis. Pemisahan tester dari implementer mencegah test disesuaikan
dengan implementasi. `security-auditor` dibuat read-only agar tidak tergoda
memperbaiki temuan tanpa melewati jalur test.

**Alternatif yang ditolak.**
- Satu agent mengerjakan semuanya — auditor yang juga penulis kode tidak pernah
  menemukan cacat pada karyanya sendiri.
- Worker paralel pada beberapa item sekaligus — konflik berkas dan status
  PROGRESS.md yang tidak konsisten lebih mahal daripada waktu yang dihemat.

**Konsekuensi yang diterima.** Setiap item memakan minimal tiga siklus delegasi,
jadi lebih lambat daripada implementasi langsung. Ambang "nol Critical" akan
sesekali menahan item karena temuan yang sebenarnya tidak dapat dieksploitasi.

---

### ADR-0002 — PRD tetap di `docs/PRD.md`

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | Diterima |
| Terkait | seluruh agent, PROGRESS.md |

**Keputusan.** Sumber requirement tunggal adalah `docs/PRD.md`. Tidak ada
`PRD.md` di root, tidak ada ringkasan, tidak ada salinan.

**Alasan.** Dua berkas requirement akan menyimpang. Yang menyimpang selalu
yang tidak dibaca saat implementasi.

**Alternatif yang ditolak.**
- `PRD.md` di root berisi ringkasan + tautan — menambah permukaan yang harus
  dijaga sinkron tanpa manfaat yang jelas.
- Memindahkan berkas ke root — memutus tautan relatif di dalam dokumen dan
  bertentangan dengan layout repo di PRD §6.13 yang sudah menempatkannya di `docs/`.

**Konsekuensi yang diterima.** Setiap agent harus tahu jalurnya; jalur itu
ditulis eksplisit di keempat berkas agent.

---

### ADR-0003 — Granularitas PROGRESS.md pada level FR

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | Diterima |
| Terkait | PROGRESS.md |

**Keputusan.** Item pelacakan adalah functional requirement (FR-101…FR-709),
bukan user story. Ditambah lima item SETUP-xx untuk scaffolding dan sepuluh
GATE-xx untuk release gate PRD §7.1.

**Alasan.** FR adalah unit terkecil yang punya acceptance criteria sendiri di
PRD, sehingga bisa diselesaikan dalam satu siklus implementer → tester →
auditor. User story terlalu besar: US-05 saja mencakup FR-101, FR-102, dan
FR-106.

**Alternatif yang ditolak.**
- Per user story (21 item) — tiap item butuh beberapa siklus delegasi, sehingga
  status `in-progress` menjadi tidak informatif.
- Dua level US + FR — informasi sama, berkas dua kali lebih panjang, dan status
  ganda yang bisa saling bertentangan.

**Konsekuensi yang diterima.** Keterlacakan ke nilai pengguna menjadi tidak
langsung; kolom Story di PRD tetap menjadi jembatannya.

---

### ADR-0004 — Jalur konversi PPTX

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | **Diusulkan** — menunggu keputusan pengguna |
| Terkait | FR-501, FR-508, FR-509, GATE-G5, PRD §8.3 D1, PRD R1 |

**Keputusan yang diusulkan.** Deteksi instalasi LibreOffice yang sudah ada dan
jalankan `soffice --headless --convert-to pdf`, lalu rasterisasi PDF dengan
`pdfium-render` yang dibundel (~7 MB). Impor PDF (FR-509) memakai tahap kedua
yang sama dan tidak memerlukan dependensi eksternal sama sekali.

**Alasan.** Tidak ada pustaka Rust murni yang matang yang me-*render* PPTX
dengan substitusi font, layout, SmartArt, chart, dan efek yang benar. Mem-parse
OOXML tractable; merendernya dengan setia adalah pekerjaan bertahun-tahun.
Membundel converter akan melanggar NFR-16 (installer < 15 MB), yang merupakan
salah satu klaim produk.

**Alternatif yang ditolak.**
- Membundel converter dan melepas NFR-16 — mengorbankan diferensiator yang
  justru menjadi alasan produk ini ada.
- Impor PDF saja untuk MVP — menutup FR-501 yang berprioritas Must.

**Konsekuensi yang diterima.** Pada mesin tanpa LibreOffice, impor PPTX tidak
tersedia; FR-508 mewajibkan pesan spesifik yang menyebut dependensi dan
menawarkan jalur PDF. Dokumentasi harus memimpin dengan "ekspor ke PDF".

> **Belum diputuskan.** Item FR-501, FR-508, dan FR-509 tidak boleh dimulai
> sebelum entri ini berstatus `Diterima`.

---

### ADR-0005 — Versi Alkitab bawaan untuk pasar Indonesia

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | **Diusulkan** — menunggu keputusan pengguna |
| Terkait | FR-208, PRD §8.3 D2 |

**Keputusan yang diusulkan.** MVP mengirimkan versi public-domain saja.
Penjajakan lisensi LAI berjalan paralel sebagai jalur bisnis, bukan sebagai
blocker teknis.

**Alasan.** Ketergantungan rilis pada negosiasi lisensi memindahkan tanggal
rilis ke tangan pihak ketiga.

**Alternatif yang ditolak.**
- Menunggu lisensi LAI sebelum MVP — menahan seluruh rilis pada satu negosiasi.

**Konsekuensi yang diterima.** Versi bawaan mungkin bukan versi yang paling
lazim dipakai jemaat sasaran. FR-208 (impor versi) menjadi jalan keluarnya,
sehingga prioritasnya tidak boleh diturunkan.

> **Diperlukan sebelum beta.**

---

### ADR-0006 — Provider lirik online

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | **Diusulkan** — menunggu tinjauan ToS |
| Terkait | FR-601…FR-605, PRD §8.3 D3, PRD R3 |

**Keputusan yang diusulkan.** FR-601…FR-605 hanya dikirim bila ada provider
yang lolos tinjauan Terms of Service. Bila tidak ada, seluruh blok ditunda
pasca-MVP.

**Alasan.** Mengambil lirik dari sumber yang melarangnya adalah risiko hukum
yang tidak sebanding dengan nilai satu fitur berprioritas Should.

**Alternatif yang ditolak.**
- Mengirim fitur dengan provider apa adanya dan menyelesaikan lisensi belakangan.

**Konsekuensi yang diterima.** Ibu Sari (persona P2) mungkin tetap harus
mengetik lirik secara manual di MVP.

> **FR-606 tetap wajib tanpa syarat.** "Nol request keluar tanpa inisiasi
> pengguna" adalah properti produk (NFR-13, gate G6), bukan bagian dari fitur
> pencarian online. Ia tetap dikerjakan meski FR-601…605 ditunda.

---

### ADR-0007 — Stage/confidence display

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | **Diusulkan** — perencanaan pasca-MVP |
| Terkait | PRD §1.5, §8.3 D4 |

**Keputusan yang diusulkan.** Ditunda ke v1.1, bergantung pada apakah budget
memori masih bertahan dengan webview ketiga.

**Alasan.** Output ketiga menambah satu proses webview. Budget NFR-01 sudah
teralokasi penuh: 22 MB host Rust + 45 MB Control Panel + 28 MB output + 5 MB
headroom.

**Alternatif yang ditolak.**
- Memasukkannya ke MVP — melanggar budget memori yang menjadi klaim utama produk.

**Konsekuensi yang diterima.** Gereja yang membutuhkan layar monitor untuk
pemusik tidak terlayani di MVP.

---

### ADR-0008 — Workspace dua crate: `aeroworship-core` terpisah dari shell Tauri

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | Diterima |
| Terkait | SETUP-01, SETUP-05, GATE-G10, PRD §6.1, PRD §6.13 |

**Keputusan.** `src-tauri/` dipecah menjadi Cargo workspace berisi dua crate:
`aeroworship-core` yang memuat seluruh logika domain (parsing referensi kitab,
slide splitting, serialisasi `.aero`, resolusi path, model data) dan **tidak
bergantung pada `tauri`**, serta crate shell yang memuat command, event, dan
window.

**Alasan.** Diukur oleh tester pada SETUP-01: siklus edit-test memakan
**1 menit 22 detik** untuk perubahan satu baris pada crate berisi 26 baris.
Biaya itu hampir seluruhnya link tiga binary Tauri (`wry`, `tao`,
`webview2-com`, crate `windows`) dan naik seiring bertambahnya kode. Dengan
GATE-G10 menuntut coverage unit test Rust ≥ 80% dan PRD §6.1 menempatkan
seluruh logika correctness-critical di Rust, biaya itu akan menjadi hambatan
nyata sebelum M3. `cargo test -p aeroworship-core` tidak menyentuh graf Tauri
sama sekali.

Manfaat kedua yang sama pentingnya: pemisahan ini menegakkan PRD §6.1 secara
**struktural**, bukan sebagai imbauan. Logika domain menjadi mustahil ditulis
di dalam kode shell, karena crate core tidak punya akses ke Tauri dan crate
shell tidak seharusnya memuat logika.

**Alternatif yang ditolak.**
- Package tunggal — lebih sederhana, tapi membayar biaya link penuh pada setiap
  siklus test dan tidak punya penghalang apa pun terhadap perembesan logika
  domain ke kode shell.
- Menunda ke SETUP-03 — tester memperingatkan bahwa pemisahan ini murah
  sekarang (26 baris untuk dipindah) dan mahal di FR-401.

**Konsekuensi yang diterima.** Layout menyimpang dari pohon PRD §6.13 yang
menggambarkan satu `src-tauri/src/`. PRD tidak menuntut satu crate, jadi ini
perluasan, bukan pelanggaran — tapi harus disebut agar tidak dibaca sebagai
kelalaian. Batas antara core dan shell perlu dijaga di setiap item berikutnya;
`project-lead` memasukkannya ke brief implementer.

---

### ADR-0009 — `panic = "unwind"` di profil release

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | Diterima |
| Terkait | SETUP-01 (temuan auditor W1), NFR-08, NFR-28, GATE-G5, GATE-G7 |

**Keputusan.** Profil release memakai `panic = "unwind"`, bukan `"abort"`.

**Alasan.** Dua release gate bertabrakan di sini dan keduanya berstatus blocker.
`abort` menghemat ukuran biner (GATE-G5, installer < 15 MB) tetapi membuat
`catch_unwind` menjadi no-op, sehingga tidak ada mekanisme apa pun untuk
mengisolasi kegagalan satu window dari window lain. NFR-08 dan GATE-G7 secara
eksplisit menuntut isolasi itu. Skenario yang diangkat auditor konkret: berkas
`.aero` malformed memicu panic di parser saat ibadah berlangsung, dan dengan
`abort` Control Panel serta Projector Output mati bersamaan di tengah tayangan —
persis mode kegagalan yang PRD §1.1 sebut sebagai hal paling terlihat yang bisa
disaksikan jemaat. Biaya ukuran dari `unwind` kecil relatif terhadap anggaran
15 MB.

**Alternatif yang ditolak.**
- `abort` disertai kompensasi (validasi total, `Result` di seluruh jalur,
  zero-unwrap policy yang ditegakkan clippy) — memindahkan properti keselamatan
  dari jaminan struktural menjadi disiplin manual di 67 FR berikutnya.
- Mengukur dulu lalu memutuskan — menunda keputusan yang arahnya sudah jelas
  dari sisi gate blocker.

**Konsekuensi yang diterima.** Biner release sedikit lebih besar; GATE-G5 harus
diukur dengan angka ini sebagai baseline. `unwind` tidak dengan sendirinya
memberi isolasi — ia hanya membuat isolasi mungkin. Mekanisme `catch_unwind`
yang sesungguhnya masih harus dibangun pada item yang mengimplementasikan
NFR-08.

---

### ADR-0010 — Bundle identifier `id.aeroworship.app`

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | Diterima |
| Terkait | SETUP-01, PRD §6.9, NFR-30 |

**Keputusan.** Identifier bundle adalah `id.aeroworship.app`.

**Alasan.** Reverse-DNS berbasis ccTLD Indonesia, sesuai pasar sasaran PRD.
Tidak ada domain terdaftar yang disebut di PRD maupun di tempat lain.

**Alternatif yang ditolak.**
- Domain organisasi nyata — belum ada yang ditetapkan.
- `com.aeroworship.app` — lebih netral secara geografis, tapi tidak
  mencerminkan pasar sasaran yang dinyatakan PRD §2.

**Konsekuensi yang diterima.** Identifier ikut menentukan nama direktori data
aplikasi (PRD §6.9, NFR-30). Mengubahnya setelah FR-2xx menulis database ke
sana berarti migrasi data pengguna. Keputusan ini karenanya dianggap final
sejak sekarang, bukan sementara.

---

### ADR-0011 — `webviewInstallMode` ditunda ke item packaging

| | |
| --- | --- |
| Tanggal | 2026-08-07 |
| Status | **Diusulkan** — ditunda, diputuskan di M6 |
| Terkait | SETUP-01 (temuan auditor W2), NFR-13, NFR-16, GATE-G5, PRD §1.3 D2 |

**Keputusan.** Nilai `bundle.windows.webviewInstallMode` tidak ditetapkan
sekarang. Ia diputuskan pada item packaging di M6, ketika ukuran installer
sebenarnya sudah terukur.

**Alasan.** Ini keputusan yang menuntut angka, dan angkanya belum ada — build
release belum pernah dijalankan karena `dist/` belum eksis. Tidak ada satu pun
item di M1–M5 yang terhalang olehnya.

**Alternatif yang ditolak.**
- `embedBootstrapper` sekarang — kompromi tengah yang diambil tanpa data.
- `offlineInstaller` sekarang — ~127 MB, melanggar telak NFR-16; hanya boleh
  diambil bersamaan dengan keputusan sadar merevisi GATE-G5.

**Konsekuensi yang diterima.** Selama ditunda, default Tauri `downloadBootstrapper`
berlaku: installer mengunduh WebView2 dari server Microsoft secara diam-diam
bila runtime absen. Auditor mengangkat skenarionya — gereja memasang di PC ruang
ibadah tanpa internet, instalasi gagal tanpa penjelasan yang berguna. Ini
bertentangan dengan klaim offline-first (PRD §1.3 D2) dan **tidak boleh ikut
terbawa ke rilis tanpa keputusan eksplisit.** Entri ini adalah pengingatnya.

---

### ADR-0012 — Koreksi bukti ADR-0008 dan baseline GATE-G5

| | |
| --- | --- |
| Tanggal | 2026-08-08 |
| Status | Diterima |
| Terkait | ADR-0008, ADR-0009, GATE-G5, SETUP-01 siklus 2 |

**Keputusan.** ADR-0008 tetap berlaku tanpa perubahan. Entri ini mengoreksi
**angka pendukungnya**, yang terbukti tidak sebanding, dan mencatat baseline
GATE-G5 yang pertama.

**Alasan.** Verifikasi siklus 2 mengukur ulang dengan metodologi konsisten dan
menemukan dua hal. Pertama, angka implementer terlalu **pesimistis**: siklus
edit-test `cargo test -p aeroworship-core` adalah **4,06 detik**, bukan 28 detik.
Kedua, dan lebih penting, baseline 1 m 22 s yang dikutip ADR-0008 **tidak dapat
direproduksi** — pengukuran ulang crate shell hari ini menghasilkan 16,7 detik.
Angka 1 m 22 s karena itu membandingkan dua kondisi yang tidak setara dan tidak
boleh dikutip lagi.

Rasionya bertahan dan bahkan membaik: **4,1×** dengan metodologi konsisten
(16,7 s → 4,06 s). Klaim isolasi terbukti telak lewat eksperimen yang
menentukan: dengan `CARGO_TARGET_DIR` kosong, `cargo test -p aeroworship-core`
selesai dalam **4,78 detik** dan menghasilkan **6 berkas artefak, seluruhnya
`aeroworship_core`** — nol `wry`, nol `tao`, nol `webview2-com`, nol crate
`windows`. Dari target dir kosong, build yang menyentuh graf Tauri tidak
mungkin selesai secepat itu.

**Baseline GATE-G5.** `cargo build --release` berhasil tanpa `dist/`:
`aeroworship.exe` = **5.681.664 byte (5,42 MB)**, stripped, `opt-level = "s"`,
`lto = true`, `panic = "unwind"`. Anggaran NFR-16 adalah 15 MB **untuk
installer**, bukan biner, jadi angka ini adalah lantai, bukan hasil. Ia tetap
berguna: ia membuktikan `panic = "unwind"` (ADR-0009) tidak membuat biner
membengkak, dan memberi titik banding untuk setiap dependency yang ditambahkan
sesudah ini.

**Alternatif yang ditolak.**
- Menyunting ADR-0008 di tempat — melanggar sifat append-only yang ditetapkan
  di kepala berkas ini. Keputusannya tidak berubah; hanya buktinya yang
  dikoreksi, dan koreksi itu harus punya tanggal sendiri.

**Konsekuensi yang diterima.** Pembaca ADR-0008 harus membaca entri ini juga
untuk mendapat angka yang benar. Itu biaya yang melekat pada log append-only,
dan lebih murah daripada riwayat yang bisa ditulis ulang.

---

### ADR-0013 — Matikan feature `dynamic-acl`

| | |
| --- | --- |
| Tanggal | 2026-08-08 |
| Status | Diterima |
| Terkait | SETUP-01 siklus 3, NFR-14, GATE-G5 |

**Keputusan.** Crate shell berhenti memakai default feature `tauri` dan
menyatakan daftar feature secara eksplisit, **tanpa `dynamic-acl`**.
`compression` tetap dipertahankan.

**Alasan.** `features = []` tidak mematikan default, sehingga `dynamic-acl`
aktif. Dengan feature itu, `Manager::add_capability()` tersedia dan menerima
apa pun yang `AsRef<str>` untuk diparse menjadi capability **saat runtime**.
Permission dapat ditambahkan tanpa pernah muncul di `capabilities/*.json`
maupun di `gen/schemas/capabilities.json`.

Itu melumpuhkan metode verifikasi NFR-14, yang berbunyi "Capability manifest
reviewed at each release". Review manifest akan melaporkan permukaan minimal
sementara biner berjalan dengan permukaan yang lebih luas — dan tidak ada
artefak statis yang menunjukkan selisihnya. Tidak ada jalur eksploitasi hari
ini (nol command, nol frontend), tetapi biayanya naik pada setiap item yang
menambah command.

`compression` dipertahankan karena menjatuhkannya membesarkan biner dan
menekan GATE-G5. Ini disebut eksplisit karena eksperimen feature sebelumnya
memakai `["wry", "common-controls-v6"]` tanpa `compression`, sehingga angkanya
tidak dapat dipakai ulang apa adanya.

**Alternatif yang ditolak.**
- Membiarkan `dynamic-acl` dan mengandalkan disiplin "jangan panggil
  `add_capability`" — menggantikan properti yang dapat diverifikasi dengan
  konvensi yang tidak dapat diverifikasi, pada requirement berstatus Must.
- Menunda ke item yang pertama menambah command — justru saat itulah
  penyalahgunaannya menjadi menggoda.

**Konsekuensi yang diterima.** Daftar feature eksplisit harus dipelihara: bila
Tauri menambah feature default yang dibutuhkan di versi mendatang, ia tidak
akan ikut secara otomatis dan kegagalannya muncul sebagai error kompilasi.
Itu trade-off yang disengaja — kejutan saat kompilasi lebih baik daripada
permukaan permission yang membesar diam-diam.

---

### ADR-0014 — Sisa jalur ACL runtime yang tidak tertutup ADR-0013

| | |
| --- | --- |
| Tanggal | 2026-08-08 |
| Status | Diterima |
| Terkait | ADR-0013, NFR-14, SETUP-01 siklus 3 |

**Keputusan.** Mematikan `dynamic-acl` (ADR-0013) **tidak** menutup seluruh
jalur penambahan capability saat runtime. Sisa berikut diterima sebagai risiko
laten, dan ditangani lewat checklist review, bukan lewat konfigurasi build.

**Sisa yang dimaksud.** Di `tauri` 2.11.5:

| Item | Lokasi | Status gate |
| --- | --- | --- |
| `RuntimeAuthority::__allow_command()` | `src/ipc/authority.rs:136-146` | Hanya `#[doc(hidden)]` — **tidak** di balik `cfg(feature = "dynamic-acl")` |
| `Context::runtime_authority_mut()` | `src/lib.rs:478-482` | Ungated, `pub`, mengembalikan `&mut RuntimeAuthority` |

Keduanya `pub`, sehingga kode di crate shell dapat menyisipkan entri ke
`allowed_commands` dengan `windows: vec!["*"]` tanpa meninggalkan jejak di
`capabilities/*.json` maupun di `gen/schemas/capabilities.json`.

**Alasan menerimanya.** Tidak ada konfigurasi build yang dapat menghilangkannya
— berbeda dari `dynamic-acl`, ini bukan feature. Tidak ada pemanggilan di repo
hari ini, dan jalur ini tidak dapat dijangkau dari frontend maupun dari berkas
masukan; ia hanya dapat dipakai oleh kode yang kita tulis sendiri. Menahan
pekerjaan karenanya tidak akan mengubah apa pun.

**Alternatif yang ditolak.**
- Menambal atau mem-fork `tauri` — biaya pemeliharaan yang tidak sebanding
  dengan risiko laten yang tidak dapat dijangkau penyerang.
- Membiarkannya tidak tercatat — akan membuat ADR-0013 terbaca sebagai
  penutupan yang lebih luas daripada yang benar-benar dicapai, dan review
  NFR-14 berikutnya tidak akan tahu harus mencari apa.

**Konsekuensi yang diterima.** Metode verifikasi NFR-14 ("Capability manifest
reviewed at each release") tidak lagi cukup dengan membaca manifest saja.
Ia harus disertai dua langkah:

1. `grep` atas `__allow_command` dan `runtime_authority_mut` di seluruh
   `src-tauri/`. Kemunculan apa pun wajib dibenarkan atau dihapus.
2. Regenerasi `gen/schemas/capabilities.json` lalu diff terhadap
   `capabilities/*.json`. Berkas itu ter-gitignore (`/src-tauri/gen/`), jadi
   ia bukan artefak versioned dan tidak boleh dibaca apa adanya dari repo.

Langkah ini menjadi bagian dari checklist rilis, dan `project-lead`
memasukkannya ke brief auditor pada setiap item yang menambah command Tauri.

---

### ADR-0015 — `style-src 'self'`; `devCsp` sengaja TIDAK disetel

| | |
| --- | --- |
| Tanggal | 2026-08-08 |
| Status | Diterima |
| Terkait | SETUP-02, NFR-14, NFR-28, FR-401…FR-406 |

**Keputusan.** CSP produksi menurunkan `style-src` dari `'self' 'unsafe-inline'`
menjadi `'self'`. `app.security.devCsp` **tidak** disetel, dan tidak boleh
disetel di kemudian hari dengan harapan ia menegakkan CSP di mode dev.

**Alasan.** Diverifikasi implementer lalu dikonfirmasi ulang auditor dari
sumber, tiga premis:

1. Build produksi tidak membutuhkan `'unsafe-inline'`. Vite mengekstrak seluruh
   CSS SFC ke berkas yang dirujuk `<link rel="stylesheet">`; `dist/index.html`
   dan `dist/output.html` tidak memuat blok `<style>` maupun atribut `style=""`.
2. CSP tidak ditegakkan sama sekali saat `tauri dev`. Header hanya dipasang di
   `tauri-2.11.5/src/protocol/tauri.rs:182` untuk aset yang dilayani lewat
   `tauri://localhost`; dengan `build.devUrl` terisi, `get_app_url()`
   (`manager/mod.rs:353`) mengembalikan URL Vite dan dokumen tidak pernah
   melewati handler itu. `set_csp` (`manager/mod.rs:53`) juga tidak menyisipkan
   `<meta http-equiv>` apa pun — tidak ada jalur kedua.
3. **`devCsp` tidak memperbaiki (2).** Nilainya dikonsumsi di dalam
   `get_asset()` — jalur yang persis dilewati ketika `devUrl` terisi.
   Menyetelnya menghasilkan nol perubahan perilaku. Deskripsi skema resminya
   ("will be injected on all HTML files on development") menyesatkan untuk
   konfigurasi berbasis `devUrl`.

**Alternatif yang ditolak.**
- Mempertahankan `'unsafe-inline'` demi HMR — HMR tidak pernah tunduk pada CSP
  ini sejak awal, jadi alasannya tidak pernah ada.
- Menyetel `devCsp` — biayanya bukan nol melainkan negatif: ia menciptakan
  kepercayaan yang salah tempat pada review berikutnya.

**Konsekuensi yang diterima.**

Pelanggaran `style-src` baru **hanya terlihat di build produksi**, yaitu paling
lambat. `server.headers` di `vite.config.ts` dapat menangkap regresi
`script-src`/`img-src`/`font-src`/`frame-src`/`object-src` lebih awal, tetapi
**tidak** `style-src` — klien Vite menyuntik `<style>` untuk hot-update CSS.
Karena itu penjaga `style-src` adalah **assertion build-time atas `dist/*.html`**
(tolak bila ditemukan `<style`, `style="`, atau `<script>` ber-isi), yang
menjadi persyaratan SETUP-03. Tidak ada penyelamat otomatis: `csp_hashes.styles`
dideklarasikan dan dikonsumsi runtime Tauri tetapi **tidak pernah diisi** —
`tauri-codegen-2.6.3/src/embedded_assets.rs:171` hanya menghitung hash untuk
`.js`/`.mjs`.

Renderer FR-4xx terikat oleh keputusan ini. **Boleh** dipakai, karena tidak
diatur `style-src`: binding `:style` Vue (menulis lewat CSSOM),
`style.setProperty()`, `insertRule()`, `CSSStyleSheet` konstruktabel +
`adoptedStyleSheets`, dan atribut presentasi SVG (`fill`, `stroke`).
**Akan diblokir:** `setAttribute('style', …)` langsung, `<style>` yang dibuat
runtime, `<style>` di dalam markup SVG inline, CSS-in-JS runtime, dan `v-html`
berisi atribut style. Geometri ternormalisasi→piksel FR-406 aman.

Dua batasan yang akan tampil sebagai "bug renderer" bila tidak dicatat sekarang:
`font-src 'self'` akan menolak font kustom pengguna dari disk, dan
`img-src 'self' data:` akan menolak gambar background FR-402 dari disk. Keduanya
menuntut `assetProtocol` beserta sumber `http://asset.localhost`, yaitu
perubahan CSP tersendiri yang wajib melewati auditor.

---

### ADR-0016 — Guard `custom-protocol`; koreksi baseline GATE-G5

| | |
| --- | --- |
| Tanggal | 2026-08-08 |
| Status | Diterima |
| Terkait | SETUP-02 (temuan auditor W1), ADR-0012, NFR-13, NFR-14, GATE-G5, GATE-G6 |

**Keputusan.** Crate shell menolak kompilasi pada kombinasi
`not(debug_assertions)` + `not(feature = "custom-protocol")`. Angka GATE-G5
hanya sah bila diukur dari artefak keluaran `npm run tauri build`.

**Alasan.** `custom-protocol` bukan feature default `tauri`, dan hanya
ditambahkan oleh `@tauri-apps/cli`. `tauri-2.11.5/build.rs:256` menetapkan
`let dev = !has_feature("custom-protocol")`, sehingga `cargo build --release`
polos menghasilkan biner dengan `cfg(dev)` aktif. Biner itu tidak meng-embed
aset dan menavigasi webview ke `http://localhost:1420`
(`manager/webview.rs:443`) — memuat dokumen dari soket lokal yang dapat
diduduki proses mana pun, ke dalam origin yang membawa bridge IPC dan grant
kapabilitas aplikasi, **tanpa header CSP sama sekali** (alasan yang sama seperti
ADR-0015 premis 2).

Yang membuatnya berbahaya bukan tingkat keparahannya melainkan cara ia lolos:
di mesin developer Vite biasanya sudah berjalan, sehingga biner ber-flavour dev
**berfungsi sempurna**, dan cacatnya hanya muncul di mesin pengguna. Repo ini
memang sudah rutin membangun biner release lewat Cargo polos untuk pengukuran
A/B GATE-G5, jadi ini praktik yang sedang berjalan, bukan risiko hipotetis.

**Koreksi terhadap ADR-0012.** Baseline **5.078.016 byte** yang tercatat di sana
adalah biner ber-flavour dev — ia tidak meng-embed `dist/` dan karenanya bukan
pembanding yang setara. Angka yang benar, dari `npm run tauri build`:

| Artefak | Byte |
| --- | --- |
| `aeroworship.exe` (rilis sungguhan) | 5.132.800 |
| Installer NSIS | 1.394.695 |

Angka installer diukur `project-lead`; implementer melaporkan 1.393.858 pada
artefak yang sama. Selisih 837 byte diduga nondeterminisme NSIS dan tidak
mengubah kesimpulan: **9,3% dari anggaran NFR-16 (< 15 MB)**. Sesuai definisi
NFR-16, angka ini tidak memuat bootstrapper WebView2 — lihat ADR-0011.

**Alternatif yang ditolak.**
- Menambahkan `custom-protocol` ke daftar feature tetap di `Cargo.toml` —
  akan mematikan alur `tauri dev` yang justru membutuhkan flavour dev.
- Hanya mendokumentasikan "jangan pakai `cargo build --release`" — konvensi
  tanpa penegakan, pada cacat yang gejalanya tidak terlihat di mesin developer.

**Konsekuensi yang diterima.** `cargo build --release` akan gagal dengan pesan
eksplisit. Itu yang diinginkan, tetapi berarti siapa pun yang terbiasa dengan
perintah itu akan menemuinya sebagai kejutan — pesannya harus menyebut
`npm run tauri build` sebagai gantinya. `cargo check`, `cargo clippy`, dan
`cargo test` tidak terpengaruh karena berjalan di profil dev.

---

### ADR-0017 — Koreksi mekanisme guard ADR-0016

| | |
| --- | --- |
| Tanggal | 2026-08-08 |
| Status | Diterima |
| Terkait | ADR-0016, SETUP-02 siklus 2 |

**Keputusan.** Guard ADR-0016 tetap berlaku, tetapi **mekanismenya berbeda dari
ejaan literal yang tertulis di sana**. Ejaan itu keliru dan akan menghasilkan
kebalikan dari yang dimaksud.

**Apa yang salah.** ADR-0016 menulis kondisi `not(feature = "custom-protocol")`
seolah-olah feature itu hidup di crate `aeroworship`. Ia tidak. Tauri CLI
mengaktifkannya sebagai **`tauri/custom-protocol`** — feature milik crate
`tauri`. Diverifikasi implementer dari fingerprint build rilis yang berhasil:
`target/release/.fingerprint/aeroworship-*/bin-aeroworship.json` mencatat
`features []`, sementara `tauri-*/lib-tauri.json` memuat `custom-protocol`.

Konsekuensinya: `cfg(feature = "custom-protocol")` di crate kita **selalu**
false, sehingga guard dengan ejaan literal ADR-0016 akan menolak kompilasi
**juga** pada `npm run tauri build` — mematikan satu-satunya jalur build yang
benar.

**Mekanisme yang dipakai.** `tauri::is_dev()`, sebuah `pub const fn` di
`tauri-2.11.5/src/lib.rs:308` yang isinya persis `!cfg!(feature =
"custom-protocol")` tetapi dievaluasi **di dalam crate `tauri`**, tempat feature
itu benar-benar hidup. Kondisinya identik dengan maksud ADR-0016; sumber
evaluasinya yang berbeda. Bentuknya `const _: () = assert!(!tauri::is_dev(), …)`
di bawah `#[cfg(not(debug_assertions))]`, sehingga tidak perlu menambah blok
`[features]` ke `Cargo.toml`.

**Alternatif yang ditolak.**
- Mendeklarasikan `custom-protocol = ["tauri/custom-protocol"]` di `Cargo.toml`
  lalu berharap CLI memilih feature milik kita — bergantung pada heuristik CLI
  yang tidak terdokumentasi.
- `build.rs` membaca `DEP_TAURI_DEV` (tersedia: `tauri` punya `links = "Tauri"`
  dan mencetak `cargo:dev={dev}`) lalu memanggil `compile_error!`. Ini
  menghasilkan pesan yang lebih rapi — `error: <pesan>` alih-alih `E0080
  evaluation panicked` — dengan biaya satu cfg kustom dan plumbing build script.
  Ditolak karena keuntungannya kosmetik: pesan lengkapnya tetap tercetak utuh
  di baris pertama error. Jalur ini sudah terverifikasi tersedia bila suatu saat
  bentuk `compile_error!` dianggap sepadan.

**Konsekuensi yang diterima.** Guard bersandar pada `tauri::is_dev()`, yang
merupakan API publik tetapi bukan kontrak stabilitas yang dijanjikan lintas
mayor. Setiap bump mayor `tauri` harus memverifikasi fungsi itu masih ada dan
masih bermakna sama — masuk ke checklist upgrade yang sudah dibuka ADR-0013.

**Koreksi angka.** Installer NSIS terukur tiga kali pada isi yang sama dan
menghasilkan tiga angka: 1.393.858 · 1.394.695 · 1.394.292. Selisihnya di bawah
900 byte dan mengonfirmasi nondeterminisme NSIS. `aeroworship.exe` cocok
**byte-for-byte** di dua pengukuran independen: 5.132.800. Untuk GATE-G5,
gunakan angka installer sebagai kisaran ~1,39 MB, bukan sebagai nilai tunggal.

---

### ADR-0018 — Guard pindah ke `cfg(dev)`; dua koreksi dari audit SETUP-02

| | |
| --- | --- |
| Tanggal | 2026-08-08 |
| Status | Diterima |
| Terkait | ADR-0016, ADR-0017, SETUP-03, NFR-14 |

**Keputusan.** Mekanisme guard berpindah dari `tauri::is_dev()` ke
`#[cfg(all(not(debug_assertions), dev))]` + `compile_error!`. Perpindahannya
dikerjakan bersama SETUP-03, bukan sebagai siklus tersendiri — guard yang
sekarang bekerja dan terverifikasi di kedua arah.

**Koreksi pertama — penilaian biaya di ADR-0017 keliru.** ADR-0017 menolak
alternatif `cfg(dev)` dengan alasan "biaya satu cfg kustom dan plumbing build
script". Biaya itu **tidak ada**. `tauri_build::build()` — yang sudah dipanggil
di `src-tauri/build.rs:2` — memanggil `cfg_alias("dev", is_dev())` di
`tauri-build-2.6.3/src/lib.rs:519`, yang mencetak `cargo:rustc-check-cfg=cfg(dev)`
dan, bila dev, `cargo:rustc-cfg=dev`. Tidak ada build script tambahan dan tidak
ada `DEP_TAURI_DEV` yang perlu dibaca sendiri.

Lebih jauh, bentuk itu adalah **rekomendasi resmi Tauri**. Changelog CLI
(`node_modules/@tauri-apps/cli/CHANGELOG.md:1184`) berbunyi: *"To check if
running on production, use `#[cfg(not(dev))]` instead of `#[cfg(feature =
"custom-protocol")]`"*, dan menyatakan feature `custom-protocol` "is no longer
required on your application and is now ignored" untuk crate aplikasi.

Konsekuensinya, risiko upgrade yang saya catat di ADR-0017 — ketergantungan
pada `tauri::is_dev()` yang bukan kontrak stabilitas lintas mayor — **tidak
perlu ditanggung sama sekali.** Perpindahan ke `cfg(dev)` menghapusnya, dan
sekaligus mengubah pesan kegagalan dari `E0080 evaluation panicked` menjadi
`error: <pesan>` yang wajar.

**Koreksi kedua — nilai penghapusan `@tauri-apps/api` salah kalau dibaca
sebagai pengetatan IPC.** `tauri-2.11.5/src/manager/webview.rs:172-185`
menyuntikkan `window.__TAURI_INTERNALS__` **tanpa syarat**, terlepas dari
`withGlobalTauri: false`. Frontend tetap dapat memanggil `core:event:default`
lewat internals itu tanpa paket npm apa pun. Yang benar-benar berkurang dari
penghapusan tersebut adalah permukaan supply-chain dan ukuran bundle — bukan
batas keamanan.

Ini perlu tercatat karena `withGlobalTauri: false` mudah dibaca sebagai
pertahanan yang lebih kuat daripada kenyataannya. Satu-satunya batas yang nyata
adalah manifest kapabilitas, dan itu berarti setiap permission yang ditambahkan
sejak sekarang harus diasumsikan dapat dipanggil oleh kode frontend mana pun
yang berhasil dieksekusi di webview — termasuk kode yang masuk lewat XSS dari
lirik atau template.

**Alternatif yang ditolak.**
- Menjalankan siklus tersendiri untuk SETUP-02 hanya demi perpindahan ini —
  guard yang ada sudah menutup W1 dan terverifikasi; auditor menyatakan
  eksplisit ia tidak menahan penutupan.
- Membiarkan ADR-0017 berdiri tanpa koreksi — reviewer berikutnya akan
  mempercayai penilaian biaya yang keliru dan menanggung risiko upgrade yang
  tidak perlu.

**Konsekuensi yang diterima.** Sampai SETUP-03 mendarat, guard tetap bersandar
pada `tauri::is_dev()` dan risiko upgrade di ADR-0017 masih berlaku dalam
jendela itu. Ditambah temuan S9 auditor: kondisi guard memakai `debug_assertions`
sebagai proksi profil rilis, dan proksi itu dapat dimatikan senyap dengan
menyetel `debug-assertions = true` di `[profile.release]` atau lewat `RUSTFLAGS`.
Penanda silang di `[profile.release]` ikut dikerjakan bersama perpindahan ini.

---

### ADR-0019 — Guard menolak `cargo test --release`; harness perf SETUP-05 terdampak

| | |
| --- | --- |
| Tanggal | 2026-08-08 |
| Status | Diterima |
| Terkait | ADR-0016, ADR-0017, ADR-0018, SETUP-05, GATE-G1…G9 |

**Keputusan.** Guard `custom-protocol` menolak **seluruh** profil rilis, bukan
hanya build biner. Konsekuensinya diterima, dan `tests/perf/` di SETUP-05 wajib
dirancang mengelilinginya.

**Temuan.** Kondisi guard adalah `#[cfg(not(debug_assertions))]`, yang lebih
luas daripada "biner rilis". Diverifikasi tester:

```
cargo test --release --workspace --no-run  →  EXIT 101
  error: could not compile `aeroworship` (lib test)
```

`cargo test --release` dan `cargo bench` ikut tertolak, dan pesan errornya
menyarankan `npm run tauri build` — saran yang salah untuk sebuah test run.

**Mengapa ADR-0018 tidak menyelesaikannya.** Perpindahan ke
`cfg(all(not(debug_assertions), dev))` **tidak** memperbaiki arah ini:
`cargo test --release` juga tidak membawa `custom-protocol`, sehingga `dev`
tetap aktif dan guard tetap menyala. Kedua bentuk berperilaku sama di sini.

**Mengapa ini penting sekarang.** PRD §6.13 menempatkan harness NFR-01…NFR-07
di `tests/perf/`, dan pengukuran performa secara alami dijalankan di profil
rilis — itu justru gunanya. GATE-G1 sampai G9 bersandar padanya.

**Jalan keluar, keduanya terverifikasi tester.**

| Kebutuhan | Perintah |
| --- | --- |
| Harness yang menyentuh crate shell | `cargo test --release -p aeroworship --features tauri/custom-protocol --no-run` → exit 0 |
| Harness yang hanya menyentuh crate domain | `cargo build --release -p aeroworship-core` → exit 0, tidak terpengaruh sama sekali |

Baris kedua adalah keuntungan tak terduga dari ADR-0008: karena logika domain
hidup di crate yang tidak menyentuh `tauri`, sebagian besar harness performa
tidak akan pernah bersinggungan dengan guard ini.

**Alternatif yang ditolak.**
- Mempersempit guard agar mengizinkan target test — akan membuka kembali jalur
  yang persis dijaga ADR-0016, karena biner test rilis juga memuat `main`.
- Melepas guard dan mengandalkan dokumentasi — sudah ditolak di ADR-0016.

**Konsekuensi yang diterima.** SETUP-05 harus menetapkan perintah test rilis
yang benar di satu tempat (skrip npm atau alias cargo), bukan membiarkan tiap
orang menemukannya sendiri lewat pesan error yang menyesatkan. Pesan guard
sebaiknya ikut menyebut jalur `--features tauri/custom-protocol` untuk kasus
test — saat ini ia hanya menyebut `npm run tauri build`.

---

### ADR-0020 — Perintah gate Rust wajib `--all` / `--workspace`

| | |
| --- | --- |
| Tanggal | 2026-08-08 |
| Status | Diterima |
| Terkait | SETUP-03, ADR-0008, SETUP-05, NFR-32, GATE-G10 |

**Keputusan.** Dua dari enam perintah verifikasi standar berubah bentuk secara
permanen:

| Lapisan | Sebelum | **Sesudah** |
| --- | --- | --- |
| Rust format | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | `cargo fmt --all --manifest-path src-tauri/Cargo.toml -- --check` |
| Rust test | `cargo test --manifest-path src-tauri/Cargo.toml` | `cargo test --workspace --manifest-path src-tauri/Cargo.toml` |

`cargo clippy` tidak berubah. Bentuk lama tidak boleh dikutip lagi di brief,
di README, maupun di definisi agent.

**Alasan.** `--manifest-path` menamai **paket** `aeroworship`, bukan workspace,
sehingga `cargo fmt` dan `cargo test` memilih paket itu saja dan melewati
`aeroworship-core` sepenuhnya. Diverifikasi `project-lead` secara non-destruktif
lewat `cargo fmt -v`, yang mencetak daftar berkas yang benar-benar diserahkan ke
rustfmt:

```
tanpa --all : build.rs · src/lib.rs · src/main.rs
dengan --all: build.rs · crates/core/src/lib.rs · src/lib.rs · src/main.rs
```

Implementer mengonfirmasi arah yang sama secara destruktif: dengan kode
rusak-format dan sebuah `#[test]` yang pasti gagal disuntikkan ke
`crates/core/src/lib.rs`, kedua bentuk lama tetap **exit 0** — lolos palsu.

Ini bukan ketidaknyamanan, ini pembatalan diam-diam atas ADR-0008. Seluruh
logika correctness-critical PRD §6.1 — parser referensi kitab, slide splitting,
serialisasi `.aero`, resolusi path — hidup di `aeroworship-core`. NFR-32 dan
GATE-G10 menuntut coverage ≥ 80% **di crate itu**. Bentuk lama berarti perintah
gate akan melaporkan hijau sementara nol test core pernah dijalankan, dan
kegagalan itu tidak menghasilkan gejala apa pun sampai seseorang kebetulan
menjalankan cargo dari dalam `src-tauri/`. Biayanya nol hari ini karena core
belum punya test; ia menjadi tak terbatas pada item pertama yang menambahkannya.

`cargo clippy` selamat karena mekanismenya berbeda: ia melewatkan seluruh
workspace member lewat `RUSTC_WORKSPACE_WRAPPER`, dan `-D warnings` sampai ke
sana — terbukti, warning yang disuntikkan ke core menghasilkan
`error: could not compile aeroworship-core`. Kesamaan bentuk ketiga perintah itu
justru yang membuat cacatnya sulit terlihat.

**Alternatif yang ditolak.**
- Alias cargo di `.cargo/config.toml` (`fmt = "fmt --all"`) agar bentuk lama
  tetap benar — dicoba implementer, ditolak cargo:
  `error: alias fmt has unresolvable recursive definition: fmt -> fmt`.
- `[workspace] default-members` di `src-tauri/Cargo.toml` — memindahkan
  pemilihan paket ke berkas yang jarang dibaca, sehingga perintah yang sama
  bermakna berbeda tergantung isi manifest. Juga tidak pasti memengaruhi
  `cargo-fmt`, yang punya jalur pemilihan paketnya sendiri.
- Membiarkannya sebagai "Known gap" di README — persis bentuk konvensi tanpa
  penegakan yang sudah ditolak dua kali di ADR-0013 dan ADR-0016.

**Konsekuensi yang diterima.** Dua perintah menjadi lebih panjang dan tidak lagi
seragam dengan `clippy`, yang tidak memakai `--all`. Ketidakseragaman itu akan
tampak seperti kelalaian dan berpotensi "dirapikan" oleh pembaca berikutnya —
entri ini dan komentar di README adalah penjaganya. Lebih penting: bentuk lama
sudah tercetak di `.claude/agents/*.md` dan di riwayat PROGRESS.md. Riwayat
bersifat append-only dan dibiarkan apa adanya; definisi agent perlu disesuaikan
oleh pengguna, karena berkas itu di luar wewenang tulis `project-lead`.

### ADR-0021 — Batas bundle dicocokkan atas string specifier, bukan path yang diresolve

| | |
| --- | --- |
| Tanggal | 2026-08-08 |
| Status | Diterima |
| Terkait | SETUP-03, ADR-0020, PRD §6.3, PRD §5.1 |

**Keputusan.** Kedua guard batas bundle di `eslint.config.js` — `MAIN_IMPORT_GROUPS`
dan `DYNAMIC_MAIN_IMPORT_SELECTORS` — tetap mencocokkan **teks specifier**, bukan
path yang sudah diresolve. Konsekuensinya diterima secara sadar: specifier yang
memuat segmen `main` berdiri sendiri ikut terjaring meski tidak menunjuk
`src/main/`, mis. `./main`, `./widgets/main/x`, `somelib/main`. Yang diperbaiki
bukan aturannya, melainkan **klaim di sekitarnya** — JSDoc yang berbunyi "import
specifiers that resolve into `src/main/`" diganti dengan kalimat yang benar, dan
pesan pelanggarannya menyebutkan kemungkinan false positive beserta jalan
keluarnya.

**Alasan.** Tester siklus 2 melaporkan enam bentuk terjaring padahal tak satupun
menyentuh `src/main/`. Dampak nyatanya hari ini nol — tidak ada berkas atau
direktori bernama `main` di bawah `src/output/` maupun `src/shared/`, dan tidak
ada dependency yang diimpor lewat subpath `main`. Yang membuatnya tetap dicatat
sebagai kegagalan adalah komentarnya, bukan perilakunya: komentar mengklaim
presisi yang tidak dimiliki kodenya, yaitu persis kelas kegagalan yang seluruh
siklus 2 dibuka untuk menutupnya (ADR-0020).

Untuk impor statik, keluasan ini **pra-ada** — `MAIN_IMPORT_GROUPS` identik dengan
daftar siklus 1 yang sudah diluluskan; refactor tidak mengubahnya. Yang baru di
siklus 2 hanyalah perambatannya ke jalur dinamis, lewat cabang `$` pada
`(^|/)main(/|$)` yang ditambahkan untuk menutup `import('../main')`. Menutup
celah itu tanpa membawa keluasannya berarti memperlakukan bentuk statik dan
dinamis secara berbeda — dua aturan yang menjaga satu kontrak, dengan himpunan
yang tidak sama. Itu lebih buruk.

**Alternatif yang ditolak.**
- **Resolusi path sungguhan** (`eslint-plugin-import` dengan resolver, atau rule
  kustom yang memanggil `resolve`): menghilangkan false positive, tetapi menambah
  dependency dan menuntut pass resolusi per berkas per run. `tsconfig.json` sudah
  menolak `recommendedTypeChecked` atas alasan biaya yang sama; menerimanya di
  sini untuk masalah yang lebih kecil tidak konsisten.
- **Negasi gitignore** (`group: ['**/main', '!./main', …]`) untuk mengecualikan
  bentuk relatif-sendiri: bekerja, tetapi mengubah daftar yang terbaca jelas
  menjadi teka-teki, dan salah-baca satu negasi melubangi guard tanpa gejala.
- **Menyempitkan regex ke `\.\./main`**: mematikan bentuk alias `@/main` dan
  bentuk relatif dalam yang justru paling mungkin dipakai.

**Konsekuensi yang diterima.** Seseorang yang menamai direktori `main` di dalam
`src/output/` atau `src/shared/` akan tertahan lint dengan pesan yang, sebelum
perbaikan ini, menceritakan masalah yang bukan masalahnya. Arah kesalahannya
disengaja: guard batas bundle lebih baik gagal-tertutup dan berisik daripada
gagal-terbuka dan senyap, karena biaya false negative-nya adalah bundle projector
yang membengkak tanpa gejala sampai seseorang mengukur memori (PRD §5.1). False
positive-nya keras, langsung terlihat, dan jalan keluarnya — menamai ulang
direktori — lebih murah daripada infrastruktur resolusi path.

### ADR-0022 — Guard `dist/*.html` tidak memeriksa origin subresource; ditunda ke item pertama yang menyentuhnya

| | |
| --- | --- |
| Tanggal | 2026-08-09 |
| Status | Diterima (penundaan sadar) |
| Terkait | SETUP-03, ADR-0015, NFR-13, FR-606, PRD §5.1 |

**Keputusan.** Temuan auditor siklus 4–5 (`scripts/dist-html-guard.js:281`) **ditunda**, tidak diperbaiki di SETUP-03. Asersi positif `hasModuleScript` menerima `src` apa pun yang tidak kosong, termasuk `https://cdn…`, dan `findCspViolations` tidak memeriksa `<link rel="stylesheet" href="https://…">`. Penundaan ini dicatat di sini sesuai H2, dan menjadi **persyaratan yang dibawa** oleh item pertama yang benar-benar menambahkan subresource ke `src/index.html` atau `src/output.html`.

**Alasan.** Lubangnya nyata dan menyentuh NFR-13: di `tauri dev` CSP tidak ditegakkan ([ADR-0015](#adr-0015)), sehingga sumber daya remote akan diam-diam ditarik dari jaringan dan tampak bekerja; di build rilis `default-src 'self'` menolaknya dan jendela terbuka kosong — persis kegagalan yang `hasModuleScript` ditambahkan untuk menangkap.

Yang membuatnya layak ditunda adalah pemicunya. Ia menuntut seseorang menuliskan URL remote ke dalam salah satu dari dua berkas HTML yang ditulis tangan — tindakan yang tidak akan lolos review di aplikasi yang seluruh premisnya offline-first, dan yang hari ini tidak ada wujudnya: kedua dokumen memuat tepat satu `<script type="module" src>` relatif yang dibangkitkan Vite.

Yang membuatnya **tidak** boleh dilupakan adalah arah kegagalannya. Guard ini gagal-terbuka untuk kasus itu: build yang menarik CDN dilaporkan "clean". Seluruh sisa permukaan guard sudah gagal-tertutup, jadi ini satu-satunya pengecualian, dan pengecualian tanpa catatan adalah bagaimana ADR-0020 lahir.

**Alternatif yang ditolak.**
- **Memperbaikinya sekarang.** SETUP-03 sudah lima siklus. Menambah cara baru guard menolak build berarti satu putaran tester penuh lagi untuk permukaan yang belum punya pemakai, sementara auditor menilai — dan saya setuju — bahwa tiga permukaan yang paling mahal bila salah sudah tertutup dan tidak ada lagi temuan bertipe gagal-terbuka selain yang satu ini.
- **Melarang subresource remote lewat komentar di kedua berkas HTML.** Bentuk konvensi tanpa penegakan yang sudah ditolak di ADR-0013, ADR-0016, dan ADR-0020.

**Konsekuensi yang diterima.** Sampai item itu datang, satu-satunya penjaga terhadap subresource remote adalah CSP saat runtime rilis — yang gejalanya adalah jendela kosong di tengah ibadah, bukan build merah. Itu justru gejala yang paling mahal, dan itulah alasan penundaan ini ditulis sebagai persyaratan yang dibawa, bukan sebagai backlog.

**Bentuk perbaikannya, agar item berikutnya tidak perlu menemukannya lagi.** Perlakukan `src`/`href` yang memuat skema (`://`) atau berawalan `//` sebagai temuan tersendiri — bukan sebagai "dokumen tidak memuat modul", karena pesan itu akan mengarahkan pembaca ke `build.rollupOptions.input` yang tidak ada hubungannya. Pesannya menyebut NFR-13.

### ADR-0023 — Fixture `.aero` mencabut satu-satunya sinyal yang menahan dokumen ibadah; guard-nya dibawa ke SETUP-05

| | |
| --- | --- |
| Tanggal | 2026-08-09 |
| Status | Diterima (penundaan sadar, dengan pemilik) |
| Terkait | SETUP-06, SETUP-05, NFR-15, NFR-28, NFR-13, FR-706 |

**Keputusan.** Temuan auditor SETUP-06 (`.gitignore:87-88`) **tidak dapat ditutup di SETUP-06** dan dibawa ke **SETUP-05** sebagai persyaratan, bukan sebagai backlog. `.gitignore` bukan alatnya.

**Masalahnya.** `*.aero` dan `*.aerotpl` diabaikan justru karena isinya dokumen jemaat. NFR-15 ("hostile path fixtures") dan NFR-28 (korpus berkas termutasi) menuntut sebagian berkas itu **ada di repo**, sehingga `!**/fixtures/**/*.aero` wajib ada. Tetapi pengecualian itu bekerja dengan mencabut satu-satunya sinyal yang selama ini menahan `.aero`: keterabaian. Di dalam `fixtures/`, berkas ibadah nyata tampak normal di `git status` dan lolos seperti berkas biasa.

**Mengapa jalurnya realistis, bukan teoretis.** Justru fixture terpenting yang paling berisiko. NFR-28 menuntut korpus termutasi; cara termudah membuatnya adalah mengambil `.aero` valid dan memutasinya, dan `.aero` valid paling gampang didapat di mesin developer adalah susunan ibadah sungguhan. FR-706 memperberatnya: portable export **menyematkan teks lirik penuh** agar berkasnya mandiri — persis sifat yang membuatnya fixture paling menarik sekaligus muatan paling besar. Fixture hostile-path NFR-15 risikonya rendah karena sintetis menurut sifatnya; korpus NFR-28 yang berbahaya.

Review diff tidak menangkapnya: `.aero` adalah JSON besar, dan yang dilihat reviewer adalah blob, bukan lirik.

**Bentuk guard yang diminta.** Deteksi **positif** — fixture dibuktikan sintetis, bukan diasumsikan. Repo sudah punya presedennya di `scripts/check-dist-html.js` dan `scripts/dist-html-guard.js`: pemeriksa yang terikat ke lifecycle npm, dengan bagian murni yang dapat diuji unit.

**Cakupan guard — tiga hal, jangan sampai lahir setengah.**

1. **Kedua ekstensi yang di-unignore, bukan hanya `.aero`.** Baris 88 `.gitignore` juga meng-unignore `*.aerotpl`, dan FR-409 menjadikan `.aerotpl` format ekspor berdiri sendiri; tabel `template_media` berarti template nyata membawa referensi media milik gereja. Bobotnya di bawah lirik, tetapi implementer yang membaca entri ini tanpa kalimat ini akan membangun guard yang hanya memeriksa `*.aero`.
2. **Artefak render dan media biner, terlepas dari path-nya.** Ini melipat temuan W1 audit siklus 2 SETUP-06. Pola ber-anchor `/cache/` dan `/media/` hanya menutup bentuk "isi data root ditumpahkan ke akar repo"; bentuk yang lebih wajar — menyalin direktorinya utuh sehingga menjadi `AeroWorship/cache/decks/…`, atau menaruhnya di `tmp/data/`, atau bundel relink FR-705 di `tests/integration/fixtures/relink-{a,b}/media/` — semuanya lolos, dan tidak ada pola ekstensi di `.gitignore` yang menyentuh `.webp`. Itu bukan cacat dua baris tersebut melainkan batas prinsip `.gitignore`: berkas apa pun bisa mendarat di lokasi apa pun, dan tidak ada pola berbasis-lokasi yang menutup itu tanpa menelan direktori sumber yang sah (`src/assets/media/` ada di pohon kerja hari ini). Pemeriksa staged-content menilai **apa** yang di-stage, bukan **di mana** — jadi ia menutup ini dengan mekanisme yang sama.
3. **Tiga lubang yang `.gitignore` secara prinsip tidak bisa jaga**, dan yang satu pemeriksa tutup sekaligus: fixture tercemar (entri ini), `git add -f` yang melewati `.gitignore`, dan berkas yang terlanjur terlacak.

Satu catatan mekanis untuk penulis guard: `fixtures/` yang kebetulan berada di dalam direktori yang diabaikan tidak akan pernah terjangkau pengecualian baris 87–88, karena git tidak menuruni direktori yang diabaikan. Auditor menilai bobotnya rendah — arah kegagalannya aman untuk privasi, dan §6.13 mengarahkan fixture ke `tests/` — tetapi pemeriksa staged-content melewati soal ini sepenuhnya, dan itu satu alasan lagi bentuk ini yang dipilih.

**Mengapa SETUP-05 pemiliknya.** Item itu yang membuat kerangka `tests/{unit,integration,perf}`, yaitu tempat `fixtures/` pertama kali lahir. Menaruhnya di sana berarti guard ada **sebelum** fixture pertama ditulis, bukan sesudah — logika yang sama yang menempatkan SETUP-06 sebelum SETUP-04.

**Alternatif yang ditolak.**
- **Mempersempit pengecualian ke lokasi tertentu.** Tidak menyentuh masalahnya: yang berbahaya adalah isi berkas, bukan letaknya, dan mempersempit lokasi justru menghidupkan kegagalan senyap yang penanda nama direktori dipilih untuk menghindarinya.
- **Mengandalkan review diff.** Sudah dijelaskan di atas mengapa tidak bekerja.
- **Membuat item tersendiri.** Menunda guard sampai sesudah `fixtures/` terisi, yaitu sesudah risikonya terwujud.

**Konsekuensi yang diterima.** Antara sekarang dan SETUP-05, satu-satunya penjaga terhadap dokumen ibadah masuk lewat `fixtures/` adalah kehati-hatian penulisnya. Jendelanya sempit — direktori `fixtures/` belum ada, dan tidak ada kode yang memproduksi `.aero` sampai FR-7xx — tetapi jendela itu **tidak boleh** dibiarkan terbuka melewati item yang menciptakan direktorinya.

### ADR-0024 — `*.aerotpl` sudah mengabaikan sumber template bawaan FR-410; format sumbernya harus diputuskan sebelum berkasnya lahir

| | |
| --- | --- |
| Tanggal | 2026-08-09 |
| Status | Diterima (persyaratan yang dibawa) |
| Terkait | SETUP-06, FR-409, FR-410, PRD §6.9, PRD §6.13, Appendix A |

**Temuan.** `.gitignore:70` mengabaikan `*.aerotpl` di kedalaman berapa pun, dan pengecualian di baris 87–88 hanya berlaku di bawah direktori bernama `fixtures/`. FR-410 menuntut template bawaan hadir sejak first launch. Bila sumber ketiga template itu kelak berbentuk berkas `.aerotpl` di dalam repo — bentuk yang paling wajar diambil orang — **berkasnya sudah terabaikan hari ini**, tanpa satu pun tanda di `git status`. Build lokal tetap jalan karena berkasnya ada di disk; mesin lain dan CI kehilangan template bawaan.

**Keputusan.** Item yang mengimplementasikan FR-410 wajib memutuskan format sumber template bawaan **sebelum** berkas pertamanya ditulis, dan memilih salah satu secara eksplisit:

- **JSON seed di dalam migrasi** — menghindari masalahnya sepenuhnya, dan sejalan dengan Appendix A yang menyimpan template di tabel SQLite (`is_builtin`, `document TEXT`), bukan sebagai berkas.
- **Berkas `.aerotpl` di repo** — sah, tetapi menuntut pengecualian ber-lokasi ditambahkan ke `.gitignore` **pada perubahan yang sama**, bukan sesudahnya.

**Mengapa ini tidak diperbaiki di SETUP-06.** Tidak ada berkas yang perlu dijaga hari ini, dan menambahkan pengecualian untuk berkas yang belum diputuskan akan ada menghasilkan aturan yang tidak pernah teruji — alasan yang sama yang dipakai menolak `!.env.example` di item ini.

**Peringatan dari tester: suite tidak akan menangkapmu, ia akan menahanmu.** `tests/unit/gitignore-guard.test.js` mengasersi bahwa `*.aerotpl` **diabaikan** — benar hari ini, dan itu memang perilaku yang diinginkan sekarang. Konsekuensinya: pada hari sumber template bawaan FR-410 masuk repo di luar direktori `fixtures/`, baris 70 menelannya diam-diam **dan suite tetap hijau**, karena ia mengasersi cacat itu sebagai perilaku yang benar. Jadi jangan mengandalkan `npm test` untuk memberi tahu; test itu harus **diubah pada perubahan yang sama** yang memutuskan format sumbernya.

**Koreksi yang perlu tercatat.** Entri ini lahir dari penalaran `project-lead` yang **tidak berdiri**. Saya menolak menambahkan `/templates/` dengan alasan "pola direktori akan memblokir template bawaan yang harus masuk repo". Auditor memeriksanya: PRD §6.13 tidak memuat `templates/` di akar repo sama sekali, dan §6.9 menempatkan `templates/builtin/` di dalam data root `%APPDATA%\AeroWorship` — di luar repo. Jadi `/templates/` ber-anchor akar tidak akan menyentuh apa pun yang FR-410 kapalkan. Kesimpulannya kebetulan benar (`/templates/` memang tidak perlu ditambahkan), tetapi alasannya salah, dan alasan yang salah itu **menyembunyikan cacat yang aktif hari ini di baris 70** — yang justru isi entri ini. Dicatat apa adanya karena kesimpulan benar dari premis keliru adalah bentuk kegagalan yang paling sulit ditemukan lagi nanti.

### ADR-0025 — Data root memakai nama produk, bukan identifier bundle

| | |
| --- | --- |
| Tanggal | 2026-08-09 |
| Status | Diterima |
| Terkait | SETUP-04, ADR-0010, PRD §6.9, NFR-30 |

**Keputusan.** Berkas data aplikasi hidup di `%APPDATA%\AeroWorship\`, bukan di `%APPDATA%\id.aeroworship.app\` yang dihasilkan `PathResolver::app_data_dir()` bawaan Tauri. Diputuskan pengguna.

`%APPDATA%` tetap diresolve lewat API Tauri (`app.path().data_dir()`); yang menjadi konstanta hanya daun `"AeroWorship"`, dan itu nama produk dari PRD §6.9, bukan path yang dirakit tangan. Tidak ada string `%APPDATA%` maupun path absolut di kode.

**Alasan.** NFR-30 menuntut satu data root yang dapat **ditemukan dan disalin pengguna**. Aplikasi ini dipakai relawan gereja, dan skenario yang paling mungkin — "backup dulu data gerejanya sebelum ganti komputer" — menuntut folder yang terbaca sebagai nama aplikasi. `id.aeroworship.app` benar secara konvensi dan buram bagi orang yang harus menemukannya.

PRD §6.9 juga menuliskannya secara literal sebagai `{APP_DATA}/AeroWorship/`. Mengikuti Tauri berarti mengubah PRD; mengikuti PRD tidak menuntut apa pun kecuali entri ini.

**Yang dikoreksi dari ADR-0010.** Catatan konsekuensi ADR-0010 menyatakan identifier bundle "ikut menentukan nama direktori data aplikasi". Itu benar untuk perilaku bawaan Tauri, dan tidak lagi benar untuk aplikasi ini. Identifier tetap `id.aeroworship.app` dan tetap menentukan hal-hal lain yang disebut ADR-0010 — ia hanya tidak lagi menentukan lokasi data.

**Konsekuensi yang diterima.** Kita keluar dari jalur bawaan Tauri, sehingga setiap plugin atau kode masa depan yang memanggil `app_data_dir()` akan menunjuk direktori **yang berbeda** dari milik kita, dan keduanya bisa hidup berdampingan tanpa gejala. Itu jebakan nyata: gejalanya adalah "datanya hilang" padahal ia ada di folder sebelah. Penjaganya adalah satu jalur resolusi path di `src-tauri/src/db.rs` — jangan pernah memanggil `app_data_dir()` langsung di tempat lain.

### ADR-0026 — `default_arrangement_id` dijaga pada INSERT, bukan hanya UPDATE

| | |
| --- | --- |
| Tanggal | 2026-08-09 |
| Status | Diterima |
| Terkait | SETUP-04, PRD Appendix A, PRD §6.6 |

**Keputusan.** PRD Appendix A **diperbaiki** — atas keputusan pengguna, sesuai H4 — dengan menambahkan trigger `BEFORE INSERT ON songs` berkondisi identik dengan `trg_songs_default_arrangement_fk` yang sudah ada. Skema mengikutinya.

Sumbernya yang diperbaiki, bukan hanya gejalanya, supaya siapa pun yang membaca Appendix A nanti tidak menemukan celah yang sama dan menyalinnya ulang.

**Celahnya, terbukti empiris.** `trg_songs_default_arrangement_fk` hanya `BEFORE UPDATE OF default_arrangement_id`. Diukur implementer SETUP-04 pada database sungguhan:

```
UPDATE song-b -> arr-a (milik lagu lain): ditolak, "default_arrangement_id must belong to this song"
INSERT song-c  -> arr-a (milik lagu lain): DITERIMA, 1 row
```

Jadi sebuah lagu dapat lahir dengan `default_arrangement_id` menunjuk arrangement milik lagu lain, dan tidak ada apa pun di database yang menahannya. Referensi silang itu tidak dapat ditangkap `REFERENCES` biasa karena batasannya bukan "baris itu ada" melainkan "baris itu milik lagu ini" — persis alasan trigger dipakai di sini sejak awal.

**Konsekuensi yang perlu dipahami, bukan sekadar diterima.** Setelah trigger ini ada, `INSERT INTO songs` dengan `default_arrangement_id` non-NULL akan **selalu** ditolak. Itu bukan efek samping yang disayangkan — itu satu-satunya urutan yang mungkin: `song_arrangements.song_id` mereferensikan `songs(id)`, jadi arrangement milik lagu baru tidak dapat ada sebelum lagunya ada. Alur yang sah selalu tiga langkah: sisipkan lagu dengan `default_arrangement_id` NULL, buat arrangement-nya, lalu UPDATE.

Trigger ini menjadikan urutan itu **ditegakkan skema**, bukan konvensi yang harus diingat penulis service.

**Alternatif yang ditolak.**
- **Menutupnya di lapisan service FR-2xx.** Database berhenti menjaga dirinya sendiri, dan setiap jalur yang melewati service — impor, migrasi, perbaikan manual lewat `sqlite3` — dapat menulis data tidak konsisten. Bentuk konvensi tanpa penegakan yang sudah ditolak di ADR-0013, ADR-0016, dan ADR-0020.
- **Menambah trigger tanpa memperbaiki PRD.** Meninggalkan spesifikasi yang tidak cocok dengan skema sesungguhnya, di repo yang sudah tujuh siklus menghukum dokumen yang mengklaim berbeda dari kodenya.

### ADR-0027 — Kegagalan startup database ditunda, bukan diabaikan: ia tidak punya permukaan diagnostik dan itu bukan pekerjaan item skema

| | |
| --- | --- |
| Tanggal | 2026-08-12 |
| Status | Diterima (ditunda dengan pemicu) |
| Terkait | SETUP-04 (auditor W1), NFR-30, PRD §6.9 `logs/`, item logging |

**Keputusan.** Warning W1 auditor SETUP-04 **tidak diperbaiki di SETUP-04**, dan alasannya bukan biaya melainkan cakupan: memperbaikinya berarti memperkenalkan permukaan pelaporan galat — dialog, log, atau keduanya — yang merupakan item tersendiri dan bukan bagian dari "skema SQLite + migrasi". H5 melarang menariknya ke sini. Ditunda dengan pemicu eksplisit di bawah, bukan dengan harapan seseorang mengingatnya.

**Cacatnya.** `db::init` yang gagal mengembalikan `Err` dari `.setup()`, yang menjalar ke `.expect(...)` di `run()` (`src-tauri/src/lib.rs:71`) dan mem-panic. Pada build rilis `windows_subsystem = "windows"` (`src-tauri/src/main.rs:4`) melepas console, sehingga stderr tidak terikat ke apa pun yang dapat dilihat pengguna. Belum ada item logging — `logs/` di PRD §6.9 belum dibangun.

Hasilnya: **aplikasi tidak menampilkan apa pun.** Tidak ada jendela, tidak ada dialog, tidak ada baris log. Prosesnya sekadar menghilang.

**Mengapa ini bukan sekadar kekasaran.** Jalur yang memicunya justru jalur yang PRD sendiri dorong. NFR-30 menuntut data root yang dapat pengguna temukan, salin, dan pulihkan — dan penyalinan naif `aeroworship.db` tanpa `-wal`/`-shm` di tengah tulisan menghasilkan berkas yang gagal dibuka. Relawan gereja yang melakukan persis apa yang dokumentasi sarankan mendapat aplikasi yang tidak bisa dibuka dan nol petunjuk kenapa. Pemulihan yang gagal total tanpa jejak bukan pemulihan.

Empat varian `DbError` seluruhnya bermuara ke sini, termasuk dua yang sengaja dirancang untuk **memberi tahu** pengguna: `FutureSchema` menulis kalimat "Update AeroWorship to open it — it has not been modified", dan `InconsistentVersion` menyebut kedua angka. Kalimat-kalimat itu ditulis untuk dibaca manusia dan hari ini tidak sampai ke manusia mana pun. Itulah bagian yang paling perlu dicatat: pekerjaannya sudah dilakukan di satu ujung dan menganggur karena ujung lain belum ada.

**Batasnya jujur.** Ini murni ketersediaan. Nol kebocoran data, nol eksekusi kode, nol korupsi yang disebabkan kode ini — auditor menilainya Warning atas dasar itu dan penilaian itu diterima apa adanya.

**Dibawa keluar.** Item pertama yang memperkenalkan logging **atau** pelaporan galat ke pengguna wajib membuat kegagalan `db::init` terlihat: minimal satu baris di `logs/`, dan lebih baik lagi dialog yang menampilkan `Display` dari `DbError` — yang sudah ditulis untuk itu. Sampai saat itu, satu-satunya cara mendiagnosis adalah menjalankan biner dari console.

**Alternatif yang ditolak.**
- **Menampilkan dialog seadanya sekarang.** Menempatkan keputusan bentuk pelaporan galat di item skema, tempat ia akan diambil tergesa dan diwarisi seluruh aplikasi.
- **Membiarkan aplikasi tetap berjalan tanpa database.** Setiap fitur berikutnya harus menangani "tidak ada storage" sebagai keadaan sah selamanya, demi kasus yang seharusnya gagal keras.

### ADR-0028 — Larangan mengedit migrasi yang sudah dikapalkan ditunda sampai ada yang benar-benar dikapalkan

| | |
| --- | --- |
| Tanggal | 2026-08-12 |
| Status | Diterima (ditunda dengan pemicu) |
| Terkait | SETUP-04 (auditor S3), ADR-0026, item rilis pertama |

**Keputusan.** Test checksum yang menolak perubahan isi migrasi yang sudah dikapalkan **tidak dibuat di SETUP-04**. Pemicunya: **item yang menghasilkan artefak rilis pertama untuk pengguna di luar mesin pengembangan.**

**Alasannya adalah tanggal, bukan nilai.** Assertion `const _` sudah menjaga *urutan* versi saat kompilasi, tetapi nol hal menjaga *isi* `001_initial_schema.sql`. Menambahkan guard itu hari ini akan mengunci berkas yang siklus 2 baru saja amandemen dengan sah — dan pembenaran amandemen itu (ADR-0026) berdiri di atas premis "nol database di luar sana sudah menerapkan 001", yang diverifikasi empiris. Guard yang melarang persis tindakan yang baru saja benar akan menjadi guard yang orang pertama pelajari cara mem-bypass.

**Yang dijaga guard itu nanti.** `migrate()` memfilter `version > current`, jadi database ber-`user_version = 1` yang memakai teks 001 versi lama akan diam-diam dianggap mutakhir. Nol error, nol peringatan, skema salah. Implementer menyatakan jalur ini sendiri alih-alih membulatkannya, dan menyebut jaminan yang menutupinya sebagai *"jaminan yang hanya berlaku hari ini, dan hari ini satu-satunya hari ia gratis."*

**Kapan jendela itu tertutup.** Bukan saat kode di-commit, melainkan saat sebuah database menerapkan 001 di luar kendali kita. Verifikasi runtime coordinator sesudah siklus 3 menutupnya di mesin ini; rilis pertama menutupnya untuk semua orang. Sejak titik itu, mengedit 001 berhenti menjadi murah dan guard-nya berhenti menjadi opsional.

### ADR-0029 — Isolasi database bersandar pada akun OS; nol enkripsi at-rest, dan itu keputusan, bukan kelalaian

| | |
| --- | --- |
| Tanggal | 2026-08-12 |
| Status | Diterima |
| Terkait | SETUP-04 (auditor S4), PRD §6.9, NFR-30 |

**Keputusan.** Database dan seluruh data root disimpan **tanpa enkripsi**, dilindungi semata oleh ACL yang `create_dir_all` warisi dari `%APPDATA%`. Dicatat supaya ini menjadi asumsi yang tertulis, bukan asumsi yang diam.

**PRD tidak menuntut lebih, dan itu diperiksa bukan diasumsikan.** Auditor mencari `enkripsi|encrypt|at.rest|SQLCipher` di seluruh PRD: nol hasil. Jadi ini bukan requirement yang terlewat.

**Batas sesungguhnya.** Isolasi `%APPDATA%\<user>\Roaming` hanya berlaku bila tiap operator memakai akun Windows sendiri. PC gereja yang memakai satu login bersama untuk semua relawan — konfigurasi yang lazim dan mungkin justru yang paling umum di sasaran produk ini — tidak mendapat proteksi apa pun dari lapisan ini. Siapa pun yang bisa masuk ke mesin itu bisa membaca lirik, isi ibadah, dan catatan khotbah.

Bobotnya jujur: kandungan datanya bukan kredensial dan bukan data finansial, dan enkripsi at-rest yang kuncinya harus tersedia bagi aplikasi offline pada mesin yang sama memindahkan masalah alih-alih menyelesaikannya. Yang tidak dapat dibenarkan adalah **tidak menuliskannya**, karena keputusan diam tidak dapat ditinjau ulang saat asumsinya berubah.

**Yang membatalkan keputusan ini.** Requirement mana pun yang menempatkan data pribadi jemaat — bukan sekadar isi ibadah — di dalam database yang sama.
