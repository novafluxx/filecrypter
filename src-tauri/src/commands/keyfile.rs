// commands/keyfile.rs - Key File Generation Command
//
// This module provides the Tauri command for generating key files.
// Key files are used as a second authentication factor for file encryption.

use std::path::Path;

use tauri::command;

use crate::commands::file_utils::validate_no_symlinks;
use crate::commands::CryptoResponse;
use crate::error::{CryptoError, CryptoResult};

/// Validate that the output path's parent directory exists, is a directory,
/// and contains no symlinks.
fn validate_output_path(path: &Path) -> CryptoResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| CryptoError::InvalidPath("Output path has no parent directory".into()))?;

    if !parent.exists() {
        return Err(CryptoError::InvalidPath(
            "Parent directory does not exist".into(),
        ));
    }

    if !parent.is_dir() {
        return Err(CryptoError::InvalidPath(
            "Parent path is not a directory".into(),
        ));
    }

    validate_no_symlinks(parent)?;

    Ok(())
}

/// Generate a key file containing 32 cryptographically random bytes.
///
/// # Arguments
/// * `output_path` - Path where the key file will be saved
///
/// # Returns
/// A success response with the output path
#[command]
pub async fn generate_key_file(output_path: String) -> CryptoResult<CryptoResponse> {
    log::info!("Generating key file: {}", output_path);

    let path = Path::new(&output_path);

    validate_output_path(path)?;

    crate::crypto::keyfile::generate_key_file(path)?;

    Ok(CryptoResponse {
        message: format!("Key file generated successfully: {}", output_path),
        output_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_output_path_valid() {
        let dir = tempfile::tempdir().unwrap();
        // Canonicalize to resolve platform symlinks (e.g., /tmp -> /private/tmp on macOS)
        let canonical_dir = dir.path().canonicalize().unwrap();
        let path = canonical_dir.join("keyfile.bin");
        assert!(validate_output_path(&path).is_ok());
    }

    #[test]
    fn test_validate_output_path_missing_parent() {
        let dir = tempfile::tempdir().unwrap();
        let canonical_dir = dir.path().canonicalize().unwrap();
        let path = canonical_dir.join("nonexistent").join("keyfile.bin");
        let err = validate_output_path(&path).unwrap_err();
        assert!(matches!(err, CryptoError::InvalidPath(_)));
    }

    #[test]
    fn test_validate_output_path_parent_is_file() {
        let dir = tempfile::tempdir().unwrap();
        let canonical_dir = dir.path().canonicalize().unwrap();
        let file_path = canonical_dir.join("not_a_dir");
        std::fs::write(&file_path, b"data").unwrap();

        let path = file_path.join("keyfile.bin");
        let err = validate_output_path(&path).unwrap_err();
        assert!(matches!(err, CryptoError::InvalidPath(_)));
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_output_path_rejects_symlink_parent() {
        let dir = tempfile::tempdir().unwrap();
        let real_dir = dir.path().join("real");
        std::fs::create_dir(&real_dir).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real_dir, &link).unwrap();

        let path = link.join("keyfile.bin");
        let err = validate_output_path(&path).unwrap_err();
        assert!(matches!(err, CryptoError::InvalidPath(_)));
    }
}
