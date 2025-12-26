// security/mod.rs - Platform-specific Security Utilities

#[cfg(windows)]
pub mod windows_acl;

#[cfg(windows)]
pub use windows_acl::set_owner_only_dacl;

// Provide a no-op stub for non-Windows platforms to simplify conditional compilation
#[cfg(not(windows))]
pub fn set_owner_only_dacl<P: AsRef<std::path::Path>>(_path: P) -> Result<(), u32> {
    Ok(())
}
