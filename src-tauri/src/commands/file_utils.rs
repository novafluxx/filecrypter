// commands/file_utils.rs - Secure File Operation Utilities
//
// This module provides secure file operations including:
// - Writing files with restrictive permissions (0o600 on Unix)
// - Atomic file writes (write to temp, then rename)
// - Path validation (symlink detection, canonicalization)
// - File size validation

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{CryptoError, CryptoResult};

/// Maximum file size for in-memory operations (100 MB)
pub const MAX_IN_MEMORY_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum number of files in a batch operation
pub const MAX_BATCH_FILES: usize = 1000;

/// Write data to a file with secure permissions (owner read/write only)
///
/// On Unix systems, sets file permissions to 0o600.
/// On Windows, uses default permissions (ACLs inherited from parent).
#[cfg(unix)]
pub fn secure_write<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<(), std::io::Error> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600) // Owner read/write only
        .open(path)?;
    file.write_all(data)?;
    file.flush()?;
    Ok(())
}

#[cfg(windows)]
pub fn secure_write<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<(), std::io::Error> {
    // Windows uses ACLs which are inherited from parent directory
    // For enhanced security, consider using SetSecurityInfo API
    fs::write(path, data)
}

/// Write data atomically: write to temp file, then rename
///
/// This ensures that the output file is never partially written.
/// If the process crashes, only the temp file is left behind.
pub fn atomic_write<P: AsRef<Path>>(path: P, data: &[u8]) -> CryptoResult<()> {
    let path = path.as_ref();
    let temp_path = path.with_extension("tmp");

    // Write to temp file with secure permissions
    secure_write(&temp_path, data)?;

    // Atomically rename to final path
    fs::rename(&temp_path, path).map_err(|e| {
        // Try to clean up temp file on rename failure
        let _ = fs::remove_file(&temp_path);
        CryptoError::Io(e)
    })?;

    Ok(())
}

/// Validate a file path for security
///
/// Checks:
/// - Path exists (for input files)
/// - Path is not a symlink (prevents symlink attacks)
/// - Returns canonicalized path
pub fn validate_input_path(path: &str) -> CryptoResult<PathBuf> {
    let path = Path::new(path);

    // Check if path exists
    if !path.exists() {
        return Err(CryptoError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        )));
    }

    // Check if it's a symlink
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(CryptoError::InvalidPath(
            "Symlinks are not allowed for security reasons".to_string(),
        ));
    }

    // Canonicalize the path
    let canonical = fs::canonicalize(path)?;
    Ok(canonical)
}

/// Validate file size for in-memory operations
///
/// Returns an error if the file is too large to process in memory.
pub fn validate_file_size(path: &str) -> CryptoResult<u64> {
    let metadata = fs::metadata(path)?;
    let size = metadata.len();

    if size > MAX_IN_MEMORY_SIZE {
        return Err(CryptoError::FileTooLarge(format!(
            "File is {} MB, maximum for in-memory encryption is {} MB. Use streaming API for large files.",
            size / (1024 * 1024),
            MAX_IN_MEMORY_SIZE / (1024 * 1024)
        )));
    }

    Ok(size)
}

/// Validate batch file count
///
/// Returns an error if too many files are selected.
pub fn validate_batch_count(count: usize) -> CryptoResult<()> {
    if count > MAX_BATCH_FILES {
        return Err(CryptoError::TooManyFiles(format!(
            "Selected {} files, maximum is {}",
            count, MAX_BATCH_FILES
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_secure_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        secure_write(path, b"test data").unwrap();

        let content = fs::read(path).unwrap();
        assert_eq!(content, b"test data");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(path).unwrap();
            let mode = metadata.permissions().mode();
            // Check that only owner has read/write (0o600 = 384 in decimal)
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn test_atomic_write() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        atomic_write(path, b"atomic data").unwrap();

        let content = fs::read(path).unwrap();
        assert_eq!(content, b"atomic data");

        // Temp file should not exist
        let temp_path = path.with_extension("tmp");
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_validate_file_size() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        // Small file should pass
        fs::write(path, b"small content").unwrap();
        assert!(validate_file_size(path).is_ok());
    }

    #[test]
    fn test_validate_batch_count() {
        assert!(validate_batch_count(100).is_ok());
        assert!(validate_batch_count(1000).is_ok());
        assert!(validate_batch_count(1001).is_err());
    }
}
