// commands/mod.rs - Tauri Command Handlers Module
//
// This module exports all Tauri commands that can be invoked from the frontend.
// These commands are registered in main.rs and called via the Tauri IPC system.

use serde::Serialize;

// Archive utilities for batch archive mode (TAR+ZSTD compression).
// pub(crate) visibility: Used internally by batch.rs but not exported as Tauri commands.
// The archive module provides helper functions for creating and extracting archives,
// which are called by the batch_encrypt_archive and batch_decrypt_archive commands.
pub(crate) mod archive;

mod batch;
mod decrypt;
mod encrypt;
pub mod file_utils;

/// Standard response for encrypt/decrypt commands.
///
/// Returned by `encrypt_file` and `decrypt_file` commands to indicate
/// success and provide the resolved output path to the frontend.
#[derive(Clone, Serialize)]
pub struct CryptoResponse {
    /// Human-readable success message for display in UI
    pub message: String,
    /// Resolved output file path (may differ from requested if auto-renamed)
    pub output_path: String,
}

// Re-export commands for registration in main.rs
pub use batch::{batch_decrypt, batch_decrypt_archive, batch_encrypt, batch_encrypt_archive};
pub use decrypt::decrypt_file;
pub use encrypt::encrypt_file;
