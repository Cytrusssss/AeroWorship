//! AeroWorship domain logic.
//!
//! Everything whose correctness matters lives here (PRD §6.1): book-reference
//! parsing, slide splitting, `.aero` serialisation, path resolution, the data
//! model and the database layer. None of it may depend on `tauri` — the crate
//! boundary is what enforces PRD §6.1 structurally instead of by convention
//! (ADR-0008), and it is what lets `cargo test -p aeroworship-core` run without
//! linking `wry`/`tao`/`webview2-com`.
//!
//! **Where does a new module go?** If it has to name a `tauri` type it belongs
//! to the shell crate (`src-tauri/src/`); otherwise it belongs here. In
//! practice that puts the whole of `models/` and `db/` here, together with the
//! pure part of every PRD §6.13 service, named after the domain concept rather
//! than after the service. The shell's `services/` keeps only the thin
//! orchestration that owns an `AppHandle` or a `WebviewWindow` — the Display
//! Service being the clear example (PRD §6.3).
//!
//! Neither `.aero` files nor templates are trusted input: nothing parsed by
//! this crate is ever executed (NFR-28).

pub mod db;
pub mod models;
