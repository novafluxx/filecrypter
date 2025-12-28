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

use std::fs;
use tauri::{command, AppHandle, Emitter};

use crate::commands::file_utils::{atomic_write, validate_file_size, validate_input_path};
use crate::commands::CryptoResponse;
use crate::crypto::{decrypt, derive_key, EncryptedFile, Password};
use crate::error::CryptoResult;
use crate::events::{ProgressEvent, CRYPTO_PROGRESS_EVENT};

/// Internal decryption implementation (used by tests)
///
/// This function contains the core decryption logic without Tauri dependencies.
#[cfg(test)]
pub fn decrypt_file_impl(
    input_path: &str,
    output_path: &str,
    password: &str,
) -> CryptoResult<String> {
    // Validate password is not empty
    if password.is_empty() {
        return Err(crate::error::CryptoError::FormatError(
            "Password cannot be empty".to_string(),
        ));
    }

    // Step 1: Read the encrypted file from disk
    let encrypted_data = fs::read(input_path)?;

    // Step 2: Parse the encrypted file format
    let encrypted_file = EncryptedFile::deserialize(&encrypted_data)?;

    // Step 3: Derive decryption key from password + salt
    let password = Password::new(password.to_string());
    let key = derive_key(&password, &encrypted_file.salt)?;

    // Step 4: Decrypt the ciphertext with AES-256-GCM
    let plaintext = decrypt(&key, &encrypted_file.nonce, &encrypted_file.ciphertext)?;

    // Step 5: Write the plaintext to the output file
    fs::write(output_path, plaintext)?;

    Ok(format!("File decrypted successfully: {}", output_path))
}

/// Decrypt an encrypted file with password
///
/// This Tauri command decrypts a file that was encrypted with `encrypt_file`.
/// It verifies the authentication tag to ensure the file hasn't been tampered with.
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
/// - File format is invalid or corrupted
/// - Wrong password (authentication tag verification fails)
/// - File has been tampered with (tag mismatch)
/// - Output file cannot be written
///
/// # Security Notes
/// - Password is wrapped in `Password` type and zeroized after use
/// - Authentication tag is automatically verified by AES-GCM
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

    // Validate password is not empty
    if password.is_empty() {
        return Err(crate::error::CryptoError::FormatError(
            "Password cannot be empty".to_string(),
        ));
    }

    // Emit: Reading file
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::reading());

    // Validate input path (check for symlinks, canonicalize)
    let validated_input = validate_input_path(&input_path)?;

    // Validate file size for in-memory operation
    validate_file_size(&input_path)?;

    // Step 1: Read the encrypted file from disk
    let encrypted_data = fs::read(&validated_input)?;

    log::info!("Read {} bytes from encrypted file", encrypted_data.len());

    // Step 2: Parse the encrypted file format
    // This extracts: salt, nonce, and ciphertext (with tag)
    // Validates file format version and structure
    let encrypted_file = EncryptedFile::deserialize(&encrypted_data)?;

    log::info!(
        "Parsed encrypted file: salt={} bytes, nonce={} bytes, ciphertext={} bytes",
        encrypted_file.salt.len(),
        encrypted_file.nonce.len(),
        encrypted_file.ciphertext.len()
    );

    // Emit: Deriving key (the slow step)
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::deriving_key());

    // Step 3: Derive decryption key from password + salt
    // The salt is read from the file (it was stored during encryption)
    // This must produce the same key as during encryption if password is correct
    let password = Password::new(password);
    let key = derive_key(&password, &encrypted_file.salt)?;

    log::info!("Decryption key derived successfully");

    // Emit: Decrypting
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::decrypting());

    // Step 4: Decrypt the ciphertext with AES-256-GCM
    // This automatically verifies the authentication tag
    // If the tag doesn't match (wrong password or tampered data), this will fail
    let plaintext = decrypt(&key, &encrypted_file.nonce, &encrypted_file.ciphertext)?;

    log::info!("Decryption successful: {} bytes decrypted", plaintext.len());

    // Emit: Writing file
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::writing());

    let allow_overwrite = allow_overwrite.unwrap_or(false);

    // Step 5: Write the plaintext to the output file with secure permissions
    let resolved_path = atomic_write(&output_path, &plaintext, allow_overwrite)?;

    log::info!("Decrypted file written to: {}", resolved_path.display());

    // Emit: Complete
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::decrypt_complete());

    // Return success message to frontend
    let output_path = resolved_path.to_string_lossy().to_string();
    Ok(CryptoResponse {
        message: format!("File decrypted successfully: {}", output_path),
        output_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::encrypt::encrypt_file_impl;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_decrypt_file_success() {
        // Create a test file
        let original_content = b"This is a test file for decryption";
        let input_file = NamedTempFile::new().unwrap();
        let input_path = input_file.path().to_str().unwrap();
        fs::write(input_path, original_content).unwrap();

        // Encrypt it
        let encrypted_file = NamedTempFile::new().unwrap();
        let encrypted_path = encrypted_file.path().to_str().unwrap();
        encrypt_file_impl(input_path, encrypted_path, "test_password").unwrap();

        // Decrypt it
        let decrypted_file = NamedTempFile::new().unwrap();
        let decrypted_path = decrypted_file.path().to_str().unwrap();
        let result = decrypt_file_impl(encrypted_path, decrypted_path, "test_password");

        assert!(result.is_ok());

        // Verify content matches original
        let decrypted_content = fs::read(decrypted_path).unwrap();
        assert_eq!(original_content, decrypted_content.as_slice());
    }

    #[test]
    fn test_decrypt_file_wrong_password() {
        // Create and encrypt a file
        let input_file = NamedTempFile::new().unwrap();
        let input_path = input_file.path().to_str().unwrap();
        fs::write(input_path, b"Secret content").unwrap();

        let encrypted_file = NamedTempFile::new().unwrap();
        let encrypted_path = encrypted_file.path().to_str().unwrap();
        encrypt_file_impl(input_path, encrypted_path, "correct_password").unwrap();

        // Try to decrypt with wrong password
        let decrypted_file = NamedTempFile::new().unwrap();
        let decrypted_path = decrypted_file.path().to_str().unwrap();
        let result = decrypt_file_impl(encrypted_path, decrypted_path, "wrong_password");

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::error::CryptoError::InvalidPassword)
        ));
    }

    #[test]
    fn test_decrypt_file_empty_password() {
        let input_file = NamedTempFile::new().unwrap();
        let output_file = NamedTempFile::new().unwrap();

        let result = decrypt_file_impl(
            input_file.path().to_str().unwrap(),
            output_file.path().to_str().unwrap(),
            "",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_file_nonexistent_input() {
        let output_file = NamedTempFile::new().unwrap();

        let result = decrypt_file_impl(
            "/nonexistent/encrypted.file",
            output_file.path().to_str().unwrap(),
            "password",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_corrupted_file() {
        // Create a corrupted "encrypted" file
        let corrupted_file = NamedTempFile::new().unwrap();
        let corrupted_path = corrupted_file.path().to_str().unwrap();
        fs::write(corrupted_path, b"This is not a valid encrypted file").unwrap();

        let output_file = NamedTempFile::new().unwrap();
        let output_path = output_file.path().to_str().unwrap();

        let result = decrypt_file_impl(corrupted_path, output_path, "password");

        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // Full roundtrip test
        let original_content =
            b"Hello, FileCrypter! Testing full roundtrip encryption and decryption.";
        let password = "SecurePassword123!";

        // Create original file
        let original_file = NamedTempFile::new().unwrap();
        let original_path = original_file.path().to_str().unwrap();
        fs::write(original_path, original_content).unwrap();

        // Encrypt
        let encrypted_file = NamedTempFile::new().unwrap();
        let encrypted_path = encrypted_file.path().to_str().unwrap();
        encrypt_file_impl(original_path, encrypted_path, password).unwrap();

        // Decrypt
        let decrypted_file = NamedTempFile::new().unwrap();
        let decrypted_path = decrypted_file.path().to_str().unwrap();
        decrypt_file_impl(encrypted_path, decrypted_path, password).unwrap();

        // Verify
        let decrypted_content = fs::read(decrypted_path).unwrap();
        assert_eq!(original_content, decrypted_content.as_slice());
    }
}
