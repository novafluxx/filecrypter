// commands/streaming.rs - Streaming Encryption/Decryption Commands
//
// This module provides Tauri commands for streaming (chunked) file encryption.
// Use these for large files (>10MB) to avoid loading the entire file into memory.
//
// Progress events are emitted during processing to update the UI.

use std::path::PathBuf;
use std::sync::Arc;
use tauri::{command, AppHandle, Emitter};

use crate::commands::file_utils::validate_input_path;
use crate::crypto::{
    decrypt_file_streaming, encrypt_file_streaming, should_use_streaming, Password,
    DEFAULT_CHUNK_SIZE, STREAMING_THRESHOLD,
};
use crate::error::CryptoResult;
use crate::events::{ProgressEvent, CRYPTO_PROGRESS_EVENT};

/// Encrypt a file using streaming encryption
///
/// This command encrypts large files in chunks without loading them entirely
/// into memory. Use this for files larger than 10MB.
///
/// # Arguments
/// * `app` - Tauri AppHandle for emitting progress events
/// * `input_path` - Path to the file to encrypt
/// * `output_path` - Path where encrypted file will be saved
/// * `password` - User's password
///
/// # Returns
/// Success message or error
#[command]
pub async fn encrypt_file_streamed(
    app: AppHandle,
    input_path: String,
    output_path: String,
    password: String,
) -> CryptoResult<String> {
    log::info!("Streaming encrypt: {}", input_path);

    // Emit: Starting
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::reading());

    // Emit: Deriving key
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::deriving_key());

    // Validate input path (check for symlinks, canonicalize)
    let validated_input = validate_input_path(&input_path)?;
    let validated_output = PathBuf::from(&output_path);
    let password = Password::new(password);

    // Create progress callback
    let app_handle = Arc::new(app.clone());
    let progress_callback = move |bytes_processed: u64, total_bytes: u64| {
        let percent = if total_bytes > 0 {
            ((bytes_processed as f64 / total_bytes as f64) * 100.0) as u32
        } else {
            0
        };
        let percent = percent.min(99); // Cap at 99% until complete

        let _ = app_handle.emit(
            CRYPTO_PROGRESS_EVENT,
            ProgressEvent::new(
                "encrypting",
                percent,
                "Encrypting file chunks...",
            ),
        );
    };

    // Perform streaming encryption
    encrypt_file_streaming(
        validated_input,
        validated_output,
        password.as_str(),
        DEFAULT_CHUNK_SIZE,
        Some(Box::new(progress_callback)),
    )?;

    // Emit: Complete
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::encrypt_complete());

    log::info!("Streaming encryption complete: {}", output_path);
    Ok(format!("File encrypted successfully: {}", output_path))
}

/// Decrypt a file using streaming decryption
///
/// This command decrypts files that were encrypted with streaming encryption.
///
/// # Arguments
/// * `app` - Tauri AppHandle for emitting progress events
/// * `input_path` - Path to the encrypted file
/// * `output_path` - Path where decrypted file will be saved
/// * `password` - User's password
///
/// # Returns
/// Success message or error
#[command]
pub async fn decrypt_file_streamed(
    app: AppHandle,
    input_path: String,
    output_path: String,
    password: String,
) -> CryptoResult<String> {
    log::info!("Streaming decrypt: {}", input_path);

    // Emit: Starting
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::reading());

    // Emit: Deriving key
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::deriving_key());

    // Validate input path (check for symlinks, canonicalize)
    let validated_input = validate_input_path(&input_path)?;
    let validated_output = PathBuf::from(&output_path);
    let password = Password::new(password);

    // Create progress callback
    let app_handle = Arc::new(app.clone());
    let progress_callback = move |bytes_processed: u64, total_bytes: u64| {
        let percent = if total_bytes > 0 {
            ((bytes_processed as f64 / total_bytes as f64) * 100.0) as u32
        } else {
            0
        };
        let percent = percent.min(99); // Cap at 99% until complete

        let _ = app_handle.emit(
            CRYPTO_PROGRESS_EVENT,
            ProgressEvent::new(
                "decrypting",
                percent,
                "Decrypting file chunks...",
            ),
        );
    };

    // Perform streaming decryption
    decrypt_file_streaming(
        validated_input,
        validated_output,
        password.as_str(),
        Some(Box::new(progress_callback)),
    )?;

    // Emit: Complete
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::decrypt_complete());

    log::info!("Streaming decryption complete: {}", output_path);
    Ok(format!("File decrypted successfully: {}", output_path))
}

/// Check if a file should use streaming encryption
///
/// Returns true if the file size exceeds the streaming threshold (10MB).
/// The frontend can use this to decide which encryption method to use.
#[command]
pub fn check_use_streaming(file_path: String) -> CryptoResult<bool> {
    let file_size = std::fs::metadata(&file_path)?.len();
    Ok(should_use_streaming(file_size, STREAMING_THRESHOLD))
}

/// Get the streaming threshold in bytes
///
/// Returns the file size threshold above which streaming encryption is recommended.
#[command]
pub fn get_streaming_threshold() -> u64 {
    STREAMING_THRESHOLD
}
