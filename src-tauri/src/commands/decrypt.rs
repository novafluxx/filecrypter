// commands/decrypt.rs - File Decryption Command Handler
//
// This module implements the Tauri command for decrypting files using streaming
// (chunked) decryption. Encrypted files use Version 4 (no compression) or
// Version 5 (ZSTD compression) formats, both with per-chunk authentication.
//
// Decryption workflow:
// 1. Validate input path and resolve output path
// 2. Read and parse Version 4/5 header (KDF params, salt, nonce, chunk info)
// 3. Derive decryption key from password using stored KDF parameters
// 4. Create secure temporary file
// 5. Decrypt each chunk using unique per-chunk nonce
// 6. Verify authentication tag for each chunk (detects tampering)
// 7. Write decrypted chunks to temporary file
// 8. Atomically rename temporary file to final output
//
// File Format: Version 4/5 (streaming format)
// - Header authenticated as AAD (Additional Authenticated Data) for every chunk
// - Each chunk has unique nonce: BLAKE3(base_nonce || chunk_index)
// - Each chunk verified with AES-GCM authentication tag
//
// Security:
// - Wrong password fails at first chunk (tag verification failure)
// - Tampered data detected immediately (authentication failure)
// - Timing-safe comparison prevents timing attacks
// - Header tampering detected (used as AAD in each chunk)

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
/// - File format is invalid or corrupted (Version 4/5 format expected)
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
    // Note: AppHandle is internally reference-counted, so clone is cheap
    let app_handle = app.clone();
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
            None,
        )
        .unwrap();

        // Now decrypt it
        let decrypted_path = temp_dir.path().join("decrypted.txt");
        let result =
            decrypt_file_streaming(&encrypted_path, &decrypted_path, &password, None, false);

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
            None,
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

        let result =
            decrypt_file_streaming(corrupted_file.path(), &output_path, &password, None, false);

        assert!(result.is_err());
    }
}
