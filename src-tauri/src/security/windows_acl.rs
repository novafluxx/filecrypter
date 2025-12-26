// security/windows_acl.rs - Windows DACL Implementation
//
// This module provides Windows-specific file permission functions
// to mirror Unix 0o600 (owner read/write only) permissions.
//
// On Windows, this is achieved by:
// 1. Getting the current user's SID
// 2. Creating a DACL with only the current user having FILE_GENERIC_READ | FILE_GENERIC_WRITE
// 3. Removing inherited ACEs to prevent other users from accessing the file

use std::path::Path;

use windows_acl::acl::ACL;
use windows_acl::helper::{current_user, name_to_sid};

// Windows file access rights for read/write (equivalent to Unix 0o600)
// FILE_GENERIC_READ = 0x120089
// FILE_GENERIC_WRITE = 0x120116
const FILE_GENERIC_READ: u32 = 0x120089;
const FILE_GENERIC_WRITE: u32 = 0x120116;

/// Custom error code returned when we can't get the current user.
/// Uses u32::MAX to avoid collision with actual Windows error codes
/// (which are typically in the lower ranges, e.g., ERROR_ACCESS_DENIED = 5).
const ERROR_NO_CURRENT_USER: u32 = 0xFFFF_FFFF;

/// Set restrictive DACL on a file: current user read/write only.
///
/// This mirrors Unix 0o600 permissions by:
/// 1. Removing all existing ACEs (including inherited ones)
/// 2. Adding an explicit allow ACE for the current user with read/write access
///
/// # Arguments
/// * `path` - Path to the file to secure
///
/// # Returns
/// Ok(()) on success, Err with Windows error code on failure
///
/// # Security Note
/// This function should be called immediately after file creation
/// to minimize the window where default ACLs are in effect.
pub fn set_owner_only_dacl<P: AsRef<Path>>(path: P) -> Result<(), u32> {
    let path_str = path.as_ref().to_string_lossy();

    // Get current user's name (e.g., "username" or "DOMAIN\\username")
    let current_user_name = current_user().ok_or(ERROR_NO_CURRENT_USER)?;

    // Convert username to SID bytes using name_to_sid
    // The second parameter is the system/domain scope - None means local
    let current_user_sid = name_to_sid(&current_user_name, None)?;

    // Get the file's ACL (false = don't get SACL, only DACL)
    let mut acl = ACL::from_file_path(&path_str, false)?;

    // Get all existing entries and remove them (clears inherited ACEs)
    let entries = acl.all()?;
    for entry in entries {
        // Remove all entries for this SID (both Allow and Deny types)
        // entry.sid is Option<Vec<u16>>, so we need to handle it
        if let Some(ref sid) = entry.sid {
            // Propagate ACE removal errors - leaving old permissions is a security risk
            acl.remove(sid.as_ptr() as *mut _, None, None)?;
        }
    }

    // Add allow entry for current user: read + write
    let access_mask = FILE_GENERIC_READ | FILE_GENERIC_WRITE;
    acl.allow(
        current_user_sid.as_ptr() as *mut _,
        false, // Not inheritable (files don't have children)
        access_mask,
    )?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_set_owner_only_dacl() {
        // Create a temp file
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write some content
        fs::write(path, b"test content").unwrap();

        // Apply restrictive DACL
        let result = set_owner_only_dacl(path);
        assert!(result.is_ok(), "Failed to set DACL: {:?}", result);

        // Verify file is still readable by current user
        let content = fs::read(path);
        assert!(content.is_ok(), "File should still be readable by owner");
        assert_eq!(content.unwrap(), b"test content");
    }

    #[test]
    fn test_set_owner_only_dacl_file_remains_writable() {
        // Create a temp file
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write initial content
        fs::write(path, b"initial").unwrap();

        // Apply restrictive DACL
        let result = set_owner_only_dacl(path);
        assert!(result.is_ok(), "Failed to set DACL: {:?}", result);

        // Verify file is still writable by current user
        let write_result = fs::write(path, b"updated content");
        assert!(write_result.is_ok(), "File should still be writable by owner");

        // Verify the update was successful
        let content = fs::read(path).unwrap();
        assert_eq!(content, b"updated content");
    }
}
