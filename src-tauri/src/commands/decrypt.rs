// commands/decrypt.rs - File Decryption Command Handler
//
// This module implements the Tauri command for decrypting files.
// It handles the complete decryption workflow:
// 1. Read encrypted file from disk
// 2. Parse file format and extract metadata (salt, nonce, ciphertext)
// 3. Derive decryption key from password using stored salt
// 4. Decrypt ciphertext with AES-256-GCM and verify authentication tag
// 5. Write decrypted plaintext to disk
//
// Security:
// - Authentication tag is verified automatically by AES-GCM
// - Wrong password results in tag verification failure
// - Any tampering with ciphertext is detected

use std::sync::Arc;
use tauri::{command, AppHandle, Emitter};

use crate::commands::file_utils::{resolve_output_path, validate_input_path};
use crate::commands::CryptoResponse;
use crate::crypto::{decrypt_file_streaming, Password};
use crate::error::CryptoResult;
use crate::events::{ProgressEvent, CRYPTO_PROGRESS_EVENT};

/// Decrypt an encrypted file with password
///
/// This Tauri command decrypts a file that was encrypted with `encrypt_file`.
/// All files are decrypted using streaming (chunked) decryption for consistent behavior.
///
/// # Arguments
/// * `input_path` - Path to the encrypted file (.encrypted)
/// * `output_path` - Path where the decrypted file will be saved
/// * `password` - User's password (must match the one used for encryption)
/// * `allow_overwrite` - Allow overwriting existing files (default: false)
///
/// # Returns
/// A success response containing the message and resolved output path
///
/// # Errors
/// Returns `CryptoError` if:
/// - Input file cannot be read or doesn't exist
/// - File format is invalid or corrupted (Version 4 format expected)
/// - Wrong password (authentication tag verification fails)
/// - File has been tampered with (tag mismatch)
/// - Output file cannot be written
///
/// # Security Notes
/// - Password is wrapped in `Password` type and zeroized after use
/// - Authentication tag is automatically verified by AES-GCM for each chunk
/// - Timing-safe comparison prevents timing attacks
/// - Salt is read from the encrypted file (not secret)
///
/// # Frontend Usage
/// ```typescript
/// await invoke('decrypt_file', {
///   inputPath: '/path/to/file.txt.encrypted',
///   outputPath: '/path/to/file.txt',
///   password: 'user_password',
///   allowOverwrite: false
/// });
/// ```
#[command]
pub async fn decrypt_file(
    app: AppHandle,
    input_path: String,
    output_path: String,
    password: String,
    allow_overwrite: Option<bool>,
) -> CryptoResult<CryptoResponse> {
    // Log the operation (password is NOT logged)
    log::info!("Decrypting file: {}", input_path);

    // Emit progress
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::reading());
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::deriving_key());

    // Validate input
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
            ProgressEvent::new("decrypting", percent, "Decrypting file..."),
        );
    };

    // Use streaming for all files
    decrypt_file_streaming(
        validated_input,
        &validated_output,
        &password,
        Some(Box::new(progress_callback)),
        allow_overwrite,
    )?;

    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::decrypt_complete());

    let output_path = validated_output.to_string_lossy().to_string();
    Ok(CryptoResponse {
        message: format!("File decrypted successfully: {}", output_path),
        output_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{encrypt_file_streaming, DEFAULT_CHUNK_SIZE};
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_decrypt_file_streaming_success() {
        // Create and encrypt a file first
        let temp_dir = tempfile::tempdir().unwrap();
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), b"Test decryption content").unwrap();

        let encrypted_path = temp_dir.path().join("encrypted.bin");
        let password = Password::new("test_password".to_string());

        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            DEFAULT_CHUNK_SIZE,
            None,
            false,
        )
        .unwrap();

        // Now decrypt it
        let decrypted_path = temp_dir.path().join("decrypted.txt");
        let result = decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &password,
            None,
            false,
        );

        assert!(result.is_ok());

        // Verify decrypted content matches original
        let decrypted_content = fs::read(&decrypted_path).unwrap();
        assert_eq!(decrypted_content, b"Test decryption content");
    }

    #[test]
    fn test_decrypt_file_wrong_password() {
        // Create and encrypt a file
        let temp_dir = tempfile::tempdir().unwrap();
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), b"Secret content").unwrap();

        let encrypted_path = temp_dir.path().join("encrypted.bin");
        let password = Password::new("correct_password".to_string());

        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            DEFAULT_CHUNK_SIZE,
            None,
            false,
        )
        .unwrap();

        // Try to decrypt with wrong password
        let decrypted_path = temp_dir.path().join("decrypted.txt");
        let wrong_password = Password::new("wrong_password".to_string());
        let result = decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &wrong_password,
            None,
            false,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_corrupted_file() {
        // Create a corrupted "encrypted" file
        let corrupted_file = NamedTempFile::new().unwrap();
        fs::write(corrupted_file.path(), b"This is not a valid encrypted file").unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("decrypted.txt");
        let password = Password::new("password".to_string());

        let result = decrypt_file_streaming(
            corrupted_file.path(),
            &output_path,
            &password,
            None,
            false,
        );

        assert!(result.is_err());
    }
}
