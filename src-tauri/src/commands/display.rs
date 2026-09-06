use aeroworship_core::models::Monitor;
use tauri::{AppHandle, Runtime};

use crate::services::display;

#[tauri::command]
pub fn list_monitors<R: Runtime>(app: AppHandle<R>) -> Vec<Monitor> {
    display::connected_monitors(&app)
}
