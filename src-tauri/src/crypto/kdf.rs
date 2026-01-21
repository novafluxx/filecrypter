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

/// Supported KDF algorithms for file encryption.
///
/// These identifiers are serialized into encrypted file headers to make each file
/// self-describing. Unknown values must be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdfAlgorithm {
    Argon2id = 1,
}

impl KdfAlgorithm {
    pub fn from_u8(value: u8) -> CryptoResult<Self> {
        match value {
            1 => Ok(KdfAlgorithm::Argon2id),
            _ => Err(CryptoError::FormatError(
                "Unsupported KDF algorithm".to_string(),
            )),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

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
/// This is public to allow consistent validation across streaming encryption operations
pub const SALT_LENGTH: usize = 16;

const MIN_MEMORY_COST: u32 = 8 * 1024;
const MAX_MEMORY_COST: u32 = 256 * 1024;
const MIN_TIME_COST: u32 = 1;
const MAX_TIME_COST: u32 = 10;
const MIN_PARALLELISM: u32 = 1;
const MAX_PARALLELISM: u32 = 16;
const MIN_SALT_LENGTH: u32 = 16; // Current default, minimum for security
const MAX_SALT_LENGTH: u32 = 64; // Allow future flexibility without format changes
                                 // AES-256 requires a 32-byte key, so the allowed range is fixed for now.
const MIN_KEY_LENGTH: u32 = 32;
const MAX_KEY_LENGTH: u32 = 32;

/// KDF parameters stored in encrypted file headers.
///
/// These are public, integrity-protected metadata. They must be validated to
/// prevent malicious inputs from causing excessive CPU/memory usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    pub algorithm: KdfAlgorithm,
    pub memory_cost_kib: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub key_length: u32,
    pub salt_length: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            algorithm: KdfAlgorithm::Argon2id,
            memory_cost_kib: MEMORY_COST,
            time_cost: TIME_COST,
            parallelism: PARALLELISM,
            key_length: KEY_LENGTH as u32,
            salt_length: SALT_LENGTH as u32,
        }
    }
}

impl KdfParams {
    /// Validate KDF parameters and enforce guardrails.
    ///
    /// This rejects values that are too small (weak) or too large (DoS risk),
    /// and currently pins key length to AES-256 (32 bytes).
    pub fn validate(&self) -> CryptoResult<()> {
        match self.algorithm {
            KdfAlgorithm::Argon2id => {}
        }

        if self.memory_cost_kib < MIN_MEMORY_COST || self.memory_cost_kib > MAX_MEMORY_COST {
            return Err(CryptoError::FormatError(format!(
                "Invalid KDF memory cost: {} KiB (must be {}-{} KiB)",
                self.memory_cost_kib, MIN_MEMORY_COST, MAX_MEMORY_COST
            )));
        }
        if self.time_cost < MIN_TIME_COST || self.time_cost > MAX_TIME_COST {
            return Err(CryptoError::FormatError(format!(
                "Invalid KDF time cost: {} (must be {}-{})",
                self.time_cost, MIN_TIME_COST, MAX_TIME_COST
            )));
        }
        if self.parallelism < MIN_PARALLELISM || self.parallelism > MAX_PARALLELISM {
            return Err(CryptoError::FormatError(format!(
                "Invalid KDF parallelism: {} (must be {}-{})",
                self.parallelism, MIN_PARALLELISM, MAX_PARALLELISM
            )));
        }
        if self.key_length < MIN_KEY_LENGTH || self.key_length > MAX_KEY_LENGTH {
            return Err(CryptoError::FormatError(format!(
                "Invalid KDF key length: {} bytes (must be {}-{})",
                self.key_length, MIN_KEY_LENGTH, MAX_KEY_LENGTH
            )));
        }
        if self.salt_length < MIN_SALT_LENGTH || self.salt_length > MAX_SALT_LENGTH {
            return Err(CryptoError::FormatError(format!(
                "Invalid KDF salt length: {} bytes (must be {}-{})",
                self.salt_length, MIN_SALT_LENGTH, MAX_SALT_LENGTH
            )));
        }

        Ok(())
    }
}

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
    derive_key_with_params(password, salt, &KdfParams::default())
}

/// Derive a key using explicit KDF parameters (stored in the file header).
///
/// This allows per-file settings and forward compatibility when defaults change.
pub fn derive_key_with_params(
    password: &Password,
    salt: &[u8],
    params: &KdfParams,
) -> CryptoResult<SecureBytes> {
    params.validate()?;

    // Validate salt length matches the header parameter (not just range).
    if salt.len() != params.salt_length as usize {
        return Err(CryptoError::FormatError(format!(
            "Invalid salt length: expected {} bytes, got {}",
            params.salt_length,
            salt.len()
        )));
    }

    let argon2_params = Params::new(
        params.memory_cost_kib,
        params.time_cost,
        params.parallelism,
        Some(params.key_length as usize),
    )
    .map_err(|_| CryptoError::EncryptionFailed)?;

    let argon2 = match params.algorithm {
        KdfAlgorithm::Argon2id => Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params),
    };

    let salt_string = SaltString::encode_b64(salt)
        .map_err(|_| CryptoError::FormatError("Invalid salt".to_string()))?;

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt_string)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    let hash = password_hash.hash.ok_or(CryptoError::EncryptionFailed)?;
    let key_bytes = hash.as_bytes()[..params.key_length as usize].to_vec();

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
    generate_salt_with_len(SALT_LENGTH)
}

/// Generate a random salt with a caller-specified length.
pub fn generate_salt_with_len(len: usize) -> CryptoResult<Vec<u8>> {
    let mut salt = vec![0u8; len];

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

    #[test]
    fn test_derive_key_invalid_salt_length() {
        let password = Password::new("test".to_string());

        // Too short
        let short_salt = vec![0u8; 8];
        let result = derive_key(&password, &short_salt);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid salt length"));

        // Too long
        let long_salt = vec![0u8; 32];
        let result = derive_key(&password, &long_salt);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid salt length"));

        // Empty
        let result = derive_key(&password, &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid salt length"));

        // Valid length should work
        let valid_salt = vec![0u8; SALT_LENGTH];
        assert!(derive_key(&password, &valid_salt).is_ok());
    }

    #[test]
    fn test_kdf_params_validate_rejects_out_of_bounds() {
        let mut params = KdfParams::default();

        params.memory_cost_kib = MIN_MEMORY_COST - 1;
        assert!(params.validate().is_err());
        params.memory_cost_kib = MAX_MEMORY_COST + 1;
        assert!(params.validate().is_err());
        params.memory_cost_kib = MEMORY_COST;

        params.time_cost = MIN_TIME_COST - 1;
        assert!(params.validate().is_err());
        params.time_cost = MAX_TIME_COST + 1;
        assert!(params.validate().is_err());
        params.time_cost = TIME_COST;

        params.parallelism = MIN_PARALLELISM - 1;
        assert!(params.validate().is_err());
        params.parallelism = MAX_PARALLELISM + 1;
        assert!(params.validate().is_err());
        params.parallelism = PARALLELISM;

        params.key_length = MIN_KEY_LENGTH - 1;
        assert!(params.validate().is_err());
        params.key_length = MAX_KEY_LENGTH + 1;
        assert!(params.validate().is_err());
        params.key_length = KEY_LENGTH as u32;

        params.salt_length = MIN_SALT_LENGTH - 1;
        assert!(params.validate().is_err());
        params.salt_length = MAX_SALT_LENGTH + 1;
        assert!(params.validate().is_err());
    }
}
