# AeroWorship

Offline-first church presentation software for Windows. See `docs/PRD.md` for
the requirements and `decisions.md` for the architecture decision log.

## Prerequisites

- Node.js 22 LTS with npm 10
- Rust stable (see `rust-version` in `src-tauri/Cargo.toml`)
- The Tauri 2 prerequisites for Windows (WebView2 runtime, MSVC build tools)

## Install

```
npm ci
```

**`npm ci`, not `npm install`.** A bare `npm install` may re-resolve version
ranges and rewrite `package-lock.json` as a side effect of what looks like a
read-only setup step; the drift then lands in whatever commit happens to be
open, and two people "who just installed" build against different dependency
trees. `npm ci` installs exactly what the lockfile pins and fails instead of
editing it.

This is enforced, not just documented: the `preinstall` hook
(`scripts/check-install-command.js`) rejects a bare `npm install`. Adding a
dependency on purpose is unaffected — npm does not run the root lifecycle
scripts for `npm install <package>`:

```
npm install <package>              # runtime dependency
npm install --save-dev <package>   # tooling
npm update <package>
```

Commit the resulting `package-lock.json` with the change.

## Verification commands

These six are the gate. All of them must exit 0.

| Layer | Command |
| --- | --- |
| Rust test | `cargo test --workspace --manifest-path src-tauri/Cargo.toml` |
| Rust lint | `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` |
| Rust format | `cargo fmt --all --manifest-path src-tauri/Cargo.toml -- --check` |
| Frontend test | `npm run test` |
| Frontend lint | `npm run lint` |
| Typecheck | `npm run typecheck` |

The `--all` on `cargo fmt` and the `--workspace` on `cargo test` are load-bearing;
do not "tidy" them away for symmetry with `clippy` (ADR-0020). `--manifest-path`
selects the `aeroworship` package, not the workspace, so without those flags
`aeroworship-core` — where the correctness-critical logic of PRD §6.1 and the
targets of NFR-32 / GATE-G10 live — is skipped in silence and the command still
exits 0, which is worse than a red build. `cargo clippy` needs no such flag: it
reaches every workspace member through `RUSTC_WORKSPACE_WRAPPER` as it builds
them. The asymmetry is the correct state, not an oversight.

`npm run lint` runs ESLint and then, through the `postlint` lifecycle,
`prettier --check`. `npm run format` rewrites the files Prettier owns.

## Build

```
npm run build          # Vite, both entry bundles, then the dist HTML guard
npm run tauri build    # full installer; runs `npm run build` first
```

`npm run build` is followed by `postbuild`, which runs
`scripts/check-dist-html.js` over every `dist/*.html` and fails the build on an
inline `<style>` block, an inline `style` attribute in any of its three value
syntaxes (double-quoted, single-quoted, unquoted) or a `<script>` element with a
body. The production CSP is `style-src 'self'; script-src 'self'`
(ADR-0015) and is not enforced at all during `tauri dev`, so this check is the
only place such a regression becomes visible before a user sees it. Because
`tauri.conf.json` sets `beforeBuildCommand: "npm run build"`, it also gates
`npm run tauri build`.

## Development

```
npm run dev            # Vite dev server on :1420 (frontend only)
npm run tauri dev      # the app
```

Do not build a release binary with plain `cargo build --release`: without
`tauri/custom-protocol` the binary embeds no assets and loads its UI from
`http://localhost:1420`. The shell crate refuses to compile in that
configuration (ADR-0016 / ADR-0018). For a release-profile test or benchmark,
pass the feature explicitly:

```
cargo test --release -p aeroworship --features tauri/custom-protocol
```
