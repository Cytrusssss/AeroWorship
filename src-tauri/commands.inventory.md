# IPC command inventory

Every command this application exposes to its webview, in one place.

## Why this file exists

App-defined Tauri commands used to be **not ACL-gated** for a local origin
([ADR-0039](../decisions.md#adr-0039)): the gate in `tauri` 2.11.5
(`webview/mod.rs:1820`) fires only for `plugin:` commands, for remote origins, or
when the app ships its own ACL manifest, and this app shipped none. So a command
became reachable from the webview the moment it was named in `generate_handler!`
in `src-tauri/src/lib.rs`, and it left **no trace anywhere else**: not in
`capabilities/*.json`, not in `gen/schemas/acl-manifests.json`, not in
`gen/schemas/capabilities.json`.

That had a consequence for NFR-14, whose verification method is "capability
manifest reviewed at each release": reviewing the manifests reported a
**smaller** surface than the one that existed, and its diff stayed clean no
matter how many commands were added. This file is the artefact that review reads
instead. `generate_handler!` remains the only *authoritative* list, because it is
what the runtime obeys; this file is kept identical to it by a guard, and exists
so the list is readable, justified, and shows up in a diff as prose someone has
to write.

## What SEC-01 changed, and what it did not

`src-tauri/permissions/` now exists, so the paragraphs above are history rather
than description ([ADR-0041](../decisions.md#adr-0041)). `tauri_build::build()`
globs that directory with no build-script change of ours, and from the first
permission it finds, `has_app_acl_manifest` is `true`: app-defined commands are
gated exactly like `plugin:` ones, `"windows"` scoping finally binds them, and
every **app-defined** command not named by some capability is rejected at invoke
time with `Command {} not allowed by ACL`.

Not *every* command, and the exception is in the gate itself: the condition at
`tauri-2.11.5/src/webview/mod.rs:1823-1826` ends
`&& request.cmd != FETCH_CHANNEL_DATA_COMMAND && invoke.acl.is_none()`, so
`plugin:__TAURI_CHANNEL__|fetch` is reachable from any window whatever the
capabilities say. It costs nothing today because this application creates no
`Channel` at all — no entry below uses one — and it stops costing nothing on the
day one is created, because the queue behind that command is application-wide
rather than per-webview. The first item that introduces a `Channel` owns that
problem; `src-tauri/src/commands/mod.rs` states it in full. No list this file
reconciles can see it, which is why it is written here instead.

Three consequences a reviewer should carry into the entries below.

The `output` window is granted **zero** app-defined commands, and
`capabilities/output-window.json` exists to say so rather than to grant anything.
Until SEC-01 that window could invoke every command listed here, leaving no trace
in any manifest; now it can invoke none of them. What used to rest on an import
rule — the output bundle imports no Tauri API — rests on the framework.

The cliff is fail-closed, and reopening it takes a rebuild. `dynamic-acl` is
deliberately off ([ADR-0013](../decisions.md#adr-0013)), so a command registered
without a permission written for it is dead until somebody rebuilds — no restart
and no configuration file brings it back. "No runtime escape" would be too
strong: `Context::runtime_authority_mut` and `RuntimeAuthority::__allow_command`
are `pub` and are not behind that feature, and the second grants to
`windows: ["*"]`. Reaching either takes Rust code in this crate, so it takes a
diff, and `scripts/check-command-inventory.js` makes that diff red — by grepping
`src-tauri/src/` for those two names with `//` comments removed. That is a text
search, not a compiler: it catches both forms that matter (a call behind an
inline block comment, and `*ctx.runtime_authority_mut() = authority`) and it
does not see an alias, a macro, a re-export, or any crate but this one. Every
command added
below therefore needs three things in the same change: the registration, the
entry here, and an `allow-…` permission under `src-tauri/permissions/` named in
`capabilities/main-window.json`.

NFR-14's verification method is no longer structurally blind. `list_monitors`
used to appear zero times under `gen/schemas/`; it now appears in
`gen/schemas/acl-manifests.json`, under the `__app-acl__` key, as
`permissions["allow-list-monitors"].commands.allow`, and the permission
identifier appears in `gen/schemas/capabilities.json` under the granting
capability. Those files are gitignored build output and are still not a substitute
for reading `generate_handler!`, but a diff of them no longer hides an app
command by construction.

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
**four** lists — the `#[tauri::command]` definitions under `src-tauri/src/`, the
`generate_handler!` registrations, the entries below, and the commands
`src-tauri/capabilities/*.json` grant through `src-tauri/permissions/` — and is
red unless all four name the same set. The fourth list is why a command added
without a permission fails `npm run lint` instead of failing silently at runtime,
and why a permission left behind by a deleted command cannot idle unnoticed.

Note the two namespaces, because they are not the same and the guard has to
translate. The three lists above name a command by its Rust path
(`commands::display::list_monitors`); the ACL, like the `invoke()` call itself,
names it by the bare function name (`list_monitors`). That namespace is flat, so
two commands with the same function name in different modules are one command to
`tauri` — the guard reports that collision rather than trying to resolve it.

## Commands

- `commands::display::list_monitors` — enumerates the displays the OS reports (FR-101); takes no arguments, reads only `AppHandle`, returns `Vec<Monitor>`. It is the single source of display facts for the frontend, which is why `core:window:default` is deliberately not granted in `capabilities/main-window.json`. Granted to the `main` window alone, by `allow-list-monitors` (`src-tauri/permissions/list_monitors.json`).
