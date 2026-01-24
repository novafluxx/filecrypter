// commands/command_utils.rs - Shared Utilities for Command Handlers
//
// This module provides common utilities used by encrypt, decrypt, and batch
// command handlers to reduce code duplication.

use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

use crate::commands::file_utils::{resolve_output_path, validate_batch_count, validate_input_path};
use crate::commands::CryptoResponse;
use crate::crypto::Password;
use crate::error::{CryptoError, CryptoResult};
use crate::events::{ProgressEvent, CRYPTO_PROGRESS_EVENT};

/// Validated paths and password for crypto operations
pub struct ValidatedCryptoInputs {
    pub input: PathBuf,
    pub output: PathBuf,
    pub password: Password,
}

/// Validate input/output paths and wrap password for crypto operations
///
/// This function performs common validation steps:
/// 1. Emit reading and deriving_key progress events
/// 2. Validate and canonicalize input path (check symlinks, existence)
/// 3. Resolve output path (handle collisions if !allow_overwrite)
/// 4. Wrap password in secure Password type
pub fn validate_crypto_inputs(
    app: &AppHandle,
    input_path: &str,
    output_path: &str,
    password: String,
    allow_overwrite: bool,
) -> CryptoResult<ValidatedCryptoInputs> {
    // Emit progress events
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::reading());
    let _ = app.emit(CRYPTO_PROGRESS_EVENT, ProgressEvent::deriving_key());

    // Validate input path (check for symlinks, canonicalize)
    let validated_input = validate_input_path(input_path)?;
    let validated_output = resolve_output_path(output_path, allow_overwrite)?;
    let password = Password::new(password);

    Ok(ValidatedCryptoInputs {
        input: validated_input,
        output: validated_output,
        password,
    })
}

/// Create a progress callback for streaming operations
///
/// Returns a boxed callback that calculates percentage and emits progress events.
/// The percentage is capped at 99 to leave room for the completion event.
pub fn create_progress_callback(
    app: AppHandle,
    stage: &'static str,
    message: &'static str,
) -> Box<dyn Fn(u64, u64) + Send + Sync> {
    Box::new(move |bytes_processed: u64, total_bytes: u64| {
        let percent = if total_bytes > 0 {
            ((bytes_processed as f64 / total_bytes as f64) * 100.0) as u32
        } else {
            0
        }
        .min(99);

        let _ = app.emit(
            CRYPTO_PROGRESS_EVENT,
            ProgressEvent::new(stage, percent, message),
        );
    })
}

/// Format a success response for crypto operations
pub fn format_success_response(output_path: &Path, operation: &str) -> CryptoResponse {
    let output_path_str = output_path.to_string_lossy().to_string();
    CryptoResponse {
        message: format!("File {} successfully: {}", operation, output_path_str),
        output_path: output_path_str,
    }
}

/// Validate common inputs for batch operations
///
/// Checks:
/// - Password is not empty
/// - Input paths list is not empty
/// - File count is within limits (MAX_BATCH_FILES)
/// - Output directory exists
pub fn validate_batch_inputs(
    password: &str,
    input_paths: &[String],
    output_dir: &str,
) -> CryptoResult<()> {
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_batch_inputs_empty_password() {
        let temp_dir = std::env::temp_dir();
        let temp_dir_str = temp_dir.to_string_lossy();
        let result = validate_batch_inputs("", &["file.txt".to_string()], &temp_dir_str);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Password"));
    }

    #[test]
    fn test_validate_batch_inputs_empty_files() {
        let temp_dir = std::env::temp_dir();
        let temp_dir_str = temp_dir.to_string_lossy();
        let result = validate_batch_inputs("password", &[], &temp_dir_str);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No files"));
    }

    #[test]
    fn test_validate_batch_inputs_invalid_output_dir() {
        let result = validate_batch_inputs(
            "password",
            &["file.txt".to_string()],
            "/nonexistent/directory",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("directory"));
    }

    #[test]
    fn test_format_success_response() {
        let path = Path::new("/tmp/test.encrypted");
        let response = format_success_response(path, "encrypted");
        assert!(response.message.contains("encrypted"));
        assert!(response.output_path.contains("test.encrypted"));
    }
}
