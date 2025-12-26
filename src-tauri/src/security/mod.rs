// security/mod.rs - Platform-specific Security Utilities

#[cfg(windows)]
pub mod windows_acl;

#[cfg(windows)]
pub use windows_acl::{create_secure_file, set_owner_only_dacl, DaclError};

// Provide stubs for non-Windows platforms to simplify conditional compilation
#[cfg(not(windows))]
pub fn set_owner_only_dacl<P: AsRef<std::path::Path>>(_path: P) -> Result<(), u32> {
    Ok(())
}

#[cfg(not(windows))]
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
