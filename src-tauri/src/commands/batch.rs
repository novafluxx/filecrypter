// commands/batch.rs - Batch Encryption/Decryption Commands
//
// This module implements batch processing for multiple files.
// Each file is encrypted/decrypted independently with its own unique encryption key:
// - A unique salt is generated per file
// - Argon2id key derivation runs separately for each file, producing different keys
// - The `Password` wrapper is reused across the batch to avoid repeated allocations
//
// Key characteristics:
// - Each file is encrypted/decrypted independently using streaming (1MB chunks)
// - No per-file size limit (can handle files of any size)
// - Unique salt generated per file (each file has independent key derivation)
// - Password wrapper reused across batch (avoids repeated allocations)
// - Maximum 1000 files per batch (configurable via MAX_BATCH_FILES)
//
// Progress tracking:
// - Emits BatchProgress events after each file completes
// - Reports: current file name, file index, total files, stage, percentage
// - Frontend can display per-file progress and overall batch progress
//
// Error handling:
// - Failed files don't stop the batch (continues to next file)
// - Each file result includes success status and error message
// - BatchResult aggregates all individual file results

use serde::Serialize;
use std::path::Path;
use tauri::{command, AppHandle, Emitter};

use crate::commands::archive::{
    create_tar_zstd_archive, extract_tar_zstd_archive, generate_archive_name,
};
use crate::commands::file_utils::{resolve_output_path, validate_batch_count, validate_input_path};
use crate::crypto::{
    decrypt_file_streaming, encrypt_file_streaming, CompressionConfig, Password, DEFAULT_CHUNK_SIZE,
};
use crate::error::{CryptoError, CryptoResult};

/// Progress event for batch operations.
///
/// Emitted after each file is processed to update the frontend on batch progress.
/// Listen for `BATCH_PROGRESS_EVENT` ("batch-progress") events in the frontend.
#[derive(Clone, Serialize)]
pub struct BatchProgress {
    /// Name of the current file being processed (filename only, not full path)
    pub current_file: String,
    /// Index of current file (0-based, ranges from 0 to total_files-1)
    pub file_index: usize,
    /// Total number of files in the batch
    pub total_files: usize,
    /// Current stage: "encrypting", "decrypting", or "complete"
    pub stage: String,
    /// Overall batch progress percentage (0-100)
    pub percent: u32,
}

/// Result for a single file in a batch operation.
///
/// Contains the outcome of encrypting or decrypting one file within a batch.
#[derive(Clone, Serialize)]
pub struct FileResult {
    /// Original input file path as provided by the user
    pub input_path: String,
    /// Resolved output path where the result was saved (None if operation failed)
    pub output_path: Option<String>,
    /// Whether encryption/decryption succeeded for this file
    pub success: bool,
    /// Error message describing why the operation failed (None if successful)
    pub error: Option<String>,
}

/// Aggregated result of a batch encrypt/decrypt operation.
///
/// Contains individual results for each file plus summary statistics.
#[derive(Clone, Serialize)]
pub struct BatchResult {
    /// Individual results for each file in the batch (in order of processing)
    pub files: Vec<FileResult>,
    /// Count of files that were successfully processed
    pub success_count: usize,
    /// Count of files that failed to process
    pub failed_count: usize,
}

/// Event name for batch progress
pub const BATCH_PROGRESS_EVENT: &str = "batch-progress";

/// Event name for archive progress
pub const ARCHIVE_PROGRESS_EVENT: &str = "archive-progress";

/// Progress event for archive operations.
///
/// Emitted during archive encrypt/decrypt to update the frontend on progress.
/// Contains phase information and detailed progress tracking.
#[derive(Clone, Serialize)]
pub struct ArchiveProgress {
    /// Current phase: "archiving", "encrypting", "decrypting", "extracting"
    pub phase: String,
    /// Name of the current file being processed (if applicable)
    pub current_file: Option<String>,
    /// Number of files processed in current phase
    pub files_processed: usize,
    /// Total number of files in current phase
    pub total_files: usize,
    /// Overall progress percentage (0-100)
    pub percent: u32,
}

/// Result of an archive encrypt/decrypt operation.
#[derive(Clone, Serialize)]
pub struct ArchiveResult {
    /// Path to the created archive (encrypt) or output directory (decrypt)
    pub output_path: String,
    /// Number of files included in the archive (encrypt) or extracted (decrypt)
    pub file_count: usize,
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if operation failed
    pub error: Option<String>,
}

/// Emit a batch progress event for the current file.
///
/// Extracts the filename from the input path and calculates the overall
/// percentage based on the file index.
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

/// Emit a batch completion event indicating all files have been processed.
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

/// Core implementation of batch encryption.
///
/// This is separated from the Tauri command to allow unit testing without
/// requiring a Tauri runtime.
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

/// Core implementation of batch decryption.
///
/// This is separated from the Tauri command to allow unit testing without
/// requiring a Tauri runtime.
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
/// Each file is encrypted independently with its own unique salt, which means
/// a new encryption key is derived for each file (via Argon2id KDF).
/// This ensures files encrypted with the same password have different keys.
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

/// Encrypt a single file (internal helper for batch operations)
///
/// Uses streaming encryption to handle files of any size. The output filename
/// is automatically generated by appending ".encrypted" to the input filename.
/// Compression is always enabled for batch operations.
///
/// # Arguments
/// * `password` - Reused password wrapper across batch
/// * `input_path` - Path to file to encrypt
/// * `output_dir` - Directory where encrypted file will be saved
/// * `allow_overwrite` - Whether to overwrite existing files
///
/// # Returns
/// The path to the encrypted file as a String
///
/// # Note
/// No progress callback is provided because batch operations track progress
/// at the file level, not the chunk level.
fn encrypt_single_file(
    password: &Password,
    input_path: &str,
    output_dir: &str,
    allow_overwrite: bool,
) -> CryptoResult<String> {
    // Validate input path (check for symlinks)
    let validated_path = validate_input_path(input_path)
        .map_err(|e| CryptoError::FormatError(format!("File '{}': {}", input_path, e)))?;

    // Create output path
    let input_filename = validated_path
        .file_name()
        .ok_or_else(|| CryptoError::FormatError("Invalid input path".to_string()))?;
    let output_filename = format!("{}.encrypted", input_filename.to_string_lossy());
    let output_path = Path::new(output_dir).join(&output_filename);
    let resolved_output_path = resolve_output_path(&output_path, allow_overwrite)?;

    // Use streaming encryption with compression for batch operations
    // Compression is always enabled for batch mode
    encrypt_file_streaming(
        validated_path,
        &resolved_output_path,
        password,
        DEFAULT_CHUNK_SIZE,
        None, // No progress callback - batch has its own progress tracking
        allow_overwrite,
        Some(CompressionConfig::default()), // ZSTD level 3 compression
    )?;

    Ok(resolved_output_path.to_string_lossy().to_string())
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

/// Decrypt a single file (internal helper for batch operations)
///
/// Uses streaming decryption to handle files of any size. If the input filename
/// ends with ".encrypted", that extension is removed; otherwise ".decrypted" is appended.
///
/// # Arguments
/// * `password` - Reused password wrapper across batch
/// * `input_path` - Path to encrypted file
/// * `output_dir` - Directory where decrypted file will be saved
/// * `allow_overwrite` - Whether to overwrite existing files
///
/// # Returns
/// The path to the decrypted file as a String
///
/// # Note
/// No progress callback is provided because batch operations track progress
/// at the file level, not the chunk level.
fn decrypt_single_file(
    password: &Password,
    input_path: &str,
    output_dir: &str,
    allow_overwrite: bool,
) -> CryptoResult<String> {
    // Validate input path (check for symlinks)
    let validated_path = validate_input_path(input_path)
        .map_err(|e| CryptoError::FormatError(format!("File '{}': {}", input_path, e)))?;

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
    let resolved_output_path = resolve_output_path(&output_path, allow_overwrite)?;

    // Use streaming decryption for all files
    decrypt_file_streaming(
        &validated_path,
        &resolved_output_path,
        password,
        None, // No progress callback - batch has its own progress tracking
        allow_overwrite,
    )?;

    Ok(resolved_output_path.to_string_lossy().to_string())
}

// =============================================================================
// Archive Mode Commands
// =============================================================================

/// Encrypt multiple files as a single encrypted archive.
///
/// This creates a compressed TAR archive from the input files, then encrypts
/// the entire archive as a single unit. This is useful when you want to bundle
/// multiple files together and protect them with a single password.
///
/// # Arguments
/// * `app` - Tauri app handle for emitting progress events
/// * `input_paths` - List of file paths to include in the archive
/// * `output_dir` - Directory where the encrypted archive will be saved
/// * `password` - Password for encryption
/// * `archive_name` - Optional custom name for the archive (without extension)
/// * `allow_overwrite` - Allow overwriting existing files (default: false)
///
/// # Returns
/// ArchiveResult with the path to the encrypted archive
#[command]
pub async fn batch_encrypt_archive(
    app: AppHandle,
    input_paths: Vec<String>,
    output_dir: String,
    password: String,
    archive_name: Option<String>,
    allow_overwrite: Option<bool>,
) -> CryptoResult<ArchiveResult> {
    log::info!(
        "Batch archive encrypting {} files to {}",
        input_paths.len(),
        output_dir
    );

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
    if !Path::new(&output_dir).is_dir() {
        return Err(CryptoError::FormatError(
            "Output directory does not exist".to_string(),
        ));
    }

    let allow_overwrite = allow_overwrite.unwrap_or(false);
    let total_files = input_paths.len();

    // Emit initial progress
    let emit_archive_progress =
        |phase: &str, current_file: Option<&str>, processed: usize, total: usize, percent: u32| {
            let _ = app.emit(
                ARCHIVE_PROGRESS_EVENT,
                ArchiveProgress {
                    phase: phase.to_string(),
                    current_file: current_file.map(|s| s.to_string()),
                    files_processed: processed,
                    total_files: total,
                    percent,
                },
            );
        };

    // Generate archive filename
    let archive_filename = generate_archive_name(archive_name.as_deref());
    let archive_path = Path::new(&output_dir).join(&archive_filename);
    let encrypted_path = Path::new(&output_dir).join(format!("{}.encrypted", archive_filename));
    let resolved_encrypted_path = resolve_output_path(&encrypted_path, allow_overwrite)?;

    // Phase 1: Create compressed TAR archive
    emit_archive_progress("archiving", None, 0, total_files, 0);

    let archive_progress_callback = {
        let app = app.clone();
        Box::new(move |processed: usize, total: usize, current: &str| {
            let percent = if total > 0 {
                ((processed * 25) / total) as u32 // 0-25% for archiving phase
            } else {
                0
            };
            let _ = app.emit(
                ARCHIVE_PROGRESS_EVENT,
                ArchiveProgress {
                    phase: "archiving".to_string(),
                    current_file: if current.is_empty() {
                        None
                    } else {
                        Some(current.to_string())
                    },
                    files_processed: processed,
                    total_files: total,
                    percent,
                },
            );
        })
    };

    // Create the archive
    let input_path_refs: Vec<&Path> = input_paths.iter().map(Path::new).collect();
    if let Err(e) = create_tar_zstd_archive(
        &input_path_refs,
        &archive_path,
        Some(archive_progress_callback),
    ) {
        return Ok(ArchiveResult {
            output_path: String::new(),
            file_count: 0,
            success: false,
            error: Some(e.to_string()),
        });
    }

    // Phase 2: Encrypt the archive
    emit_archive_progress(
        "encrypting",
        Some(&archive_filename),
        total_files,
        total_files,
        25,
    );

    let password_wrapper = Password::new(password);

    // Encrypt progress callback (25-100%)
    let encrypt_progress_callback = {
        let app = app.clone();
        let archive_filename_clone = archive_filename.clone();
        // Capture the number of input files for progress reporting (distinct from processed/total bytes)
        let input_file_count = total_files;
        Box::new(move |processed: u64, total: u64| {
            let encrypt_percent = if total > 0 {
                ((processed * 75) / total) as u32 + 25 // 25-100% for encryption phase
            } else {
                25
            };
            let _ = app.emit(
                ARCHIVE_PROGRESS_EVENT,
                ArchiveProgress {
                    phase: "encrypting".to_string(),
                    current_file: Some(archive_filename_clone.clone()),
                    files_processed: input_file_count,
                    total_files: input_file_count,
                    percent: encrypt_percent,
                },
            );
        })
    };

    // Encrypt the archive (no additional compression since archive is already compressed)
    let result = encrypt_file_streaming(
        &archive_path,
        &resolved_encrypted_path,
        &password_wrapper,
        DEFAULT_CHUNK_SIZE,
        Some(encrypt_progress_callback),
        allow_overwrite,
        None, // No compression - archive is already ZSTD compressed
    );

    // Clean up temporary archive file.
    // This is a best-effort cleanup - we log failures rather than returning an error
    // because the encryption itself succeeded. The temp file will eventually be
    // cleaned up by the OS or user, but we want visibility into cleanup failures
    // for debugging purposes.
    if let Err(e) = std::fs::remove_file(&archive_path) {
        log::warn!("Failed to clean up temporary archive file: {}", e);
    }

    match result {
        Ok(()) => {
            emit_archive_progress("complete", None, total_files, total_files, 100);
            log::info!(
                "Archive encryption complete: {} files -> {}",
                total_files,
                resolved_encrypted_path.display()
            );
            Ok(ArchiveResult {
                output_path: resolved_encrypted_path.to_string_lossy().to_string(),
                file_count: total_files,
                success: true,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Archive encryption failed: {}", e);
            Ok(ArchiveResult {
                output_path: String::new(),
                file_count: 0,
                success: false,
                error: Some(e.to_string()),
            })
        }
    }
}

/// Decrypt an encrypted archive and extract its contents.
///
/// This decrypts an encrypted .tar.zst.encrypted file and extracts all files
/// from the archive to the specified output directory.
///
/// # Arguments
/// * `app` - Tauri app handle for emitting progress events
/// * `input_path` - Path to the encrypted archive file
/// * `output_dir` - Directory where extracted files will be saved
/// * `password` - Password for decryption
/// * `allow_overwrite` - Allow overwriting existing files (default: false)
///
/// # Returns
/// ArchiveResult with the output directory and number of extracted files
#[command]
pub async fn batch_decrypt_archive(
    app: AppHandle,
    input_path: String,
    output_dir: String,
    password: String,
    allow_overwrite: Option<bool>,
) -> CryptoResult<ArchiveResult> {
    log::info!("Batch archive decrypting {} to {}", input_path, output_dir);

    if password.is_empty() {
        return Err(CryptoError::FormatError(
            "Password cannot be empty".to_string(),
        ));
    }

    // Verify output directory exists
    if !Path::new(&output_dir).is_dir() {
        return Err(CryptoError::FormatError(
            "Output directory does not exist".to_string(),
        ));
    }

    let allow_overwrite = allow_overwrite.unwrap_or(false);

    // Emit initial progress
    let emit_archive_progress =
        |phase: &str, current_file: Option<&str>, processed: usize, total: usize, percent: u32| {
            let _ = app.emit(
                ARCHIVE_PROGRESS_EVENT,
                ArchiveProgress {
                    phase: phase.to_string(),
                    current_file: current_file.map(|s| s.to_string()),
                    files_processed: processed,
                    total_files: total,
                    percent,
                },
            );
        };

    // Phase 1: Decrypt the archive
    emit_archive_progress("decrypting", None, 0, 0, 0);

    let password_wrapper = Password::new(password);
    let input_file_name = Path::new(&input_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Generate temp path for decrypted archive
    let decrypted_archive_name = input_file_name
        .strip_suffix(".encrypted")
        .unwrap_or(&input_file_name);
    let temp_archive_path = Path::new(&output_dir).join(format!(".tmp_{}", decrypted_archive_name));

    // Decrypt progress callback (0-50%)
    let decrypt_progress_callback = {
        let app = app.clone();
        let file_name = input_file_name.clone();
        Box::new(move |processed: u64, total: u64| {
            let decrypt_percent = if total > 0 {
                ((processed * 50) / total) as u32 // 0-50% for decryption phase
            } else {
                0
            };
            let _ = app.emit(
                ARCHIVE_PROGRESS_EVENT,
                ArchiveProgress {
                    phase: "decrypting".to_string(),
                    current_file: Some(file_name.clone()),
                    files_processed: 0,
                    total_files: 0,
                    percent: decrypt_percent,
                },
            );
        })
    };

    // Decrypt the archive
    if let Err(e) = decrypt_file_streaming(
        &input_path,
        &temp_archive_path,
        &password_wrapper,
        Some(decrypt_progress_callback),
        true, // Always overwrite temp file
    ) {
        // Attempt cleanup of the temp file after a decryption error.
        // Best-effort: we don't fail the operation if cleanup fails, just log it.
        if let Err(cleanup_err) = std::fs::remove_file(&temp_archive_path) {
            log::warn!(
                "Failed to clean up temporary file after decryption error: {}",
                cleanup_err
            );
        }
        return Ok(ArchiveResult {
            output_path: output_dir.clone(),
            file_count: 0,
            success: false,
            error: Some(e.to_string()),
        });
    }

    // Phase 2: Extract the archive
    emit_archive_progress("extracting", None, 0, 0, 50);

    let extract_progress_callback = {
        let app = app.clone();
        Box::new(move |processed: usize, total: usize, current: &str| {
            let extract_percent = if total > 0 {
                50 + ((processed * 50) / total) as u32 // 50-100% for extraction phase
            } else {
                50
            };
            let _ = app.emit(
                ARCHIVE_PROGRESS_EVENT,
                ArchiveProgress {
                    phase: "extracting".to_string(),
                    current_file: if current.is_empty() {
                        None
                    } else {
                        Some(current.to_string())
                    },
                    files_processed: processed,
                    total_files: total,
                    percent: extract_percent,
                },
            );
        })
    };

    let result = extract_tar_zstd_archive(
        &temp_archive_path,
        &output_dir,
        allow_overwrite,
        Some(extract_progress_callback),
    );

    // Clean up temporary decrypted archive file.
    // Best-effort cleanup - the extraction succeeded so we don't fail on cleanup errors.
    // Logging helps diagnose permission or filesystem issues during development.
    if let Err(e) = std::fs::remove_file(&temp_archive_path) {
        log::warn!("Failed to clean up temporary decrypted archive: {}", e);
    }

    match result {
        Ok(extracted_paths) => {
            let file_count = extracted_paths.len();
            emit_archive_progress("complete", None, file_count, file_count, 100);
            log::info!(
                "Archive decryption complete: {} files extracted to {}",
                file_count,
                output_dir
            );
            Ok(ArchiveResult {
                output_path: output_dir,
                file_count,
                success: true,
                error: None,
            })
        }
        Err(e) => {
            log::error!("Archive extraction failed: {}", e);
            Ok(ArchiveResult {
                output_path: output_dir,
                file_count: 0,
                success: false,
                error: Some(e.to_string()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::file_utils::MAX_BATCH_FILES;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn write_input_file(dir: &Path, name: &str, content: &[u8]) -> String {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        // Canonicalize to resolve any symlinks in the temp directory path
        fs::canonicalize(&path)
            .unwrap()
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn test_batch_encrypt_multiple_files() {
        let input_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        let input_paths = vec![
            write_input_file(input_dir.path(), "file1.txt", b"alpha"),
            write_input_file(input_dir.path(), "file2.txt", b"beta"),
        ];
        // Canonicalize output directory to resolve symlinks
        let output_dir_str = fs::canonicalize(output_dir.path())
            .unwrap()
            .to_string_lossy()
            .to_string();
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
    fn test_batch_encrypt_auto_renames_on_collision() {
        let input_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        let input_path = write_input_file(input_dir.path(), "sample.txt", b"alpha");
        let output_dir_str = fs::canonicalize(output_dir.path())
            .unwrap()
            .to_string_lossy()
            .to_string();
        let password = Password::new("password123".to_string());

        let first_output =
            encrypt_single_file(&password, &input_path, &output_dir_str, false).unwrap();
        let second_output =
            encrypt_single_file(&password, &input_path, &output_dir_str, false).unwrap();

        assert_ne!(first_output, second_output);
        assert!(Path::new(&first_output).exists());
        assert!(Path::new(&second_output).exists());
        assert!(second_output.ends_with("sample.txt (1).encrypted"));
    }

    #[test]
    fn test_batch_encrypt_partial_failure() {
        let input_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        let valid_path = write_input_file(input_dir.path(), "file1.txt", b"alpha");
        // Canonicalize the directory path for the missing file to avoid symlink issues
        let missing_path = fs::canonicalize(input_dir.path())
            .unwrap()
            .join("missing.txt")
            .to_string_lossy()
            .to_string();
        let input_paths = vec![valid_path, missing_path];
        // Canonicalize output directory to resolve symlinks
        let output_dir_str = fs::canonicalize(output_dir.path())
            .unwrap()
            .to_string_lossy()
            .to_string();
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
        // Canonicalize before creating missing subdirectory path
        let missing_output = fs::canonicalize(output_dir.path()).unwrap().join("missing");
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
        // Canonicalize encrypt directory to resolve symlinks
        let encrypt_dir_canonical = fs::canonicalize(encrypt_dir.path())
            .unwrap()
            .to_string_lossy()
            .to_string();
        let encrypted_path = encrypt_single_file(
            &Password::new("correct_password".to_string()),
            &input_path,
            &encrypt_dir_canonical,
            false,
        )
        .unwrap();
        let input_paths = vec![encrypted_path];
        let mut no_progress = |_progress: BatchProgress| {};
        // Canonicalize decrypt directory to resolve symlinks
        let decrypt_dir_canonical = fs::canonicalize(decrypt_dir.path())
            .unwrap()
            .to_string_lossy()
            .to_string();

        let result = batch_decrypt_impl(
            &input_paths,
            &decrypt_dir_canonical,
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
    fn test_batch_decrypt_auto_renames_on_collision() {
        let input_dir = tempdir().unwrap();
        let encrypt_dir = tempdir().unwrap();
        let decrypt_dir = tempdir().unwrap();
        let input_path = write_input_file(input_dir.path(), "sample.txt", b"alpha");
        let encrypt_dir_canonical = fs::canonicalize(encrypt_dir.path())
            .unwrap()
            .to_string_lossy()
            .to_string();
        let decrypt_dir_canonical = fs::canonicalize(decrypt_dir.path())
            .unwrap()
            .to_string_lossy()
            .to_string();

        let encrypted_path = encrypt_single_file(
            &Password::new("password123".to_string()),
            &input_path,
            &encrypt_dir_canonical,
            false,
        )
        .unwrap();

        let password = Password::new("password123".to_string());
        let first_output =
            decrypt_single_file(&password, &encrypted_path, &decrypt_dir_canonical, false).unwrap();
        let second_output =
            decrypt_single_file(&password, &encrypted_path, &decrypt_dir_canonical, false).unwrap();

        assert_ne!(first_output, second_output);
        assert!(Path::new(&first_output).exists());
        assert!(Path::new(&second_output).exists());
        assert!(second_output.ends_with("sample (1).txt"));
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
