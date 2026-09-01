//! The IPC surface: one module per command group.
//!
//! A command reads as *deserialise, call core, emit or return*. Anything longer
//! than that has domain logic in it and belongs in [`aeroworship_core`]
//! (ADR-0008).
//!
//! Every command added here must also be listed in the `generate_handler!` call
//! in [`crate::run`]; a command that is defined but never registered compiles
//! cleanly and fails only when the frontend invokes it. It must also be listed
//! in `src-tauri/commands.inventory.md`, and it must be granted by a capability
//! — see the next paragraph. `scripts/check-command-inventory.js` (the `prelint`
//! hook, so `npm run lint` covers it) reconciles all four lists and is red
//! unless they name the same set. The inventory exists because one of those
//! directions — registered, reachable, and never written down anywhere a
//! reviewer looks — is otherwise silent.
//!
//! **Capabilities, and the fact that they now bind.** Since SEC-01 this app
//! ships an ACL manifest of its own: `src-tauri/permissions/` exists, so
//! `tauri_build::build()` picks it up with no build-script change and
//! `has_app_acl_manifest` (`tauri` 2.11.5, `webview/mod.rs:1823`) is `true`.
//! From that point `webview/mod.rs:1850` rejects every **app-defined** command
//! no capability names, with `"Command {} not allowed by ACL"`. So a new command
//! needs three things in the same commit, not two: the registration, the
//! inventory entry, and an `allow-…` permission under `src-tauri/permissions/`
//! named in `capabilities/main-window.json`. Miss the third and the command
//! compiles, passes every Rust gate, and is dead the first time the frontend
//! calls it — `dynamic-acl` is off (ADR-0013), so no restart and no
//! configuration file reopens it; only new Rust code, and therefore a rebuild.
//! The guard above is what turns that into a red gate minutes earlier.
//!
//! Two narrowings, both of which used to be written here as absolutes, and both
//! of which are the kind of overclaim that stops a reader from looking:
//!
//! 1. **Not every command — one is exempt by construction.** The condition at
//!    `webview/mod.rs:1823-1826` ends
//!    `&& request.cmd != FETCH_CHANNEL_DATA_COMMAND && invoke.acl.is_none()`, so
//!    `plugin:__TAURI_CHANNEL__|fetch` reaches its handler from any window, ours
//!    or otherwise, whatever the capabilities say. **Cost today: zero, because
//!    this application creates no [`tauri::ipc::Channel`].** It is not zero the
//!    moment one is created, and the reason is worth stating before that item is
//!    written rather than after: `ChannelDataIpcQueue` is a
//!    `HashMap<u32, InvokeResponseBody>` held as **application** state, not
//!    per-webview (`tauri/src/ipc/channel.rs:46`); ids come from a process-wide
//!    `AtomicU32` starting at 0 (`:42`); and `fetch` **removes** the entry it
//!    returns (`:329`). A window guessing small integers therefore reads another
//!    window's channel payload *and* makes the legitimate reader lose it.
//!    **Obligation: the first item that introduces a `Channel` owns this.** Its
//!    acceptance criteria must say what stops the output window from draining
//!    that queue — nothing in `capabilities/*.json` can, and no list
//!    `scripts/check-command-inventory.js` reconciles can even see it.
//! 2. **"No runtime escape" is the practical consequence, not a guarantee.**
//!    `dynamic-acl` being off removes `Manager::add_capability`, and nothing
//!    else. `Context::runtime_authority_mut` (`tauri/src/lib.rs:478-482`) and
//!    `RuntimeAuthority::__allow_command` (`tauri/src/ipc/authority.rs:136-146`)
//!    are both `pub`, neither is behind that feature, and the second inserts
//!    `windows: vec!["*"]` — every window at once, recorded in no capability
//!    file and in nothing under `gen/schemas/`. `#[doc(hidden)]` hides them from
//!    rustdoc and from nothing else. What remains true is that reaching them
//!    takes Rust code in this crate, so it takes a rebuild and a diff; and
//!    `findRuntimeAclEscapes` in `scripts/command-inventory-guard.js` makes that
//!    diff red. What that guard is, exactly, so this paragraph is not read as
//!    wider than it is: a per-line text search of `src-tauri/src/` for
//!    `__allow_command` and `runtime_authority_mut`, with `//` comments removed
//!    first. It catches the two forms that matter — a call behind an inline
//!    block comment, and `*ctx.runtime_authority_mut() = authority` — and it is
//!    a grep, not a compiler. It does not see a call reached through an alias, a
//!    macro, a re-export under another name, or any crate other than this one,
//!    and it says nothing about a dependency that does the same thing. The
//!    claim it supports is "this cannot land in this crate unnoticed", not
//!    "this cannot happen".
//!
//! **Per-window scoping, which is the point of the change.**
//! `capabilities/main-window.json` reads `"windows": ["main"]`, and until
//! SEC-01 that restriction bound `plugin:` commands only — the ACL that read it
//! did not gate anything in this module. Since FR-102 that was a live hole
//! rather than a forecast: the Display Service creates the `output` window at
//! start-up, no capability named it, and it could call `list_monitors` — and
//! every command added after it — with no trace of that reach in any manifest
//! (ADR-0041). It is now closed. `capabilities/output-window.json` names that
//! window and grants it nothing, deliberately and in writing, so an app-defined
//! command reaches the projector **over IPC** only if someone adds it there.
//!
//! IPC is the qualifier, and ADR-0042 is why it has to
//! be said out loud: `index.html` and `output.html` are served from the same
//! origin, so script running in the output document is same-origin with the
//! Control Panel and can talk to it through `BroadcastChannel`, `localStorage`
//! or a `MessageChannel` without an `invoke` and without passing the ACL at all.
//! The capability files bound one path, not every path. Not exploitable today —
//! the only `invoke()` in the frontend is in `src/main/App.vue`, and it relays
//! nothing — but a Control Panel that ever acts on a same-origin message is
//! offering the projector window exactly the reach `output-window.json` was
//! written to withhold. PRD §6.3 still rests the bundle separation on an import
//! rule, and an import rule does not constrain script arriving through injected
//! lyrics or a hostile template — but the reach that script would have over IPC
//! is now the ACL's answer, not an unguarded default.
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
