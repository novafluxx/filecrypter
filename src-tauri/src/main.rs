// src-tauri/src/main.rs - Tauri application binary entrypoint
//
// The app's setup and command registration live in the library crate (`filecypter_lib::run`).
// Keeping that logic in `lib.rs` makes it reusable across targets and easier to test.
//
// This attribute prevents an additional console window on Windows in release builds.
// Keep it unless you explicitly want a console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Delegate to the library crate so there's a single source of truth for app setup.
    filecypter_lib::run();
}
