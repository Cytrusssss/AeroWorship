#[cfg(all(not(debug_assertions), dev))]
compile_error!(
    "release build without tauri's `custom-protocol` feature: this binary would \
     load its UI from http://localhost:1420 instead of the embedded `dist/`. \
     Build with `npm run tauri build`, not `cargo build --release` — the Tauri \
     CLI builds the frontend first and enables `tauri/custom-protocol`. If you \
     really do want plain Cargo, pass `--features tauri/custom-protocol` and \
     make sure `dist/` is up to date yourself. The same flag is the way to run \
     a release-profile test or benchmark, which this guard also rejects: \
     `cargo test --release -p aeroworship --features tauri/custom-protocol` \
     (ADR-0019)."
);

pub mod commands;
pub mod db;
pub mod services;

pub fn run() {
    tauri::Builder::default()
       .plugin(services::navigation::guard())
        .setup(|app| {
            db::init(app.handle())?;
            services::display::init(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::display::list_monitors])
        .run(tauri::generate_context!())
        .expect("failed to start the AeroWorship application");
}
