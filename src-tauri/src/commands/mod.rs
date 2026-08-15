//! The IPC surface: one module per command group.
//!
//! A command reads as *deserialise, call core, emit or return*. Anything longer
//! than that has domain logic in it and belongs in [`aeroworship_core`]
//! (ADR-0008).
//!
//! Every command added here must also be listed in the `generate_handler!` call
//! in [`crate::run`]; a command that is defined but never registered compiles
//! cleanly and fails only when the frontend invokes it. It must also be listed
//! in `src-tauri/commands.inventory.md`, and that requirement is the one with
//! teeth: `scripts/check-command-inventory.js` (the `prelint` hook, so
//! `npm run lint` covers it) reconciles all three lists and is red unless they
//! name the same set. The inventory exists because the *other* direction —
//! registered, reachable, and never written down anywhere a reviewer looks — is
//! silent, for the reason the next paragraph gives.
//!
//! **Capabilities.** App-defined commands are not ACL-gated for a local origin
//! in `tauri` 2.11.5 — `webview/mod.rs:1820` only enforces the ACL for `plugin:`
//! commands, for remote origins, or when the app ships its own ACL manifest,
//! and this app ships none (there is no `src-tauri/permissions/` directory and
//! `build.rs` calls plain `tauri_build::build()`). So nothing here needs an
//! entry in `capabilities/*.json`, and adding commands does not widen the
//! permission surface NFR-14 asks to be reviewed. It does widen the surface
//! reachable from the webview, which is why each command has to justify itself
//! on its own.
//!
//! **What reverses that.** The paragraph above stops being true the instant
//! this app gains an ACL manifest of its own: `has_app_acl_manifest`
//! (`tauri/webview/mod.rs:1823`) turns `true` as soon as `src-tauri/permissions/`
//! exists or `build.rs` switches to `Attributes::app_manifest(...)`, and from
//! that moment **every** command not named by some capability is rejected with
//! `"Command {} not allowed by ACL"` (`:1850`). Adding a `permissions/`
//! directory looks like an obviously correct hardening step and would instead
//! take the whole IPC surface down — every command below would start failing at
//! once. Whoever makes that change has to name every command in
//! `capabilities/main-window.json` (and in the output window's capability) in
//! the same commit; `src-tauri/commands.inventory.md` is the list to work from.
//!
//! **No per-window scoping.** `capabilities/main-window.json:5` reads
//! `"windows": ["main"]`, and that restriction binds `plugin:` commands only —
//! the same ACL that does not gate the commands here is the only thing that
//! reads it. App-defined commands have no per-window scoping at all, and since
//! FR-102 that is a live fact rather than a forecast: the Display Service
//! creates the `output` window at start-up, no capability names it, and it can
//! call `list_monitors` — and every command added after it — with no trace of
//! that reach in any manifest. That asymmetry is worth stating plainly:
//! `core:event:default` really is confined to the main window; nothing in this
//! module is. It is not exploitable today — the output bundle is verified to
//! contain zero `__TAURI_INTERNALS__` — but PRD §6.3 rests that isolation on an
//! import rule, and an import rule does not constrain script that arrives
//! through injected lyrics or a hostile template.
//!
//! **Arguments.** Everything a command receives comes from the webview and is
//! untrusted input, not a parameter our own code chose. `list_monitors` takes
//! none, so this is written before there is anything to correct rather than
//! after:
//!
//! - Name a concrete type for every argument and let `serde` reject the rest.
//!   No free-form blob (`serde_json::Value`, `HashMap<String, _>`) crosses this
//!   boundary; a command that cannot say what it accepts cannot validate it.
//! - Any path argument is canonicalised and checked to sit under a permitted
//!   root before it is opened, and rejected — not clamped, not trimmed — when it
//!   does not (NFR-15). That check belongs to `aeroworship_core`'s path
//!   resolution, so it is testable against hostile fixtures without a webview.
//! - Any argument of unbounded length (text, lists, file contents) is
//!   size-checked before it is copied, parsed or persisted (NFR-28). Malformed
//!   or oversized input produces a diagnostic, never a crash.
//! - No `unwrap`, `expect`, slice indexing or integer cast that can panic on a
//!   path reachable from an argument. A panic in a command unwinds into the
//!   runtime rather than into a caller that could handle it, and the input that
//!   triggers it is chosen by whoever authored the file being opened. Return an
//!   error instead.
//!
//! The commands next in Appendix D are the first to carry any of this:
//! `set_output_monitor { monitorId }` and `create_output_window { monitorId? }`
//! take an identifier the frontend supplies, and the FR-5xx group takes paths.

pub mod display;
