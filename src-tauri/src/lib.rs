//! AeroWorship Tauri shell.
//!
//! `main.rs` is a thin binary shell; the builder configuration lives here so it
//! stays reachable from a normal function. This crate owns only what needs to
//! name a `tauri` type:
//!
//! - `commands/` — one module per command group, the IPC surface.
//! - `services/` — the orchestration that owns an `AppHandle` or a
//!   `WebviewWindow`, the Display Service above all (PRD §6.3).
//! - window and event wiring, including the slide-advance hot path, which
//!   broadcasts an event rather than awaiting a command (PRD §6.5).
//!
//! Everything else — models, storage, parsing, splitting, `.aero`
//! serialisation, path resolution — belongs to [`aeroworship_core`], which
//! cannot depend on `tauri` (ADR-0008). A command should read as: deserialise,
//! call core, emit or return.
//!
//! Those directories exist but are still empty; each is populated by its own
//! backlog item and declared here when it gains a first module.

// Refuse to produce a release binary carrying Tauri's *dev* flavour (ADR-0016).
//
// `custom-protocol` is not one of `tauri`'s default features; only the Tauri
// CLI turns it on. Without it `tauri`'s own build script sets `cfg(dev)`, and
// the resulting binary embeds no assets: it navigates the webview to
// `http://localhost:1420` — a local socket any process may occupy — inside the
// origin that carries the IPC bridge and this app's capability grants, with no
// CSP header at all. On a developer machine Vite is usually running, so such a
// binary behaves perfectly and the defect only surfaces on a user's machine.
// That is why this is a compile error and not a note in the README.
//
// The condition is read through `tauri::is_dev()`, which is
// `!cfg!(feature = "custom-protocol")` evaluated inside the `tauri` crate —
// where the feature actually lives. A `cfg(feature = "custom-protocol")` test
// in *this* crate would always be false, because the CLI enables the feature as
// `tauri/custom-protocol` rather than re-exporting it here.
#[cfg(not(debug_assertions))]
const _: () = assert!(
    !tauri::is_dev(),
    "release build without tauri's `custom-protocol` feature: this binary would \
     load its UI from http://localhost:1420 instead of the embedded `dist/`. \
     Build with `npm run tauri build`, not `cargo build --release` — the Tauri \
     CLI builds the frontend first and enables `tauri/custom-protocol`. If you \
     really do want plain Cargo, pass `--features tauri/custom-protocol` and \
     make sure `dist/` is up to date yourself."
);

/// Builds and runs the Tauri application.
///
/// Kept separate from `main` so the binary stays a one-liner and the builder
/// configuration is a normal library function.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to start the AeroWorship application");
}
