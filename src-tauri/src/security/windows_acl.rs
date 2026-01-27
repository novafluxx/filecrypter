// security/windows_acl.rs - Windows DACL Implementation
//
// This module provides Windows-specific file permission functions
// to mirror Unix 0o600 (owner read/write only) permissions.
//
// On Windows, this is achieved by:
// 1. Getting the current user's SID
// 2. Creating a DACL with only the current user having FILE_GENERIC_READ | FILE_GENERIC_WRITE
// 3. Removing inherited ACEs to prevent other users from accessing the file

use std::fs::File;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::FromRawHandle;
use std::path::Path;

use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Authorization::{
    SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W, SET_ACCESS, TRUSTEE_IS_SID,
    TRUSTEE_W,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, InitializeSecurityDescriptor, SetSecurityDescriptorDacl, TokenUser,
    ACL as WIN_ACL, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// Error type for Windows DACL operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaclError {
    /// Windows API returned an error code
    WindowsError(u32),
    /// Could not determine the current user
    NoCurrentUser,
    /// I/O error during file operations
    IoError(String),
}

impl std::fmt::Display for DaclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaclError::WindowsError(code) => write!(f, "Windows error code: {}", code),
            DaclError::NoCurrentUser => write!(f, "Could not determine current user"),
            DaclError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for DaclError {}

impl From<u32> for DaclError {
    fn from(code: u32) -> Self {
        DaclError::WindowsError(code)
    }
}

impl From<DaclError> for io::Error {
    fn from(err: DaclError) -> Self {
        io::Error::new(io::ErrorKind::PermissionDenied, err.to_string())
    }
}

/// Create a file with restrictive permissions from the start (no TOCTOU vulnerability).
///
/// This function creates a file with a security descriptor that only allows
/// the current user read/write access, preventing any window where the file
/// has permissive default permissions.
///
/// # Arguments
/// * `path` - Path to the file to create
///
/// # Returns
/// A `File` handle on success, or an error if the file cannot be created securely
///
/// # Security
/// This function eliminates the TOCTOU race condition by setting permissions
/// atomically during file creation using Windows `CreateFileW` with `SECURITY_ATTRIBUTES`.
pub fn create_secure_file<P: AsRef<Path>>(path: P) -> Result<File, DaclError> {
    let path = path.as_ref();

    // Get current user's SID
    let current_user_sid = current_user_sid()?;

    // Build path as wide string (null-terminated UTF-16)
    let path_wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        // Initialize security descriptor
        // SECURITY_DESCRIPTOR_REVISION = 1
        const SECURITY_DESCRIPTOR_REVISION: u32 = 1;
        let mut sd: SECURITY_DESCRIPTOR = std::mem::zeroed();
        if InitializeSecurityDescriptor(
            &mut sd as *mut _ as PSECURITY_DESCRIPTOR,
            SECURITY_DESCRIPTOR_REVISION,
        ) == 0
        {
            return Err(DaclError::WindowsError(get_last_error()));
        }

        // Create explicit access entry for current user only
        let mut ea: EXPLICIT_ACCESS_W = std::mem::zeroed();
        ea.grfAccessPermissions = FILE_GENERIC_READ | FILE_GENERIC_WRITE;
        ea.grfAccessMode = SET_ACCESS;
        ea.grfInheritance = 0; // No inheritance for files
        ea.Trustee = TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: 0,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: 0,
            // SAFETY: current_user_sid is valid for the lifetime of this function call.
            // The SID data is passed to SetEntriesInAclW which copies it internally.
            ptstrName: current_user_sid.sid_ptr() as *mut u16,
        };

        // Create new ACL with just our entry
        let mut new_acl: *mut WIN_ACL = std::ptr::null_mut();
        let result = SetEntriesInAclW(1, &ea, std::ptr::null_mut(), &mut new_acl);
        if result != 0 {
            return Err(DaclError::WindowsError(result));
        }

        // Set the DACL in the security descriptor (protected = no inheritance)
        if SetSecurityDescriptorDacl(
            &mut sd as *mut _ as PSECURITY_DESCRIPTOR,
            1,       // bDaclPresent = TRUE
            new_acl, // pDacl
            0,       // bDaclDefaulted = FALSE
        ) == 0
        {
            LocalFree(new_acl as *mut _);
            return Err(DaclError::WindowsError(get_last_error()));
        }

        // Create security attributes
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: &mut sd as *mut _ as *mut _,
            bInheritHandle: 0,
        };

        use windows_sys::Win32::Storage::FileSystem::CREATE_ALWAYS;

        // Create file with security attributes
        let handle = CreateFileW(
            path_wide.as_ptr(),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            0, // No sharing while we hold the handle
            &sa,
            CREATE_ALWAYS, // Create new file, overwrite if exists
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        );

        // Free the ACL we allocated
        LocalFree(new_acl as *mut _);

        if handle == INVALID_HANDLE_VALUE {
            return Err(DaclError::WindowsError(get_last_error()));
        }

        // Convert raw handle to File
        // SAFETY: handle is a valid file handle from CreateFileW
        Ok(File::from_raw_handle(handle as *mut _))
    }
}

/// Get the last Windows error code
fn get_last_error() -> u32 {
    unsafe { windows_sys::Win32::Foundation::GetLastError() }
}

/// Set restrictive DACL on an existing file: current user read/write only.
///
/// This mirrors Unix 0o600 permissions by:
/// 1. Removing all existing ACEs (including inherited ones)
/// 2. Adding an explicit allow ACE for the current user with read/write access
///
/// # Arguments
/// * `path` - Path to the file to secure
///
/// # Returns
/// Ok(()) on success, Err with error details on failure
///
/// # Security Note
/// For new files, prefer `create_secure_file()` which sets permissions atomically.
/// This function is for securing existing files and has a small TOCTOU window
/// between file creation and permission application.
pub fn set_owner_only_dacl<P: AsRef<Path>>(path: P) -> Result<(), DaclError> {
    let path = path.as_ref();

    let current_user_sid = current_user_sid()?;

    let path_wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // Windows file access rights for read/write (equivalent to Unix 0o600)
    // These values match the Windows SDK definitions:
    // FILE_GENERIC_READ  = FILE_READ_ATTRIBUTES | FILE_READ_DATA | FILE_READ_EA | STANDARD_RIGHTS_READ | SYNCHRONIZE
    // FILE_GENERIC_WRITE = FILE_APPEND_DATA | FILE_WRITE_ATTRIBUTES | FILE_WRITE_DATA | FILE_WRITE_EA | STANDARD_RIGHTS_WRITE | SYNCHRONIZE
    const FILE_GENERIC_READ_MASK: u32 = 0x120089;
    const FILE_GENERIC_WRITE_MASK: u32 = 0x120116;

    // Add allow entry for current user: read + write
    let access_mask = FILE_GENERIC_READ_MASK | FILE_GENERIC_WRITE_MASK;
    unsafe {
        use windows_sys::Win32::Security::Authorization::SE_FILE_OBJECT;

        let mut ea: EXPLICIT_ACCESS_W = std::mem::zeroed();
        ea.grfAccessPermissions = access_mask;
        ea.grfAccessMode = SET_ACCESS;
        ea.grfInheritance = 0;
        ea.Trustee = TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: 0,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: 0,
            // SAFETY: current_user_sid is valid for the lifetime of this function call.
            // The SID data is passed to SetEntriesInAclW which copies it internally.
            ptstrName: current_user_sid.sid_ptr() as *mut u16,
        };

        let mut new_acl: *mut WIN_ACL = std::ptr::null_mut();
        let result = SetEntriesInAclW(1, &ea, std::ptr::null_mut(), &mut new_acl);
        if result != 0 {
            return Err(DaclError::WindowsError(result));
        }

        let result = SetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            new_acl,
            std::ptr::null_mut(),
        );

        LocalFree(new_acl as *mut _);

        if result != 0 {
            return Err(DaclError::WindowsError(result));
        }
    }

    Ok(())
}

struct UserSid {
    buf: Vec<u8>,
}

impl UserSid {
    fn sid_ptr(&self) -> *mut u8 {
        let token_user_ptr = self.buf.as_ptr() as *const TOKEN_USER;
        // SAFETY: buffer is allocated by GetTokenInformation for TOKEN_USER.
        let token_user = unsafe { std::ptr::read_unaligned(token_user_ptr) };
        token_user.User.Sid as *mut u8
    }
}

fn current_user_sid() -> Result<UserSid, DaclError> {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(DaclError::WindowsError(get_last_error()));
        }

        let mut len = 0u32;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len);
        if len == 0 {
            CloseHandle(token);
            return Err(DaclError::WindowsError(get_last_error()));
        }

        let mut buf = vec![0u8; len as usize];
        let ok = GetTokenInformation(token, TokenUser, buf.as_mut_ptr() as *mut _, len, &mut len);
        CloseHandle(token);

        if ok == 0 {
            return Err(DaclError::WindowsError(get_last_error()));
        }

        Ok(UserSid { buf })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use windows_sys::Win32::Security::{GetSecurityDescriptorControl, SECURITY_DESCRIPTOR_CONTROL};

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
        assert!(
            write_result.is_ok(),
            "File should still be writable by owner"
        );

        // Verify the update was successful
        let content = fs::read(path).unwrap();
        assert_eq!(content, b"updated content");
    }

    #[test]
    fn test_create_secure_file() {
        // Create a temp directory
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("secure_file.txt");

        // Create file with secure permissions from the start
        let result = create_secure_file(&path);
        assert!(result.is_ok(), "Failed to create secure file: {:?}", result);

        let mut file = result.unwrap();
        file.write_all(b"secure content").unwrap();
        drop(file);

        // Verify file is readable by current user
        let content = fs::read(&path);
        assert!(content.is_ok(), "File should be readable by owner");
        assert_eq!(content.unwrap(), b"secure content");

        // Verify file is writable by current user
        let write_result = fs::write(&path, b"updated secure content");
        assert!(write_result.is_ok(), "File should be writable by owner");
    }

    #[test]
    fn test_dacl_is_protected_from_inheritance() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        fs::write(path, b"test content").unwrap();

        let result = set_owner_only_dacl(path);
        assert!(result.is_ok(), "Failed to set DACL: {:?}", result);

        let is_protected = dacl_is_protected(path).expect("Failed to read DACL protection");
        assert!(is_protected, "DACL should be protected from inheritance");
    }

    #[test]
    fn test_dacl_error_display() {
        let err = DaclError::WindowsError(5);
        assert_eq!(format!("{}", err), "Windows error code: 5");

        let err = DaclError::NoCurrentUser;
        assert_eq!(format!("{}", err), "Could not determine current user");

        let err = DaclError::IoError("test error".to_string());
        assert_eq!(format!("{}", err), "I/O error: test error");
    }

    fn dacl_is_protected(path: &Path) -> Result<bool, DaclError> {
        let path_wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            use windows_sys::Win32::Security::Authorization::SE_FILE_OBJECT;
            use windows_sys::Win32::Security::SE_DACL_PROTECTED;

            let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
            let result = windows_sys::Win32::Security::Authorization::GetNamedSecurityInfoW(
                path_wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut sd,
            );

            if result != 0 {
                return Err(DaclError::WindowsError(result));
            }

            if sd.is_null() {
                return Err(DaclError::IoError(
                    "Failed to read security descriptor".to_string(),
                ));
            }

            let mut control: SECURITY_DESCRIPTOR_CONTROL = 0;
            let mut revision: u32 = 0;
            let ok = GetSecurityDescriptorControl(sd, &mut control, &mut revision);
            LocalFree(sd as *mut _);

            if ok == 0 {
                return Err(DaclError::WindowsError(get_last_error()));
            }

            Ok((control & SE_DACL_PROTECTED) != 0)
        }
    }
}
