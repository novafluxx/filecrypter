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

use std::fs;
use tauri::{command, AppHandle, Emitter};

use crate::commands::file_utils::{atomic_write, validate_file_size, validate_input_path};
use crate::commands::CryptoResponse;
use crate::crypto::{derive_key, encrypt, generate_salt, EncryptedFile, Password};
use crate::error::CryptoResult;
use crate::events::{ProgressEvent, CRYPTO_PROGRESS_EVENT};

/// Internal encryption implementation (used by tests)
///
/// This function contains the core encryption logic without Tauri dependencies.
#[cfg(test)]
fn encrypt_file_impl(
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

    // Step 1: Read the plaintext file into memory
    let plaintext = fs::read(input_path)?;

    // Step 2: Generate a random salt for key derivation
    let salt = generate_salt()?;

    // Step 3: Derive encryption key from password + salt
    let password = Password::new(password.to_string());
    let key = derive_key(&password, &salt)?;

    // Step 4: Encrypt the file content with AES-256-GCM
    let (nonce, ciphertext) = encrypt(&key, &plaintext)?;

    // Step 5: Create the encrypted file structure with all metadata
    let encrypted_file = EncryptedFile {
        salt,
        nonce,
        ciphertext,
    };

    // Step 6: Serialize to binary format
    let output_data = encrypted_file.serialize();

    // Step 7: Write encrypted file to disk
    fs::write(output_path, output_data)?;

    Ok(format!("File encrypted successfully: {}", output_path))
}

/// Encrypt a file with password protection
///
/// This Tauri command encrypts a file using AES-256-GCM with a password-derived key.
/// The encrypted file includes all metadata needed for decryption (salt, nonce).
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
/// - Nonce is randomly generated (never reused)
/// - File is read entirely into memory (suitable for files <100MB)
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

    // Emit progress events during encryption
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::reading());

    // Validate password is not empty
    if password.is_empty() {
        return Err(crate::error::CryptoError::FormatError(
            "Password cannot be empty".to_string(),
        ));
    }

    // Validate input path (check for symlinks, canonicalize)
    let validated_input = validate_input_path(&input_path)?;

    // Validate file size for in-memory operation
    validate_file_size(&input_path)?;

    // Read plaintext
    let plaintext = fs::read(&validated_input)?;
    log::info!("Read {} bytes from input file", plaintext.len());

    // Emit: Deriving key (the slow step)
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::deriving_key());

    // Generate salt and derive key
    let salt = generate_salt()?;
    let password = Password::new(password);
    let key = derive_key(&password, &salt)?;
    log::info!("Key derived successfully");

    // Emit: Encrypting
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::encrypting());

    // Encrypt the file content
    let (nonce, ciphertext) = encrypt(&key, &plaintext)?;
    log::info!(
        "Encryption complete: {} bytes -> {} bytes (including tag)",
        plaintext.len(),
        ciphertext.len()
    );

    // Create the encrypted file structure
    let encrypted_file = EncryptedFile {
        salt,
        nonce,
        ciphertext,
    };

    // Emit: Writing file
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::writing());

    let allow_overwrite = allow_overwrite.unwrap_or(false);

    // Write encrypted file to disk with secure permissions and atomic write
    let output_data = encrypted_file.serialize();
    let resolved_path = atomic_write(&output_path, &output_data, allow_overwrite)?;
    log::info!("Encrypted file written to: {}", resolved_path.display());

    // Emit: Complete
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::encrypt_complete());

    let output_path = resolved_path.to_string_lossy().to_string();
    Ok(CryptoResponse {
        message: format!("File encrypted successfully: {}", output_path),
        output_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_encrypt_file_success() {
        // Create a temporary input file
        let input_file = NamedTempFile::new().unwrap();
        let input_path = input_file.path().to_str().unwrap();
        fs::write(input_path, b"Test content").unwrap();

        // Create output path
        let output_file = NamedTempFile::new().unwrap();
        let output_path = output_file.path().to_str().unwrap();

        // Encrypt using implementation function
        let result = encrypt_file_impl(input_path, output_path, "test_password");

        assert!(result.is_ok());

        // Verify output file was created
        let output_data = fs::read(output_path).unwrap();
        assert!(!output_data.is_empty());

        // Verify it's not the same as input (it's encrypted)
        assert_ne!(output_data, b"Test content");
    }

    #[test]
    fn test_encrypt_file_empty_password() {
        let input_file = NamedTempFile::new().unwrap();
        let output_file = NamedTempFile::new().unwrap();

        let result = encrypt_file_impl(
            input_file.path().to_str().unwrap(),
            output_file.path().to_str().unwrap(),
            "",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_file_nonexistent_input() {
        let output_file = NamedTempFile::new().unwrap();

        let result = encrypt_file_impl(
            "/nonexistent/file.txt",
            output_file.path().to_str().unwrap(),
            "password",
        );

        assert!(result.is_err());
    }
}
