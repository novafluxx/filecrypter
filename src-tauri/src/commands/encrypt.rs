// commands/encrypt.rs - File Encryption Command Handler
//
// This module implements the Tauri command for encrypting files using streaming
// (chunked) encryption. All files, regardless of size, use the same streaming
// approach for consistent behavior and optimal memory usage.
//
// Encryption workflow:
// 1. Validate input path and resolve output path
// 2. Create secure temporary file
// 3. Generate random salt and derive encryption key using Argon2id
// 4. Encrypt file in 1MB chunks using AES-256-GCM
// 5. Write encrypted chunks to temporary file
// 6. Atomically rename temporary file to final output
//
// File Format: Version 4 (streaming format with chunk-level authentication)
// - Header contains KDF parameters, salt, base nonce, chunk size, and total chunks
// - Each chunk has a unique nonce derived from (base_nonce, chunk_index)
// - Each chunk is authenticated with AES-GCM tag
//
// Tauri IPC:
// - Called from frontend using invoke('encrypt_file', {...})
// - Emits progress events during encryption for UI updates
// - Returns success message with resolved output path
// - Async to avoid blocking the UI thread

use tauri::{command, AppHandle, Emitter};

use crate::commands::command_utils::{
    create_progress_callback, format_success_response, validate_crypto_inputs,
};
use crate::commands::CryptoResponse;
use crate::crypto::{encrypt_file_streaming, CompressionConfig, DEFAULT_CHUNK_SIZE};
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
/// * `compression_enabled` - Enable ZSTD compression before encryption (default: false)
/// * `compression_level` - ZSTD compression level 1-22 (default: 3)
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
///   allowOverwrite: false,
///   compressionEnabled: true,
///   compressionLevel: 3
/// });
/// ```
#[command]
#[allow(clippy::too_many_arguments)]
pub async fn encrypt_file(
    app: AppHandle,
    input_path: String,
    output_path: String,
    password: String,
    allow_overwrite: Option<bool>,
    compression_enabled: Option<bool>,
    compression_level: Option<i32>,
    key_file_path: Option<String>,
) -> CryptoResult<CryptoResponse> {
    // Log the operation (password is NOT logged)
    log::info!("Encrypting file: {}", input_path);

    // Validate inputs and emit initial progress events
    let allow_overwrite = allow_overwrite.unwrap_or(false);
    let validated =
        validate_crypto_inputs(&app, &input_path, &output_path, password, allow_overwrite)?;

    // Build compression config if enabled
    let compression = if compression_enabled.unwrap_or(false) {
        Some(CompressionConfig::new(compression_level.unwrap_or(3)))
    } else {
        None
    };

    // Create progress callback for streaming
    let progress_callback =
        create_progress_callback(app.clone(), "encrypting", "Encrypting file...");

    // Validate key file path if provided
    let kf_path = key_file_path.as_deref().map(std::path::Path::new);

    // Use streaming for all files
    encrypt_file_streaming(
        validated.input,
        &validated.output,
        &validated.password,
        DEFAULT_CHUNK_SIZE,
        Some(progress_callback),
        allow_overwrite,
        compression,
        kf_path,
    )?;

    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::encrypt_complete());

    Ok(format_success_response(&validated.output, "encrypted"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{decrypt_file_streaming, Password};
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

        // Encrypt using streaming (no compression)
        let password = Password::new("test_password".to_string());
        let result = encrypt_file_streaming(
            input_path,
            &output_path,
            &password,
            DEFAULT_CHUNK_SIZE,
            None,
            false,
            None,
            None,
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
        decrypt_file_streaming(&output_path, &decrypted_path, &password, None, false, None)
            .unwrap();
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
            None,
            None,
        );

        assert!(result.is_ok());

        // Verify Version 4 format
        let output_data = fs::read(&output_path).unwrap();
        assert_eq!(output_data[0], 4);
    }

    #[test]
    fn test_encrypt_file_with_compression() {
        // Create a temporary input file with compressible content
        let input_file = NamedTempFile::new().unwrap();
        let input_path = input_file.path();
        let content = b"Test content for streaming ".repeat(100);
        fs::write(input_path, &content).unwrap();

        // Create output path
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("encrypted_compressed.bin");

        // Encrypt with compression
        let password = Password::new("test_password".to_string());
        let result = encrypt_file_streaming(
            input_path,
            &output_path,
            &password,
            DEFAULT_CHUNK_SIZE,
            None,
            false,
            Some(CompressionConfig::default()),
            None,
        );

        assert!(result.is_ok());

        // Verify output file was created
        let output_data = fs::read(&output_path).unwrap();
        assert!(!output_data.is_empty());

        // Verify it's Version 5 format
        assert_eq!(output_data[0], 5);

        // Verify we can decrypt it
        let decrypted_path = temp_dir.path().join("decrypted.txt");
        decrypt_file_streaming(&output_path, &decrypted_path, &password, None, false, None)
            .unwrap();
        let decrypted_content = fs::read(&decrypted_path).unwrap();
        assert_eq!(decrypted_content, content);
    }
}
