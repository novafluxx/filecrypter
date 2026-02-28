// security/mod.rs - Platform-specific Security Utilities
//
// This module centralizes file-permission hardening helpers used by the backend.
//
// Why this exists:
// - On Unix-like systems, "owner read/write only" can be expressed as mode 0o600.
// - On Windows, we need to apply a restrictive DACL (Access Control List) instead.
//
// Call sites generally want "make this file readable/writable by the current user only"
// without sprinkling `#[cfg(windows)]` throughout the codebase, so we provide:
// - Real implementations on Windows (`windows_acl`).
// - Small, safe stubs on non-Windows targets.

use std::fs;
use std::path::Path;

use tempfile::NamedTempFile;

use crate::error::{CryptoError, CryptoResult};

#[cfg(windows)]
pub mod windows_acl;

#[cfg(windows)]
pub use windows_acl::{create_secure_file, set_owner_only_dacl, DaclError};

// Provide stubs for non-Windows platforms to simplify conditional compilation
#[cfg(not(windows))]
/// No-op on non-Windows platforms.
///
/// On Windows this function applies a restrictive DACL to an existing file. On Unix-like
/// systems we rely on mode bits at file creation time (e.g. `OpenOptionsExt::mode(0o600)`),
/// so there's nothing additional to do here.
pub fn set_owner_only_dacl<P: AsRef<std::path::Path>>(_path: P) -> Result<(), std::io::Error> {
    Ok(())
}

#[cfg(not(windows))]
/// Create a file with restrictive permissions (owner read/write only).
///
/// This mirrors the Windows `create_secure_file` API but uses Unix mode bits when available.
pub fn create_secure_file<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<std::fs::File, std::io::Error> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

/// Create a temporary file with restrictive permissions (owner read/write only).
///
/// The file is created in the specified parent directory. On Unix, permissions are set to 0o600.
/// On Windows, a restrictive DACL is applied via `set_owner_only_dacl`.
pub fn create_secure_tempfile(parent: &Path) -> CryptoResult<NamedTempFile> {
    let temp_file = NamedTempFile::new_in(parent).map_err(CryptoError::Io)?;

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

    #[cfg(windows)]
    {
        if let Err(err) = set_owner_only_dacl(temp_file.path()) {
            let _ = fs::remove_file(temp_file.path());
            return Err(CryptoError::Io(err.into()));
        }
    }

    Ok(temp_file)
}
