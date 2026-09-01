//! The two pieces of WebView2 chrome that no `tauri` API can switch off, turned
//! off on the Projector Output window (FR-105, ADR-0042 finding 2).
//!
//! **What was wrong, stated as the runtime actually behaves.** `wry` 0.55.1
//! gives both of these `true` by default (`src/lib.rs:1687-1688`, commented
//! "This is WebView2's default behavior") and `tauri-runtime-wry` 2.11.4 never
//! calls `with_default_context_menus` or `with_browser_accelerator_keys` — it
//! imports `WebViewBuilderExtWindows` and uses it for four other things only
//! (`src/lib.rs:61`, `:4837`, `:5056`, `:5063`, `:5070`). So on the projector:
//! a right click raises the Edge context menu **on the congregation's screen**,
//! and Ctrl-S / Ctrl-P raise the native save and print dialogs — a filesystem
//! write path out of the one window `capabilities/output-window.json` declares
//! granted nothing.
//!
//! # Why the settings object and not CSS or an initialisation script
//!
//! `contextmenu` can be cancelled from the page, and `keydown` cancels Ctrl-S
//! and Ctrl-P in Chromium, so a script would close the same two holes without a
//! single dependency. It is not used, for one reason: that script would live
//! **inside the document**, and NFR-28 says the document is where untrusted
//! input ends up. A capture-phase listener registered on `window` by anything
//! that loads later can call `stopImmediatePropagation` and the suppression is
//! gone — with every gate still green, and with nothing visible until somebody
//! right-clicks in front of a congregation. A WebView2 setting is held by the
//! browser process; no API reachable from the page can put it back.
//!
//! **What it costs, said plainly.** The failure direction is the *open* one: if
//! the closure below never runs, or a COM call returns a failing `HRESULT`, the
//! window keeps WebView2's defaults and the only trace is a line on stderr —
//! which a release build has no console to receive, exactly as
//! [`crate::services::navigation`] already records for its own refusals
//! (NFR-34 owns that). The CSS route fails in the same direction but louder: a
//! stylesheet that does not load takes the whole projector surface with it.
//! Text selection is closed in CSS regardless, in `src/output/Renderer.vue`,
//! because no WebView2 setting governs it.
//!
//! # This is applied to the Projector Output window only
//!
//! [`crate::services::display`] is the only caller, and it passes the `output`
//! window. The Control Panel is deliberately left alone, and not only because
//! FR-105 names the output window: `AreDefaultContextMenusEnabled = false`
//! removes cut/copy/paste and the spell-check suggestions from every text field
//! in the editor, which is a product decision belonging to whoever builds those
//! fields. Hardening `main` would also need a different call site — that window
//! is created by `tauri` from `tauri.conf.json`, so this crate never holds its
//! builder and would have to reach it by label — the same structural asymmetry
//! ADR-0053 decision 1 records for navigation.
//!
//! # Debug builds keep F12, on purpose
//!
//! `AreBrowserAcceleratorKeysEnabled = false` switches off the whole documented
//! browser-accelerator set, and **F12 and Ctrl-Shift-I are in it**. Doing that
//! in a debug build would take the developer console away from the one window
//! whose console the pending SEC-01 and SEC-02 runtime verifications have to be
//! typed into. So it is applied on release profiles only. The context menu is
//! switched off in **both** profiles, which keeps the surface most likely to
//! regress identical between them, and no devtools route is lost by it that F12
//! does not still provide.
//!
//! Nothing here touches devtools itself, because nothing needs to: `wry`
//! defaults `WebViewAttributes::devtools` to `true` under `debug_assertions`
//! and `false` without it (`wry` `src/lib.rs:834-837`), and
//! `tauri-runtime-wry` only calls `with_devtools` at all inside
//! `#[cfg(any(debug_assertions, feature = "devtools"))]` (`src/lib.rs:5209`).
//! `src-tauri/Cargo.toml` does not enable `tauri`'s `devtools` feature and its
//! `[profile.release]` pins `debug-assertions = false`, so a shipped binary
//! reaches `settings.SetAreDevToolsEnabled(false)` (`wry`
//! `src/webview2/mod.rs:573`) without a line of ours. A
//! `WebviewWindowBuilder::devtools(true)` call would be worse than useless
//! here: on a release profile `tauri-runtime-wry` never reads the field, so it
//! would read as a switch that does nothing.
//!
//! # What this does not cover
//!
//! Left open-ended on purpose; a closed enumeration has been wrong every time
//! this repository has written one.
//!
//! 1. **`window.print()` and `window.find()` called from script** are not
//!    accelerator keys and are not affected. The setting closes the *keys*.
//! 2. **Ctrl-C, Ctrl-V and Ctrl-X are not in the set** — they are editing
//!    shortcuts, not browser ones, and they stay. Clipboard access is a
//!    separate `wry` attribute, `false` by default and enabled nowhere in this
//!    crate.
//! 3. **The window between `build()` and this call.** The settings are applied
//!    after the webview exists. Nothing untrusted is loaded there today —
//!    `Renderer.vue` renders one string constant — and the window is still
//!    `visible(false)` when this runs, but "no chrome has ever been reachable"
//!    is not a claim this module can make.
//! 4. **The list of keys is Microsoft's, not `wry`'s.** `wry` forwards one
//!    boolean (`src/webview2/mod.rs:582-586`); which keys it covers is
//!    documented by WebView2 and has *not* been verified here by running it.
//!    That verification is the user's, like the rest of FR-105.
//!
//! # Whether any of this can be asserted headlessly
//!
//! No. `MockRuntime::with_webview` (`tauri` 2.11.5
//! `src/test/mock_runtime.rs:568-570`) takes the closure and returns `Ok(())`
//! **without calling it** — the body is one line. `grep -c` on that file gives
//! zero for both `context_menu` and `accelerator`, the same nothing SEC-02
//! found for `navigation`. A headless test can prove that `harden` is reached
//! and returns; it cannot prove that a single setting changed. That part is the
//! user's to verify with a projector, and it is written here rather than left
//! for a tester to discover.

use tauri::{Runtime, WebviewWindow};

/// Switches off WebView2's default context menu on `window`, and on release
/// profiles its browser accelerator keys as well.
///
/// Takes a `&WebviewWindow` rather than a builder because neither setting has a
/// builder to sit on: they live on `ICoreWebView2Settings`, which does not
/// exist until the webview does. See the module docs for the profile split and
/// for what stays open.
///
/// Reports failure to stderr and returns, like
/// [`crate::services::display::init`]: a projector that can show a context menu
/// is worse than one that cannot, but it is not worse than no projector at all
/// (NFR-08).
///
/// **Runs synchronously here.** `with_webview` posts a runtime message, and
/// `tauri-runtime-wry` executes those inline when the caller is already on the
/// main thread (`src/lib.rs:235-247`); the setup hook this is reached from is.
/// The `#[cfg(windows)]` arm at `src/lib.rs:4030-4036` is what hands the
/// closure its `controller`.
pub fn harden<R: Runtime>(window: &WebviewWindow<R>) {
    let label = window.label().to_owned();
    let label_for_closure = label.clone();

    if let Err(error) = window.with_webview(move |webview| apply(&label_for_closure, webview)) {
        eprintln!(
            "FR-105: window `{label}` kept WebView2's default context menu and accelerator keys: \
             the platform webview could not be reached ({error})."
        );
    }
}

/// The Windows arm: everything FR-105 asks for that is a WebView2 setting.
#[cfg(windows)]
fn apply(label: &str, webview: tauri::webview::PlatformWebview) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
    use windows_core::Interface;

    // SAFETY: `controller()` returns the `ICoreWebView2Controller` wry built for
    // this window, and this closure runs on the thread that owns it. Both calls
    // are WebView2 property getters that take no pointer of ours and report
    // failure as an `HRESULT` rather than by returning garbage.
    let settings = unsafe {
        webview
            .controller()
            .CoreWebView2()
            .and_then(|core| core.Settings())
    };
    let settings = match settings {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!(
                "FR-105: window `{label}` kept WebView2's default chrome: its settings object \
                 could not be read ({error})."
            );
            return;
        }
    };

    // SAFETY: a property setter on the interface obtained above, taking a plain
    // `BOOL`.
    if let Err(error) = unsafe { settings.SetAreDefaultContextMenusEnabled(false) } {
        eprintln!(
            "FR-105: window `{label}` still raises the WebView2 context menu on right click \
             ({error})."
        );
    }

    // Release profiles only — see the module docs. F12 is in the set this
    // switches off, and a debug build is where SEC-01's and SEC-02's runtime
    // verification has to be typed.
    if cfg!(debug_assertions) {
        return;
    }

    // `ICoreWebView2Settings3` is a later revision of the same object, so this
    // is a `QueryInterface` on an interface already held; a WebView2 runtime
    // older than that revision answers `E_NOINTERFACE` rather than failing to
    // load.
    match settings.cast::<ICoreWebView2Settings3>() {
        Ok(settings3) => {
            // SAFETY: as above.
            if let Err(error) = unsafe { settings3.SetAreBrowserAcceleratorKeysEnabled(false) } {
                eprintln!(
                    "FR-105: window `{label}` still answers Ctrl-S and Ctrl-P with a native \
                     dialog ({error})."
                );
            }
        }
        Err(error) => {
            eprintln!(
                "FR-105: window `{label}` still answers Ctrl-S and Ctrl-P with a native dialog: \
                 this WebView2 runtime does not implement ICoreWebView2Settings3 ({error})."
            );
        }
    }
}

/// Everywhere else: nothing, and that is not an omission.
///
/// NFR-17 ships Windows only, and both settings are properties of WebView2. The
/// arm exists so this module still compiles on a machine that is not Windows,
/// rather than being `#[cfg]`-ed out of the crate — which would move the
/// failure from "the projector keeps its context menu" to "the module is gone".
#[cfg(not(windows))]
fn apply(label: &str, _webview: tauri::webview::PlatformWebview) {
    eprintln!(
        "FR-105: window `{label}` keeps its default context menu and accelerator keys: this is \
         not Windows, and both are WebView2 settings (NFR-17)."
    );
}
