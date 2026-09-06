use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

const DATA_ROOT: &str = "AeroWorship";

const DATABASE_FILE: &str = "aeroworship.db";

pub fn data_root<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, tauri::Error> {
    Ok(app.path().data_dir()?.join(DATA_ROOT))
}

pub fn database_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, tauri::Error> {
    Ok(data_root(app)?.join(DATABASE_FILE))
}

pub fn init<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(data_root(app)?)?;
    aeroworship_core::db::open_and_migrate(&database_path(app)?)?;
    Ok(())
}
