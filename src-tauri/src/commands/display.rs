//! Display enumeration (FR-101).
//!
//! This is the only place the frontend learns which displays exist. The window
//! API that would give it a second answer — `core:window:default` brings
//! `allow-available-monitors`, `allow-primary-monitor` and friends — is
//! deliberately not granted in `capabilities/main-window.json`, so display
//! facts have exactly one source of truth and it is this command (PRD §6.3).

use aeroworship_core::models::Monitor;
use tauri::{AppHandle, Runtime};

use crate::services::display;

/// Every display the OS currently reports (FR-101).
///
/// The reading of the OS, the identification of the primary display and the
/// caveats about list ordering all live in
/// [`services::display::connected_monitors`](display::connected_monitors),
/// which is also what the Display Service itself uses to place the windows
/// (FR-102) — the frontend and the service must not be able to disagree about
/// the topology. This command is the frontend's view of that one answer, and
/// returns no `Result` for the reason given there.
#[tauri::command]
pub fn list_monitors<R: Runtime>(app: AppHandle<R>) -> Vec<Monitor> {
    display::connected_monitors(&app)
}
