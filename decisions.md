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
