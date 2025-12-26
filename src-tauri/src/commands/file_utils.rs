// commands/file_utils.rs - Secure File Operation Utilities
//
// This module provides secure file operations including:
// - Writing files with restrictive permissions (0o600 on Unix)
// - Atomic file writes (write to temp, then rename)
// - Path validation (symlink detection, canonicalization)
// - File size validation

use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use tempfile::NamedTempFile;

use crate::error::{CryptoError, CryptoResult};

/// Maximum file size for in-memory operations (100 MB)
pub const MAX_IN_MEMORY_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum number of files in a batch operation
pub const MAX_BATCH_FILES: usize = 1000;

/// Write data to a file with secure permissions (owner read/write only)
///
/// On Unix systems, sets file permissions to 0o600.
/// On Windows, uses default permissions (ACLs inherited from parent).
///
/// Note: Prefer `atomic_write()` for better safety (prevents partial writes).
/// This function is kept for cases where atomic writes aren't feasible.
#[allow(dead_code)]
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

#[allow(dead_code)]
#[cfg(windows)]
pub fn secure_write<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<(), std::io::Error> {
    use crate::security::set_owner_only_dacl;

    // Write the file first
    fs::write(&path, data)?;

    // Then apply restrictive DACL (current user read/write only)
    // Log warning on failure but don't fail the operation
    if let Err(code) = set_owner_only_dacl(&path) {
        log::warn!(
            "Failed to set restrictive DACL on {:?}: Windows error code {}",
            path.as_ref(),
            code
        );
    }

    Ok(())
}

/// Write data atomically: write to temp file, then rename
///
/// This ensures that the output file is never partially written.
/// If the process crashes, only the temp file is left behind.
pub fn atomic_write<P: AsRef<Path>>(path: P, data: &[u8]) -> CryptoResult<()> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    let mut temp_file = NamedTempFile::new_in(parent).map_err(CryptoError::Io)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = temp_file
            .as_file()
            .metadata()
            .map_err(CryptoError::Io)?
            .permissions();
        perms.set_mode(0o600);
        fs::set_permissions(temp_file.path(), perms).map_err(CryptoError::Io)?;
    }

    temp_file.write_all(data).map_err(CryptoError::Io)?;
    temp_file.flush().map_err(CryptoError::Io)?;

    if let Err(e) = temp_file.persist(path) {
        let _ = fs::remove_file(e.file.path());
        return Err(CryptoError::Io(e.error));
    }

    // On Windows, apply restrictive DACL after persist
    #[cfg(windows)]
    {
        use crate::security::set_owner_only_dacl;
        if let Err(code) = set_owner_only_dacl(path) {
            log::warn!(
                "Failed to set restrictive DACL on {:?}: Windows error code {}",
                path,
                code
            );
        }
    }

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

    // Check for symlinks in any path component
    validate_no_symlinks(path)?;

    // Canonicalize the path
    let canonical = fs::canonicalize(path)?;
    Ok(canonical)
}

fn validate_no_symlinks(path: &Path) -> CryptoResult<()> {
    let mut current = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().map_err(CryptoError::Io)?
    };

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                current.push(prefix.as_os_str());
            }
            Component::RootDir => {
                current.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                current.pop();
            }
            Component::Normal(_) => {
                current.push(component.as_os_str());
                let metadata = fs::symlink_metadata(&current)?;
                if metadata.file_type().is_symlink() {
                    return Err(CryptoError::InvalidPath(
                        "Symlinks are not allowed for security reasons".to_string(),
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Validate file size for in-memory operations
///
/// Returns an error if the file is too large to process in memory.
pub fn validate_file_size<P: AsRef<Path>>(path: P) -> CryptoResult<u64> {
    let metadata = fs::metadata(path.as_ref())?;
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
        // Create a temp file and immediately close the handle
        // On Windows, the file must be closed before we can overwrite it atomically
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        drop(temp_file); // Close the file handle

        atomic_write(&path, b"atomic data").unwrap();

        let content = fs::read(&path).unwrap();
        assert_eq!(content, b"atomic data");

        // Temp file should not exist
        let temp_path = path.with_extension("tmp");
        assert!(!temp_path.exists());

        // Clean up
        let _ = fs::remove_file(&path);
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
