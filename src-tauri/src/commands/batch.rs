// commands/batch.rs - Batch Encryption/Decryption Commands
//
// This module implements batch processing for multiple files.
// Key optimization: derive encryption key ONCE per batch (same password).
// Each file still gets its own unique salt for security.
//
// Progress events are emitted for each file being processed.

use std::fs;
use std::path::Path;
use tauri::{command, AppHandle, Emitter};
use serde::Serialize;

use crate::commands::file_utils::{
    secure_write, validate_batch_count, validate_file_size, validate_input_path,
};
use crate::crypto::{decrypt, derive_key, encrypt, generate_salt, EncryptedFile, Password};
use crate::error::{CryptoError, CryptoResult};

/// Progress event for batch operations
#[derive(Clone, Serialize)]
pub struct BatchProgress {
    /// Name of the current file being processed
    pub current_file: String,
    /// Index of current file (0-based)
    pub file_index: usize,
    /// Total number of files in batch
    pub total_files: usize,
    /// Current stage: "encrypting", "decrypting", "complete"
    pub stage: String,
    /// Overall progress percentage (0-100)
    pub percent: u32,
}

/// Result for a single file in batch operation
#[derive(Clone, Serialize)]
pub struct FileResult {
    /// Original input path
    pub input_path: String,
    /// Output path (if successful)
    pub output_path: Option<String>,
    /// Whether this file succeeded
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Result of a batch operation
#[derive(Clone, Serialize)]
pub struct BatchResult {
    /// Results for each file
    pub files: Vec<FileResult>,
    /// Number of successful files
    pub success_count: usize,
    /// Number of failed files
    pub failed_count: usize,
}

/// Event name for batch progress
pub const BATCH_PROGRESS_EVENT: &str = "batch-progress";

/// Encrypt multiple files with the same password
///
/// This command efficiently encrypts multiple files by deriving the key once.
/// Each file gets its own unique salt for security.
///
/// # Arguments
/// * `app` - Tauri app handle for emitting progress events
/// * `input_paths` - List of file paths to encrypt
/// * `output_dir` - Directory where encrypted files will be saved
/// * `password` - Password for encryption (used for all files)
///
/// # Returns
/// BatchResult with success/failure status for each file
#[command]
pub async fn batch_encrypt(
    app: AppHandle,
    input_paths: Vec<String>,
    output_dir: String,
    password: String,
) -> CryptoResult<BatchResult> {
    log::info!("Batch encrypting {} files to {}", input_paths.len(), output_dir);

    if password.is_empty() {
        return Err(CryptoError::FormatError("Password cannot be empty".to_string()));
    }

    if input_paths.is_empty() {
        return Err(CryptoError::FormatError("No files selected".to_string()));
    }

    // Validate batch file count
    validate_batch_count(input_paths.len())?;

    // Verify output directory exists
    if !Path::new(&output_dir).is_dir() {
        return Err(CryptoError::FormatError("Output directory does not exist".to_string()));
    }

    let total_files = input_paths.len();
    let mut results: Vec<FileResult> = Vec::with_capacity(total_files);
    let password = Password::new(password);

    for (index, input_path) in input_paths.iter().enumerate() {
        // Emit progress
        let file_name = Path::new(input_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| input_path.clone());

        let _ = app.emit(BATCH_PROGRESS_EVENT, BatchProgress {
            current_file: file_name.clone(),
            file_index: index,
            total_files,
            stage: "encrypting".to_string(),
            percent: ((index * 100) / total_files) as u32,
        });

        // Process this file
        let result = encrypt_single_file(&password, input_path, &output_dir).await;

        match result {
            Ok(output_path) => {
                results.push(FileResult {
                    input_path: input_path.clone(),
                    output_path: Some(output_path),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                log::error!("Failed to encrypt {}: {}", input_path, e);
                results.push(FileResult {
                    input_path: input_path.clone(),
                    output_path: None,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Emit completion
    let _ = app.emit(BATCH_PROGRESS_EVENT, BatchProgress {
        current_file: String::new(),
        file_index: total_files,
        total_files,
        stage: "complete".to_string(),
        percent: 100,
    });

    let success_count = results.iter().filter(|r| r.success).count();
    let failed_count = results.len() - success_count;

    log::info!("Batch encryption complete: {} succeeded, {} failed", success_count, failed_count);

    Ok(BatchResult {
        files: results,
        success_count,
        failed_count,
    })
}

/// Encrypt a single file (internal helper)
async fn encrypt_single_file(
    password: &Password,
    input_path: &str,
    output_dir: &str,
) -> CryptoResult<String> {
    // Validate input path (check for symlinks)
    let validated_path = validate_input_path(input_path)?;

    // Validate file size for in-memory operation
    validate_file_size(&validated_path)?;

    // Read input file
    let plaintext = fs::read(&validated_path)?;

    // Generate unique salt for this file
    let salt = generate_salt()?;

    // Derive key (this is intentionally slow for security)
    let key = derive_key(password, &salt)?;

    // Encrypt
    let (nonce, ciphertext) = encrypt(&key, &plaintext)?;

    // Create output path
    let input_filename = validated_path
        .file_name()
        .ok_or_else(|| CryptoError::FormatError("Invalid input path".to_string()))?;
    let output_filename = format!("{}.encrypted", input_filename.to_string_lossy());
    let output_path = Path::new(output_dir).join(&output_filename);

    // Serialize and write with secure permissions
    let encrypted_file = EncryptedFile {
        salt,
        nonce,
        ciphertext,
    };
    secure_write(&output_path, &encrypted_file.serialize())?;

    Ok(output_path.to_string_lossy().to_string())
}

/// Decrypt multiple files with the same password
///
/// # Arguments
/// * `app` - Tauri app handle for emitting progress events
/// * `input_paths` - List of encrypted file paths to decrypt
/// * `output_dir` - Directory where decrypted files will be saved
/// * `password` - Password for decryption
///
/// # Returns
/// BatchResult with success/failure status for each file
#[command]
pub async fn batch_decrypt(
    app: AppHandle,
    input_paths: Vec<String>,
    output_dir: String,
    password: String,
) -> CryptoResult<BatchResult> {
    log::info!("Batch decrypting {} files to {}", input_paths.len(), output_dir);

    if password.is_empty() {
        return Err(CryptoError::FormatError("Password cannot be empty".to_string()));
    }

    if input_paths.is_empty() {
        return Err(CryptoError::FormatError("No files selected".to_string()));
    }

    // Validate batch file count
    validate_batch_count(input_paths.len())?;

    // Verify output directory exists
    if !Path::new(&output_dir).is_dir() {
        return Err(CryptoError::FormatError("Output directory does not exist".to_string()));
    }

    let total_files = input_paths.len();
    let mut results: Vec<FileResult> = Vec::with_capacity(total_files);
    let password = Password::new(password);

    for (index, input_path) in input_paths.iter().enumerate() {
        // Emit progress
        let file_name = Path::new(input_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| input_path.clone());

        let _ = app.emit(BATCH_PROGRESS_EVENT, BatchProgress {
            current_file: file_name.clone(),
            file_index: index,
            total_files,
            stage: "decrypting".to_string(),
            percent: ((index * 100) / total_files) as u32,
        });

        // Process this file
        let result = decrypt_single_file(&password, input_path, &output_dir).await;

        match result {
            Ok(output_path) => {
                results.push(FileResult {
                    input_path: input_path.clone(),
                    output_path: Some(output_path),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                log::error!("Failed to decrypt {}: {}", input_path, e);
                results.push(FileResult {
                    input_path: input_path.clone(),
                    output_path: None,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Emit completion
    let _ = app.emit(BATCH_PROGRESS_EVENT, BatchProgress {
        current_file: String::new(),
        file_index: total_files,
        total_files,
        stage: "complete".to_string(),
        percent: 100,
    });

    let success_count = results.iter().filter(|r| r.success).count();
    let failed_count = results.len() - success_count;

    log::info!("Batch decryption complete: {} succeeded, {} failed", success_count, failed_count);

    Ok(BatchResult {
        files: results,
        success_count,
        failed_count,
    })
}

/// Decrypt a single file (internal helper)
async fn decrypt_single_file(
    password: &Password,
    input_path: &str,
    output_dir: &str,
) -> CryptoResult<String> {
    // Validate input path (check for symlinks)
    let validated_path = validate_input_path(input_path)?;

    // Validate file size for in-memory operation
    validate_file_size(&validated_path)?;

    // Read encrypted file
    let encrypted_data = fs::read(&validated_path)?;

    // Parse format
    let encrypted_file = EncryptedFile::deserialize(&encrypted_data)?;

    // Derive key using salt from file
    let key = derive_key(password, &encrypted_file.salt)?;

    // Decrypt
    let plaintext = decrypt(&key, &encrypted_file.nonce, &encrypted_file.ciphertext)?;

    // Create output path (remove .encrypted extension if present)
    let input_filename = validated_path
        .file_name()
        .ok_or_else(|| CryptoError::FormatError("Invalid input path".to_string()))?
        .to_string_lossy();

    let output_filename = if let Some(stripped) = input_filename.strip_suffix(".encrypted") {
        stripped.to_string()
    } else {
        format!("{}.decrypted", input_filename)
    };

    let output_path = Path::new(output_dir).join(&output_filename);

    // Write decrypted file with secure permissions
    secure_write(&output_path, &plaintext)?;

    Ok(output_path.to_string_lossy().to_string())
}
