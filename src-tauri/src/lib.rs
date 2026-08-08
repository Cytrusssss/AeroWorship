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

/// Builds and runs the Tauri application.
///
/// Kept separate from `main` so the binary stays a one-liner and the builder
/// configuration is a normal library function.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to start the AeroWorship application");
}
