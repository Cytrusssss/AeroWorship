//! Display Service: the one place that reads the display topology, and the sole
//! owner of the Projector Output window's lifetime (PRD §6.3).
//!
//! The frontend never creates, moves or destroys the output window;
//! `commands::display` only reads from here. Everything that could be *wrong* —
//! which display is primary, which one the output belongs on — is decided by
//! pure functions in [`aeroworship_core::models`], because nothing that needs an
//! `AppHandle` can be shown two displays by a test (Tauri 2.11.5's mock runtime
//! reports none, and `tauri::Monitor`'s fields are `pub(crate)`).
//!
//! **What this module knows about the runtime it is written against**, all of it
//! read from the installed `tauri` 2.11.5 / `tauri-runtime-wry` 2.11.4 /
//! `tao` 0.35.3 sources rather than assumed, because every one of these decides
//! whether the output lands on the projector or on the operator's screen:
//!
//! - `WebviewWindowBuilder::position` takes **logical** pixels
//!   (`tauri-runtime-wry` `lib.rs:1003` builds a `TaoLogicalPosition`), and tao
//!   resolves that pair by converting it with *each* monitor's scale factor in
//!   turn and taking the first monitor whose physical rectangle contains the
//!   result (`tao` `windows/window.rs:1174`). Under mixed DPI that can match a
//!   display other than the intended one, and when it matches none the position
//!   is dropped for `CW_USEDEFAULT` on the primary. So this module never places
//!   through the builder: it places afterwards with a `PhysicalPosition`, which
//!   `set_outer_position` applies verbatim (`tao` `windows/window.rs:229`).
//! - `set_fullscreen(true)` becomes `Fullscreen::Borderless(None)`, and tao
//!   resolves `None` to `MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)`
//!   (`tao` `windows/window.rs:789`). **The window must already sit on the
//!   target display before fullscreen is requested** — hence the order below.
//!   (`tauri-runtime-wry` does have a "fullscreen with an explicit position
//!   resolves the target monitor" fixup, at `lib.rs:4608`, but it is
//!   `#[cfg(any(macos, linux))]` and does not run here.)
//! - Asking the *builder* for fullscreen would additionally call
//!   `force_window_active` on creation (`tao` `windows/window.rs:1338`), which
//!   is precisely the focus theft FR-109 forbids. Fullscreen is therefore set
//!   after the window exists, never on the builder.
//! - The sequence is ordered and synchronous because the setup hook runs on the
//!   main thread, where `send_user_message` executes inline
//!   (`tauri-runtime-wry` `lib.rs:239`) as does tao's `execute_in_thread`
//!   (`tao` `windows/event_loop.rs:527`).
//!
//! **The Control Panel needs no code here.** FR-102 also asks for it on the
//! primary display, and `tauri.conf.json` already delivers that: `"center":
//! true` with no explicit position makes `tauri-runtime-wry` centre the window
//! on `event_loop.primary_monitor()` (`lib.rs:4534`), which on Windows is
//! `MonitorFromPoint((0,0), MONITOR_DEFAULTTOPRIMARY)` (`tao`
//! `windows/monitor.rs:116`) — the primary display, by the Windows invariant
//! that the primary's top-left corner *is* the virtual-screen origin. Placing it
//! a second time from here would fight the centring for no gain.

use aeroworship_core::models::{flag_primary, monitor_id, select_output_monitor, Monitor};
use tauri::{
    window::Color, AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};

/// Label of the Control Panel window, as declared in `tauri.conf.json` and as
/// scoped in `capabilities/main-window.json`.
const CONTROL_WINDOW_LABEL: &str = "main";

/// Label of the Projector Output window.
///
/// It is not `"main"`, so `capabilities/main-window.json` — which reads
/// `"windows": ["main"]` — grants this window nothing, and since SEC-01 that
/// holds for this application's own commands too, not only for `plugin:` ones.
/// `src-tauri/permissions/` exists, so the app ships an ACL manifest and
/// `webview/mod.rs:1823` gates app-defined commands exactly like plugin ones; a
/// window no capability names gets `"Command {} not allowed by ACL"`.
/// `capabilities/output-window.json` names this label and grants it nothing, in
/// writing, so the emptiness is a statement rather than an omission.
///
/// Until SEC-01 the opposite was true and this doc comment said so: app-defined
/// commands had no per-window scoping at all, and this window could invoke every
/// entry in `commands.inventory.md` from the moment it existed (ADR-0041). That
/// is closed. `src-tauri/tests/acl_window_scoping.rs` is the proof, not this
/// sentence.
///
/// At least three things it still does not cover, all owned elsewhere and all
/// named here so this label is not read as a stronger boundary than it is. The
/// count is open on purpose: a closed enumeration has been wrong three times in
/// this repository already, and each time the item that was missing was the one
/// nobody had looked for yet.
///
/// 1. `plugin:__TAURI_CHANNEL__|fetch` is exempt from that gate inside tauri's
///    own condition, so no capability decides it.
/// 2. `output.html` shares an origin with `index.html`, so script here reaches
///    the Control Panel document without an `invoke` and without the ACL at all
///    (ADR-0018, ADR-0042).
/// 3. The ACL says nothing about how the document in a window *got there*. A
///    remote page navigated into this window would be same-origin with nothing
///    and granted nothing, but it would also not be this bundle. Since SEC-02
///    that navigation is refused — by
///    [`crate::services::navigation`], a plugin hook rather than anything on
///    the builder below, because this window is only one of the two that needed
///    covering. What the guard does *not* cover is stated in its own module
///    docs; this list stops naming it as an open surface, not as a closed one.
///
/// The first two are written up in `crate::commands`.
pub const OUTPUT_WINDOW_LABEL: &str = "output";

/// The output bundle's entry point, resolved against `frontendDist` in release
/// and against `devUrl` in development — never against a path literal.
const OUTPUT_ENTRY: &str = "output.html";

/// Window title. Never drawn (the window is undecorated); it is what the OS
/// shows in Alt-Tab and in a task manager, where "AeroWorship" alone would be
/// indistinguishable from the Control Panel.
const OUTPUT_TITLE: &str = "AeroWorship Projector Output";

/// Black is the resting state of a projector, and this is the colour the window
/// shows in the gap between being mapped and the webview's first paint. Without
/// it that gap is a white rectangle in front of a congregation.
const OUTPUT_BACKGROUND: Color = Color(0, 0, 0, 255);

/// Every display the OS currently reports (FR-101).
///
/// Ordering is whatever the platform enumerates; on Windows that is
/// `EnumDisplayMonitors` order, which is not the left-to-right arrangement the
/// user sees in Display Settings and must not be treated as such. The primary
/// display is identified by [`Monitor::is_primary`], never by position in this
/// list, and the output display is chosen by
/// [`select_output_monitor`](aeroworship_core::models::select_output_monitor),
/// never by index either.
///
/// **Why this returns no `Result`.** Appendix D says every command returns
/// `Result<T, AppError>`, and `AppError` does not exist yet — inventing its
/// shape is the job of the item that first has a failure worth reporting
/// (ADR-0036), and this is not it. `AppHandle::available_monitors()` is typed
/// `tauri::Result` but cannot actually fail: in `tauri` 2.11.5
/// (`src/app.rs:888`) every arm reachable from an `AppHandle` returns `Ok`, and
/// the only other arm is `unreachable!()`. So the error branch below is dead
/// code today. It resolves to "no displays" rather than to a panic, because
/// zero displays is a state the app has to render anyway (FR-106's
/// single-display fallback is the same code path with one), whereas a panic in
/// a command unwinds into the runtime. When `AppError` lands, this signature is
/// the first that should change.
pub fn connected_monitors<R: Runtime>(app: &AppHandle<R>) -> Vec<Monitor> {
    // Which display is primary is not on `tauri::Monitor` at all, so it is
    // asked for separately and matched back by identity. Matching on the
    // derived id rather than on the name directly keeps the comparison
    // consistent with the ids handed to the frontend, including on the
    // unnamed-monitor fallback path.
    let primary_id = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor_id(monitor.name().map(String::as_str), position_of(&monitor)));

    // This function reads the OS and does nothing else. The matching itself is
    // `flag_primary`, in `aeroworship_core`, because nothing that needs an
    // `AppHandle` can be put in front of any displays by a test, let alone two.
    // On tauri 2.11.5 the two calls below reach `MockRuntimeHandle` (or
    // `MockRuntime`), and there both `primary_monitor` and `available_monitors`
    // are `unimplemented!()` - `test/mock_runtime.rs:245`/`:253` and
    // `:1276`/`:1284`. They panic; they do not return an empty list. (The one
    // mock arm that does return `Ok(Vec::new())`, `:797`, is
    // `WindowDispatch::available_monitors`, reached from a `Window`, which is
    // not the receiver this function holds.) And `tauri::Monitor`'s fields are
    // `pub(crate)` (`window/mod.rs:59-63`), so no test can hand-build one to
    // feed in either way. Everything below that could be wrong in a way a
    // single-display development machine would not show is on the other side of
    // that call (PRD §6.1).
    let reported = app
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|monitor| {
            Monitor::from_os_report(
                monitor.name().map(String::as_str),
                (monitor.size().width, monitor.size().height),
                position_of(monitor),
                monitor.scale_factor(),
                // Placeholder: `flag_primary` sets this field on every entry
                // from `primary_id` alone and never reads what is passed here.
                false,
            )
        })
        .collect();

    flag_primary(reported, primary_id.as_deref())
}

/// Places the windows at start-up (FR-102): the Control Panel is already on the
/// primary display by configuration, and the Projector Output is opened
/// fullscreen and borderless on the first non-primary display.
///
/// Does nothing when there is no display to project onto — a single-display
/// machine must not get a fullscreen window over the operator's own screen.
/// FR-106's windowed preview and its "no projector attached" statement are that
/// item's work, not this one's; what is guaranteed here is only that nothing
/// wrong is opened.
///
/// Returns `()` on purpose: this is called from the setup hook, and a projector
/// window that fails to open must not take the Control Panel down with it
/// (NFR-08). The failure is reported to stderr, which a debug build shows in its
/// console and a release build discards — there is no logging or diagnostics
/// facility yet (NFR-34), and inventing one here would be a second item's
/// decision taken quietly inside this one.
pub fn init<R: Runtime>(app: &AppHandle<R>) {
    let monitors = connected_monitors(app);
    let Some(target) = select_output_monitor(&monitors) else {
        return;
    };
    // No cleanup here on failure, and no lifetime binding here on success:
    // `open_output_window` binds the window to the Control Panel the moment the
    // window exists, so a partially built output window is already tied to the
    // Control Panel by the time the error reaches this line. See the comment at
    // that call.
    if let Err(error) = open_output_window(app, target) {
        eprintln!("FR-102: could not open the output window: {error}");
    }
}

/// Makes the output window die with the Control Panel.
//
// Not tidiness: the process exits when the *last* window is destroyed
// (`tauri-runtime-wry` `lib.rs:4310`), and this item is the first to create a
// second one. Without this, closing the Control Panel leaves AeroWorship running
// with only the output window alive — undecorated, off the taskbar, on a display
// that is often switched off — and Task Manager is the only way out of it.
//
// Whatever FR-104 later decides about creating and destroying the output window
// as displays come and go, "it does not outlive the window that drives it" is
// not a decision that belongs to a hot-plug handler.
fn close_output_with_control_panel<R: Runtime>(app: &AppHandle<R>) {
    let Some(control) = app.get_webview_window(CONTROL_WINDOW_LABEL) else {
        return;
    };
    let app = app.clone();
    // Closing from inside a window-event handler is supported: the runtime drops
    // its borrow of the window store before dispatching to listeners, precisely
    // so that a handler may create or close windows (`tauri-runtime-wry`
    // `lib.rs:4276`).
    control.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            if let Some(output) = app.get_webview_window(OUTPUT_WINDOW_LABEL) {
                // The only failure this can report is that the window is already
                // gone, which is the state being asked for.
                let _ = output.close();
            }
        }
    });
}

/// Creates the Projector Output window, fullscreen and borderless, filling
/// `target`.
///
/// The order of the four placement calls is load-bearing (ADR-0040); see the
/// module documentation before changing it. The lifetime binding that sits
/// between `build` and those four calls is load-bearing for a different reason,
/// stated where it happens.
fn open_output_window<R: Runtime>(app: &AppHandle<R>, target: &Monitor) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(
        app,
        OUTPUT_WINDOW_LABEL,
        WebviewUrl::App(OUTPUT_ENTRY.into()),
    )
    .title(OUTPUT_TITLE)
    // `use_https_scheme` is deliberately NOT called here, and that is load
    // bearing outside this file: this window has no `tauri.conf.json` entry, so
    // `services::navigation` cannot read its spelling and falls back to the
    // documented `WebviewAttributes` default, `false` — i.e. it expects this
    // window to be served from `http://tauri.localhost`. Turning it on here
    // without teaching `app_origins` about this label makes the guard refuse
    // this window's own pages (SEC-02).
    .background_color(OUTPUT_BACKGROUND)
    // "Borderless" (FR-102). The rest of the chrome FR-105 asks about — context
    // menu, text selection, dev-tools — is that item's, and is not touched here.
    .decorations(false)
    // An undecorated window still keeps its resize border on Windows unless
    // this is off, so without it a stray drag at the edge of the projector
    // image resizes the surface this function just measured.
    .resizable(false)
    // The projector is not a window the operator switches to. A taskbar button
    // is one click away from raising it over the Control Panel.
    .skip_taskbar(true)
    // Focus stays with the Control Panel, which by now exists and is active:
    // windows declared in `tauri.conf.json` are created before the setup hook
    // runs (`tauri` `app.rs:2524`). `focused(false)` sets tao's
    // `MARKER_DONT_FOCUS`, which the first `show()` consumes as
    // `SW_SHOWNOACTIVATE` (`tao` `windows/window_state.rs:325`) — so it only
    // does its job in combination with `visible(false)` here and an explicit
    // `show()` at the end.
    .focused(false)
    // Created hidden so the placement below is never seen happening: a window
    // mapped at the OS's default position and then moved is a flash of the
    // wrong content on the wrong display.
    .visible(false)
    .build()?;

    // Bound here — after `build`, before any placement — rather than by the
    // caller once the whole function has succeeded.
    //
    // `build` has already registered the window with the runtime's window
    // store, so from this line onwards a window exists that the runtime counts
    // when it decides whether the last one is gone. Every `?` below is an early
    // return out of this function; an early return taken before the binding
    // would leave that window alive, still `visible(false)`, and attached to
    // nothing — the exact state `close_output_with_control_panel` exists to
    // prevent, reachable only on the error path.
    //
    // Of the two ways to close that hole, this is the one that survives being
    // edited. Unwinding on the error path instead would need a cleanup at every
    // failure point, which makes a step added later orphaning by default;
    // binding first covers steps that do not exist yet. What it costs is that
    // the binding outlives a failed placement, and that costs nothing: a window
    // that failed to be placed should die with the Control Panel too.
    //
    // Position: this call must stay between `build` and the placement below —
    // it cannot move earlier, and moving it later re-opens the hole. It is not
    // part of the ADR-0040 sequence; the four calls after it are, and those may
    // not be reordered among themselves at all.
    close_output_with_control_panel(app);

    // Physical pixels, applied verbatim. `Monitor` reports the OS's own
    // virtual-screen coordinates (never divided by `scale_factor`), and this is
    // the only placement API that does not re-derive a scale factor of its own.
    window.set_position(PhysicalPosition::new(target.x, target.y))?;
    // Sized to the display before going fullscreen rather than relying on the
    // fullscreen transition alone: this is the geometry tao saves as the
    // pre-fullscreen placement, so if fullscreen is ever dropped — FR-104's
    // disconnect is the obvious way — the window is still exactly the output
    // display rather than an 800x600 default somewhere else.
    window.set_size(PhysicalSize::new(target.width, target.height))?;
    // Only now: fullscreen resolves the display from where the window *is*.
    window.set_fullscreen(true)?;
    window.show()?;

    // Deliberately not set here: `always_on_top`. FR-109 wants the output above
    // its own display only, and never stealing focus; a bare
    // `.always_on_top(true)` is `WS_EX_TOPMOST`, which is global, and would put
    // the projector surface over the Control Panel's dialogs on the other
    // display. That is FR-109's problem to solve properly.
    Ok(())
}

/// The monitor's top-left corner as a plain pair, so `tauri` types stop at this
/// module's edge and `aeroworship_core` never sees one.
fn position_of(monitor: &tauri::Monitor) -> (i32, i32) {
    let position = monitor.position();
    (position.x, position.y)
}
