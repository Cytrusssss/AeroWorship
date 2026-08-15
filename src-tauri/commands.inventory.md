# IPC command inventory

Every command this application exposes to its webview, in one place.

## Why this file exists

App-defined Tauri commands are **not ACL-gated** for a local origin
([ADR-0039](../decisions.md#adr-0039)): the gate in `tauri` 2.11.5
(`webview/mod.rs:1820`) fires only for `plugin:` commands, for remote origins, or
when the app ships its own ACL manifest, and this app ships none. So a command
becomes reachable from the webview the moment it is named in `generate_handler!`
in `src-tauri/src/lib.rs`, and it leaves **no trace anywhere else**: not in
`capabilities/*.json`, not in `gen/schemas/acl-manifests.json`, not in
`gen/schemas/capabilities.json`. Verified rather than assumed — `list_monitors`
appears zero times under `gen/schemas/`.

That has a consequence for NFR-14, whose verification method is "capability
manifest reviewed at each release": reviewing the manifests reports a **smaller**
surface than the one that exists, and its diff stays clean no matter how many
commands are added. This file is the artefact that review should read instead.
`generate_handler!` remains the only *authoritative* list, because it is what the
runtime obeys; this file is kept identical to it by a guard, and exists so the
list is readable, justified, and shows up in a diff as prose someone has to
write.

Two properties a reviewer should know before reading the entries, both of them
consequences of the same fact:

`capabilities/main-window.json` says `"windows": ["main"]`, and that scoping does
**not** apply to anything listed here — per-window scoping exists only for
ACL-gated commands. Every command below is callable from any window this app
creates, and as of FR-102 that is no longer hypothetical: the Display Service
creates the `output` window at start-up, it is granted no capability at all, and
it can nonetheless invoke every command listed below. The output bundle imports
no Tauri API today, but that is an import rule, not an enforcement boundary.

Conversely, creating `src-tauri/permissions/` or calling
`Attributes::app_manifest(...)` in `build.rs` switches ACL enforcement **on** for
everything below, and every command not then named in a capability is rejected
with `Command {} not allowed by ACL`. See the note in `src-tauri/src/commands/mod.rs`.

## How to change it

Each entry is one Markdown list item: `- `, then the command's full Rust path in
backticks, then a space, an em dash (`—`), a space, and one non-empty line saying
what it is for and why it may be reached from the webview. Nothing else in this
file may start a Markdown list item; the guard rejects any list line it cannot
parse as an entry, so a mistyped entry fails loudly instead of quietly dropping a
command from the inventory. Prose here uses paragraphs, which is why the grammar
above is described rather than shown.

The guard is `scripts/check-command-inventory.js` (decisions in
`scripts/command-inventory-guard.js`). It runs on the `prelint` hook, so
`npm run lint` covers it; `npm run commands:check` runs it alone. It reconciles
three lists — the `#[tauri::command]` definitions under `src-tauri/src/`, the
`generate_handler!` registrations, and the entries below — and is red unless all
three name the same set.

## Commands

- `commands::display::list_monitors` — enumerates the displays the OS reports (FR-101); takes no arguments, reads only `AppHandle`, returns `Vec<Monitor>`. It is the single source of display facts for the frontend, which is why `core:window:default` is deliberately not granted in `capabilities/main-window.json`.
