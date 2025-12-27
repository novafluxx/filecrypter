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
pub fn set_owner_only_dacl<P: AsRef<std::path::Path>>(_path: P) -> Result<(), u32> {
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
