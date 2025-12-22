// commands/mod.rs - Tauri Command Handlers Module
//
// This module exports all Tauri commands that can be invoked from the frontend.
// These commands are registered in main.rs and called via the Tauri IPC system.

mod batch;
mod decrypt;
mod encrypt;
mod streaming;

// Re-export commands for registration in main.rs
pub use batch::{batch_decrypt, batch_encrypt};
pub use decrypt::decrypt_file;
pub use encrypt::encrypt_file;
pub use streaming::{
    check_use_streaming, decrypt_file_streamed, encrypt_file_streamed, get_streaming_threshold,
};
