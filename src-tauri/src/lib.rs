// lib.rs - FileCrypter Main Library
//
// This is the main entry point for the Tauri application.
// It registers all commands and plugins, then starts the app.

// Declare modules
mod commands;
pub mod crypto;
mod error;
pub mod events;
pub mod security;

// Import commands for registration
use commands::{batch_decrypt, batch_encrypt, decrypt_file, encrypt_file};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Build and run the Tauri application.
///
/// This function registers plugins and IPC commands, then starts the Tauri runtime.
/// The desktop binary (`src-tauri/src/main.rs`) delegates to this so the setup lives
/// in one place.
pub fn run() {
    tauri::Builder::default()
        // Register plugins
        .plugin(tauri_plugin_dialog::init()) // File dialogs (open/save)
        .setup(|app| {
            // Setup logging in debug mode
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        // Register Tauri commands that can be called from the frontend
        .invoke_handler(tauri::generate_handler![
            encrypt_file,  // Streaming encryption (all files)
            decrypt_file,  // Streaming decryption (all files)
            batch_encrypt, // Batch encrypt multiple files
            batch_decrypt, // Batch decrypt multiple files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
