// error.rs - Custom error types for FileCrypter
//
// This module defines all error types used throughout the application.
// Using thiserror makes error definitions clean and implements std::error::Error automatically.
// All errors are serializable so they can be sent to the frontend via Tauri IPC.

use thiserror::Error;

/// Main error type for all cryptographic operations
#[derive(Error, Debug)]
pub enum CryptoError {
    /// Password validation failed (wrong password during decryption)
    #[error("Invalid password or corrupted file")]
    InvalidPassword,

    /// File format is invalid or corrupted
    #[error("Invalid file format: {0}")]
    FormatError(String),

    /// Encryption operation failed
    #[error("Encryption failed")]
    EncryptionFailed,

    /// Decryption operation failed
    #[error("Decryption failed")]
    DecryptionFailed,

    /// File version is not supported
    #[error("Unsupported file version")]
    InvalidVersion,

    /// I/O error (file not found, permission denied, etc.)
    #[error("File error: {0}")]
    Io(#[from] std::io::Error),

    /// File too large for processing (exceeds maximum chunk count)
    #[error("FileTooLarge: {0}")]
    FileTooLarge(String),

    /// Too many files in batch operation
    #[error("TooManyFiles: {0}")]
    TooManyFiles(String),

    /// Invalid file path (symlinks, etc.)
    #[error("InvalidPath: {0}")]
    InvalidPath(String),

    /// Archive operation error
    #[error("ArchiveError: {0}")]
    ArchiveError(String),

    /// Path traversal attempt detected
    #[error("PathTraversal: {0}")]
    PathTraversal(String),

    /// File was encrypted with a key file but none was provided
    #[error("This file was encrypted with a key file. Please provide the key file to decrypt.")]
    KeyFileRequired,

    /// Key file operation error
    #[error("Key file error: {0}")]
    KeyFileError(String),
}

/// Result type alias for crypto operations
pub type CryptoResult<T> = Result<T, CryptoError>;

#[cfg(windows)]
impl From<crate::security::DaclError> for CryptoError {
    fn from(err: crate::security::DaclError) -> Self {
        CryptoError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            err.to_string(),
        ))
    }
}

// Implement Serialize for CryptoError so it can be sent to the frontend
// Tauri requires all command return types to be serializable
impl serde::Serialize for CryptoError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize the error as a string message
        // This ensures users see friendly error messages in the UI
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_serialization() {
        let error = CryptoError::InvalidPassword;
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Invalid password"));
    }

    #[test]
    fn test_format_error() {
        let error = CryptoError::FormatError("test".to_string());
        assert_eq!(error.to_string(), "Invalid file format: test");
    }
}
