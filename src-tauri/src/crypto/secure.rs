// crypto/secure.rs - Memory-safe wrappers for sensitive data
//
// This module provides secure wrappers for passwords and encryption keys.
// The Zeroize trait ensures that sensitive data is securely cleared from memory
// when it's no longer needed, preventing potential memory dump attacks.
//
// Key Security Features:
// - Automatic memory zeroing on drop (RAII pattern)
// - Prevents accidental logging or display of sensitive data
// - Type safety ensures passwords/keys are handled correctly

use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secure wrapper for byte arrays (encryption keys, salts, nonces)
///
/// This wrapper automatically zeros the contained bytes when dropped,
/// preventing sensitive cryptographic material from lingering in memory.
///
/// # Example
/// ```no_run
/// use filecrypter_lib::crypto::SecureBytes;
/// let _key = SecureBytes::new(vec![1, 2, 3, 4]);
/// // key.0 is automatically zeroed when key goes out of scope
/// ```
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecureBytes(Vec<u8>);

impl SecureBytes {
    /// Create a new SecureBytes instance from a Vec<u8>
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    /// Create SecureBytes from a slice (copies the data)
    #[allow(dead_code)]
    pub fn from_slice(data: &[u8]) -> Self {
        Self(data.to_vec())
    }

    /// Get a reference to the inner bytes
    ///
    /// Use this carefully - avoid storing references that outlive the SecureBytes
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Get the length of the byte array
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the byte array is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// Implement Debug to prevent accidental logging of sensitive data
impl std::fmt::Debug for SecureBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecureBytes([REDACTED {} bytes])", self.len())
    }
}

/// Secure wrapper for passwords
///
/// This wrapper ensures passwords are zeroized from memory after use.
/// It provides restricted access to prevent accidental exposure.
///
/// # Security Notes
/// - Passwords are never logged or displayed
/// - Memory is automatically zeroed on drop
/// - Clone is intentionally not implemented
///
/// # Example
/// ```no_run
/// use filecrypter_lib::crypto::Password;
/// let password = Password::new("my_secret".to_string());
/// let _bytes = password.as_bytes(); // Access for cryptographic operations
/// // password is automatically zeroed when it goes out of scope
/// ```
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Password(String);

impl Password {
    /// Create a new Password instance
    ///
    /// The provided String will be moved into the Password wrapper
    /// and will be zeroed when the Password is dropped.
    pub fn new(password: String) -> Self {
        Self(password)
    }

    /// Get the password as bytes for cryptographic operations
    ///
    /// This is the primary way to access the password data for
    /// use with key derivation functions or other crypto operations.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Get the password as a string slice
    ///
    /// Use sparingly - prefer `as_bytes()` for crypto operations
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the length of the password in bytes
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the password is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// Implement Debug to prevent accidental logging of passwords
impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Password([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_bytes_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let secure = SecureBytes::new(data.clone());

        assert_eq!(secure.as_slice(), &data);
        assert_eq!(secure.len(), 5);
        assert!(!secure.is_empty());
    }

    #[test]
    fn test_secure_bytes_from_slice() {
        let data = [1, 2, 3];
        let secure = SecureBytes::from_slice(&data);

        assert_eq!(secure.as_slice(), &data);
    }

    #[test]
    fn test_secure_bytes_debug() {
        let secure = SecureBytes::new(vec![1, 2, 3]);
        let debug_output = format!("{:?}", secure);

        // Should not contain actual data
        assert!(debug_output.contains("REDACTED"));
        assert!(debug_output.contains("3 bytes"));
    }

    #[test]
    fn test_password_creation() {
        let password = Password::new("test_password".to_string());

        assert_eq!(password.as_str(), "test_password");
        assert_eq!(password.as_bytes(), b"test_password");
        assert_eq!(password.len(), 13);
        assert!(!password.is_empty());
    }

    #[test]
    fn test_password_debug() {
        let password = Password::new("secret123".to_string());
        let debug_output = format!("{:?}", password);

        // Should not contain actual password
        assert!(debug_output.contains("REDACTED"));
        assert!(!debug_output.contains("secret123"));
    }

    #[test]
    fn test_empty_password() {
        let password = Password::new(String::new());

        assert!(password.is_empty());
        assert_eq!(password.len(), 0);
    }

    // Note: Testing that memory is actually zeroed on drop is difficult
    // and platform-specific. The zeroize crate is well-tested for this.
    // In production, you'd use tools like Valgrind or memory analyzers
    // to verify secure erasure.
}
