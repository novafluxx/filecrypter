// commands/encrypt.rs - File Encryption Command Handler
//
// This module implements the Tauri command for encrypting files.
// It handles the complete encryption workflow:
// 1. Read plaintext file from disk
// 2. Generate random salt
// 3. Derive encryption key from password using Argon2id
// 4. Encrypt file content with AES-256-GCM
// 5. Serialize encrypted data with metadata
// 6. Write encrypted file to disk
//
// Tauri IPC:
// - This is called from the frontend using invoke('encrypt_file', {...})
// - Returns a success message or error
// - Async function allows long-running operations without blocking UI

use std::sync::Arc;
use tauri::{command, AppHandle, Emitter};

use crate::commands::file_utils::{resolve_output_path, validate_input_path};
use crate::commands::CryptoResponse;
use crate::crypto::{encrypt_file_streaming, Password, DEFAULT_CHUNK_SIZE};
use crate::error::CryptoResult;
use crate::events::{ProgressEvent, CRYPTO_PROGRESS_EVENT};

/// Encrypt a file with password protection
///
/// This Tauri command encrypts a file using AES-256-GCM with a password-derived key.
/// All files are encrypted using streaming (chunked) encryption for consistent behavior
/// and optimal memory usage.
///
/// # Arguments
/// * `app` - Tauri AppHandle for emitting progress events
/// * `input_path` - Path to the file to encrypt
/// * `output_path` - Path where the encrypted file will be saved
/// * `password` - User's password (will be zeroized after use)
/// * `allow_overwrite` - Allow overwriting existing files (default: false)
///
/// # Returns
/// A success response containing the message and resolved output path
///
/// # Errors
/// Returns `CryptoError` if:
/// - Input file cannot be read (doesn't exist, no permission, etc.)
/// - Password is empty
/// - Encryption fails
/// - Output file cannot be written
///
/// # Security Notes
/// - Password is wrapped in `Password` type and zeroized after key derivation
/// - Unique salt is generated for each encryption
/// - Nonce is randomly generated per chunk (never reused)
/// - Files are processed in 1MB chunks, regardless of size
///
/// # Frontend Usage
/// ```typescript
/// await invoke('encrypt_file', {
///   inputPath: '/path/to/file.txt',
///   outputPath: '/path/to/file.txt.encrypted',
///   password: 'user_password',
///   allowOverwrite: false
/// });
/// ```
#[command]
pub async fn encrypt_file(
    app: AppHandle,
    input_path: String,
    output_path: String,
    password: String,
    allow_overwrite: Option<bool>,
) -> CryptoResult<CryptoResponse> {
    // Log the operation (password is NOT logged)
    log::info!("Encrypting file: {}", input_path);

    // Emit progress events
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::reading());
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::deriving_key());

    // Validate input path (check for symlinks, canonicalize)
    let validated_input = validate_input_path(&input_path)?;
    let allow_overwrite = allow_overwrite.unwrap_or(false);
    let validated_output = resolve_output_path(&output_path, allow_overwrite)?;
    let password = Password::new(password);

    // Progress callback for streaming
    let app_handle = Arc::new(app.clone());
    let progress_callback = move |bytes_processed: u64, total_bytes: u64| {
        let percent = if total_bytes > 0 {
            ((bytes_processed as f64 / total_bytes as f64) * 100.0) as u32
        } else {
            0
        }
        .min(99);

        let _ = app_handle.emit(
            CRYPTO_PROGRESS_EVENT,
            ProgressEvent::new("encrypting", percent, "Encrypting file..."),
        );
    };

    // Use streaming for all files
    encrypt_file_streaming(
        validated_input,
        &validated_output,
        &password,
        DEFAULT_CHUNK_SIZE,
        Some(Box::new(progress_callback)),
        allow_overwrite,
    )?;

    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::encrypt_complete());

    let output_path = validated_output.to_string_lossy().to_string();
    Ok(CryptoResponse {
        message: format!("File encrypted successfully: {}", output_path),
        output_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::decrypt_file_streaming;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_encrypt_file_streaming_success() {
        // Create a temporary input file
        let input_file = NamedTempFile::new().unwrap();
        let input_path = input_file.path();
        fs::write(input_path, b"Test content for streaming").unwrap();

        // Create output path
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("encrypted.bin");

        // Encrypt using streaming
        let password = Password::new("test_password".to_string());
        let result = encrypt_file_streaming(
            input_path,
            &output_path,
            &password,
            DEFAULT_CHUNK_SIZE,
            None,
            false,
        );

        assert!(result.is_ok());

        // Verify output file was created
        let output_data = fs::read(&output_path).unwrap();
        assert!(!output_data.is_empty());

        // Verify it's Version 4 format
        assert_eq!(output_data[0], 4);

        // Verify it's not the same as input (it's encrypted)
        assert_ne!(output_data, b"Test content for streaming");

        // Verify we can decrypt it
        let decrypted_path = temp_dir.path().join("decrypted.txt");
        decrypt_file_streaming(&output_path, &decrypted_path, &password, None, false).unwrap();
        let decrypted_content = fs::read(&decrypted_path).unwrap();
        assert_eq!(decrypted_content, b"Test content for streaming");
    }

    #[test]
    fn test_encrypt_file_streaming_small_file() {
        // Test that streaming works correctly for very small files
        let input_file = NamedTempFile::new().unwrap();
        let input_path = input_file.path();
        fs::write(input_path, b"tiny").unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("encrypted.bin");

        let password = Password::new("test_password".to_string());
        let result = encrypt_file_streaming(
            input_path,
            &output_path,
            &password,
            DEFAULT_CHUNK_SIZE,
            None,
            false,
        );

        assert!(result.is_ok());

        // Verify Version 4 format
        let output_data = fs::read(&output_path).unwrap();
        assert_eq!(output_data[0], 4);
    }
}
