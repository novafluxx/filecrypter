// commands/mod.rs - Tauri Command Handlers Module
//
// This module exports all Tauri commands that can be invoked from the frontend.
// These commands are registered in main.rs and called via the Tauri IPC system.

use serde::Serialize;

mod batch;
mod decrypt;
mod encrypt;
pub mod file_utils;

/// Standard response for encrypt/decrypt commands.
#[derive(Clone, Serialize)]
pub struct CryptoResponse {
    pub message: String,
    pub output_path: String,
}

// Re-export commands for registration in main.rs
pub use batch::{batch_decrypt, batch_encrypt};
pub use decrypt::decrypt_file;
pub use encrypt::encrypt_file;
