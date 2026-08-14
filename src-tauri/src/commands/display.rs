//! Display enumeration (FR-101).
//!
//! This is the only place the frontend learns which displays exist. The window
//! API that would give it a second answer — `core:window:default` brings
//! `allow-available-monitors`, `allow-primary-monitor` and friends — is
//! deliberately not granted in `capabilities/main-window.json`, so display
//! facts have exactly one source of truth and it is this command (PRD §6.3).

use aeroworship_core::models::{flag_primary, monitor_id, Monitor};
use tauri::{AppHandle, Runtime};

/// Every display the OS currently reports (FR-101).
///
/// Ordering is whatever the platform enumerates; on Windows that is
/// `EnumDisplayMonitors` order, which is not the left-to-right arrangement the
/// user sees in Display Settings and must not be treated as such. The primary
/// display is identified by [`Monitor::is_primary`], never by position in this
/// list.
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
#[tauri::command]
pub fn list_monitors<R: Runtime>(app: AppHandle<R>) -> Vec<Monitor> {
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
    // `AppHandle` can be put in front of two displays by a test: Tauri 2.11.5's
    // mock runtime hardcodes `available_monitors()` to an empty `Vec`
    // (`test/mock_runtime.rs:797`) and `tauri::Monitor`'s fields are
    // `pub(crate)`, so no test can hand-build one either. Everything below that
    // could be wrong in a way a single-display development machine would not
    // show is on the other side of that call (PRD §6.1).
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

/// The monitor's top-left corner as a plain pair, so `tauri` types stop at this
/// module's edge and `aeroworship_core` never sees one.
fn position_of(monitor: &tauri::Monitor) -> (i32, i32) {
    let position = monitor.position();
    (position.x, position.y)
}
