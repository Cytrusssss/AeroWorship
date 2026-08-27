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

**Koreksi terhadap ADR-0014, dari audit FR-101 (2026-08-14).** Daftar dua langkah di atas terbaca sebagai daftar **tertutup**, dan sejak perintah `#[tauri::command]` pertama mendarat ia tidak lagi lengkap: **langkah 2 secara struktural buta terhadap perintah app-defined.** Diverifikasi — `list_monitors` muncul **nol kali** di seluruh `src-tauri/gen/schemas/`, jadi diff dua berkas yang sama-sama tidak memuatnya akan selalu bersih, dan peninjau yang mengikuti langkah ini secara harfiah akan melaporkan permukaan IPC kosong. Langkah ketiga karena itu wajib: **baca isi `generate_handler![…]` di `src-tauri/src/lib.rs`** — itulah satu-satunya daftar otoritatif permukaan IPC aplikasi ini. `capabilities/*.json` tetap wajib dibaca, tetapi cakupannya harus dinamai: ia hanya mengikat perintah `plugin:` (termasuk seluruh `core:*` yang dipanggil lewat `@tauri-apps/api`). Langkah 1 diperiksa ulang pada audit itu dan tetap terpenuhi: nol kemunculan `__allow_command`/`runtime_authority_mut`/`add_capability` di seluruh kode.

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

**Koreksi terhadap ADR-0018, dari audit FR-101 (2026-08-14).** Premis pertama kalimat di atas kini **terbalik**, sementara kesimpulannya menjadi **lebih luas**. Manifest kapabilitas bukan "satu-satunya batas yang nyata": untuk perintah **app-defined** tidak ada batas sama sekali. Gerbang ACL di `tauri` 2.11.5 (`webview/mod.rs:1823-1827`) hanya menyala untuk perintah `plugin:`, origin remote, atau bila aplikasi punya manifest ACL sendiri — dan repo ini tidak punya ([ADR-0039](decisions.md#adr-0039)). Jadi yang benar dibaca: **setiap perintah yang ditambahkan** — bukan sekadar setiap permission — harus diasumsikan dapat dipanggil oleh kode frontend mana pun yang berhasil dieksekusi di webview, termasuk kode yang masuk lewat XSS dari lirik atau template. Koreksi ini diletakkan di sini, bukan hanya di ADR-0039, sebab inilah satu-satunya pernyataan kategoris di repo dan ia berdiri tepat di ADR yang membahas ancaman itu.

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


### ADR-0030 — `.aero` yang diganti nama **dapat** ditangkap lewat sentinel Appendix C; alasan penundaan versi pertama ADR ini keliru dan diganti

| | |
| --- | --- |
| Tanggal | 2026-08-13 (direvisi total pada hari yang sama sesudah audit) |
| Status | Diterima |
| Terkait | SETUP-05 (auditor W1 siklus 1; audit siklus 3 temuan W3), [ADR-0023](decisions.md#adr-0023), FR-701, PRD Appendix C |

**Masalahnya.** Guard menangkap berkas lewat dua pegangan: ekstensi terdaftar (`.aero`/`.aerotpl`) dan magic byte media (`sniffMediaKind`). `.aero` sungguhan yang di-*commit* bernama `service.dat` lolos keduanya.

**Versi pertama ADR ini menerima celah itu dengan alasan yang tidak tahan uji, dan auditor membongkarnya.** Yang saya tulis: JSON tidak punya magic byte, jadi `sniffMediaKind` tidak dapat diperluas — "tidak ada yang bisa ditambahkan ke daftar itu"; satu-satunya jalan adalah pencocokan struktur terhadap skema yang belum ada, dengan biaya membaca setiap berkas teks. **Tiga bagiannya salah, dan ketiganya terverifikasi:**

1. **Ada sentinel literal, dan PRD sudah mewajibkannya.** `docs/PRD.md:1467` (skema) dan `:1526` (contoh terkerja) sama-sama menuntut kunci tingkat-atas `"kind": "aeroworship.session"`. Itu string ASCII unik yang wajib hadir di setiap `.aero` sungguhan, apa pun namanya.
2. **Skemanya sudah ada.** Appendix C lengkap di `docs/PRD.md:1460–1549`. Yang belum ada adalah implementasi Rust-nya, bukan skemanya — dan `kind`/`schema_version` justru dua kunci paling stabil di seluruh dokumen.
3. **Argumen biaya salah.** Guard **sudah** membaca isi lengkap setiap berkas ter-track non-allowlist; `check-fixture-content.js:72–74` menyatakannya sendiri. Buffer-nya sudah di tangan saat `sniffMediaKind` dipanggil. Biaya tambahannya nol.

Auditor juga menunjuk pegangan keempat yang tidak saya sebut: `findFixturesRoot` sudah mewajibkan deklarasi manifest bagi **setiap** path di bawah root `fixtures/`, tanpa peduli ekstensi maupun konten — yang justru menutup skenario yang saya sendiri sebut sebagai risiko utamanya ("seseorang menyalin berkas ibadah sungguhan ke `fixtures/`").

**Keputusan.** Tambahkan deteksi sentinel Appendix C ke jalur pengendusan. Celah ini ditutup, bukan diterima.

**Tetapi bukan sebagai pencocokan substring — dan ini bukan detail.** `docs/PRD.md` **ter-track** dan memuat literal `aeroworship.session` dua kali. `buffer.includes('aeroworship.session')` yang polos akan menandai PRD sebagai kandidat, menolaknya karena ia tak punya leluhur `fixtures/`, dan membuat `npm test` merah pada repo bersih. Perbaikan yang paling jelas karena itu **lebih lebar daripada kenyataan** — kelas cacat yang sama persis dengan yang ia perbaiki. Deteksinya wajib menuntut buffer yang **parse sebagai JSON** dengan `kind === 'aeroworship.session'`; PRD gugur di syarat pertama, `.aero` sungguhan lolos apa pun namanya.

**Terimplementasi pada siklus 4**, dengan penyaring termurah lebih dulu: byte non-*whitespace* pertama (sesudah BOM opsional) harus `{` — PRD gugur di sini karena diawali `#` — lalu `buffer.includes('aeroworship.session')`, baru `JSON.parse` dengan `kind` di tingkat atas. Sengaja **tanpa batas ukuran di dalam fungsi ini**: cap 256 KB akan menjadi cara terdokumentasi menyelundupkan `.aero` lewat *padding*, sedangkan dua penyaring pertama sudah membuat `JSON.parse` praktis tak pernah berjalan — audit penutup mengukurnya dan mendapati literal sentinel hanya ada di lima berkas repo, **tak satu pun berawal `{`**, sehingga `JSON.parse` hari ini berjalan **nol kali** atas repo sungguhan.

**Koreksi sesudah audit: "tanpa batas ukuran" benar secara lokal dan menyesatkan secara sistem.** Plafon efektif ada, besarnya 64 MiB, dan letaknya di hulu — `MAX_BUFFER` di `check-fixture-content.js:80`, yang menggugurkan seluruh run dengan exit 1 alih-alih menyelundupkan. Argumen menolak cap 256 KB tetap sah; yang keliru adalah menyiratkan tidak ada plafon sama sekali, sementara [ADR-0032](decisions.md#adr-0032) — yang saya tulis sendiri tentang berkas yang sama — justru membahas plafon itu panjang lebar.

**Batas yang tetap terbuka sesudah perbaikan ini.** `.aero` yang **dienkripsi, dikompresi, atau sengaja dirusak** tetap tak terdeteksi, karena ia berhenti menjadi JSON. Itu penyelundupan berniat, dan tidak ada guard di lapisan ini yang menjawabnya — pertahanannya tinjauan manusia, sama seperti [ADR-0031](decisions.md#adr-0031).

**Dan satu batas yang lebih tajam: `.aerotpl` tidak ikut tertutup.** Appendix B tidak mendefinisikan sentinel tingkat-atas untuk template — diperiksa coordinator, dan setiap `"kind"` di sana **bersarang** (`solid`, `gradient`, `image` untuk latar; `rect`, `rounded_rect`, `ellipse`, `path` untuk geometri), bukan penanda dokumen. Jadi template sungguhan yang diganti nama tetap tak terlihat. Ketimpangan ini dinamai supaya `.aero`/`.aerotpl` tidak lagi dianggap dijaga sama kuat hanya karena selalu disebut berpasangan.

**Koreksi sesudah audit penutup: itu *belum dikerjakan*, bukan *tidak dapat dikerjakan*.** Versi pertama paragraf ini menutup pintu yang sebenarnya terbuka. Appendix B punya tanda tangan **struktural** tingkat atas yang sama stabilnya dengan sentinel — diperiksa coordinator di `docs/PRD.md:1303-1312`: `schema_version`, plus `canvas.aspect_ratio` yang enum tiga nilai, plus `layers` sebagai array. Memeriksanya menuntut `JSON.parse` yang sama, dan itu **bukan** heuristik "mirip-mirip" yang ADR-0023 tolak — penolakan itu tentang membedakan lirik nyata dari lirik sintetis, bukan tentang mengenali bentuk dokumen. Yang benar dikatakan: menutup `.aerotpl` menunggu item yang benar-benar membutuhkannya, bukan menunggu PRD berubah.

**Yang membatalkan keputusan ini.** Perubahan Appendix C yang menghapus atau mengganti nilai `kind`. Sentinel adalah kontrak; kalau ia bergerak, guard ikut buta.

### ADR-0031 — Manifest membuktikan adanya klaim, bukan kebenarannya — dan verifikasinya hanya satu arah: berkas → manifest

| | |
| --- | --- |
| Tanggal | 2026-08-13 (direvisi pada hari yang sama sesudah audit) |
| Status | Diterima |
| Terkait | SETUP-05 (auditor S1 siklus 1; audit siklus 3 temuan W4), [ADR-0023](decisions.md#adr-0023), [ADR-0030](decisions.md#adr-0030) |

**Keputusan.** `fixtures.manifest.json` bekerja lewat **deklarasi diri**: `synthetic: true` menyatakan sebuah fixture dibuat-buat. Guard memverifikasi klaim itu **ada dan terbentuk benar**; ia tidak dapat, dan tidak berpura-pura dapat, memverifikasi bahwa isinya memang sintetis. Berkas ibadah sungguhan yang dideklarasikan `synthetic: true` akan lolos.

**Koreksi atas versi pertama ADR ini.** Saya menulis bahwa guard juga memverifikasi deklarasi itu "menunjuk berkas yang benar-benar di-*stage*". **Tidak.** Verifikasi berjalan satu arah saja: `evaluateCandidate` berangkat dari path kandidat yang ditemukan di index, lalu mencari kuncinya di manifest. Tak ada satu baris pun yang mengiterasi `entries` untuk memeriksa arah sebaliknya. Manifest berisi `"hantu.aero": { synthetic: true, … }` tanpa berkas `hantu.aero` di mana pun **lolos tanpa keluhan**; entri yatim tidak pernah terdeteksi.

Bahwa entri yatim tidak berbahaya — ia mendeklarasikan berkas yang tidak ada — tidak membuat klaim itu benar. Yang berbahaya adalah ADR yang menyatakan sebuah pemeriksaan berjalan padahal tidak, karena pembaca berikutnya akan bersandar padanya.

**Dua klausul lain terverifikasi, dan salah satunya lebih kuat dari yang saya tulis.** "Ada": guard menuntut manifest hadir di **index**, bukan di working tree — manifest yang belum di-*stage* ditolak. "Terbentuk benar": JSON valid, root objek, `entries` objek, entri objek, `synthetic === true` literal, `purpose` ≥ 20 karakter sesudah `trim`. Batasnya: hanya subpohon yang dipakai yang divalidasi; entri lain di manifest yang sama tak pernah disentuh, dan manifest di `fixtures/` yang tak punya kandidat tak pernah diambil sama sekali.

**Mengapa deklarasi diri tetap bernilai meski bisa dibohongi.** Ia memindahkan kelalaian menjadi pernyataan. Tanpa manifest, menaruh lirik sungguhan di `fixtures/` tak menuntut siapa pun mengatakan apa pun. Dengan manifest, orang yang sama harus **menuliskan klaim** ke berkas yang muncul di diff dan bertanda tangan di riwayat git. Kecelakaan berubah menjadi keputusan sadar, dan keputusan sadar bisa ditinjau — sebagian besar pelanggaran yang dikhawatirkan ADR-0023 adalah kecelakaan.

**Konsekuensi yang harus diterima terang-terangan.** Kontrol ADR-0023 **bukan kontrol otomatis penuh**. Bagian otomatisnya menjamin tidak ada berkas berisiko masuk tanpa klaim; bagian manusianya — pembaca *review* yang melihat `synthetic: true` baru dan bertanya "sungguh?" — satu-satunya yang menilai kebenaran klaim. Yang benar dikatakan: **guard ini mencegah konten berisiko masuk tanpa klaim tertulis.**

**Yang membatalkan keputusan ini.** `synthetic: true` keliru yang pernah lolos ke `main`, atau manifest yang tumbuh cukup besar sehingga entri yatim menjadi kebisingan yang menyamarkan entri sungguhan.

### ADR-0032 — `maxBuffer` 64 MiB gagal-tertutup; yang mengalir lewatnya adalah **seluruh isi repo ter-track**, bukan hanya kandidat

| | |
| --- | --- |
| Tanggal | 2026-08-13 (direvisi pada hari yang sama sesudah audit) |
| Status | Diterima |
| Terkait | SETUP-05 (auditor S2 siklus 1; audit siklus 3 temuan W2), [ADR-0023](decisions.md#adr-0023) |

**Keputusan.** `check-fixture-content.js` menampung seluruh keluaran `git ls-files -z -s` dan `git cat-file --batch` di memori lewat `execFileSync` dengan `maxBuffer` 64 MiB (`:77,86`). Dipertahankan; tidak diubah menjadi streaming sekarang.

**Koreksi atas versi pertama ADR ini.** Saya menulis bahwa yang mengalir lewat `--batch` "hanya konten kandidat". **Salah.** `toInspect` adalah **setiap** path ter-index yang bukan gitlink, bukan `unsafe`, dan bukan allowlist — isi lengkapnya ditarik ke satu Buffer, karena magic byte hanya bisa diendus dari byte yang sudah dibaca. Berkas itu menyatakannya sendiri di `:72–74`. Variabel penskalaannya bukan jumlah fixture melainkan **total isi repo ter-track dikurangi `src-tauri/icons/`**.

**Konsekuensinya bukan salah kata, melainkan pemicu yang salah.** Versi pertama menetapkan pembatalnya sebagai "item pertama yang menambahkan fixture media besar". Dengan premis yang benar, pemicunya jauh lebih luas: berkas ter-track besar apa pun — `package-lock.json`, korpus Alkitab, aset apa pun — menghabiskan anggaran yang sama **tanpa satu fixture pun ada**. Pemicu lama tidak akan pernah menyala untuk penyebab yang paling mungkin.

**Marginnya, diukur bukan dikira.** 64 berkas ter-track, total **0,8 MiB**. Terhadap 64 MiB itu sekitar 80×. Jarak itu nyata, dan itulah alasan penundaannya sah.

**Klaim gagal-tertutup terverifikasi, dengan satu nuansa.** Auditor menelusurinya persis seperti diminta: tak ada `try`/`catch` yang menelan galat buffer menjadi jalur bersih. `git()` hanya menangkap untuk memperkaya pesan `ENOENT` lalu melempar ulang; kedua pemanggilnya menangkap dan `return 1`. Nuansanya: galat itu **memang ditangkap** — kalimat "exception, bukan laporan bersih" terlalu ringkas. Yang benar: ia diubah menjadi **exit 1 bercetak pesan**, bukan menjadi laporan bersih. Kelas hasilnya sama, dan itulah yang menentukan sebuah batas boleh ditunda: kalau kegagalannya berupa pemotongan senyap, ia wajib diperbaiki hari ini.

**Yang membatalkan keputusan ini.** Berkas ter-track besar apa pun yang membuat total isi repo mendekati anggaran — bukan khusus fixture. Perbaikannya sudah jelas bentuknya: aliri `--batch` lewat stdio alih-alih menampungnya, dan korelasikan respons sambil membaca.

### ADR-0033 — Symlink lolos; path non-UTF-8 **menggugurkan seluruh run**, bukan sekadar dirinya sendiri

| | |
| --- | --- |
| Tanggal | 2026-08-13 (direvisi pada hari yang sama sesudah audit) |
| Status | Diterima |
| Terkait | SETUP-05 (auditor S3 siklus 1; audit siklus 3 temuan W1 dan verifikasi ADR-0033), [ADR-0023](decisions.md#adr-0023), [ADR-0024](decisions.md#adr-0024) |

**Symlink (mode `120000`) — terverifikasi seperti tertulis semula.** Hanya `160000` yang diperlakukan khusus; symlink dibaca sebagai blob biasa yang isinya **teks path tujuannya**. Maka `media/photo.png` yang sebenarnya symlink ke berkas ibadah sungguhan tidak cocok magic byte mana pun dan lolos. Yang tidak lolos: symlink berekstensi terdaftar tetap wajib punya deklarasi manifest, sebab cek ekstensi tak melihat konten sama sekali. Bobotnya rendah karena isi yang "diselundupkan" tidak ikut ter-*commit* — hanya path tujuannya yang tersimpan sebagai objek git.

**Path non-UTF-8 — lokasi benar, mekanisme salah, dan konsekuensinya lebih berat.** Saya menulis bahwa path itu jatuh ke `unreadable` berkat perbaikan W-new-1, sehingga gagal-tertutup dan yang kurang hanya kualitas pesan galat. **Ia tidak pernah sampai ke sana.** `parseCatFileBatch` mendekode header sebagai `latin1` (pilihan yang benar untuk melindungi framing biner) lalu membandingkannya dengan `` `:${key} missing` `` yang ber-UTF-8. Untuk path non-ASCII, gema byte-mentah git menjadi mojibake yang tak pernah `===`; eksekusi jatuh ke regex header yang juga tak cocok, dan fungsi **melempar**. Pemanggilnya menangkap dan `return 1` — sebelum `identifyCandidates` berjalan untuk **satu path pun**.

**Jadi yang kurang bukan kualitas diagnostik, melainkan cakupan:** satu path bermasalah menggugurkan penilaian atas seluruh path lain. Arahnya tetap gagal-tertutup, dan karena itu ini bukan celah keamanan — tetapi mekanisme yang saya kreditkan tidak pernah menyala, dan W-new-1 tidak berperan sama sekali.

**Keputusan.** Bandingkan sentinel `missing` pada level byte, atau dekode header sebagai UTF-8 — perbaikan satu baris yang mengembalikan perilaku per-path yang W-new-1 rancang. **Ini diperbaiki, tidak ditunda.** Symlink tetap ditunda: AeroWorship tidak memakai symlink di mana pun, dan menanganinya berarti menambah jalur kode yang tak dapat diuji terhadap kebutuhan nyata mana pun.

**Yang membatalkan penundaan symlink.** Symlink pertama yang di-*stage* dengan sengaja.

### ADR-0034 — NFR-33 dipenuhi lewat `ts-rs`, dan ekspornya **test-time**; janji "build time" PRD §6.4 tidak dapat ditepati oleh alat yang PRD §6.12 namai

| | |
| --- | --- |
| Tanggal | 2026-08-13 |
| Status | Diterima — pertentangan PRD **ditutup 2026-08-15 oleh keputusan pengguna** |
| Terkait | SETUP-05 (NFR-33), PRD §6.4 (baris 585), PRD §6.12 (baris 814), NFR-16 |

**Penutupan, 2026-08-15.** Pengguna memilih mengubah kalimat PRD, bukan mengganti alat. Kalimat §6.4 kini berbunyi tipe "generated into TypeScript **by a checked gate that fails if the committed contract does not match the Rust types**". Ini satu-satunya suntingan yang dibuat pada `docs/PRD.md`, dan ia dibuat atas keputusan pengguna ([H4](PROGRESS.md)) — bukan atas kesimpulan ADR ini.

**Mengapa kalimat penggantinya berbentuk begitu.** Ia menyebut **mekanisme yang benar-benar ada** — `npm run bindings:check` di `pretypecheck` ([ADR-0035](decisions.md#adr-0035)) — alih-alih menyebut fase kompilasi yang tidak pernah menjalankan apa pun. Janji yang sesungguhnya ingin dibeli PRD adalah *"kontrak tidak dapat menyimpang"*, dan gate itulah yang membelinya; "at build time" hanyalah tebakan tentang **kapan** hal itu terjadi, dan tebakan itu salah. NFR-33 sendiri (baris 504) tidak pernah menyebut build time, jadi tidak ada yang perlu diubah di sana — persis satu kalimat di seluruh PRD yang memuat klaim itu.

**Yang tersisa dan sudah dinamai:** komentar `.cargo/config.toml` mengutip frasa lama itu verbatim untuk menjelaskan mengapa ia tidak dapat ditepati. Kutipan itu kini menunjuk kalimat yang tidak ada lagi, dan membiarkannya adalah kelas cacat yang SETUP-05 seluruhnya membahasnya — klaim yang lebih lebar atau berbeda daripada kenyataan. Ia dijadwalkan diperbaiki begitu ronde tester FR-102 yang sedang berjalan selesai, sebab menyunting berkas itu di tengah `cargo test` mengubah lingkungan yang sedang diukur.

**Keputusan.** Kontrak tipe Rust→TypeScript dibangkitkan `ts-rs` 12.0.1, dipasang sebagai **dev-dependency** `aeroworship-core`.

**Pertentangan PRD, dinyatakan bukan dibulatkan.** PRD §6.4 menjanjikan tipe "generated into TypeScript **at build time**". `ts-rs` tidak bisa, dan itu bukan penilaian kami melainkan kalimat penulisnya sendiri di `ts-rs-12.0.1/src/lib.rs:168`: *"bindings \_\_cannot\_\_ be exported during compile time"* — karena makro prosedural Rust dievaluasi sebelum tahap kompilasi lain. `#[ts(export)]` mengembang menjadi `#[test]`. Terukur di repo ini: test `db::migrations::export_bindings_migration` muncul di `cargo test`, sementara `cargo build` dan `cargo clippy` tidak menghasilkan satu berkas pun.

Jadi alat yang PRD sendiri namai lebih dulu tidak dapat memenuhi janji yang PRD tulis. **`docs/PRD.md` tidak diubah** — PRD hanya berubah lewat keputusan pengguna ([H4](PROGRESS.md)). Yang tidak boleh terjadi adalah menyebut `cargo test` sebagai "build time" dan menganggap perkara selesai; itu klaim yang lebih lebar daripada kenyataan, kelas cacat yang [ADR-0020](decisions.md#adr-0020)/[ADR-0021](decisions.md#adr-0021) buktikan mahal di repo ini.

**Pilihan `ts-rs` atas `specta`, dalam kalimat yang bisa dibantah.** `specta` menagih pembayaran di **biner yang dikirim** untuk sesuatu yang hanya dibutuhkan saat generate; `ts-rs` tidak. Ekspor `specta` adalah pemanggilan fungsi runtime, bukan `#[test]` yang dibangkitkan, sehingga `derive(Type)` harus menjadi dependency **normal** dan masuk graf biner rilis — menekan NFR-16. `tauri-specta` mengambil kontraknya dari `#[tauri::command]`, yang **nol** di repo ini hari ini, jadi ia belum bisa membuktikan apa pun. `specta` v2 juga masih `2.0.0-rc.25`.

**Bantahan yang diterima di muka:** kalau kelak katalog Appendix D memang ingin dibangkitkan otomatis dari `#[tauri::command]`, `tauri-specta` mengerjakan yang `ts-rs` tidak bisa, dan biaya biner di atas menjadi harga yang wajar. Keputusan ini tidak mengunci pintu itu.

**NFR-16 terverifikasi, bukan diasumsikan.** `cargo tree -p aeroworship --edges normal` memberi **nol** `ts-rs`. Tiga crate yang masuk `Cargo.lock` (`ts-rs`, `ts-rs-macros`, `termcolor`) seluruhnya dev-only; nol byte masuk installer.

**Yang membatalkan keputusan ini.** Perintah `#[tauri::command]` pertama yang benar-benar mendarat, yang membuat perbandingan dengan `tauri-specta` menjadi nyata alih-alih hipotetis.

### ADR-0035 — Gate anti-drift dipasang di `pretypecheck`; `npm run typecheck` karena itu menuntut toolchain Rust, dan `cargo test` menulis ke berkas ter-track

| | |
| --- | --- |
| Tanggal | 2026-08-13 |
| Status | Diterima |
| Terkait | SETUP-05 (NFR-33), [ADR-0034](decisions.md#adr-0034), [ADR-0020](decisions.md#adr-0020) |

**Keputusan.** `npm run bindings:check` membangkitkan ulang kontrak ke direktori temp, membandingkannya byte-per-byte dengan berkas ter-*commit*, dan gagal keras bila berbeda. Ia dipasang sebagai `pretypecheck`, bukan `pretest`.

**Mengapa `pretypecheck` dan bukan `pretest`.** Binding basi **tidak** membuat `vue-tsc` merah — ia membuatnya **hijau terhadap kontrak kemarin**. Gate yang mengonsumsi berkas ini adalah `typecheck`, jadi di situlah kesegarannya harus dijamin. `npm test` sengaja dibiarkan bersih dari cargo.

**Harga yang harus diterima terang-terangan.** `npm run typecheck` kini menuntut toolchain Rust terpasang. Kontributor yang hanya menyentuh frontend tidak lagi bisa menjalankan gate tipe tanpa `cargo`. Itu biaya nyata, dan diterima karena alternatifnya — kontrak yang boleh basi selama `vue-tsc` hijau — meniadakan seluruh maksud NFR-33.

**Efek samping yang lebih halus, dan jendela buta yang ditimbulkannya.** `TS_RS_EXPORT_DIR` disetel di `.cargo/config.toml` akar repo, sehingga `cargo test --workspace` biasa **menulis ke `src/shared/bindings/`** yang ter-track. Akibatnya working tree menyembuhkan dirinya sendiri: mengubah tipe Rust lalu menjalankan `cargo test` memperbarui `.ts` diam-diam, dan `bindings:check` sesudahnya **hijau** karena ia membandingkan terhadap berkas yang baru saja diperbarui. Drift tidak lagi muncul sebagai gate merah melainkan sebagai berkas termodifikasi di `git status`.

Diterima, karena keadaan akhir yang diinginkan justru itu — kedua berkas berubah bersama dalam satu perubahan. Yang tersisa sebagai risiko adalah **commit selektif**: seseorang meng-*commit* perubahan Rust tanpa `.ts`-nya. Backstop normalnya adalah CI, dan **repo ini belum punya konfigurasi CI sama sekali** — jadi hari ini backstop-nya adalah tinjauan manusia atas diff. Itu harus dikatakan, bukan diasumsikan.

**Mengapa pengalihan itu benar-benar bekerja, ditemukan audit penutup dan layak dicatat sebagai alasannya.** `ts-rs` 12.0.1 membaca `TS_RS_EXPORT_DIR` **saat runtime**, lewat `std::env::var` di `Config::from_env` (`ts-rs-12.0.1/src/lib.rs:618-627`) — bukan saat makro prosedural mengembang. Kalau ia dibaca saat kompilasi, nilai lama bisa ter-*bake* ke binary test yang di-*cache* dan gate akan menulis ke direktori ter-*commit* tanpa rebuild. Itu skenario kegagalan yang paling berbahaya bagi keputusan ini, dan ia tidak ada.

**Dua syarat yang versi pertama ADR ini tidak sebut.** Penyembuhan-diri oleh `cargo test` hanya berlaku bila `.cargo/config.toml` benar-benar ditemukan cargo (ia dicari dari **cwd**, bukan dari `--manifest-path`) dan bila `TS_RS_EXPORT_DIR` belum lebih dulu ada di lingkungan pemanggil — variabel eksternal menang atas `[env]` tanpa `force`, dan justru itulah yang membuat gate dapat mengalihkan regenerasinya. Di luar dua syarat itu, `cargo test` tidak menyembuhkan apa pun dan drift kembali muncul sebagai gate merah: arah yang lebih aman, tetapi bukan yang digambarkan di atas.

**Satu sifat yang tidak diklaim tetapi ternyata ada, dan ia yang paling menenangkan.** Seandainya pengalihan ke temp dir gagal total sehingga generator menulis ke direktori ter-*commit*, `collectBindings` atas temp dir mengembalikan peta kosong → `generatedNothing` → exit 1. Gate karena itu **secara struktural tidak dapat hijau sambil diam-diam menambal drift**; paling buruk ia merah dengan pesan yang salah alamat.

**Dua sifat gate yang diukur, bukan dianggap.** (1) Regenerasi masuk temp dir, jadi gate tidak diam-diam memperbaiki drift yang seharusnya ia laporkan — berkas ter-*commit* terbukti utuh saat merah, dan dikonfirmasi coordinator lewat `git status` sesudah menjalankannya. (2) `cargo test <filter>` yang tidak cocok apa pun **exit 0** (terverifikasi: `zzz_no_such_test` → 0), sehingga "generator menghasilkan nol berkas" dijadikan kegagalan eksplisit. Tanpa klausa itu, menghapus `#[ts(export)]` terakhir akan mematikan seluruh mekanisme dengan gate tetap hijau — persis kelas ADR-0020.

**Yang membatalkan keputusan ini.** Konfigurasi CI pertama, yang memindahkan backstop commit-selektif dari manusia ke mesin dan boleh mengubah perhitungan di atas.

### ADR-0036 — `Migration` dibangkitkan sebagai **bukti mekanisme**, bukan sebagai awal katalog Appendix D

| | |
| --- | --- |
| Tanggal | 2026-08-13 |
| Status | Diterima |
| Terkait | SETUP-05 (NFR-33), [ADR-0034](decisions.md#adr-0034), PRD Appendix D |

**Keputusan.** Satu-satunya tipe yang diekspor hari ini adalah `db::Migration`. Ia dipilih sebagai tipe nyata terkecil yang membuktikan pipeline bekerja — tiga field primitif, tanpa tipe eksternal, tanpa atribut serde yang perlu ditafsirkan.

**Mengapa bukan `DbError`, dan mengapa `AppError` tidak diciptakan.** `DbError` bukan `Serialize`, dan varian `Sqlite`-nya membungkus `rusqlite::Error` yang tak punya proyeksi TS sama sekali. Appendix D menyatakan setiap perintah mengembalikan `Result<T, AppError>`, tetapi `AppError` belum ada — dan mengarang bentuk kontrak galat aplikasi supaya ada sesuatu untuk dibangkitkan adalah pekerjaan item FR, bukan item ini. Mekanisme yang terbukti pada satu tipe nyata lebih bernilai daripada kontrak karangan yang harus dibongkar bulan depan.

**`Migration` bukan tipe kawat, dan tidak boleh menjadi tipe kawat.** Ia internal crate `core`. Kehadirannya di `src/shared/bindings/` adalah artefak bukti, bukan pernyataan bahwa frontend boleh memakainya. Komentar di atas struct-nya memuat instruksi pembongkarannya: **saat tipe kawat pertama mendarat di `models/`, kedua atribut `cfg_attr` dilepas dan `Migration.ts` dihapus dalam perubahan yang sama.**

**Yang membatalkan keputusan ini.** Tipe kawat pertama itu sendiri. Sejak ia ada, mempertahankan `Migration` di direktori binding berhenti menjadi bukti dan mulai menjadi kebisingan yang menyamarkan kontrak sungguhan.

### ADR-0037 — Identitas monitor dibangun dari nama perangkat GDI, bertag skema; kunci yang benar-benar stabil ada di OS tetapi tidak di permukaan Tauri

| | |
| --- | --- |
| Tanggal | 2026-08-14 |
| Status | Diterima |
| Terkait | FR-101, FR-103, PRD Appendix D |

**Keputusan.** `id = "gdi:" + <nama perangkat GDI>` (di Windows `\\.\DISPLAY1`), dengan fallback `"pos:{x},{y}"` bila OS tidak memberi nama.

**Mengapa alasannya negatif, bukan positif.** Tauri 2.11.5 memberi lima hal tentang sebuah monitor: `name`, `size`, `position`, `work_area`, `scale_factor`. **Empat di antaranya adalah pengukuran atas setelan display saat ini** — ukuran berubah saat resolusi diubah, `scale_factor` saat slider penskalaan digeser, posisi saat display disusun ulang. Ketiganya persis operasi yang penetapan FR-103 **harus selamat** melewatinya. Jadi tak satu pun boleh masuk ke dalam id, dan yang tersisa hanya `name`. Ini bukan pilihan terbaik dari beberapa yang bagus; ini satu-satunya yang tidak langsung salah.

**Bukti bahwa Windows sendiri tidak memakai nama slot sebagai kunci.** `HKLM\SYSTEM\CurrentControlSet\Enum\DISPLAY` di mesin pengembang mencatat **tujuh** monitor yang pernah terpasang, seluruhnya dikunci oleh **kode EDID pabrikan+produk** — `AUO5C2D`, `HPN36A6`, `LEN63EB`, dan seterusnya — **tidak pernah** oleh `\\.\DISPLAYn`. Itu bukti terkuat yang dapat diambil tanpa mencabut kabel, dan ia mengatakan terus terang bahwa keputusan ini memakai kunci yang lebih lemah daripada yang OS pakai sendiri.

**Di mana ia tidak stabil — dinamai, bukan disamarkan.**

1. **Replug ke port berbeda, atau perangkat berbeda ke port yang sama.** `\\.\DISPLAYn` adalah **slot**, bukan monitor. Cabut proyektor, colok proyektor lain ke port yang sama, dan penetapan tersimpan menunjuk perangkat keras yang berbeda **tanpa gejala apa pun**. Ini kegagalan paling mungkin di gereja sungguhan.
2. **Dua monitor identik** hanya dibedakan oleh slot; menukar kabelnya menukar id-nya.
3. **Jalur fallback `pos:`** tidak selamat dari penyusunan ulang — karena itu ia bertag skema berbeda. Di Windows cabang ini praktis tak terjangkau (`name` hanya `None` bila `GetMonitorInfoW` gagal atas handle yang baru saja dihasilkan `EnumDisplayMonitors`).

**Yang stabil:** perubahan resolusi, perubahan DPI/penskalaan, penyusunan ulang display, dan restart dengan topologi sama.

**Mengapa tag skema, dan mengapa ia bukan hiasan.** Kunci yang benar-benar stabil **ada** di level OS — device interface path ber-EDID, atau `QueryDisplayConfig` — hanya saja tidak ada di permukaan API Tauri; menjangkaunya menuntut FFI Win32 dan dependency `windows`, keputusan tersendiri yang bukan milik FR-101. Bila kelak diambil, setiap kunci yang sudah tertulis ke database gereja harus **dapat dikenali sebagai skema lama supaya dibuang** — bukan diam-diam dicocokkan ke display yang salah. Tag itu yang membuat migrasi kelak mungkin dilakukan dengan jujur.

**Yang membatalkan keputusan ini.** Laporan pertama bahwa penetapan display hilang atau tertukar sesudah kabel dipindah — atau item yang memang membutuhkan identitas berbasis EDID, yang menjadikan FFI Win32 berbayar.

### ADR-0038 — `list_monitors` mengembalikan `Vec<Monitor>` telanjang, menyimpang dari Appendix D; `AppError` ditunda karena bentuknya belum dapat ditepati

| | |
| --- | --- |
| Tanggal | 2026-08-14 |
| Status | Diterima |
| Terkait | FR-101, PRD Appendix D, [ADR-0036](decisions.md#adr-0036) |

**Keputusan.** Perintah pertama repo ini **tidak** mengembalikan `Result<T, AppError>`, meskipun Appendix D menyatakan setiap perintah melakukannya.

**Alasan pertama, terukur.** Di `tauri` 2.11.5 (`src/app.rs:888`) setiap arm yang terjangkau dari `AppHandle` mengembalikan `Ok`; sisanya `unreachable!()`. Cabang `Err` untuk enumerasi monitor adalah **kode mati**. Ia diresolusikan menjadi "nol display" alih-alih panik — dan nol display toh state yang wajib dirender ([FR-106](docs/PRD.md)).

**Alasan kedua — DICABUT oleh audit FR-101, dan koreksinya penting bagi item `AppError`.** Versi pertama ADR ini menyatakan `detail?: string` Appendix D tidak dapat ditepati sebab `ts-rs` akan memancarkan `detail: string | null`, sehingga PRD atau kode harus mengalah. **Salah.** `ts-rs` 12.0.1 menyediakan `#[ts(optional)]` yang justru mengubah `Option<T>` menjadi `t?: T` (`ts-rs-macros-12.0.1/src/optional.rs:9-11,79-90`), menghasilkan persis bentuk Appendix D; dan jalur kedua sudah aktif — dengan `serde-compat` menyala, `#[serde(skip_serializing_if)]` + `#[serde(default)]` menghasilkan field opsional lewat `attr.maybe_omitted && attr.has_default`. **Tidak ada yang perlu mengalah: bentuk PRD dapat ditepati hari ini dengan satu atribut.** Penundaan tetap sah, tetapi **hanya** karena alasan pertama. Item yang melahirkan `AppError` tidak mewarisi dilema apa pun — ia hanya perlu memasang atribut itu. Paragraf berikut dipertahankan sebagai catatan atas apa yang keliru saya tulis: `AppError` di Appendix D bertuliskan `detail?: string` — **opsional**. `ts-rs` akan memancarkan `detail: string | null` tanpa `#[ts(optional)]`. Melahirkan tipe kawat pertama yang bentuknya **tidak cocok dengan PRD** adalah harga yang tidak boleh dibayar diam-diam, dan memilih di antara keduanya adalah keputusan pengguna, bukan keputusan implementer. Menunda lebih jujur daripada menebak.

**Konsekuensi yang harus dibaca terang-terangan.** Appendix D **belum ditepati** untuk perintah pertama ini. Tanda tangan `list_monitors` adalah yang **pertama harus berubah** saat `AppError` mendarat, dan item yang melahirkan `AppError` wajib memutuskan bentuk `detail` secara eksplisit — termasuk apakah PRD atau kode yang mengalah.

**Yang membatalkan keputusan ini.** Perintah pertama yang benar-benar dapat gagal karena sebab yang perlu disampaikan ke pengguna.

### ADR-0039 — Perintah Tauri app-defined tidak ter-ACL; meninjau `capabilities/*.json` tidak akan pernah menampilkannya

| | |
| --- | --- |
| Tanggal | 2026-08-14 |
| Status | Diterima — dicatat sebagai fakta lingkungan, bukan sebagai pilihan |
| Terkait | FR-101, NFR-14, [ADR-0014](decisions.md#adr-0014) |

**Fakta.** Di `tauri` 2.11.5 (`webview/mod.rs:1820`) gerbang ACL hanya menyala untuk perintah `plugin:`, untuk origin remote, atau bila aplikasi punya manifest ACL-nya sendiri. `tauri-build` 2.6.3 (`acl.rs:265`) hanya membuat manifest itu dari `Attributes::app_manifest(...).commands(...)` atau dari direktori `src-tauri/permissions/`. Repo ini punya `build.rs` polos dan tidak punya direktori itu.

**Diverifikasi coordinator pada artefak, bukan disimpulkan dari dokumentasi:** `src-tauri/gen/schemas/acl-manifests.json` **tidak memuat `__app-acl__`**, `src-tauri/permissions/` tidak ada, dan `list_monitors` tidak muncul di `capabilities/*.json` mana pun.

**Mengapa ini dicatat.** Setiap perintah baru **langsung terjangkau dari webview** begitu ia masuk `generate_handler!`, tanpa meninggalkan jejak apa pun di berkas kapabilitas. Konsekuensinya untuk peninjauan: **membaca `capabilities/*.json` bukan cara mengetahui permukaan IPC aplikasi ini** — `generate_handler!` di `src-tauri/src/lib.rs` adalah satu-satunya daftar yang benar. Auditor mana pun yang mengandalkan manifest akan melaporkan permukaan yang lebih kecil daripada yang sesungguhnya ada.

**Mengapa tidak diubah sekarang.** Menyalakan manifest ACL app-defined mengubah model perizinan seluruh aplikasi dan menyentuh setiap perintah masa depan; itu keputusan arsitektur, bukan pekerjaan sampingan item enumerasi display. Yang tidak dapat dibenarkan adalah **tidak menuliskannya**, sebab asumsi diam bahwa "kapabilitas mendaftar semuanya" adalah asumsi yang salah dan tampak benar.

**Ditutup sebagian pada siklus 3 FR-101 — oleh artefak, bukan oleh gerbang.** `src-tauri/commands.inventory.md` kini mendaftar setiap perintah yang terjangkau webview, dan `scripts/command-inventory-guard.js` (dipasang di `prelint`) merah kecuali **tiga** himpunan menamai hal yang sama: definisi `#[tauri::command]` di seluruh `src-tauri/src/`, registrasi `generate_handler!`, dan entri inventaris. Himpunan ketiga bukan redundansi: perbandingan dua-sisi tetap **hijau** untuk arah yang berbahaya, sebab perintah baru ditambahkan ke definisi dan handler dalam commit yang sama sehingga keduanya sepakat justru karena diedit bersamaan. Yang ditambahkan inventaris bukan informasi melainkan **kewajiban menuliskannya** — prosa yang harus ditulis manusia dan muncul di diff. Ini juga jawaban atas [ADR-0014](decisions.md#adr-0014): peninjauan NFR-14 kini punya satu berkas yang benar untuk dibaca.

**Kondisi pembalik, yang wajib diketahui sebelum ada yang mencoba mengerasi ini.** Membuat `src-tauri/permissions/` atau memakai `Attributes::app_manifest(...)` membuat `has_app_acl_manifest` menjadi `true`, dan sejak momen itu **setiap** perintah yang tidak dinamai di sebuah capability ditolak dengan `"Command {} not allowed by ACL"`. Jadi tindakan pengerasan yang tampak paling jelas benar akan **mematikan seluruh IPC aplikasi** sampai setiap perintah didaftarkan. Ditulis di `src-tauri/src/commands/mod.rs` supaya orang yang hendak melakukannya membacanya lebih dulu.

**Yang tetap terbuka: pelingkupan per-window.** `capabilities/main-window.json` berbunyi `"windows": ["main"]`, tetapi pembatasan itu hanya mengikat perintah `plugin:`. Perintah app-defined tidak punya pelingkupan per-window sama sekali, sehingga jendela output FR-102/FR-105 akan dapat memanggil setiap perintah begitu ia ada. Hari ini tak dapat dieksploitasi — bundel output terbukti nol `__TAURI_INTERNALS__` — tetapi PRD §6.3 menyandarkan isolasi itu pada **aturan impor**, dan aturan impor tidak menghalangi kode yang masuk lewat XSS lirik atau template. Menutupnya menuntut keputusan arsitektur dan menjadi persyaratan yang dibawa FR-102/FR-105.

**Yang membatalkan keputusan ini.** Perintah pertama yang menyentuh berkas pengguna, jaringan, atau kredensial — di situ "terjangkau tanpa jejak" berhenti menjadi catatan dan mulai menjadi permukaan serangan.

### ADR-0040 — "First non-primary display" dibaca dari posisi, bukan dari urutan enumerasi; dan penempatan dilakukan dalam piksel fisik sesudah jendela dibuat

| | |
| --- | --- |
| Tanggal | 2026-08-15 |
| Status | Diterima |
| Terkait | FR-102, FR-103, FR-108, [ADR-0037](decisions.md#adr-0037) |

**Keputusan pertama.** Display keluaran dipilih sebagai display non-primer yang sudut kiri-atasnya datang lebih dulu dalam urutan baca virtual-screen: `x` terkecil, seri → `y` terkecil, seri → `id` terkecil.

**Mengapa bukan urutan enumerasi.** Urutan `EnumDisplayMonitors` tidak dijanjikan apa pun — doc FR-101 sendiri sudah memperingatkannya — sehingga `monitors[1]` bukan "display kedua" dalam arti apa pun yang dikenali operator. Posisi adalah properti yang operator **benar-benar atur** di Display Settings, jadi di situlah "pertama" seharusnya dibaca. Menyertakan `id` di kunci pengurutan membuat hasilnya fungsi dari **himpunan** monitor, bukan dari daftarnya: urutan daftar tidak dapat mengubah jawaban, dan justru sifat itu yang membuatnya dapat diuji di `core` dengan nilai `Monitor` rakitan tangan.

**Batas yang dinyatakan, bukan disembunyikan.** Dengan tepat dua display — yaitu kriteria terima FR-102 — **setiap** definisi yang masuk akal memberi jawaban sama; hanya ada satu display non-primer. Definisi ini baru menggigit pada tiga display atau lebih, dan di sana "non-primer paling kiri" adalah tebakan yang berpakaian aturan. Jawaban sesungguhnya untuk kasus itu adalah FR-103 (penetapan manual yang persisten), bukan heuristik yang lebih pintar.

**Dua penolakan, keduanya mengembalikan "jangan buat jendela".** Bila tidak ada display bertanda primer, dan bila tidak ada display non-primer. Menolak lebih baik daripada menutupi layar operator sendiri dengan jendela hitam fullscreen: keadaan "output tidak terbuka" dapat dipulihkan lewat FR-103, sedangkan gambar proyektor yang menimpa layar kontrol di tengah ibadah tidak.

**Keputusan kedua, dan ini menghindari jebakan yang tidak terlihat dari dokumentasi.** Penempatan dilakukan **sesudah** jendela dibuat, dalam **piksel fisik**, tidak pernah lewat builder. `WebviewWindowBuilder::position` menerima koordinat **logis**, dan tao meresolusinya dengan mengonversi pasangan itu memakai faktor skala **setiap** monitor lalu mengambil monitor pertama yang rect-nya memuat hasilnya. Di bawah DPI campuran itu dapat cocok ke display yang salah; bila tidak cocok ke mana pun, posisinya diganti diam-diam menjadi `CW_USEDEFAULT` di display primer. Runtime memang punya perbaikan "fullscreen + position → `Borderless(Some(monitor))`", tetapi ia `#[cfg(any(macos, linux))]` dan **tidak berjalan di Windows** — satu-satunya platform sasaran produk ini.

**Urutan panggilannya menanggung beban.** Buat tersembunyi → `set_position(Physical)` → `set_size(Physical)` → `set_fullscreen(true)` → `show()`. `set_fullscreen(true)` menjadi `Fullscreen::Borderless(None)`, yang tao resolusikan lewat `MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)` — jendela **harus sudah** berada di display sasaran saat itu. Membangun langsung dengan `fullscreen(true)` juga akan memanggil `force_window_active()`, yaitu persis pencurian fokus yang FR-109 larang.

**Control Panel tidak butuh kode sama sekali.** `"center": true` tanpa posisi eksplisit membuat runtime memusatkannya di `primary_monitor()`, yang di Windows adalah `MonitorFromPoint((0,0), MONITOR_DEFAULTTOPRIMARY)`. Separuh FR-102 sudah dipenuhi `tauri.conf.json`. Ini juga mengoreksi premis coordinator: "primary" bukan tebakan murni di Windows — sudut kiri-atas display primer **adalah** titik asal virtual-screen.

**Catatan yang ditambahkan sesudah bukti negatif, 2026-08-15.** Aturan pengurutan di atas menyebut **arah prioritas** — `x` lebih dulu, `y` hanya sebagai pemecah seri — dan arah itu ternyata **tidak dijaga test mana pun** selama satu ronde penuh. Suite 19-test-nya hijau, keenam gate hijau, tetapi mutasi yang menukar kuncinya menjadi `(y, x, id)` lolos dengan nol kegagalan: pada setiap arrangement yang ditulis, dua kandidat selalu berbagi `x` atau berbagi `y`, sehingga kedua urutan menjawab sama. Kasus yang membedakannya — kedua sumbu berselisih — tak pernah ada. Ditutup dengan `a_smaller_x_beats_a_smaller_y_when_the_two_axes_disagree`. Dicatat di sini, bukan hanya di changelog, sebab kalimat aturan di ADR inilah yang separuhnya tak teruji, dan sebab ia contoh bersih dari hal yang berulang di seluruh proyek ini: **implementasi yang benar dan suite yang hijau tidak bersama-sama membuktikan bahwa spesifikasinya teruji.**

**Yang membatalkan keputusan ini.** FR-103, yang menggantikan pemilihan otomatis dengan penetapan manual yang persisten. Sejak saat itu definisi di atas menjadi hanya nilai awal, bukan aturan.

### ADR-0041 — Jendela output dapat memanggil setiap perintah IPC; manifest ACL aplikasi dinyalakan sebagai item tersendiri, sesudah verifikasi runtime FR-102

| | |
| --- | --- |
| Tanggal | 2026-08-15 |
| Status | Diterima |
| Terkait | FR-102, FR-105, NFR-14, [ADR-0013](decisions.md#adr-0013) · [ADR-0018](decisions.md#adr-0018) · [ADR-0039](decisions.md#adr-0039) |

**Fakta, dan ia berhenti menjadi ramalan pada commit ini.** Sejak FR-102 melahirkan jendela `output`, jendela itu **ada**, tidak dinamai oleh capability mana pun, dan dapat memanggil `list_monitors` — beserta setiap perintah yang ditambahkan sesudahnya — tanpa meninggalkan jejak di `capabilities/*.json` maupun di `gen/schemas/`. Pelingkupan `"windows": ["main"]` nyata, tetapi hanya untuk perintah `plugin:`.

**Paparan hari ini kecil dan jujur disebut kecil:** satu enumerasi display yang hanya membaca. Bundel output terverifikasi nol `__TAURI_INTERNALS__`, jadi ia bahkan tidak mengimpor jalan untuk memanggilnya. Risikonya bukan hari ini; risikonya adalah permukaan itu tumbuh menuju ~28 perintah Appendix D — beberapa menerima path dan menulis berkas — sementara ketiadaan pelingkupan tetap **tidak terlihat**.

**Keputusan.** Nyalakan manifest ACL aplikasi, **sebagai item tersendiri**, dan **sesudah** pengguna memverifikasi FR-102 secara runtime.

**Mengapa dinyalakan, bukan ditunda lagi.** Tebingnya nyata dan gagal-tertutup: begitu manifest ada, setiap perintah yang tidak dinamai sebuah capability ditolak dengan `Command {} not allowed by ACL`. Tebing itu **paling murah hari ini, pada satu perintah**, dan menjadi mahal secara monoton pada tiap item berikutnya — di FR-5xx ia menjadi migrasi 25 perintah yang tidak akan ada yang mau menjadwalkannya. Ia juga memperbaiki metode verifikasi NFR-14, yang hari ini melaporkan permukaan **lebih kecil** daripada yang ada.

**Mengapa item tersendiri, bukan di dalam FR-102.** Ia menyentuh `build.rs`, kedua berkas capability, dan guard inventaris; dan regresi `Command not allowed by ACL` **tidak terlihat sampai frontend benar-benar memanggil sesuatu**. Menggabungkannya berarti verifikasi runtime pengguna tidak dapat membedakan cacat penempatan dari cacat perizinan — satu-satunya kesempatan menguji dua hal yang belum pernah diuji akan terbuang untuk membingungkan keduanya.

**Bantahan yang diterima di muka.** Ia tidak membeli apa pun hari ini, dan ia menambah mekanisme build-script yang dapat mematikan seluruh permukaan IPC dalam satu commit yang salah — tanpa jalan keluar runtime, sebab `dynamic-acl` sengaja mati ([ADR-0013](decisions.md#adr-0013)). Bila keberatan itu menang, jalan mundur yang jujur adalah yang murah: setiap perintah menerima `webview: tauri::Webview<R>` dan menolak `webview.label() == "output"` lewat satu helper bersama, dengan aturannya ditulis di `commands/mod.rs` bersebelahan dengan aturan argumen. Itu penegakan lewat konvensi dan tinjauan kode, bukan lewat framework — lebih lemah, tetapi ia meninggalkan jejak di diff, dan itu sudah lebih daripada yang ada sekarang.

**Pemicunya dikoreksi, 2026-08-15, atas bantahan auditor FR-102 yang saya terima.** Kalimat asli menjadwalkan item ACL "**sesudah** verifikasi runtime FR-102". Itu salah, dan salahnya bukan soal urutan melainkan soal jenis: verifikasi runtime menjawab "apakah jendelanya mendarat di proyektor", yang **tidak berhubungan sama sekali** dengan risiko yang keputusan ini kelola. Menjadwalkan pengerasan keamanan pada tonggak yang tidak berkorelasi berarti tanggalnya dapat lewat tanpa siapa pun menyadari bahwa alasannya tidak pernah terpenuhi.

**Pemicu yang benar dinyatakan sebagai batas akhir, bukan sebagai urutan.** Item ACL wajib selesai **sebelum** yang mana pun dari dua hal ini mendarat: (a) item pertama yang merender konten berasal-berkas di jendela output — renderer lirik FR-4xx atau `.aerotpl` §6.7, mana pun lebih dulu; (b) perintah pertama yang menerima path atau menulis berkas — kelompok FR-5xx. Sebelum salah satunya, ADR ini bersama [ADR-0042](decisions.md#adr-0042) adalah kumpulan cacat laten; sesudahnya ia menjadi satu rantai. Verifikasi runtime FR-102 tetap boleh mendahului, dan alasan pemisahan di bawah tetap berlaku — ia hanya berhenti menjadi *pemicu*.

**Yang membatalkan keputusan ini.** Perintah kedua yang mendarat sebelum item ACL dikerjakan — sejak saat itu argumen "paling murah hari ini" berhenti berlaku dan biayanya harus dihitung ulang.

### ADR-0042 — Kedua jendela berbagi satu web origin; empat permukaan yang lahir bersama jendela output ditunda di bawah satu pemicu yang dapat diperiksa

| | |
| --- | --- |
| Tanggal | 2026-08-15 |
| Status | Diterima |
| Terkait | FR-102, FR-104, FR-105, FR-109, PRD §6.3, NFR-13, NFR-14, [ADR-0018](decisions.md#adr-0018) · [ADR-0039](decisions.md#adr-0039) · [ADR-0041](decisions.md#adr-0041) |

**Fakta yang belum pernah tercatat di mana pun, dan ia membatalkan sebuah asumsi diam.** `WebviewUrl::App` menyelesaikan ke base URL aplikasi — di Windows `http://tauri.localhost` (`tauri-2.11.5/src/manager/mod.rs:338-342`), di dev `http://localhost:1420`. Jadi `index.html` dan `output.html` adalah **origin yang sama**. Grep `same-origin|localStorage|BroadcastChannel|tauri.localhost` di seluruh `*.md` repo ini memberi **nol hasil**: tidak ada satu berkas pun yang pernah menyangkal bahwa dua bundel berarti dua sandbox.

**PRD §6.3 memisahkan kode, bukan origin.** Aturan impor di `eslint.config.js:276-320` nyata dan ditegakkan — bahkan secara transitif lewat `src/shared/**` dan atas `import()` dinamis — tetapi ia batas **build-time**. Ia tidak memisahkan `localStorage`, `sessionStorage`, IndexedDB, Cache API, `BroadcastChannel`, maupun `SharedWorker`. Script yang kelak lolos ke dokumen output berada pada origin yang sama dengan Control Panel, membaca dan menulis seluruh penyimpanannya, dan `new BroadcastChannel(...)` memberinya kanal dua arah ke jendela operator **tanpa melewati IPC sama sekali** — sehingga tidak ada capability, ACL, maupun CSP yang menghalanginya. Semuanya sah menurut same-origin policy.

**Aturan yang berlaku mulai sekarang, dan ia murah selama masih murah.** Control Panel **tidak menyimpan apa pun di web storage**. Keadaan yang perlu bertahan hidup melewati restart adalah milik SQLite lewat IPC ([ADR-0008](decisions.md#adr-0008)), yang memang sudah menjadi rencananya; menuliskannya di sini mengubah kebetulan menjadi kewajiban, sebelum ada `localStorage.setItem` pertama yang membuatnya mahal untuk dicabut.

**Tiga permukaan lain yang lahir bersama jendela ini, ditunda tetapi dinamai.**

1. **Tidak ada navigation handler pada jendela mana pun.** `pending.navigation_handler` bernilai `None`, dan `tauri-runtime-wry-2.11.4/src/lib.rs:4899-4906` hanya memasang `with_navigation_handler` bila handler itu ada — tanpa itu wry mengizinkan navigasi ke URL apa pun. CSP tidak menolong: tidak ada direktif yang mengatur navigasi top-level (`navigate-to` tidak diimplementasikan Chromium; `form-action 'none'` hanya menutup submit form). Konsekuensinya menyentuh **NFR-13**: `location = 'https://x/?' + document.body.innerText` mengirim isi slide keluar dan memindahkan layar proyektor ke halaman remote, dari aplikasi yang tidak punya satu baris kode jaringan pun.
2. **Context menu WebView2 dan accelerator browser aktif** di jendela proyektor — `wry-0.55.1/src/lib.rs:1687-1688` memberi keduanya `true` secara default dan `tauri-runtime-wry` tidak pernah memanggil `with_default_context_menus`. Klik kanan memunculkan menu Edge di layar jemaat; Ctrl+S/Ctrl+P membuka dialog simpan/cetak native — jalur tulis filesystem di jendela yang sengaja tidak diberi satu pun permission filesystem. **FR-105** pemiliknya.
3. **Proyektor dicabut → jendela fullscreen berpindah ke layar operator**, tanpa dekorasi, tanpa tombol taskbar, dan tanpa entri Alt-Tab (`skip_taskbar(true)` memasang `WS_EX_TOOLWINDOW`). **FR-104** pemiliknya.

**Urutan yang wajib dipatuhi, dan ia jenis persyaratan yang biasanya hilang.** Hari ini keadaan (3) masih **dapat dipulihkan** semata-mata karena `always_on_top` sengaja tidak diset, sehingga Alt-Tab ke Control Panel menaikkannya di atas permukaan hitam. **FR-109 tidak boleh menambahkan topmost sebelum FR-104 selesai** — melakukannya mengubah "mengganggu" menjadi "tidak dapat dipulihkan tanpa Task Manager", di tengah ibadah, pada satu-satunya layar yang tersisa.

**Mengapa keempatnya ditunda, dan mengapa penundaan itu bukan kelalaian.** Tak satu pun dapat dieksploitasi hari ini: `Renderer.vue` merender satu konstanta string, nol konten tak tepercaya masuk ke jendela mana pun, dan bundel output tidak mengimpor `@tauri-apps/api`, router, store, maupun `fetch`. Yang dibeli penundaan bukan waktu melainkan **pemisahan sinyal**: memperbaiki empat hal ini di dalam FR-102 berarti verifikasi runtime pengguna tidak dapat membedakan cacat penempatan dari cacat pengerasan.

**Pemicunya sama dengan [ADR-0041](decisions.md#adr-0041), dan itu disengaja.** Keempatnya wajib tertutup **sebelum** yang mana pun lebih dulu dari: item pertama yang merender konten berasal-berkas di jendela output (FR-4xx, atau `.aerotpl` §6.7), atau perintah pertama yang menerima path atau menulis berkas (FR-5xx). Satu pemicu untuk lima temuan sebab kelimanya menjadi berbahaya oleh **sebab yang sama** — masuknya konten yang tidak kita tulis — dan memisahkan pemicunya hanya akan membuat sebagian terlewat.

**Bantahan yang diterima di muka.** Empat cacat laten yang ditunda bersama-sama adalah bentuk yang persis diketahui gagal: masing-masing tampak kecil, dan yang menutupnya kelak menghadapi empat sekaligus di bawah tekanan tenggat item lain. Bila keberatan itu menang, yang paling murah dikerjakan lebih dulu adalah navigation handler — tiga baris, menutup satu-satunya di antara keempatnya yang berakibat **keluar dari mesin**.

**Yang membatalkan keputusan ini.** `localStorage.setItem` pertama di kode Control Panel, atau item pertama yang merender konten berasal-berkas — mana pun lebih dulu.

### ADR-0043 — `parse_scripture_ref` dibuat benar-benar murni dengan memisahkan tata bahasa dari pengetahuan kitab; `ScriptureRef` dirancang di sini sebab PRD tidak pernah mendefinisikannya

| | |
| --- | --- |
| Tanggal | 2026-08-15 |
| Status | Diterima |
| Terkait | FR-205, FR-206, FR-207, FR-208, PRD Appendix A (baris 152-175), PRD Appendix D (baris 1640), [ADR-0020](decisions.md#adr-0020) · [ADR-0034](decisions.md#adr-0034) · [ADR-0038](decisions.md#adr-0038) |

**Pertentangan, dinyatakan bukan dibulatkan.** Appendix D:1640 menyebut `parse_scripture_ref` **"pure, unit-tested"**. Tetapi nama dan singkatan kitab tidak hidup di kode — ia hidup di SQLite, di `bible_book_names` dan `bible_book_abbreviations`, keduanya berkunci `language_code`. Fungsi yang harus menanyakan database untuk tahu bahwa `Yoh` adalah Yohanes **tidak murni**, dan menyebutnya murni akan menjadi klaim ketiga di repo ini yang lebih lebar daripada kenyataan sesudah "at build time" ([ADR-0034](decisions.md#adr-0034)) dan bentuk `AppError` ([ADR-0038](decisions.md#adr-0038)).

**Keputusan, dan ia menyelamatkan kalimat PRD alih-alih mengubahnya.** Tanggung jawabnya dipecah dua: tata bahasa yang mengurai `<token kitab> <pasal>:<ayat>` **tanpa mengenal satu kitab pun**, dan resolusi token→`book_id` yang menerima ejaan sebagai **argumen**. Hasilnya "pure, unit-tested" menjadi benar secara harfiah — nol SQL, nol I/O, nol `rusqlite` di modul — dan keempat kriteria terima FR-205 dapat diuji tanpa satu baris SQL. Ini kali pertama pertentangan PRD di repo ini ditutup tanpa mengubah PRD dan tanpa menerima klaim yang salah; keduanya mungkin karena kalimat itu ternyata menggambarkan **desain yang benar**, hanya saja desain itu belum ada.

**`ScriptureRef` dirancang di sini, dan itu perlu dikatakan terang-terangan.** Tipe ini muncul dua kali di Appendix D (baris 1640, 1641) dan **tidak pernah didefinisikan di mana pun** dalam PRD. Bentuknya: rekaman datar, interval tertutup, kedua ujung selalu terisi — `Mzm 23` menjadi `23/None → 23/None`, `Kej 1:1-2:3` menjadi `1/Some(1) → 2/Some(3)`.

**Mengapa datar, bukan enum bertag.** Keempat bentuk FR-205 adalah interval yang sama dilihat dari empat sudut. Konsumen sesungguhnya adalah FR-207, yang akan menjadi satu range-scan `bible_verses` terurut `(chapter, verse)`; enum bertag memaksa **setiap** konsumen — di Rust dan di TypeScript — menurunkan ulang interval itu. Menormalkan di parser membuat penurunan itu terjadi sekali, di tempat yang diuji.

**Invarian both-or-neither, dan lubang yang sengaja ditinggalkan terbuka dengan peringatan.** `start_verse` dan `end_verse` `None` bersama atau `Some` bersama; `None` berarti "seluruh ayat pasal itu". Invarian ditegakkan di **parser**, bukan di tipe, sebab field tipe kawat harus publik. Konsekuensinya nyata dan sudah ditulis di komentar modul: item yang kelak menambahkan `Deserialize` untuk FR-207 **wajib memvalidasi ulang**, sebab nilai yang datang dari frontend tidak pernah lewat parser. Invarian yang dijaga oleh satu jalur masuk berhenti dijaga begitu jalur kedua dibuka.

**Keputusan terbaik item ini adalah menghapus sebuah parameter.** `BookIndex::build(language_code, ejaan)` memilih bahasa **sekali, saat konstruksi**, sehingga pemanggil tidak punya cara mengekspresikan kesalahan "baris Indonesia + kode bahasa Inggris". Bentuk yang tampak lebih wajar — `resolve(token, language_code, rows)` — menyediakan kesalahan itu di **setiap** call site dan mendeteksinya di **nol**: ia akan me-resolve `John` terhadap baris Indonesia, menjawab `None`, dan FR-206 mengubah `None` itu menjadi full-text search yang terlihat persis seperti salah ketik operator. Kelas cacat yang paling mahal di produk ini bukan yang menjatuhkan aplikasi, melainkan yang **berhasil dan salah tanpa gejala**; API yang tidak dapat menyatakannya adalah pertahanan yang lebih kuat daripada test mana pun.

**Ambigu tidak dilebur dengan tidak-dikenali.** FR-206 menuntut keduanya jatuh diam-diam ke full-text search, jadi di permukaan `parse_scripture_ref` keduanya `None`. Tetapi hanya satu yang menandakan **data rusak**, jadi perbedaannya dipindah ke tempat yang dapat ditindaklanjuti: `BookMatch::{Unique, Ambiguous, Unknown}` per query, `ambiguous_spellings()` yang menyebutkan setiap tabrakan secara **terurut** — diagnostik tak berurutan adalah diagnostik yang tak dapat diassert — dan `is_empty()` yang memisahkan "belum ada Alkitab untuk bahasa ini" dari "operator salah ketik". Nol tabrakan untuk data sehat; tidak nol adalah fakta tentang Alkitab yang **diimpor**, layak dilaporkan sekali di FR-208 dan bukan sekali per ketukan tombol.

**Saat tabrakan terjadi, tidak ada yang menang.** Bukan `book_id` terkecil, dan bukan "nama penuh mengalahkan singkatan". Menjawab Judges kepada orang yang mengetik `Jud` untuk Jude benar separuh waktu **tanpa cara siapa pun menyadarinya** — dan layar yang salah di tengah khotbah adalah kegagalan yang tidak dapat ditarik kembali. Skema mengizinkan tabrakan itu ada (`bible_book_abbreviations` berkunci `(book_id, language_code, abbreviation)`), jadi ia bukan hipotesis.

**Rentang terbalik ditolak, tidak ditukar diam-diam.** `Yoh 3:18-16` mengembalikan `None`. Menukarnya berarti menaruh perikop yang **tidak diminta** di layar; menolaknya berarti operator melihat hasil full-text search dan mengetik ulang. Yang kedua terlihat; yang pertama tidak.

**Batas yang dinyatakan, bukan disembunyikan.** Parser tidak memvalidasi terhadap `bible_books.chapter_count` maupun keberadaan ayat — itu butuh data, dan data adalah FR-207/FR-208. `Mzm 151:1` parse dengan senang hati. **Dan konsekuensi terbesarnya:** nol kode produksi memanggil `BookIndex::build` hari ini, jadi FR-205 dapat diuji sepenuhnya tetapi **belum dapat dipakai operator**. Itu bukan cacat item ini; itu urutan yang PRD pilih. Yang tidak boleh terjadi adalah menandainya `done` lalu lupa bahwa jalur pertamanya belum pernah dijalankan dari ujung ke ujung.

**Perintah IPC-nya sengaja tidak dibuat.** Tanpa FR-207 dan tanpa data kitab, ia tidak dapat dipanggil siapa pun secara berguna — ia hanya menambah permukaan IPC tanpa menambah kemampuan, dan memicu kondisi pembatal [ADR-0041](decisions.md#adr-0041) secara cuma-cuma. Nama fungsi `core`-nya sengaja dibuat sama dengan nama perintah Appendix D supaya perintah kelak menjadi pembungkus satu baris.

**Yang membatalkan keputusan ini.** FR-207, yang menjadi konsumen nyata pertama `ScriptureRef` — bila range-scan-nya ternyata tidak berbentuk seperti yang diandaikan di sini, bentuk datar itu harus dihitung ulang sebelum ada data gereja yang tersimpan memakainya.

### ADR-0044 — Normalisasi Unicode (NFC/NFD) tidak dibeli sekarang; kontrak `BookIndex::build` menuntut NFC dan FR-208 yang menegakkannya

| | |
| --- | --- |
| Tanggal | 2026-08-26 |
| Status | Diterima |
| Terkait | FR-205, FR-206, FR-208, NFR-16, NFR-28, [ADR-0043](decisions.md#adr-0043) |

**Temuan yang memaksa keputusan ini (audit FR-205, W2).** `normalise_spelling` melipat kasus dengan `char::to_lowercase` per karakter dan **tidak menormalisasi Unicode sama sekali**. Tiga divergensi nyata: `"Éxodo"` sebagai U+00C9 dan sebagai `E`+U+0301 menghasilkan kunci **berbeda**; `char::to_lowercase` tidak menerapkan aturan sigma-akhir Yunani yang `str::to_lowercase` terapkan, sehingga `ΙΩΑΝΝΗΣ` melipat ke `ιωαννησ` sementara ejaan tersimpan berakhir `ς`; dan `İ` U+0130 melebar menjadi `i`+U+0307.

**Skenario kegagalannya diam, seperti seluruh keluarga temuan di item ini.** Alkitab Spanyol diekspor dari perkakas macOS, di mana NFD adalah default filesystem. `Éxodo` tersimpan NFD. Operator di Windows mengetik `Éxodo` — keyboard Windows memancarkan NFC — dan mendapat `Unknown` → `None` → FR-206 jatuh ke pencarian teks penuh. Nol diagnostik menyala. Lebih buruk: FTS5 dikonfigurasi `remove_diacritics 2` (`001_initial_schema.sql:288`), jadi fallback itu mungkin **menemukan sesuatu** — dan operator melihat hasil, hanya bukan hasil yang benar.

**Keputusan. Crate normalisasi Unicode tidak ditambahkan sekarang.** Sebagai gantinya kontrak `BookIndex::build` menyatakan bahwa ejaan **wajib NFC**, dan FR-208 — satu-satunya jalan data kitab masuk — yang menegakkannya di batas impor.

**Alasan pertama, dan ia terukur meski datanya belum ada.** Kedua bahasa yang FR-205 sebut namanya **tidak memuat satu karakter non-ASCII pun** dalam nama kitab bakunya. Enam puluh enam nama Indonesia — Kejadian, Keluaran, Hakim-hakim, Kidung Agung, Wahyu — dan enam puluh enam nama Inggris seluruhnya ASCII, dan pada ASCII, NFC dan NFD **identik** dan `to_lowercase` per-karakter **sama dengan** case folding. Jadi untuk seluruh cakupan yang item ini janjikan, gigitan W2 adalah **nol**. Ia mulai menggigit pada bahasa ketiga yang belum ada jadwalnya.

**Alasan kedua, NFR-16.** Repo ini sudah membayar disiplin itu berkali-kali: `ts-rs` dev-only dengan `cargo tree --edges normal` diverifikasi nol, dan `Cf` ditutup lewat tabel `matches!` justru untuk menghindari crate properti Unicode. Membeli crate normalisasi untuk masalah yang **belum dapat terjadi** akan membalik prinsip itu pada kasus pertama yang lemah.

**Yang membuat penundaan ini jujur dan bukan penundaan biasa: ia memindahkan kewajiban, bukan menghapusnya.** Kontrak `build` menyebutkan NFC, jadi FR-208 tidak dapat membacanya sebagai API yang sudah mengurus semuanya — kekeliruan yang persis dicegah W4 dari audit yang sama. Dan bila FR-208 memilih **tidak** menormalisasi, itu keputusan yang harus diambil di sana secara sadar dan tercatat, bukan diwariskan diam-diam.

**Bantahan yang diterima di muka.** "Wajib NFC" yang ditegakkan oleh konvensi dan tinjauan kode adalah penegakan yang lebih lemah daripada kode, dan repo ini sendiri sudah menuliskan mengapa ([ADR-0041](decisions.md#adr-0041)). Bila FR-208 ternyata tidak dapat menegakkannya dengan murah, jawaban yang benar adalah membeli crate itu di sana — bukan membiarkan kontrak menjanjikan sesuatu yang tak seorang pun periksa.

**Yang membatalkan keputusan ini.** Impor pertama untuk bahasa yang nama kitabnya memuat karakter non-ASCII — Spanyol, Portugis, Vietnam, Yunani, atau bahasa daerah Indonesia mana pun yang memakai diakritik. Sejak saat itu argumen "gigitannya nol" berhenti berlaku dan biayanya harus dihitung ulang di item yang sama.

### ADR-0045 — Dokumen template ditolak secara default lewat `deny_unknown_fields` menyeluruh; `name` dan `author` sengaja tetap teks manusia, dan kewajibannya dipindahkan ke konsumen

| | |
| --- | --- |
| Tanggal | 2026-08-26 |
| Status | Diterima |
| Terkait | FR-401, FR-402, FR-403, FR-405, FR-408, FR-409, NFR-15, NFR-28, NFR-33, [ADR-0018](decisions.md#adr-0018) · [ADR-0042](decisions.md#adr-0042) · [ADR-0043](decisions.md#adr-0043) |

**Konteks yang membuat item ini berbeda dari dua item sebelumnya.** FR-205 harus merancang `ScriptureRef` sebab PRD menyebutnya dua kali tanpa pernah mendefinisikannya; SETUP-05 dan FR-102 masing-masing menemukan pertentangan PRD. FR-401 tidak menemukan satu pun: Appendix B mendefinisikan dua puluh tipe secara lengkap, dan kode ini memetakannya 1:1. Yang harus diputuskan seluruhnya adalah hal yang Appendix B **tidak** atur.

**Keputusan pertama, dan ia yang paling menentukan. `deny_unknown_fields` dipasang di setiap struct dan setiap enum bertag — sebelas dan tiga.** Menolak `type` yang tidak dikenal **tidak cukup**: serde secara default **mengabaikan** field asing, sehingga `{"type":"background","onLoad":"alert(1)"}` lolos diam-diam dan duduk di dokumen sampai renderer menyebarnya ke DOM. Dan DOM itu, menurut [ADR-0042](decisions.md#adr-0042), berbagi **satu web origin** dengan Control Panel — jadi muatan yang menumpang di sana mendapat `localStorage`, `IndexedDB`, dan `BroadcastChannel` panel operator.

**Harganya dinyatakan, bukan disembunyikan:** menambah field ke Appendix B menjadi perubahan yang **memutus kompatibilitas** — build lama menolak dokumen baru mentah-mentah. Itu diterima justru karena `schema_version` ada untuk kasus itu, dan menolak keras di batas kepercayaan adalah perilaku yang diinginkan.

**Keputusan kedua. `schema_version` dibaca lewat probe longgar satu field sebelum deserialisasi ketat.** Tanpa urutan itu, dokumen versi 2 gagal sebagai `unknown field` dan operator diberi tahu berkasnya **rusak** padahal ia hanya lebih baru. FR-708 menuntut perilaku ini untuk `.aero`; ia diterapkan di sini meski FR-708 tidak mencakup template, sebab kerugiannya identik — orang membuang berkas yang sebenarnya baik-baik saja.

**Keputusan ketiga, dan ia sengaja terbelah. `name` dan `author` menolak pemisah path, tetapi menerima metakarakter markup.**

Keduanya satu-satunya string berbasis **blocklist** di dokumen ini; setiap string lain dikunci allowlist sempit — warna ke heks, id ke UUID, font ke alfanumerik, `d` ke kelas karakter perintah path.

**Pemisah path (`/`, `\`, `:`, dan `..` di mana pun) ditolak** sebab `name` adalah sumber paling alami untuk nama berkas ekspor `.aerotpl` (FR-409), dan **tidak ada nama template sah yang memuat pemisah path**. Membuatnya tak terwakili hari ini lebih murah daripada mengandalkan FR-409 mengingat untuk membersihkannya.

**`<`, `>`, `&`, `"`, `'` diterima, dan ini keputusan sadar.** "Natal & Tahun Baru" adalah nama template yang wajar; menolaknya membuat validator berdebat dengan penggunanya soal tanda baca. Pertahanan terhadap markup adalah **escaping di batas render**, bukan penyempitan di sini — menaruhnya di sini memberi rasa aman palsu **sekaligus** menolak masukan yang benar.

**Konsekuensinya dipindahkan, bukan dihapus, dan itu bagian terpenting keputusan ini.** Tabel *"What this module deliberately does not check"* di kepala modul kini menyebut kewajiban ini bersama tiga lainnya: konsumen **wajib** menempatkan `name` dan `author` di text node dan tidak pernah merangkainya menjadi HTML. Tanpa baris itu, pembaca yang baru saja membaca "colours, ids, font names and SVG path data are matched against allowlists" akan menyimpulkan `name` juga aman — dan kesimpulan itu masuk akal, salah, dan tidak terbantahkan di mana pun.

**Keputusan keempat. Tabel `Cf` dipromosikan menjadi milik bersama, bukan disalin.** Daftar dua belas karakter arah yang semula ditulis tangan **tidak konsisten dengan kriterianya sendiri**: ia memuat LRM dan RLM tetapi tidak U+061C ALM, yang melakukan hal identik dan yang tabel `Cf` di parser kitab sudah mencantumkan. Argumen asli untuk tidak menyalin — "tabel kedua yang harus dijaga sinkron, tanpa manfaat" — **benar**, dan justru itu argumen untuk berbagi. Tabel kini tinggal di `models/text.rs`, privat, sebab "code point mana yang `Cf`" adalah fakta tentang **Unicode**, bukan tentang kitab maupun template.

**Predikat template sengaja lebih luas daripada predikat parser kitab** — `Cc` ∪ `Cf` ∪ {U+2028, U+2029} ∪ empat Hangul filler — dan doc-nya menyatakan bahwa keduanya **bukan** predikat yang sama, supaya tidak ada yang menggabungkannya lalu diam-diam mengubah penghapusan separator di parser kitab. Hangul filler ikut sebab `is_alphanumeric()`-nya **true** dan render-nya kosong, sehingga sebuah nama yang seluruhnya filler lolos aturan "must not be blank" dan tetap menghasilkan baris kosong di picker — persis kerugian yang aturan itu dibuat untuk mencegah.

**Yang tidak dibeli, dan siapa pemiliknya.** Grammar path SVG penuh — hanya kelas karakternya yang ditegakkan di sini (FR-403). Penyelesaian `media_id` terhadap filesystem (NFR-15/FR-409). Enumerasi font terpasang (FR-402). Batas byte pada `validate_template`, yang menerima dokumen sudah-terdeserialisasi sehingga tidak tahu ukuran aslinya — diukur tidak dapat diperkuat menjadi amplifikasi memori, sebab setiap `Vec` dan setiap `String` punya batasnya sendiri dan dokumen yang lolos otomatis ≈210 KB.

**Konsekuensi jujur yang harus dibaca terang-terangan.** FR-401 dapat diuji sepenuhnya tetapi **belum dapat dipakai operator**: nol kode produksi memanggil `parse_template`, tidak ada renderer, dan perintah `save_template`/`get_template` sengaja tidak dibuat sebab perintah kedua memicu kondisi pembatal [ADR-0041](decisions.md#adr-0041) tanpa menambah kemampuan apa pun.

**Yang membatalkan keputusan ini.** Renderer pertama (FR-402/403/405) — ia menjadi konsumen nyata pertama, dan bila ia ternyata tidak dapat memenuhi kewajiban text-node yang keputusan ketiga pindahkan kepadanya, penyempitan `check_display_text` harus dihitung ulang di sana, bukan diwariskan lagi.

---

### ADR-0046 — `split_slides` tidak menerima argumen `canvas` meski PRD §6.8 menyebutnya; tinggi kanvas terbukti saling meniadakan, dan sumbu horizontal sengaja dinyatakan di luar cakupan

| | |
| --- | --- |
| Tanggal | 2026-08-26 |
| Status | Diterima |
| Terkait | FR-310, FR-312, FR-313, FR-402, FR-403, FR-405, FR-406, FR-407, US-16, R5, PRD §6.8, Appendix B, Appendix D, [ADR-0043](decisions.md#adr-0043) · [ADR-0045](decisions.md#adr-0045) |

**Penyimpangan dari kata-kata PRD, dinyatakan terbuka.** §6.8 menyebut pemecahan slide "a pure Rust function of `(content, template text slot, canvas)`". Fungsi yang ditulis mengambil dua yang pertama dan **menolak yang ketiga**. Ini bukan kelalaian dan bukan penyederhanaan sementara.

**Alasannya aljabar, bukan selera, dan ia dapat diperiksa sebelum kode dibaca.** Appendix B mendefinisikan `size` sebagai **pecahan tinggi kanvas** dan `line_height` sebagai **kelipatan ukuran huruf** (tak berdimensi); `Rect` ternormalisasi terhadap kanvas yang sama (FR-406). Maka pada kanvas setinggi `H` piksel, satu kotak baris adalah `size · H · line_height` piksel dan slotnya `box.h · H` piksel, sehingga jumlah kotak baris yang muat adalah

```text
(box.h · H) / (size · H · line_height)  =  box.h / (size · line_height)
```

— `H` **hilang eksak**, bukan kira-kira hilang. Lebar kanvas tidak pernah masuk sebab pemecahan hanya terjadi di batas baris, jadi tidak ada yang diukur secara horizontal.

**Mengapa parameter yang terbukti diabaikan lebih berbahaya daripada tidak ada parameter.** Menerimanya akan mengundang pemanggil percaya jawabannya bergantung padanya, lalu **menghitung ulang pemecahan per resolusi output** — dan itu persis mode gagal yang risiko R5 namai: preview dan output tidak lagi sepakat. Menghapus parameternya membuat "hitung ulang per resolusi" tidak dapat diekspresikan. `build_slides` (Appendix D) tetap menerima `CanvasSize` dari frontend untuk urusannya sendiri; ia cukup tidak meneruskannya. Keputusan ini dikunci oleh test yang gagal **kompilasi** bila dilanggar (`split_slides_takes_no_canvas_argument`), bukan oleh komentar.

**Yang tidak meniadakan diri, dan kewajibannya dipindahkan ke renderer.** Sumbu horizontal tidak dapat dihitung murni: lebar baris menuntut advance glyph dari font yang crate ini tidak pernah lihat, dan `letter_spacing` adalah pecahan **tinggi** kanvas yang diterapkan sepanjang **lebar**, sehingga rasio aspek tidak meniadakan diri sebagaimana `H` meniadakan diri. Itulah sebabnya PRD mengurung pemecahan pada batas baris. Konsekuensinya mengikat **renderer**, bukan modul ini, dan ia persyaratan yang wajib tidak hilang: **renderer yang soft-wrap satu baris panjang menaruh lebih banyak kotak baris daripada yang dihitung di sini dan memecahkan seluruh pemecahan.** Baris wajib di-layout apa adanya — `white-space: pre` — dan baris terlalu lebar adalah urusan indikator overflow FR-407, bukan urusan reflow saat tayang. FR-402, FR-403 dan FR-405 mewarisi kewajiban ini.

**Penyusutan huruf terjadi sekali untuk seluruh item, tidak pernah per slide.** Argumen visualnya jelas (dua slide berturut-turut dengan ukuran huruf berbeda terlihat dari baris belakang), tetapi yang menentukan adalah argumen struktural: per-slide bersifat **sirkular** — ukuran menentukan kapasitas, kapasitas menentukan isi slide, isi slide menentukan ukuran — sebuah fixpoint tanpa jaminan jawaban tunggal, dan itu fondasi yang buruk bagi klaim "identik di preview dan output". Ukuran dipilih **sekali**, lewat bentuk tertutup `height / (n · line_height)`: tanpa loop, tanpa step size, tanpa toleransi yang dapat berhenti di tempat berbeda pada mesin berbeda.

**`max_lines` adalah plafon dan geometri hanya boleh menurunkan.** Menyusut boleh dipakai untuk menghindari pemecahan — itu arti Appendix B "auto-shrink floor *before* splitting" — tetapi tidak pernah untuk menaruh baris kelima di slot empat baris. Itu bukan memuat teks, itu merancang ulang slide milik penulis template.

**Klaim kriteria terima FR-310 yang tidak dapat ditutup oleh FR-310.** "Identically in preview and output" mengandaikan ada **dua** renderer untuk dibandingkan, dan nol renderer ada hari ini. Yang FR-310 buktikan adalah determinisme dan bahwa hanya ada **satu** fungsi; kesamaan preview/output menjadi kewajiban FR-405 untuk memanggil fungsi yang sama. Menyatakannya di sini supaya tidak ada yang kelak membaca FR-310 `done` sebagai bukti kesamaan itu.

**Yang membatalkan keputusan ini.** Bukti bahwa `H` tidak meniadakan diri — yaitu bila Appendix B kelak mendefinisikan satu ukuran tipografi dalam satuan absolut (pt, px) alih-alih pecahan kanvas. Saat itu argumen di atas runtuh seluruhnya dan `canvas` wajib masuk sebagai argumen, bukan ditambahkan diam-diam di dalam.

---

### ADR-0047 — Model baris `split_slides` adalah model renderer, bukan `str::lines`; dan dua keadaan terpotong yang tidak dapat dideteksi dinyatakan di kontrak alih-alih ditambal

| | |
| --- | --- |
| Tanggal | 2026-08-26 |
| Status | Diterima |
| Terkait | FR-310, FR-312, FR-401, FR-402, FR-403, FR-405, FR-407, FR-501, FR-703, NFR-28, US-16, [ADR-0042](decisions.md#adr-0042) · [ADR-0045](decisions.md#adr-0045) · [ADR-0046](decisions.md#adr-0046) |

**Pola yang auditor namai, dan namanya lebih berguna daripada temuan yang melahirkannya.** [ADR-0045](decisions.md#adr-0045) menulis bentuk "invarian berhenti dijaga ketika jalur masuk kedua dibuka". FR-310 memunculkan bentuk yang **berbeda dan lebih halus**: *invarian yang dijaga dengan sempurna atas model yang lebih sempit daripada dunia yang harus dijaganya*. `overflows` **sound** di dalam modelnya sendiri — kapasitas dihitung ulang dari `font_size` yang benar-benar dikembalikan, bukan dari `size` nominal — tetapi modelnya mengandaikan satu entri `str::lines` sama dengan satu kotak baris. Di luar andaian itu ada keadaan terpotong yang melaporkan `overflows == false`.

**W1 ditutup di KODE, sebab pemicunya adalah konten.** `str::lines` memecah pada `\n` saja, sementara `white-space: pre` — mode render yang modul ini sendiri **wajibkan** ([ADR-0046](decisions.md#adr-0046)) — juga memecah pada U+000B, U+000C, U+0085, U+2028, U+2029 dan CR tunggal. Setiap satu yang terlewat adalah kotak baris di slide yang tidak pernah dihitung, di slide yang sudah penuh, dengan `overflows` gelap. Dan rutenya sudah ada di aplikasi ini: PowerPoint mengekspor soft break sebagai U+000B (FR-501), tempelan dari browser atau PDF membawa U+2028, dan `.aero` tulisan program lain membawa apa pun yang ia suka (FR-703) — yaitu argumen **paling tidak tepercaya** dari ketiganya. Himpunan yang dipilih adalah karakter mandatory-break UAX #14 kelas BK, CR, LF **dan NL**; tujuh, bukan dua. (Keempat kelas, bukan tiga: BK memberi U+000B, U+000C, U+2028, U+2029; CR memberi U+000D; LF memberi U+000A; dan **U+0085 adalah kelas NL tersendiri**. Menyebut tiga kelas saja akan menghasilkan enam karakter, dan pembaca yang memverifikasi himpunan terhadap aturan itu akan menyimpulkan U+0085 tidak termasuk lalu menghapusnya.)

**Kelas bahaya ini sudah dikenali di crate yang sama, dan itu yang membuatnya wajib ditutup di kode.** `template.rs` sudah menuliskan bahwa U+2028 dan U+2029 "break a line without being control characters, so `char::is_control` walks straight past them". `slide.rs` adalah satu-satunya tempat di crate yang belum menutupnya. Menutup separuh crate terhadap bahaya yang crate itu sendiri sudah namai adalah utang, bukan keputusan.

**Pemisah menjadi batas: yang tidak hilang adalah teks di kedua sisinya, sementara pemisahnya sendiri dikonsumsi.** Ketujuhnya dapat memiliki teks di kedua sisi, dan teks ibadah tidak boleh hilang. Mengonsumsi pemisah sebagai batas menjaga kedua paruh janji tetap bersama: sebuah slide tidak lagi dapat **memuat** pemisah, sehingga yang dihitung di sini dan yang di-layout renderer adalah baris yang sama. CRLF dihitung **satu** pemisah; pasangan lain — `\n\r` termasuk — adalah dua pemisah dengan baris kosong di antaranya, sebagaimana renderer menampilkannya. Di mana renderer tidak sepakat (browser terbelah soal U+000B dan U+0085 di bawah `white-space: pre`), modul ini **menghitung** break-nya: menghitung break yang diabaikan renderer memberi satu batas slide yang operator lihat dan dapat terima; melewatkan break yang dihormati renderer memotong baris di depan jemaat. `floor` di `lines_that_fit` condong ke arah yang sama karena alasan yang sama.

**W2 ditutup di KONTRAK, bukan di kode, dan alasannya adalah bukti yang tidak ada.** Kapasitas menyamakan tinggi kotak baris dengan tinggi **tinta**. Pada `line_height` di bawah kira-kira 1,2 half-leading CSS menjadi **negatif**: asender baris pertama dan desender baris terakhir dicat di luar slot sementara N kotak baris tetap "muat" menurut aritmetika, dan `overflows` tetap `false`. Ini bukan kasus validasi yang terlewat — `validate_template` **menerima** `line_height` sampai 0,1 — melainkan dokumen sah yang tintanya keluar dari kotaknya. Ia dinyatakan alih-alih ditambal sebab jawaban yang benar bukan milik crate ini: ia menuntut metrik font yang tidak ada di sini, **dan** satu keputusan renderer yang belum diambil — apakah slot teks **memangkas** isinya atau membiarkan tinta meluber. Margin tinta yang dikarang di sini adalah angka tanpa bukti yang tidak dapat dinaikkan pemanggil mana pun. Kewajibannya **berpindah** ke FR-402/403/405, sejajar dengan overflow horizontal yang memang sudah dinyatakan tidak tercakup. Yang tidak boleh adalah keduanya tidak terjadi: sinyal yang dipercaya wajib menyala, **atau** kontraknya wajib mengatakan ia tidak menutupinya.

**W3 — untuk KETIGA kalinya di repo ini, siapa yang membatasi input tidak dinyatakan.** Sesudah W4 FR-205 dan S2 FR-401, FR-310 mengulanginya: nol batas atas pada panjang `content`, jumlah barisnya, atau jumlah slide yang ia hasilkan. Yang membuat instansi ini lebih tajam: pengalinya adalah milik **template**, bukan hanya milik konten — `box.h: 0.0` adalah slot yang sah (layer tak terlihat), ia memuat nol kotak baris, dan itu memaksa satu baris per slide. Tempelan 2 MB berisi sejuta baris pendek menjadi sejuta `String`. Batasnya tidak dipasang di level ini dengan sengaja: angka yang benar berbeda per call site — satu bait lagu, satu deck impor utuh, preview Builder yang digambar ulang tiap ketukan tombol — dan angka yang dipilih di sini adalah angka yang tidak dapat dinaikkan oleh pemanggil yang membutuhkannya lebih besar. Pembatasan menjadi kewajiban FR-312 dan jalur parsing berkas, keduanya sudah punya batas ukuran. Ketiga kalinya sebuah pelajaran muncul adalah saat ia berhenti menjadi kebetulan: **setiap fungsi murni yang menerima input tak tepercaya wajib menyebutkan siapa yang membatasinya.**

**`max_lines: 0` dibaca sebagai *tak dideklarasikan*, bukan sebagai *nol baris muat*.** `validate_template` menolaknya — minimumnya 1 — jadi ia hanya tiba dari dokumen yang melewati validasi. Membacanya harfiah akan melaporkan `overflows` selamanya pada slot yang lapang, dan itu lebih buruk daripada kedengarannya: kontennya selamat, tetapi **alarm yang selalu menyala adalah alarm yang tidak dibaca** — sementara dua bahaya yang modul ini sama sekali tidak dapat deteksi (baris terlalu lebar, tinta di luar kotak baris) justru bergantung pada alarm ini dipercaya.

**`font_size` dinyatakan rentangnya, tidak di-clamp.** Dokumen yang melewati validasi dapat mendeklarasikan `size: 0` atau `size: -0.05`, dan field ini melaporkan angka itu kembali alih-alih mengarang angka yang masuk akal di tempatnya. Yang berlaku sebagai gantinya adalah **pasangannya**: setiap kali `fontSize` tidak positif dan finit, `overflows` bernilai `true` untuk konten apa pun yang punya baris — sebab tidak ada kotak baris yang muat pada ukuran demikian. Frontend membaca `fontSize` non-positif sebagai template rusak untuk dilaporkan, bukan sebagai ukuran untuk dicat.

**Dan teks slide itu sendiri tidak tepercaya.** Kalimat itu ditulis sebagai doc `///` pada `slides` justru supaya ts-rs menyalinnya ke `src/shared/bindings/SlideSplit.ts`, tempat pembaca frontend akan membacanya: isinya adalah apa pun yang dimuat `.aero` tulisan program lain, impor PPTX/PDF, atau tempelan operator, dan membangun markup darinya — `innerHTML`, `v-html`, template string — menjalankan skrip berkas itu di origin yang dibagi Control Panel ([ADR-0042](decisions.md#adr-0042)). Jawaban yang aman kebetulan juga jawaban yang benar: dengan `white-space: pre`, `\n` di antara dua baris **adalah** pemisah baris, sehingga `el.textContent = slides[i]` merender persis sebagaimana dimaksud dan tidak ada `<br>` yang dibutuhkan di mana pun.

**Yang membatalkan keputusan ini.** Renderer pertama (FR-402/403/405). Ia adalah pihak yang kepadanya W2 dan overflow horizontal dipindahkan, dan bila ia ternyata tidak dapat menyalakan sinyal untuk keduanya, yang harus dihitung ulang adalah keputusan "nyatakan di kontrak" — bukan diwariskan sekali lagi ke item berikutnya.

---

### ADR-0048 — Ronde mutasi wajib menyentuh mtime sesudah pemulihan dan menjalankan ulang gate penuh; pemulihan yang terbukti benar lewat sha256 tetap dapat meninggalkan hijau yang palsu

| | |
| --- | --- |
| Tanggal | 2026-08-27 |
| Status | Diterima |
| Terkait | FR-203, FR-205, FR-310, FR-401, GATE-G10, NFR-32, [ADR-0045](decisions.md#adr-0045) · [ADR-0047](decisions.md#adr-0047) |

**Mengapa ini ADR dan bukan catatan.** Seluruh disiplin repo ini bertumpu pada satu aturan: suite hijau tidak membuktikan apa pun sampai mutasi dijalankan. Empat item terakhir ditutup atas dasar itu, dengan enam puluh lebih mutasi dijalankan dan nol lolos. Temuan di bawah menyerang **kepercayaan pada ronde mutasi itu sendiri**, bukan pada satu item — jadi ia wajib duduk di tempat yang dibaca sebelum ronde berikutnya, bukan di baris changelog yang akan tenggelam.

**Cacatnya.** Prosedur ronde mutasi di repo ini adalah: salin berkas produksi ke `%TEMP%`, catat sha256, mutasi, jalankan test, **pulihkan dari salinan pristine**, buktikan pemulihan dengan sha256 yang sama. Prosedur itu benar sejauh yang ia klaim — dan tetap dapat berakhir dengan hijau yang palsu.

Sebabnya: penyalinan yang **mempertahankan mtime** (`shutil.copy2`, `cp -p`, `Copy-Item` dalam beberapa bentuk) memulihkan isi berkas **beserta waktu modifikasinya yang lama**. Fingerprint `cargo` berbasis mtime, bukan berbasis isi. Maka sesudah pemulihan, cargo menyimpulkan tidak ada yang berubah dan **tidak membangun ulang** — dan gate penuh berikutnya menjalankan **binary mutan yang masih tersimpan di `target/`**, di atas pohon sumber yang sha256-nya terbukti pristine.

**Bentuk kegagalannya asimetris, dan itu yang membuatnya berbahaya.** Ia terdeteksi pada ronde FR-203 hanya karena arah kebetulannya menguntungkan: gate melaporkan satu test **GAGAL** padahal sumbernya sudah benar, sehingga ada yang harus dijelaskan. Arah sebaliknya tidak akan meminta penjelasan apa pun — mutan yang **tidak** memerahkan test apa pun akan dilaporkan sebagai "gate hijau, item siap ditutup", dan tidak ada satu pun sinyal yang muncul. Kepercayaan diletakkan pada sha256, dan sha256-nya memang benar; yang salah adalah menyimpulkan "sumber pristine" berarti "yang dijalankan pristine".

**Keputusan.** Setiap ronde mutasi di repo ini wajib, sesudah memulihkan berkas produksi:

1. **Menyentuh mtime** setiap berkas yang dipulihkan (`touch`, atau menyalin dengan cara yang tidak mempertahankan mtime), dan
2. **Menjalankan ulang gate penuh** — bukan hanya suite yang relevan — lalu melaporkan angkanya.

Kedua langkah, bukan salah satu. `touch` tanpa gate ulang tidak membuktikan apa-apa; gate ulang tanpa `touch` adalah persis kegagalan di atas.

**Tambahan 2026-08-27, dari ronde FR-204: pemulihan wajib BINER.** Cacat di atas ditemukan pada pemulihan yang **isinya benar tetapi mtime-nya lama**. Ronde FR-204 menemukan pasangannya dari sisi berlawanan: memulihkan lewat tulis-ulang berbasis **teks** (Python `io.open(..., newline='')` atas string yang dibaca dengan universal-newlines, atau alat apa pun yang menormalisasi baris) menghasilkan berkas yang **identik secara teks tetapi berbeda 981 byte** — seluruh CRLF menjadi LF, dan `git diff` melihatnya sebagai berkas berubah total. **Yang menangkapnya adalah langkah sha256**, bukan mata dan bukan pembacaan diff; siapa pun yang membandingkan isi *sesudah* normalisasi akan melapor "pristine" atas pohon yang bukan pristine. Maka pemulihan memakai salinan biner (`shutil.copyfile`, `cp`), lalu `touch`, lalu gate ulang. **Ketiga langkah kini menjawab tiga pertanyaan yang berbeda**, dan itu sebabnya tidak satu pun boleh menggantikan yang lain: sha256 menjawab *apakah bytenya kembali*, `touch` menjawab *apakah yang dibangun ulang adalah byte itu*, dan gate ulang menjawab *apakah yang barusan dijalankan adalah hasil pembangunan itu*.

**Mengapa sha256 tetap dipertahankan dan tidak diganti.** Ia menjawab pertanyaan yang berbeda dan tetap perlu: *apakah kode produksi kembali seperti semula.* Yang ditambahkan di sini menjawab pertanyaan kedua yang selama ini diam-diam diandaikan terjawab olehnya: *apakah yang barusan dijalankan adalah kode itu.* Dua klaim, dua bukti.

**Ruang lingkup, dinyatakan supaya tidak dibaca lebih luas.** Hasil **per-mutasi** dari ronde-ronde sebelumnya tidak tercemar, dan alasannya spesifik: tiap mutan **ditulis** ke berkas produksi dengan mtime baru, sehingga cargo selalu membangun ulang untuk menjalankan mutan. Yang berisiko hanyalah **keadaan akhir** — pengukuran hijau sesudah pemulihan terakhir. Untuk FR-203 keadaan akhir itu diverifikasi ulang oleh coordinator dengan `touch` eksplisit atas ketiga berkas `db/` lalu gate penuh: 709 test, nol gagal.

**Yang membatalkan keputusan ini.** Cargo berpindah ke fingerprint berbasis isi (ia sudah punya `checksum-freshness` di belakang flag nightly). Bila itu menjadi perilaku baku pada toolchain yang repo ini pakai, langkah `touch` menjadi tidak perlu — tetapi langkah gate-ulang **tetap**, sebab ia juga menangkap pemulihan yang gagal separuh.

---

### ADR-0049 — Appendix A menegakkan lebih sedikit daripada yang komentarnya janjikan; enam relasi lintas-induk diinventarisasi dengan pemiliknya, dan jalur tulis aplikasi memikul apa yang skema tidak pegang

| | |
| --- | --- |
| Tanggal | 2026-08-27 |
| Status | Diterima |
| Terkait | FR-201, FR-202, FR-203, FR-204, FR-206, FR-207, FR-505, FR-506, FR-604, FR-701, FR-703, FR-705, NFR-15, NFR-28, Appendix A, [ADR-0045](decisions.md#adr-0045) · [ADR-0047](decisions.md#adr-0047) · [ADR-0048](decisions.md#adr-0048) |

**Konteks.** FR-203 adalah item pertama yang menulis ke SQLite. Empat item sebelumnya seluruhnya fungsi murni, jadi tiga kelas risiko masuk ke repo ini sekaligus dan untuk pertama kalinya: SQL, transaksi, dan **invarian yang ditegakkan skema alih-alih tipe**. Audit pembukanya menemukan nol Critical pada kelas pertama dan kedua — nol SQL yang dirakit, transaksi yang benar-benar rollback pada **setiap** jalur error termasuk error dari cek Rust, nol panic. Seluruh temuannya jatuh pada kelas ketiga, dan bentuknya konsisten: **skema menjanjikan lebih daripada yang ia tegakkan, dan komentarnya menjanjikan lebih daripada skemanya.**

**Temuan pertama, dan ia dibuktikan runtime alih-alih dibaca.** `songs.default_arrangement_id` **tidak punya foreign key sama sekali**. Baris 49 berbunyi `-- FK added below (circular)` dan baris 110 `-- Circular reference resolved after both tables exist`; yang benar-benar ditambahkan di bawah adalah **dua trigger**, `BEFORE UPDATE OF default_arrangement_id` dan `BEFORE INSERT`. Keduanya menjaga arah *tulis ke `songs`*. Tidak satu pun menyala ketika baris `song_arrangements` yang ditunjuk **dihapus**, dan karena tidak ada FK, tidak ada `ON DELETE` untuk membersihkannya.

Dibuktikan langsung, bukan disimpulkan dari ketiadaan `REFERENCES`: menunjuk lagu ke arrangement miliknya sendiri → `Ok(1)`; `DELETE FROM song_arrangements` → `Ok(1)`; baris arrangement tersisa **nol**; `songs.default_arrangement_id` **tetap** `Some("arr")`. Referensi gantung, diterima diam-diam.

Yang menjadikannya Warning dan bukan catatan gaya adalah **komentar baris 49 itu sendiri**: pembaca FR-204 yang membacanya akan menyimpulkan penghapusan arrangement sudah terjaga. Kesimpulan itu masuk akal, salah, dan tidak terbantahkan di mana pun dalam berkas. Ini kelas cacat yang sama — komentar yang klaimnya lebih luas daripada kodenya — yang di repo ini sudah menjadi temuan nyata **lima kali**, dua di antaranya kalimat yang saya sendiri salin ke ADR.

**Temuan kedua: asimetri tulis-baca yang tidak dinyatakan.** `insert_arrangement` menutup lubang lintas lagu untuk jalur masuknya sendiri; `load_arrangements` mengembalikan apa pun yang ada di tabel. Yang membuat ini asimetri dan bukan sekadar keterbatasan: **modul yang sama sudah menetapkan kebijakan sebaliknya untuk invarian tak-tertegakkan lainnya** — `load_sections` **menolak** `section_type` asing saat baca, dengan alasan eksplisit bahwa barisnya mungkin ditulis oleh sesuatu selain SQLite. Satu invarian yang skema tidak jaga diperiksa saat baca; yang lain tidak; dan tidak satu kalimat pun menyatakan mengapa berbeda.

Akibat keduanya lebih buruk daripada yang doc sebutkan, dan ia bukan tentang menyunting: `DELETE FROM songs` men-*cascade* `songs → song_sections → arrangement_items`, sehingga **menghapus lagu B diam-diam mencabut satu posisi dari arrangement lagu A** — meninggalkan lubang di urutan ibadah yang tak seorang pun sunting.

**Keputusan.**

1. **Jalur tulis aplikasi memikul apa yang skema tidak pegang, dan itu dinyatakan di kode, bukan diandaikan.** FR-203 sudah melakukannya untuk lintas lagu dan untuk `default_arrangement_id` saat insert. Setiap jalur tulis baru ke tabel yang sama memikul cek yang sama — impor `.aero` (FR-703) adalah penulis pertama yang melewati `insert_arrangement`, dan lubangnya kembali terbuka lewat pintu itu.
2. **Migrasi 001 tidak diubah DDL-nya.** Menambahkan FK yang hilang berarti menulis ulang tabel di SQLite dan itu milik item yang benar-benar membutuhkannya — **item pertama yang menghapus baris `song_arrangements`; BUKAN FR-204**, lihat koreksi (3) di bawah. *(Salinan ketiga atribusi lama, dan hitungan "dua tempat" di koreksi (3) meleset justru karena paragraf ini berdiri di ATAS tabel alih-alih di bawahnya — ditemukan audit verifikasi FR-204.)* Yang diperbaiki sekarang hanyalah **komentar yang berbohong** — sebab komentar itulah yang membuat pembaca berikutnya berhenti memeriksa.
3. **Kewajiban dicatat di baris item penerimanya**, bukan hanya di ADR ini. Pelajaran W1 FR-310, yang audit FR-203 langsung mengulanginya terhadap **tiga** baris sekaligus: doc menamai FR-202, FR-701 dan FR-604 sebagai pemilik batas ukuran, dan ketiga baris itu **diam** — dua kosong, satu memuat observasi ukuran tipikal yang bukan kewajiban. Kelimanya kini terisi.

**Batas ukuran: mengapa ini BUKAN sekadar pengulangan kelima.** Aturan yang [ADR-0047](decisions.md#adr-0047) tetapkan berbunyi untuk **fungsi murni**. `insert_song` bukan fungsi murni, dan perluasannya tidak setara: biaya fungsi murni bersifat sementara — `Vec` yang di-drop — sedangkan biaya di sini **persisten dan berulang**. Tempelan 20 MB ditulis sekali lalu dimaterialkan ulang oleh **setiap** `load_song` seumur database, ikut ke setiap ekspor `.aero` dan setiap backup. **Menyebut nama pembatas tidak membatalkan sebuah tulisan.** Karena itu pemicunya dituliskan presisi alih-alih dibiarkan sebagai niat: **commit pertama yang mendaftarkan `#[tauri::command]` yang menjangkau `insert_song`**.

Dan satu koreksi terhadap doc yang ada: kedua sumbu **tidak** setara. Loop insert memakai ulang satu prepared statement, jadi jumlah section adalah biaya WAL dan waktu yang linear, bukan amplifikasi heap. **Yang berbahaya adalah `content`, bukan cacahnya.**

**`deleted_at`: keputusan yang benar dengan bentuk yang tidak dapat menyatakannya.** Baca mengembalikan baris bernisan, sebab jalur pemulihan wajib dapat membaca apa yang ia pulihkan, dan parameter `include_deleted: bool` adalah kesalahan yang disediakan di setiap call site dan dideteksi di nol — argumen yang sama yang [ADR-0043](decisions.md#adr-0043) pakai untuk menolak `resolve(token, language_code, rows)`. Yang harus dinyatakan terbuka adalah harganya: panggilan yang benar dan panggilan yang lupa **identik byte per byte**, sehingga filter yang hilang tidak terlihat di call site maupun di review. Dua akibat konkretnya dicatat di baris FR-701 dan FR-201, dan yang kedua menuntut filter di **dua** tempat sekaligus: `songs_fts` adalah virtual table dan **tidak dapat membawa foreign key**, sehingga baris indeks tidak hilang saat lagu dihapus.

**Inventaris relasi lintas-induk lain di Appendix A.** Tidak satu pun disentuh FR-203; didaftar di sini supaya tidak perlu ditemukan ulang enam kali.

| Relasi | Yang tidak terikat | Siapa yang membukanya |
| --- | --- | --- |
| `songs.default_arrangement_id` | **Nol FK.** Trigger hanya menjaga arah tulis-ke-`songs`; hapus arrangement meninggalkan referensi gantung | **Item pertama yang menghapus baris `song_arrangements`** — FR-202 (hard-delete lagu), atau item CRUD arrangement bila kelak ada. *Bukan* FR-204; lihat koreksi 2026-08-27 di bawah |
| `arrangement_items.section_id` | Tidak terikat ke `song_arrangements.song_id`; arrangement lagu A boleh menunjuk section lagu B | **FR-703** (penulis), **FR-204** (pembaca) |
| `taggables.entity_id` | **Nol FK.** `entity_type` di-CHECK, `entity_id` tidak mereferensi apa pun; menghapus induk meninggalkan tag yatim | Item tagging FR-209, FR-202 |
| `songs_fts.song_id` | Virtual table tidak dapat membawa FK; nol yang mencabut baris indeks saat lagu dihapus | **FR-201** — dan inilah yang menjadikan `deleted_at` jalur pengungkapan nyata |
| `deck_slides.slide_index` vs `imported_decks.slide_count` | Nol constraint; deck boleh mengklaim 40 slide dan memuat 3 | FR-505, FR-506 |
| `bible_verses.chapter/verse` vs `bible_books.chapter_count` | Nol CHECK, bahkan nol `> 0` | Impor FR-208, konsumen FR-206 |
| `bible_book_names.language_code` vs `bible_versions.language_code` | Tidak terikat; versi bahasa X boleh diresolusi dengan nama kitab bahasa Y | FR-206, FR-207 |
| `template_media` vs isi JSON `templates.document` | Junction tidak terikat pada media yang dokumen benar-benar rujuk | FR-705 (relink-by-hash) |

**Dan kolom berbentuk path hanya dijaga komentar.** `media_assets.relative_path`, `deck_slides.image_path`/`thumb_path`, `templates.thumbnail_path`, `imported_decks.source_path`, `recent_sessions.file_path` — nol CHECK menuntut nilai-nilai ini relatif. Siapa pun yang menggabungkannya dengan sebuah root wajib mengkanonikalisasi lebih dulu (NFR-15): `.aero` atau database asing dapat menaruh `..\..\..\Windows\…` atau path absolut di sana. FR-203 tidak menyentuh satu pun kolom ini; dicatat untuk modul `queries/` berikutnya, yang akan menyalin bentuk berkasnya.

**Koreksi 2026-08-27, ditulis terbuka alih-alih disunting diam-diam — ketiganya lahir dari ronde FR-204.**

**(1) Pemicu batas ukuran diformulasi ulang atas TABEL, bukan atas nama satu fungsi.** Rumusan lama — *"commit pertama yang mendaftarkan `#[tauri::command]` yang menjangkau `insert_song`"* — dibuat saat `insert_song` adalah satu-satunya pintu tulis. FR-204 menambah `insert_song_with_default_arrangement`, yang **tidak memanggil `insert_song`**: ia memanggil `refuse_uninsertable` dan `write_song` langsung. Sebuah perintah yang menjangkau hanya fungsi baru **memenuhi huruf pemicu lama dan melewati maksudnya**, dan reviewer yang mencocokkan diff terhadap kalimat itu akan menutup itemnya tanpa batas ukuran. Ini persis bentuk [ADR-0045](decisions.md#adr-0045) — invarian berhenti dijaga saat pintu masuk kedua terbuka — diterapkan pada kewajiban ADR ini sendiri, dan itu sebabnya ia dicatat di sini alih-alih diperbaiki senyap. **Rumusan yang berlaku:** *commit pertama yang mendaftarkan `#[tauri::command]` yang menulis ke `songs`, `song_sections`, `song_arrangements` atau `arrangement_items` lewat pintu mana pun. *(Koreksi kedua, dari audit verifikasi FR-204: rumusan pertama menyebut **tiga** tabel dan modul ini menulis **empat**. Sebuah perintah yang mendaftarkan `insert_arrangement` untuk arrangement bernama **tanpa item** menjalankan hanya `INSERT INTO song_arrangements`, nol tulisan ke ketiga tabel yang disebut — memenuhi huruf pemicu dan membawa `name` tak terbatas ke disk. Arrangement kosong bukan hipotetis; modul memberkatinya. **Ini ADR-0045 untuk ketiga kalinya, dan kali ini pada rumusan yang dibuat untuk menutup ADR-0045.**)*.* Tabel tidak bertambah diam-diam; nama fungsi bertambah.

Dan pemicu itu tidak menyebut satu pun **pintu baca**, padahal `expand_arrangement` adalah pintu baca yang biayanya justru yang paling tajam. Sumbu kelima ditambahkan: **cacah item arrangement**. Empat sumbu yang didaftar kepala modul (`title`, `label`, `content`, cacah section) tidak memuatnya, sehingga kalimat `expand_arrangement` yang meneruskan kewajiban "ke kepala modul" meneruskannya ke daftar yang tidak memuat sumbunya. Yang membuat sumbu ini berbeda: satu section 2 MB dengan 10.000 item yang semuanya menunjuknya adalah berkas `.aero` **sekitar 2 MB** yang lolos setiap batas `content` maupun cacah section, dan ekspansinya membangun **20 GB**. **Ukuran berkas bukan proksi apa pun untuk biaya baca.**

**(2) Kalimat "bukan amplifikasi heap" di atas sudah tidak benar, dan pembatalnya adalah FR-204.** Kalimat itu berbunyi bahwa jumlah section hanyalah biaya WAL dan waktu yang linear. Itu benar untuk loop insert, yang memakai ulang satu prepared statement. Tetapi `insert_song_with_default_arrangement` membangun `Vec<ArrangementItem>` dengan satu `clone()` id per section **sebelum** transaksi dibuka, dan menahannya melintasi seluruh transaksi. Sumbu cacah section karenanya **sekarang** punya komponen heap pada jalur itu. Kalimat aslinya sengaja dibiarkan berdiri di atas supaya koreksi ini terbaca sebagai koreksi, bukan supaya jejaknya hilang — ia tetap benar untuk `insert_song`, dan **hanya** untuk `insert_song`.

**(3) FR-204 bukan pemilik lubang DELETE, dan baris pertama tabel di atas sudah diperbaiki.** Ketika ADR ini ditulis saya menyimpulkan FR-204 akan membuka jalur hapus arrangement. Teks requirement PRD baris 337 tidak menyebut penghapusan sama sekali — ia berbunyi *"an ordered sequence of references to that song's sections. A default arrangement is generated on creation"*. Yang FR-204 benar-benar pikul adalah arah **tulis** `default_arrangement_id`, dan arah itu justru satu-satunya yang **sudah** dijaga kedua trigger. Pemiliknya berpindah ke item pertama yang benar-benar menghapus baris `song_arrangements`. **Dua tempat lain masih menyalin atribusi lama** dan wajib ikut diperbaiki sebelum FR-204 ditutup, sebab menutupnya akan membuat keduanya menunjuk item yang sudah selesai: `001_initial_schema.sql:132-135` dan paragraf di bawah tabel ini.

**Yang membatalkan keputusan ini.** Migrasi yang benar-benar menambahkan FK yang hilang. Saat itu baris pertama tabel di atas berpindah dari "dijaga kode aplikasi" ke "dijaga skema", dan cek Rust yang bersangkutan menjadi sabuk pengaman alih-alih satu-satunya penjaga — tetapi hanya baris itu, dan hanya bila migrasinya juga membersihkan baris gantung yang sudah terlanjur ada.

---

### ADR-0050 — Gate clippy repo ini buta terhadap seluruh berkas test crate `core` sepanjang proyek, dan sebabnya satu flag yang hilang; perintah gate wajib memilih workspace, bukan package root

| | |
| --- | --- |
| Tanggal | 2026-08-27 |
| Status | Diterima |
| Terkait | SETUP-03, FR-204, GATE-G10, NFR-32, [ADR-0020](decisions.md#adr-0020) · [ADR-0048](decisions.md#adr-0048) |

**Cacatnya, dan bagaimana ia ditemukan.** Ronde penutupan audit FR-204 mengubah tanda tangan `expand_arrangement`, yang memerahkan enam call site di `crates/core/tests/song_arrangements.rs`. Implementer melaporkan sesuatu yang seharusnya mustahil: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` **exit 0**, sementara berkas test itu **tidak mengompilasi sama sekali**.

Diverifikasi coordinator, dua bentuk berdampingan pada pohon yang sama:

* `--manifest-path src-tauri/Cargo.toml --all-targets` → **exit 0**.
* `--manifest-path src-tauri/Cargo.toml --workspace --all-targets` → **`error[E0061]` × 6**.

**Sebabnya bukan bug cargo, melainkan arti `--all-targets` yang lebih sempit daripada bunyinya.** Ia memilih semua *target* dari *package yang terpilih*. Tanpa `--workspace`, yang terpilih hanyalah package root `aeroworship`; `aeroworship-core` masuk sebagai **dependency**, dan sebuah dependency dibangun sebagai **lib saja**. Target testnya tidak pernah termasuk. Jadi bendera yang namanya berbunyi "semua target" memang memberi semua target — dari separuh workspace.

**Mengapa ini ADR dan bukan koreksi satu baris.** Repo ini bertumpu pada dua janji yang saling menopang: suite hijau tidak membuktikan apa pun sampai mutasi dijalankan ([ADR-0048](decisions.md#adr-0048)), dan gate hijau berarti pohon bersih ([ADR-0020](decisions.md#adr-0020)). Temuan ini menyerang yang kedua di tempat yang paling tidak terduga: **satu-satunya crate tempat hampir seluruh test repo ini hidup**. Sepanjang FR-205, FR-401, FR-310, FR-203 dan FR-204 — lebih dari dua ratus test Rust — clippy tidak pernah sekali pun melihat berkas yang memuatnya. Setiap "clippy bersih" yang tercatat di `PROGRESS.md` adalah pernyataan tentang `src/` dan `crates/core/src/`, bukan tentang `crates/core/tests/`.

**Yang tidak ikut runtuh, dinyatakan supaya kerusakannya tidak dibaca lebih luas.** `cargo test --workspace` **memang** membangun dan menjalankan target test itu — ia memakai `--workspace` sejak awal. Karena itu kesalahan yang **menggagalkan kompilasi atau menggagalkan test** selalu tertangkap, dan tidak ada angka test yang pernah tercatat di berkas ini palsu. Yang lolos adalah kelas yang lebih sempit dan lebih sunyi: **lint** atas berkas test — `clippy::*` yang tak pernah dijalankan, dan `-D warnings` yang tak pernah ditegakkan di sana. Bahwa sebelas suite melewatinya tanpa satu peringatan pun bukan bukti kebersihan; ia belum diukur.

**Bentuk kegagalannya asimetris, sama seperti [ADR-0048](decisions.md#adr-0048), dan kali ini kami beruntung dua kali.** Ia tertangkap hanya karena `cargo test` di gate yang **sama** memerah pada berkas yang sama, sehingga ada dua laporan yang bertentangan dan salah satunya harus dijelaskan. Seandainya perubahan itu hanya melanggar lint dan bukan kompilasi, kedua gate akan hijau serentak dan tidak ada yang meminta penjelasan.

**Keputusan.**

1. **Bentuk kanonik gate clippy repo ini adalah `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings`.** `--workspace` **dan** `--all-targets`, bukan salah satu: yang pertama memilih package, yang kedua memilih target di dalamnya, dan menghilangkan mana pun mengembalikan sebagian titik buta.
2. **Setiap brief agent memakai bentuk itu**, dan brief lama tidak diperbaiki secara retroaktif — yang diperbaiki adalah ukurannya, sekali, di ronde berikutnya yang menyentuh `crates/core/tests/`.

**Cacat yang lebih dalam, dinyatakan karena ia yang membuat cacat di atas mungkin: nol perintah gate repo ini punya rumah yang dapat dieksekusi.** Keenam gate hidup sebagai **prosa** — di brief coordinator dan di baris changelog `PROGRESS.md`. Nol script npm, nol berkas CI, nol justfile memuat satu pun perintah `cargo`. `package.json` punya `lint`, `typecheck` dan `test` untuk sisi JS; sisi Rust tidak punya padanan. Perintah yang hanya hidup sebagai kalimat **tidak dapat salah dengan cara yang terlihat**: ia disalin ulang tiap ronde, dan salinan yang salah tetap terbaca benar. Itulah cara satu flag hilang selama lima item berturut-turut tanpa satu orang pun menyadarinya.

Memberi keenamnya rumah yang dapat dieksekusi adalah pekerjaan SETUP-03, dan SETUP-03 sudah `done` — jadi ia tidak diselundupkan ke dalam FR-204. **Pemicunya dituliskan alih-alih dibiarkan sebagai niat: item pertama yang menambahkan berkas CI, atau item rilis pertama, mana yang lebih dulu.** Sampai saat itu, bentuk kanonik di atas adalah satu-satunya yang boleh dikutip.

**Yang membatalkan keputusan ini.** Cargo mengubah `--all-targets` agar mencakup target test seluruh workspace secara baku, atau keenam gate mendapat rumah yang dapat dieksekusi — pada titik itu perintahnya berhenti menjadi kalimat yang disalin dan menjadi berkas yang di-review, dan ADR ini menjadi catatan sejarah alih-alih aturan.
