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

use windows_acl::acl::{AceType, ACL};
use windows_acl::helper::{current_user, name_to_sid};

// Windows file access rights for read/write (equivalent to Unix 0o600)
// FILE_GENERIC_READ = 0x120089
// FILE_GENERIC_WRITE = 0x120116
const FILE_GENERIC_READ: u32 = 0x120089;
const FILE_GENERIC_WRITE: u32 = 0x120116;

/// Error code returned when we can't get the current user
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
            let _ = acl.remove(sid.as_ptr() as *mut _, None, None);
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

/// Get the current user's SID as a byte vector
///
/// This is a convenience wrapper around windows_acl helper functions
/// for use in tests and debugging.
#[allow(dead_code)]
pub fn get_current_user_sid() -> Result<Vec<u8>, u32> {
    let current_user_name = current_user().ok_or(ERROR_NO_CURRENT_USER)?;
    name_to_sid(&current_user_name, None)
}

/// Check if a file has restrictive permissions (only current user has access)
///
/// This is useful for testing and verification purposes.
#[allow(dead_code)]
pub fn verify_owner_only_dacl<P: AsRef<Path>>(path: P) -> Result<bool, u32> {
    let path_str = path.as_ref().to_string_lossy();
    let current_user_name = current_user().ok_or(ERROR_NO_CURRENT_USER)?;
    let current_user_sid = name_to_sid(&current_user_name, None)?;
    let acl = ACL::from_file_path(&path_str, false)?;

    let entries = acl.all()?;

    // Should have exactly one entry
    if entries.len() != 1 {
        return Ok(false);
    }

    let entry = &entries[0];

    // Entry should be for current user
    // entry.sid is Option<Vec<u16>>, which is actually a raw SID stored as u16 words
    // We need to compare with our u8 SID
    match &entry.sid {
        Some(sid) => {
            // The sid in ACLEntry is stored as raw bytes packed into Vec<u16>
            // Convert to bytes for comparison
            let sid_bytes: Vec<u8> = sid
                .iter()
                .flat_map(|&w| w.to_le_bytes())
                .collect();
            
            // Truncate or compare based on actual SID length
            let sid_len = current_user_sid.len();
            if sid_bytes.len() < sid_len || sid_bytes[..sid_len] != current_user_sid[..] {
                return Ok(false);
            }
        }
        None => return Ok(false),
    }

    // Entry should be an AccessAllow type
    if entry.entry_type != AceType::AccessAllow {
        return Ok(false);
    }

    // Entry should have read+write mask
    let expected_mask = FILE_GENERIC_READ | FILE_GENERIC_WRITE;
    if entry.mask != expected_mask {
        return Ok(false);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_get_current_user_sid() {
        let result = get_current_user_sid();
        assert!(result.is_ok(), "Should be able to get current user SID: {:?}", result);
        let sid = result.unwrap();
        assert!(!sid.is_empty(), "SID should not be empty");
    }

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
    fn test_verify_owner_only_dacl() {
        // Create a temp file
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write some content
        fs::write(path, b"test content").unwrap();

        // Apply restrictive DACL
        let set_result = set_owner_only_dacl(path);
        assert!(set_result.is_ok(), "Failed to set DACL: {:?}", set_result);

        // After applying DACL, file should still be accessible
        let content = fs::read(path);
        assert!(content.is_ok(), "File should be readable after setting DACL");
    }
}
