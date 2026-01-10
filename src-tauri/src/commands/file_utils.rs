// commands/file_utils.rs - Secure File Operation Utilities
//
// This module provides secure file operations including:
// - Writing files with restrictive permissions (0o600 on Unix)
// - Atomic file writes (write to temp, then rename)
// - Path validation (symlink detection, canonicalization)
// - Output path resolution with collision handling
// - Batch operation validation
//
// Note: File size validation was removed as streaming handles all file sizes.
// The atomic_write() function is kept for testing and potential future use,
// though current operations use streaming's built-in atomic writes.

use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use tempfile::NamedTempFile;

use crate::error::{CryptoError, CryptoResult};

/// Maximum number of files in a batch operation
pub const MAX_BATCH_FILES: usize = 1000;

/// Maximum number of collision attempts when auto-renaming output files
const MAX_COLLISION_ATTEMPTS: u32 = 1000;

/// Resolve an output path based on overwrite preference.
///
/// If `allow_overwrite` is false and the target exists, this returns
/// a new path with a " (n)" suffix (e.g., "file (1).txt").
pub fn resolve_output_path<P: AsRef<Path>>(
    path: P,
    allow_overwrite: bool,
) -> CryptoResult<PathBuf> {
    let path = path.as_ref();

    if allow_overwrite || !path.exists() {
        return Ok(path.to_path_buf());
    }

    for index in 1..=MAX_COLLISION_ATTEMPTS {
        let candidate = build_collision_path(path, index)?;
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(CryptoError::InvalidPath(
        "Unable to find available output filename".to_string(),
    ))
}

fn build_collision_path(path: &Path, index: u32) -> CryptoResult<PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| CryptoError::InvalidPath("Output filename is missing".to_string()))?;

    let stem = path
        .file_stem()
        .unwrap_or(file_name)
        .to_string_lossy()
        .to_string();

    let candidate_name = if let Some(ext) = path.extension() {
        format!("{} ({}).{}", stem, index, ext.to_string_lossy())
    } else {
        format!("{} ({})", stem, index)
    };

    Ok(parent.join(candidate_name))
}

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
    use crate::security::create_secure_file;

    // Create file with restrictive permissions atomically (no TOCTOU vulnerability)
    let mut file = create_secure_file(&path)?;
    file.write_all(data)?;
    file.flush()?;
    Ok(())
}

/// Write data atomically: write to temp file, then rename
///
/// This function provides atomic file writes for in-memory data. It ensures
/// that the output file is never partially written - either the full file is
/// written successfully, or nothing is written.
///
/// # Current Usage
/// This function is currently used only in tests. Production code uses streaming
/// encryption which has its own built-in atomic write mechanism (via NamedTempFile
/// in crypto/streaming.rs). This function is kept as a utility for:
/// - Testing file operations
/// - Future utilities that need to write data atomically
/// - Reference implementation of secure atomic writes
///
/// # How It Works
/// 1. Creates a secure temporary file in the output directory
/// 2. Writes data to the temporary file with restrictive permissions (0o600)
/// 3. Atomically renames the temp file to the final output path
/// 4. If rename fails, cleans up temp file and returns error
///
/// # Arguments
/// * `path` - Target output path
/// * `data` - Data to write (entire content in memory)
/// * `allow_overwrite` - If false, auto-renames on collision
///
/// # Platform-Specific Security
/// - Unix: Sets file permissions to 0o600 (owner read/write only)
/// - Windows: Uses DACL to restrict access to file owner
///
/// When `allow_overwrite` is false, collisions are resolved by auto-renaming.
#[allow(dead_code)]
pub fn atomic_write<P: AsRef<Path>>(
    path: P,
    data: &[u8],
    allow_overwrite: bool,
) -> CryptoResult<PathBuf> {
    let requested_path = path.as_ref();
    let resolved_path = resolve_output_path(requested_path, allow_overwrite)?;
    let parent = resolved_path.parent().unwrap_or_else(|| Path::new("."));

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

    // On Windows, apply restrictive DACL to temp file BEFORE writing sensitive data
    // This minimizes the TOCTOU window - the file has secure permissions before data is written
    #[cfg(windows)]
    {
        use crate::security::set_owner_only_dacl;
        if let Err(err) = set_owner_only_dacl(temp_file.path()) {
            // Clean up temp file and fail
            let _ = fs::remove_file(temp_file.path());
            return Err(CryptoError::Io(err.into()));
        }
    }

    temp_file.write_all(data).map_err(CryptoError::Io)?;
    temp_file.flush().map_err(CryptoError::Io)?;

    if allow_overwrite && resolved_path.exists() {
        fs::remove_file(&resolved_path).map_err(CryptoError::Io)?;
    }

    match temp_file.persist(&resolved_path) {
        Ok(_) => Ok(resolved_path),
        Err(e) => {
            if !allow_overwrite && e.error.kind() == std::io::ErrorKind::AlreadyExists {
                let next_path = resolve_output_path(requested_path, false)?;
                let temp_file = e.file;
                temp_file
                    .persist(&next_path)
                    .map_err(|persist_err| CryptoError::Io(persist_err.error))?;
                return Ok(next_path);
            }

            let _ = fs::remove_file(e.file.path());
            Err(CryptoError::Io(e.error))
        }
    }
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

    // Reject non-regular files (directories, devices, FIFOs, etc.).
    // All files are processed via streaming encryption with chunking.
    let metadata = fs::metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(CryptoError::InvalidPath(
            "Input path must be a regular file".to_string(),
        ));
    }

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

    #[test]
    fn test_secure_write() {
        // Use tempdir to get a path without keeping a file handle open
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("secure_file.txt");

        secure_write(&path, b"test data").unwrap();

        let content = fs::read(&path).unwrap();
        assert_eq!(content, b"test data");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&path).unwrap();
            let mode = metadata.permissions().mode();
            // Check that only owner has read/write (0o600 = 384 in decimal)
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn test_atomic_write() {
        // Use a dedicated temp directory so we can assert that no temp artifacts remain.
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("output.bin");

        let written_path = atomic_write(&path, b"atomic data", false).unwrap();

        let content = fs::read(&written_path).unwrap();
        assert_eq!(content, b"atomic data");

        // Temp files should have been persisted/cleaned up; only the final file should remain.
        let mut files: Vec<String> = fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        assert_eq!(files, vec!["output.bin".to_string()]);
    }

    #[test]
    fn test_atomic_write_collision_renames() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("output.txt");

        atomic_write(&path, b"first", false).unwrap();
        let second_path = atomic_write(&path, b"second", false).unwrap();

        assert_ne!(path, second_path);
        assert!(second_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("output (1).txt"));
        assert_eq!(fs::read(second_path).unwrap(), b"second");
    }

    #[test]
    fn test_validate_batch_count() {
        assert!(validate_batch_count(100).is_ok());
        assert!(validate_batch_count(1000).is_ok());
        assert!(validate_batch_count(1001).is_err());
    }

    #[test]
    fn test_validate_input_path_rejects_directory() {
        let dir = tempfile::tempdir().unwrap();
        let result = validate_input_path(dir.path().to_str().unwrap());
        assert!(matches!(result, Err(CryptoError::InvalidPath(_))));
    }
}
