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
//! Each directory is populated by its own backlog item and declared here when
//! it gains a first module.

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
// The condition is `cfg(dev)`, an alias `tauri_build::build()` already emits
// from `src-tauri/build.rs`: it calls `cfg_alias("dev", is_dev())`, which
// prints `cargo:rustc-check-cfg=cfg(dev)` and, when dev, `cargo:rustc-cfg=dev`.
// So the flag is read where the feature actually lives, with no build script of
// our own — and `#[cfg(not(dev))]` is the form Tauri's own CLI changelog tells
// application crates to use, in place of testing
// `cfg(feature = "custom-protocol")` here (that test would always be false,
// since the CLI enables the feature as `tauri/custom-protocol` rather than
// re-exporting it) — ADR-0018.
//
// `debug_assertions` is the proxy for "this is a release profile". It is a
// proxy, not the thing itself: see the cross-note on `[profile.release]` in
// `Cargo.toml` before changing either side.
#[cfg(all(not(debug_assertions), dev))]
compile_error!(
    "release build without tauri's `custom-protocol` feature: this binary would \
     load its UI from http://localhost:1420 instead of the embedded `dist/`. \
     Build with `npm run tauri build`, not `cargo build --release` — the Tauri \
     CLI builds the frontend first and enables `tauri/custom-protocol`. If you \
     really do want plain Cargo, pass `--features tauri/custom-protocol` and \
     make sure `dist/` is up to date yourself. The same flag is the way to run \
     a release-profile test or benchmark, which this guard also rejects: \
     `cargo test --release -p aeroworship --features tauri/custom-protocol` \
     (ADR-0019)."
);

pub mod commands;
pub mod db;
pub mod services;

/// Builds and runs the Tauri application.
///
/// Kept separate from `main` so the binary stays a one-liner and the builder
/// configuration is a normal library function.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // The database is opened and brought up to date before anything can
            // query it; failing here aborts start-up rather than letting the app
            // come up with no storage behind it.
            db::init(app.handle())?;
            // Then the windows are placed (FR-102). The Control Panel already
            // exists by now — windows declared in `tauri.conf.json` are created
            // before this hook runs — and is centred on the primary display by
            // that configuration; this call adds the Projector Output on the
            // first non-primary display, if there is one. It cannot fail the
            // start-up: an output window that will not open leaves the operator
            // with a working Control Panel, not with no application (NFR-08).
            services::display::init(app.handle());
            Ok(())
        })
        // Every command in `commands/` has to be named here as well as defined;
        // one that is defined but not registered compiles cleanly and fails only
        // when the frontend invokes it.
        .invoke_handler(tauri::generate_handler![commands::display::list_monitors])
        .run(tauri::generate_context!())
        .expect("failed to start the AeroWorship application");
}
