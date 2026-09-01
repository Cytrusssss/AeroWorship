//! The three parts of FR-105 that live in Rust, held to the only kind of
//! assertion this runtime allows: source text and manifest text.
//!
//! **Read this before adding a test here, and before believing one.** Every
//! assertion in this file is a *grep*, not a compiler and not a running
//! webview. It proves that a line is still written in a file. It cannot prove
//! that WebView2 obeys it, that a right click raises nothing, or that Ctrl-S
//! opens no dialog. The same limitation is stated at
//! `lib_rs_still_registers_the_guard_in_the_builder_chain` in
//! `navigation_guard.rs`, which is the precedent this file follows; SEC-02
//! calls it "the weakest link", and so is every link here.
//!
//! **Why there is nothing stronger, measured rather than assumed.** In `tauri`
//! 2.11.5 `src/test/mock_runtime.rs`:
//!
//! - `with_webview` (`:568-570`) is `Ok(())` — the closure is dropped without
//!   ever being called, so the whole of `services::webview_chrome::apply` is
//!   unreachable under a mock. A test that called `harden` and asserted "it did
//!   not panic" would be asserting that `Ok(())` was returned.
//! - `grep -c` gives **0** for `context_menu`, `accelerator` and `download` in
//!   that file. There is no mock state to read back.
//! - `is_decorated` (`:757-759`) and `is_resizable` (`:761-763`) are hardcoded
//!   `Ok(false)`, and `decorations(self, _) -> Self` (`:435-437`) returns
//!   `self` without storing anything. So `assert!(!window.is_decorated())`
//!   passes with `.decorations(false)` **deleted** from
//!   `services::display`. Those tests are not written here, on purpose: a test
//!   that cannot fail is worse than a missing one, because it looks like cover.
//!
//! So: no chrome, no title bar, no context menu, no accelerator keys and no
//! refused download is verified here as *behaviour*. What is verified is that
//! the lines which produce them are still in the files that produce them, and
//! that the release profile still switches devtools off by construction.
//!
//! This file lives under `src-tauri/tests/` for the reason
//! `acl_window_scoping.rs` gives at its foot — Cargo compiles an integration
//! test only from the `tests/` directory of the package it belongs to — and
//! reads its subjects with `include_str!`, so it runs no window and needs no
//! runtime at all.

/// Strips line comments, so a commented-out line cannot satisfy an assertion
/// below.
///
/// Splits each line at the first `//`, which also truncates a `//` inside a
/// string literal. Neither file read here has one (checked, not assumed), and
/// the direction of the error is the loud one: a truncated line makes an
/// assertion fail, never pass. Same helper, same reasoning, as
/// `navigation_guard.rs`.
fn without_comments(code: &str) -> String {
    code.lines()
        .map(|line| line.split_once("//").map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strips `#` comments from TOML for the same reason, and with the same caveat:
/// a `#` inside a string would truncate the line. `src-tauri/Cargo.toml` has
/// none, and a truncation could only make the assertions below fail.
fn without_toml_comments(manifest: &str) -> String {
    manifest
        .lines()
        .map(|line| line.split_once('#').map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The context menu and the browser accelerator keys are switched off, and
/// switched off before the projector is ever mapped.
///
/// Both are `ICoreWebView2Settings` properties with no builder to sit on, so
/// the only thing that applies them is this call. Delete it and every gate
/// stays green while the projector answers a right click with the Edge menu and
/// Ctrl-S with a native save dialog — into a window
/// `capabilities/output-window.json` declares granted nothing.
///
/// The ordering half is not tidiness either: `services::display` creates the
/// window `visible(false)` and calls `show()` last precisely so that the
/// hardening lands before anything is on screen. A call moved after `show()`
/// leaves a window that is briefly live with WebView2's defaults on it, and no
/// other test in this repository would notice.
#[test]
fn display_rs_still_hardens_the_output_window_before_it_is_shown() {
    const HARDEN: &str = "webview_chrome::harden(&window)";
    const SHOW: &str = "window.show()";
    let code = without_comments(include_str!("../src/services/display.rs"));

    let hardened_at = code.find(HARDEN).unwrap_or_else(|| {
        panic!(
            "`src/services/display.rs` no longer contains `{HARDEN}` outside a comment. That \
             call is the only thing that switches off WebView2's default context menu and, on \
             release profiles, its browser accelerator keys (FR-105, ADR-0042 finding 2): \
             `tauri` wraps neither setting, so without this line the projector keeps both and \
             nothing else in this repository changes"
        )
    });
    let shown_at = code.find(SHOW).unwrap_or_else(|| {
        panic!(
            "`src/services/display.rs` no longer contains `{SHOW}` outside a comment, so the \
             ordering this test exists to pin cannot be read. If the window is now shown some \
             other way, assert the new spelling here rather than dropping the check"
        )
    });

    assert!(
        hardened_at < shown_at,
        "`src/services/display.rs` calls `{HARDEN}` after `{SHOW}`. The output window is built \
         `visible(false)` and shown last so that it is never mapped with WebView2's defaults \
         still on it; hardening after the window is on the projector leaves a window that can \
         raise the Edge context menu in front of a congregation, however briefly"
    );
}

/// Every download request from the output window is still answered with a
/// refusal.
///
/// The handler being *installed* is not the property that matters, and reading
/// the code as if it were is the mistake ADR-0042's own correction records:
/// `wry`'s `WebViewAttributes::default()` already sets
/// `download_started_handler: Some(Box::new(|_, _| true))` (`wry`
/// `src/lib.rs:830`), which permits everything. What FR-105 needs is a handler
/// whose answer is `false` — that is what reaches `args.SetCancel(true)` and
/// what keeps Edge's download bar off the congregation's screen.
///
/// So this test asserts the *answer*, not the presence of the call: the closure
/// passed to `.on_download(` must end in `false`, and must contain no bare
/// `true` line. Flipping that one word is the whole of the regression, and it
/// is a word no other test in this repository reads.
///
/// **What pins this to formatting, said plainly.** The tail expression is
/// identified as the last non-empty line before the closing `})`, which is
/// where `rustfmt` puts it. `cargo fmt --check` is a gate, so that shape is
/// enforced rather than hoped for; if the closure is ever replaced by a named
/// function this test must be rewritten to read that function, not relaxed.
#[test]
fn the_download_policy_on_the_output_window_is_still_a_refusal() {
    const HANDLER: &str = ".on_download(";
    const BUILD: &str = ".build()";
    let code = without_comments(include_str!("../src/services/display.rs"));

    let start = code.find(HANDLER).unwrap_or_else(|| {
        panic!(
            "`src/services/display.rs` no longer contains `{HANDLER}` outside a comment. \
             `tauri` sets `download_handler: None` in both `WebviewBuilder` constructors and \
             never replaces it, so without this call `wry`'s own default survives — a handler \
             that answers `true` to everything. A programmatic `<a download>` then writes a \
             file to the operator's Downloads folder from the one window \
             `capabilities/output-window.json` declares granted nothing (FR-105, SEC-02 audit \
             finding 5)"
        )
    });
    let end = code[start..]
        .find(BUILD)
        .map(|offset| start + offset)
        .unwrap_or_else(|| {
            panic!(
                "`src/services/display.rs` has `{HANDLER}` but no `{BUILD}` after it, so the \
                 download closure could not be delimited and nothing about its answer was \
                 verified"
            )
        });

    let closure: Vec<&str> = code[start..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    assert!(
        !closure.contains(&"true"),
        "the closure passed to `{HANDLER}` in `src/services/display.rs` contains a bare `true`. \
         `true` is *permit*: it lets WebView2 write the file and raise its download bar on the \
         projector. FR-105's policy is that the projector displays and does not save"
    );
    assert_eq!(
        closure.iter().rev().nth(1).copied(),
        Some("false"),
        "the closure passed to `{HANDLER}` in `src/services/display.rs` no longer ends in \
         `false`. Its return value *is* the policy: `false` reaches `args.SetCancel(true)` \
         (`wry` `src/webview2/mod.rs:857-863`) and anything else permits the download. The \
         handler being installed proves nothing — `wry`'s default handler is installed too, \
         and it permits everything"
    );
}

/// A shipped binary still reaches `SetAreDevToolsEnabled(false)` without a line
/// of ours, and both of the two facts that make that true are still in
/// `src-tauri/Cargo.toml`.
///
/// FR-105's "no dev-tools access in release builds" is satisfied by
/// construction rather than by code, through four facts that meet. Two are
/// upstream and cannot regress from this repository: `wry` defaults
/// `WebViewAttributes::devtools` to `true` under `debug_assertions` and `false`
/// without it (`wry` `src/lib.rs:834-837`), and `tauri-runtime-wry` only calls
/// `with_devtools` at all inside
/// `#[cfg(any(debug_assertions, feature = "devtools"))]` (`src/lib.rs:5209-5211`).
/// The other two are this file's, and are what this test holds:
/// `tauri`'s `devtools` feature is not enabled, and `[profile.release]` pins
/// `debug-assertions = false`.
///
/// Either one flipped puts a working F12 on the projector in a shipped
/// installer, and neither shows up in any other gate: enabling a Cargo feature
/// compiles cleanly and every test in this repository stays green.
///
/// **What this does not cover.** It reads the manifest, so it sees only
/// features this package asks for by name. A `devtools` switched on
/// transitively — another crate depending on `tauri` with that feature, unified
/// into this build — is invisible here; only a resolved feature graph
/// (`cargo tree -e features -p tauri`) would show it. Today the workspace has
/// exactly one crate that depends on `tauri` at all, and this is the manifest
/// it uses.
#[test]
fn the_release_profile_still_leaves_devtools_switched_off() {
    let manifest = without_toml_comments(include_str!("../Cargo.toml"));

    assert!(
        !manifest.contains("devtools"),
        "`src-tauri/Cargo.toml` now names `devtools` outside a comment. Enabling `tauri`'s \
         `devtools` feature makes `tauri-runtime-wry` call `with_devtools` on release profiles \
         too (`src/lib.rs:5209`), so a shipped installer opens the developer console on the \
         projector — which FR-105 forbids in release builds. Nothing else in this repository \
         fails when that feature is added"
    );

    let profile = manifest
        .split_once("[profile.release]")
        .map(|(_, after)| after.split("\n[").next().unwrap_or(after))
        .unwrap_or_else(|| {
            panic!(
                "`src-tauri/Cargo.toml` has no `[profile.release]` section. Without it \
                 `debug-assertions` falls back to the profile default, and FR-105's \
                 devtools clause rests on that value being `false` explicitly — see \
                 `src/services/webview_chrome.rs`"
            )
        });

    assert!(
        profile.contains("debug-assertions = false"),
        "`[profile.release]` in `src-tauri/Cargo.toml` no longer pins \
         `debug-assertions = false`. That single value decides `wry`'s devtools default \
         (`src/lib.rs:834-837`) and whether `tauri-runtime-wry` compiles its `with_devtools` \
         call at all — turning it on ships a release build with a reachable developer console \
         on the projector, and it also disarms the `custom-protocol` guard in `src/lib.rs` \
         that the same section's comment describes"
    );
}

/// The two settings `harden` exists to write are still written, still written
/// with `false`, and the context menu is still switched off on **both**
/// profiles.
///
/// `display.rs` calling `harden` is wiring; these two lines are the payload,
/// and nothing else in this repository reads them. `services::webview_chrome`
/// is unreachable under `MockRuntime` — `with_webview` drops the closure — so
/// deleting either call, or flipping either argument to `true`, compiles
/// cleanly and leaves every gate green while the projector answers a right
/// click with the Edge menu or Ctrl-S with a save dialog.
///
/// The ordering assertion is the module's own claim, and it is the one most
/// easily lost in an edit: the `cfg!(debug_assertions)` early return exists to
/// keep **F12** alive for the pending SEC-01/SEC-02 runtime verification, and
/// it must therefore come *after* the context menu is switched off. Moved one
/// statement earlier it takes the context menu away from debug builds too —
/// which is not a security regression but is the profile split silently
/// changing meaning, and it is the profile the next verification is done on.
#[test]
fn the_two_webview2_settings_fr_105_writes_are_still_written_and_still_false() {
    const CONTEXT_MENU: &str = "SetAreDefaultContextMenusEnabled(false)";
    const ACCELERATORS: &str = "SetAreBrowserAcceleratorKeysEnabled(false)";
    const RELEASE_ONLY: &str = "if cfg!(debug_assertions)";
    let code = without_comments(include_str!("../src/services/webview_chrome.rs"));

    let context_menu_at = code.find(CONTEXT_MENU).unwrap_or_else(|| {
        panic!(
            "`src/services/webview_chrome.rs` no longer calls `{CONTEXT_MENU}`. That call is \
             the whole of FR-105's \"no context menu\": `wry` 0.55.1 defaults the setting to \
             `true` (`src/lib.rs:1687-1688`) and `tauri-runtime-wry` never touches it, so \
             without this line a right click raises the Edge menu on the congregation's \
             screen. Check whether the argument was flipped to `true` rather than the call \
             removed"
        )
    });
    let accelerators_at = code.find(ACCELERATORS).unwrap_or_else(|| {
        panic!(
            "`src/services/webview_chrome.rs` no longer calls `{ACCELERATORS}`. That call is \
             the whole of FR-105's \"no browser accelerator keys\": without it Ctrl-S and \
             Ctrl-P open the native save and print dialogs from the one window \
             `capabilities/output-window.json` declares granted nothing. Check whether the \
             argument was flipped to `true` rather than the call removed"
        )
    });
    let release_only_at = code.find(RELEASE_ONLY).unwrap_or_else(|| {
        panic!(
            "`src/services/webview_chrome.rs` no longer contains `{RELEASE_ONLY}` outside a \
             comment, so the profile split this test pins cannot be read. If the accelerator \
             keys are now switched off on debug builds too, F12 goes with them — that is a \
             deliberate decision to take in the open, not one to make by deleting a branch"
        )
    });

    assert!(
        context_menu_at < release_only_at,
        "`src/services/webview_chrome.rs` switches off the context menu only after the \
         `{RELEASE_ONLY}` early return, so debug builds now keep the WebView2 context menu. \
         That branch exists to keep F12 alive on debug builds; the context menu is switched \
         off on both profiles on purpose, so that the surface most likely to regress is \
         identical between them"
    );
    assert!(
        release_only_at < accelerators_at,
        "`src/services/webview_chrome.rs` switches off the browser accelerator keys before the \
         `{RELEASE_ONLY}` early return, so debug builds lose F12 and Ctrl-Shift-I — the \
         console the pending SEC-01 and SEC-02 runtime verifications have to be typed into"
    );
}

/// The three builder calls that make FR-105's "no chrome, no title bar" true
/// are still on the output window's builder.
///
/// **Why this is a grep and not an assertion on the window.** It is the mock
/// that forces this, and the shape it forces is the dangerous one:
/// `MockRuntime::is_decorated` and `is_resizable` are hardcoded `Ok(false)`
/// (`tauri` 2.11.5 `src/test/mock_runtime.rs:757-763`) and
/// `decorations(self, _) -> Self` (`:435-437`) stores nothing. So
/// `assert!(!window.is_decorated())` is green with `.decorations(false)`
/// deleted — a test that passes *because the mock cannot say anything else*.
/// Reading the source is weaker in what it proves but is at least capable of
/// failing, which the alternative is not. Nothing here shows the window has no
/// title bar; it shows the line that asks for none has not been deleted.
///
/// `resizable(false)` and `skip_taskbar(true)` are asserted with it because all
/// three are one property from the operator's side — an undecorated window
/// still keeps its Windows resize border without the second, and the third is
/// what keeps a taskbar button from raising the projector over the Control
/// Panel.
#[test]
fn the_output_window_is_still_built_without_chrome() {
    let code = without_comments(include_str!("../src/services/display.rs"));

    for (call, consequence) in [
        (
            ".decorations(false)",
            "the projector gets a title bar and a border in front of the congregation; this \
             call is the whole of FR-105's \"no chrome, no title bar\"",
        ),
        (
            ".resizable(false)",
            "an undecorated window keeps its resize border on Windows, so a stray drag at the \
             edge of the projector image resizes the surface `open_output_window` just measured",
        ),
        (
            ".skip_taskbar(true)",
            "the projector gets a taskbar button, which is one click away from raising it over \
             the Control Panel",
        ),
    ] {
        assert!(
            code.contains(call),
            "`src/services/display.rs` no longer calls `{call}` on the output window's \
             builder: {consequence}. No runtime assertion can replace this one — \
             `MockRuntime` answers `is_decorated` and `is_resizable` with a hardcoded \
             `Ok(false)` whether the call is there or not"
        );
    }
}
