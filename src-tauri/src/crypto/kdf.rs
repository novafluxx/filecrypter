// crypto/kdf.rs - Argon2id Key Derivation Function
//
// This module implements password-based key derivation using Argon2id.
// Argon2id is the recommended algorithm for password hashing and key derivation
// as it's resistant to both side-channel and GPU-based attacks.
//
// Security Parameters (OWASP Recommendations):
// - Algorithm: Argon2id (hybrid of Argon2i and Argon2d)
// - Memory Cost: 64 MiB (65536 KiB) - requires 64MB RAM per operation
// - Time Cost: 3 iterations - balances security with user experience
// - Parallelism: 4 threads - utilizes multi-core CPUs efficiently
// - Output Length: 32 bytes (256 bits) - suitable for AES-256
//
// Expected Performance:
// - Modern CPU: ~100-300ms per derivation
// - This is intentionally slow to prevent brute-force attacks

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Algorithm, Argon2, Params, Version,
};
use rand::{rngs::OsRng, TryRngCore};

use crate::crypto::secure::{Password, SecureBytes};
use crate::error::{CryptoError, CryptoResult};

// Argon2 configuration constants
// These values are based on OWASP recommendations for file encryption (2025)

/// Memory cost in KiB (64 MiB = 65536 KiB)
/// Higher values increase resistance to GPU attacks but use more RAM
const MEMORY_COST: u32 = 65536;

/// Number of iterations
/// Higher values increase time cost, making brute-force slower
const TIME_COST: u32 = 3;

/// Degree of parallelism (number of threads)
/// Should match typical CPU core count for optimal performance
const PARALLELISM: u32 = 4;

/// Output key length in bytes (32 bytes = 256 bits for AES-256)
const KEY_LENGTH: usize = 32;

/// Salt length in bytes (16 bytes = 128 bits is standard)
const SALT_LENGTH: usize = 16;

/// Derive a cryptographic key from a password using Argon2id
///
/// This function uses the Argon2id algorithm with OWASP-recommended parameters
/// to derive a 256-bit encryption key from a user-provided password and salt.
///
/// # Arguments
/// * `password` - The user's password (will be zeroized after use)
/// * `salt` - Random salt bytes (should be unique per encryption)
///
/// # Returns
/// A `SecureBytes` containing the 32-byte derived key
///
/// # Security Notes
/// - The same password + salt combination always produces the same key (deterministic)
/// - Different salts with the same password produce different keys
/// - Takes ~100-300ms on modern CPUs (this is intentional for security)
///
/// # Example
/// ```no_run
/// use filecrypter_lib::crypto::{derive_key, generate_salt, Password};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let password = Password::new("my_password".to_string());
/// let salt = generate_salt()?;
/// let _key = derive_key(&password, &salt)?;
/// // key is now a 32-byte encryption key
/// # Ok(())
/// # }
/// ```
pub fn derive_key(password: &Password, salt: &[u8]) -> CryptoResult<SecureBytes> {
    // Create Argon2 parameters with our security settings
    let params = Params::new(
        MEMORY_COST,      // Memory cost (KiB)
        TIME_COST,        // Time cost (iterations)
        PARALLELISM,      // Parallelism (threads)
        Some(KEY_LENGTH), // Output length
    )
    .map_err(|_| CryptoError::EncryptionFailed)?;

    // Initialize Argon2id with our parameters
    let argon2 = Argon2::new(
        Algorithm::Argon2id, // Hybrid algorithm (best security)
        Version::V0x13,      // Latest version (0x13 = 19)
        params,
    );

    // Encode the salt as a base64 string (required by argon2 crate API)
    let salt_string = SaltString::encode_b64(salt)
        .map_err(|_| CryptoError::FormatError("Invalid salt".to_string()))?;

    // Perform the key derivation (CPU-intensive operation)
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt_string)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    // Extract the raw hash bytes (our encryption key)
    let hash = password_hash.hash.ok_or(CryptoError::EncryptionFailed)?;

    // The hash is the derived key - wrap it in SecureBytes for safe handling
    let key_bytes = hash.as_bytes()[..KEY_LENGTH].to_vec();

    Ok(SecureBytes::new(key_bytes))
}

/// Generate a cryptographically secure random salt
///
/// Salts should be unique for each encryption operation to ensure
/// that the same password produces different encryption keys for different files.
///
/// # Returns
/// A vector of 16 random bytes
///
/// # Security Notes
/// - Uses OS-level CSPRNG (OsRng) for cryptographic quality randomness
/// - Each salt should be stored with the encrypted file
/// - Salts don't need to be secret, only unique
///
/// # Example
/// ```no_run
/// use filecrypter_lib::crypto::generate_salt;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let _salt = generate_salt()?;
/// // Use this salt with derive_key()
/// # Ok(())
/// # }
/// ```
pub fn generate_salt() -> CryptoResult<Vec<u8>> {
    let mut salt = vec![0u8; SALT_LENGTH];

    // Fill with cryptographically secure random bytes from the OS
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut salt)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    Ok(salt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_produces_correct_length() {
        let password = Password::new("test_password".to_string());
        let salt = generate_salt().unwrap();

        let key = derive_key(&password, &salt).unwrap();

        assert_eq!(key.len(), KEY_LENGTH);
    }

    #[test]
    fn test_same_password_and_salt_produce_same_key() {
        let password = Password::new("consistent_password".to_string());
        let salt = vec![42u8; SALT_LENGTH]; // Fixed salt for testing

        let key1 = derive_key(&password, &salt).unwrap();
        let key2 = derive_key(&password, &salt).unwrap();

        assert_eq!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    fn test_different_salts_produce_different_keys() {
        let password = Password::new("same_password".to_string());
        let salt1 = vec![1u8; SALT_LENGTH];
        let salt2 = vec![2u8; SALT_LENGTH];

        let key1 = derive_key(&password, &salt1).unwrap();
        let key2 = derive_key(&password, &salt2).unwrap();

        assert_ne!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    fn test_different_passwords_produce_different_keys() {
        let password1 = Password::new("password1".to_string());
        let password2 = Password::new("password2".to_string());
        let salt = vec![42u8; SALT_LENGTH];

        let key1 = derive_key(&password1, &salt).unwrap();
        let key2 = derive_key(&password2, &salt).unwrap();

        assert_ne!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    fn test_generate_salt_produces_correct_length() {
        let salt = generate_salt().unwrap();

        assert_eq!(salt.len(), SALT_LENGTH);
    }

    #[test]
    fn test_generate_salt_produces_unique_salts() {
        let salt1 = generate_salt().unwrap();
        let salt2 = generate_salt().unwrap();

        // Extremely unlikely to be equal if using proper CSPRNG
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_derive_key_with_short_password() {
        let password = Password::new("a".to_string());
        let salt = generate_salt().unwrap();

        // Should work even with short passwords (though not recommended)
        let result = derive_key(&password, &salt);
        assert!(result.is_ok());
    }

    #[test]
    fn test_derive_key_with_long_password() {
        let password = Password::new("a".repeat(1000));
        let salt = generate_salt().unwrap();

        // Should handle long passwords without issues
        let result = derive_key(&password, &salt);
        assert!(result.is_ok());
    }

    // Performance test (informational - not a pass/fail test)
    #[test]
    fn test_key_derivation_performance() {
        use std::time::Instant;

        let password = Password::new("benchmark_password".to_string());
        let salt = generate_salt().unwrap();

        let start = Instant::now();
        let _key = derive_key(&password, &salt).unwrap();
        let duration = start.elapsed();

        // Log the duration for informational purposes
        println!("Argon2id key derivation took: {:?}", duration);

        // Typically should be 100-500ms on modern hardware
        // This is intentionally slow for security
        assert!(
            duration.as_millis() > 10,
            "Key derivation suspiciously fast"
        );
        assert!(duration.as_secs() < 5, "Key derivation too slow");
    }
}
