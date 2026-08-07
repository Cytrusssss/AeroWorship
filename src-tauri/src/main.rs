// Prevents an extra console window from appearing alongside the app on Windows
// release builds. Debug builds keep the console so that `println!`/tracing
// output stays visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    aeroworship::run();
}
