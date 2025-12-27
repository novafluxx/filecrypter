// commands/batch.rs - Batch Encryption/Decryption Commands
//
// This module implements batch processing for multiple files.
// Each file is encrypted/decrypted independently:
// - A unique salt is generated per encrypted file, so Argon2id key derivation runs per file.
// - The `Password` wrapper is reused across the batch to avoid repeated allocations.
//
// Progress events are emitted for each file being processed.

use serde::Serialize;
use std::fs;
use std::path::Path;
use tauri::{command, AppHandle, Emitter};

use crate::commands::file_utils::{
    atomic_write, validate_batch_count, validate_file_size, validate_input_path,
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

fn emit_batch_progress<F>(
    emit_progress: &mut F,
    input_path: &str,
    file_index: usize,
    total_files: usize,
    stage: &str,
) where
    F: FnMut(BatchProgress),
{
    let file_name = Path::new(input_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| input_path.to_string());

    emit_progress(BatchProgress {
        current_file: file_name,
        file_index,
        total_files,
        stage: stage.to_string(),
        percent: ((file_index * 100) / total_files) as u32,
    });
}

fn emit_batch_complete<F>(emit_progress: &mut F, total_files: usize)
where
    F: FnMut(BatchProgress),
{
    emit_progress(BatchProgress {
        current_file: String::new(),
        file_index: total_files,
        total_files,
        stage: "complete".to_string(),
        percent: 100,
    });
}

fn batch_encrypt_impl<F>(
    input_paths: &[String],
    output_dir: &str,
    password: &str,
    allow_overwrite: bool,
    emit_progress: &mut F,
) -> CryptoResult<BatchResult>
where
    F: FnMut(BatchProgress),
{
    if password.is_empty() {
        return Err(CryptoError::FormatError(
            "Password cannot be empty".to_string(),
        ));
    }

    if input_paths.is_empty() {
        return Err(CryptoError::FormatError("No files selected".to_string()));
    }

    // Validate batch file count
    validate_batch_count(input_paths.len())?;

    // Verify output directory exists
    if !Path::new(output_dir).is_dir() {
        return Err(CryptoError::FormatError(
            "Output directory does not exist".to_string(),
        ));
    }

    let total_files = input_paths.len();
    let mut results: Vec<FileResult> = Vec::with_capacity(total_files);
    let password = Password::new(password.to_string());

    for (index, input_path) in input_paths.iter().enumerate() {
        emit_batch_progress(emit_progress, input_path, index, total_files, "encrypting");

        let result = encrypt_single_file(&password, input_path, output_dir, allow_overwrite);

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

    emit_batch_complete(emit_progress, total_files);

    let success_count = results.iter().filter(|r| r.success).count();
    let failed_count = results.len() - success_count;

    log::info!(
        "Batch encryption complete: {} succeeded, {} failed",
        success_count,
        failed_count
    );

    Ok(BatchResult {
        files: results,
        success_count,
        failed_count,
    })
}

fn batch_decrypt_impl<F>(
    input_paths: &[String],
    output_dir: &str,
    password: &str,
    allow_overwrite: bool,
    emit_progress: &mut F,
) -> CryptoResult<BatchResult>
where
    F: FnMut(BatchProgress),
{
    if password.is_empty() {
        return Err(CryptoError::FormatError(
            "Password cannot be empty".to_string(),
        ));
    }

    if input_paths.is_empty() {
        return Err(CryptoError::FormatError("No files selected".to_string()));
    }

    // Validate batch file count
    validate_batch_count(input_paths.len())?;

    // Verify output directory exists
    if !Path::new(output_dir).is_dir() {
        return Err(CryptoError::FormatError(
            "Output directory does not exist".to_string(),
        ));
    }

    let total_files = input_paths.len();
    let mut results: Vec<FileResult> = Vec::with_capacity(total_files);
    let password = Password::new(password.to_string());

    for (index, input_path) in input_paths.iter().enumerate() {
        emit_batch_progress(emit_progress, input_path, index, total_files, "decrypting");

        let result = decrypt_single_file(&password, input_path, output_dir, allow_overwrite);

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

    emit_batch_complete(emit_progress, total_files);

    let success_count = results.iter().filter(|r| r.success).count();
    let failed_count = results.len() - success_count;

    log::info!(
        "Batch decryption complete: {} succeeded, {} failed",
        success_count,
        failed_count
    );

    Ok(BatchResult {
        files: results,
        success_count,
        failed_count,
    })
}

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
/// * `allow_overwrite` - Allow overwriting existing files (default: false)
///
/// # Returns
/// BatchResult with success/failure status for each file
#[command]
pub async fn batch_encrypt(
    app: AppHandle,
    input_paths: Vec<String>,
    output_dir: String,
    password: String,
    allow_overwrite: Option<bool>,
) -> CryptoResult<BatchResult> {
    log::info!(
        "Batch encrypting {} files to {}",
        input_paths.len(),
        output_dir
    );

    let mut emit_progress = |progress: BatchProgress| {
        let _ = app.emit(BATCH_PROGRESS_EVENT, progress);
    };

    let allow_overwrite = allow_overwrite.unwrap_or(false);

    batch_encrypt_impl(
        &input_paths,
        &output_dir,
        &password,
        allow_overwrite,
        &mut emit_progress,
    )
}

/// Encrypt a single file (internal helper)
fn encrypt_single_file(
    password: &Password,
    input_path: &str,
    output_dir: &str,
    allow_overwrite: bool,
) -> CryptoResult<String> {
    // Validate input path (check for symlinks)
    let validated_path = validate_input_path(input_path)
        .map_err(|e| CryptoError::FormatError(format!("File '{}': {}", input_path, e)))?;

    // Validate file size for in-memory operation
    validate_file_size(&validated_path)
        .map_err(|e| CryptoError::FormatError(format!("File '{}': {}", input_path, e)))?;

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

    // Serialize and write atomically with secure permissions
    let encrypted_file = EncryptedFile {
        salt,
        nonce,
        ciphertext,
    };
    let resolved_path = atomic_write(&output_path, &encrypted_file.serialize(), allow_overwrite)?;

    Ok(resolved_path.to_string_lossy().to_string())
}

/// Decrypt multiple files with the same password
///
/// # Arguments
/// * `app` - Tauri app handle for emitting progress events
/// * `input_paths` - List of encrypted file paths to decrypt
/// * `output_dir` - Directory where decrypted files will be saved
/// * `password` - Password for decryption
/// * `allow_overwrite` - Allow overwriting existing files (default: false)
///
/// # Returns
/// BatchResult with success/failure status for each file
#[command]
pub async fn batch_decrypt(
    app: AppHandle,
    input_paths: Vec<String>,
    output_dir: String,
    password: String,
    allow_overwrite: Option<bool>,
) -> CryptoResult<BatchResult> {
    log::info!(
        "Batch decrypting {} files to {}",
        input_paths.len(),
        output_dir
    );

    let mut emit_progress = |progress: BatchProgress| {
        let _ = app.emit(BATCH_PROGRESS_EVENT, progress);
    };

    let allow_overwrite = allow_overwrite.unwrap_or(false);

    batch_decrypt_impl(
        &input_paths,
        &output_dir,
        &password,
        allow_overwrite,
        &mut emit_progress,
    )
}

/// Decrypt a single file (internal helper)
fn decrypt_single_file(
    password: &Password,
    input_path: &str,
    output_dir: &str,
    allow_overwrite: bool,
) -> CryptoResult<String> {
    // Validate input path (check for symlinks)
    let validated_path = validate_input_path(input_path)
        .map_err(|e| CryptoError::FormatError(format!("File '{}': {}", input_path, e)))?;

    // Validate file size for in-memory operation
    validate_file_size(&validated_path)
        .map_err(|e| CryptoError::FormatError(format!("File '{}': {}", input_path, e)))?;

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

    // Write decrypted file atomically with secure permissions
    let resolved_path = atomic_write(&output_path, &plaintext, allow_overwrite)?;

    Ok(resolved_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::file_utils::MAX_BATCH_FILES;
    use std::path::Path;
    use tempfile::tempdir;

    fn write_input_file(dir: &Path, name: &str, content: &[u8]) -> String {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_batch_encrypt_multiple_files() {
        let input_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        let input_paths = vec![
            write_input_file(input_dir.path(), "file1.txt", b"alpha"),
            write_input_file(input_dir.path(), "file2.txt", b"beta"),
        ];
        let output_dir_str = output_dir.path().to_string_lossy().to_string();
        let mut no_progress = |_progress: BatchProgress| {};

        let result = batch_encrypt_impl(
            &input_paths,
            &output_dir_str,
            "password123",
            false,
            &mut no_progress,
        )
        .unwrap();

        assert_eq!(result.success_count, 2);
        assert_eq!(result.failed_count, 0);
        for file_result in result.files {
            assert!(file_result.success);
            let output_path = file_result.output_path.unwrap();
            assert!(Path::new(&output_path).exists());
        }
    }

    #[test]
    fn test_batch_encrypt_partial_failure() {
        let input_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        let valid_path = write_input_file(input_dir.path(), "file1.txt", b"alpha");
        let missing_path = input_dir
            .path()
            .join("missing.txt")
            .to_string_lossy()
            .to_string();
        let input_paths = vec![valid_path, missing_path];
        let output_dir_str = output_dir.path().to_string_lossy().to_string();
        let mut no_progress = |_progress: BatchProgress| {};

        let result = batch_encrypt_impl(
            &input_paths,
            &output_dir_str,
            "password123",
            false,
            &mut no_progress,
        )
        .unwrap();

        assert_eq!(result.success_count, 1);
        assert_eq!(result.failed_count, 1);
        assert!(result.files.iter().any(|file| file.success));
        assert!(result.files.iter().any(|file| !file.success));
    }

    #[test]
    fn test_batch_encrypt_empty_list() {
        let output_dir = tempdir().unwrap();
        let input_paths: Vec<String> = Vec::new();
        let mut no_progress = |_progress: BatchProgress| {};

        let result = batch_encrypt_impl(
            &input_paths,
            output_dir.path().to_str().unwrap(),
            "password123",
            false,
            &mut no_progress,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_batch_encrypt_nonexistent_output_dir() {
        let input_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        let missing_output = output_dir.path().join("missing");
        let input_paths = vec![write_input_file(input_dir.path(), "file1.txt", b"alpha")];
        let mut no_progress = |_progress: BatchProgress| {};

        let result = batch_encrypt_impl(
            &input_paths,
            missing_output.to_str().unwrap(),
            "password123",
            false,
            &mut no_progress,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_batch_decrypt_wrong_password() {
        let input_dir = tempdir().unwrap();
        let encrypt_dir = tempdir().unwrap();
        let decrypt_dir = tempdir().unwrap();
        let input_path = write_input_file(input_dir.path(), "file1.txt", b"alpha");
        let encrypted_path = encrypt_single_file(
            &Password::new("correct_password".to_string()),
            &input_path,
            encrypt_dir.path().to_str().unwrap(),
            false,
        )
        .unwrap();
        let input_paths = vec![encrypted_path];
        let mut no_progress = |_progress: BatchProgress| {};

        let result = batch_decrypt_impl(
            &input_paths,
            decrypt_dir.path().to_str().unwrap(),
            "wrong_password",
            false,
            &mut no_progress,
        )
        .unwrap();

        assert_eq!(result.success_count, 0);
        assert_eq!(result.failed_count, 1);
        assert!(result.files.iter().all(|file| !file.success));
    }

    #[test]
    fn test_batch_file_count_limit() {
        let output_dir = tempdir().unwrap();
        let input_paths = vec!["missing".to_string(); MAX_BATCH_FILES + 1];
        let mut no_progress = |_progress: BatchProgress| {};

        let result = batch_encrypt_impl(
            &input_paths,
            output_dir.path().to_str().unwrap(),
            "password123",
            false,
            &mut no_progress,
        );

        assert!(result.is_err());
    }
}
