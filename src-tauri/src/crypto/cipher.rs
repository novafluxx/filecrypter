// crypto/cipher.rs - AES-256-GCM Encryption and Decryption
//
// This module implements authenticated encryption using AES-256-GCM.
// GCM (Galois/Counter Mode) provides both confidentiality and authenticity,
// protecting against tampering and ensuring data integrity.
//
// AES-256-GCM Properties:
// - Encryption: AES in counter mode with 256-bit keys
// - Authentication: GMAC (Galois Message Authentication Code)
// - Nonce: 96 bits (12 bytes) - must be unique for each encryption
// - Tag: 128 bits (16 bytes) - verifies data hasn't been tampered
//
// Security Features:
// - Authenticated encryption (AEAD) - detects any modifications
// - Protects against chosen-ciphertext attacks
// - Industry standard (used in TLS, IPSec, etc.)
//
// Performance:
// - Very fast (hardware acceleration on most modern CPUs)
// - Encryption/decryption is typically <10ms for small files

use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use rand::{rngs::OsRng, TryRngCore};

use crate::crypto::secure::SecureBytes;
use crate::error::{CryptoError, CryptoResult};

/// Nonce size for AES-GCM (12 bytes = 96 bits is the standard)
const NONCE_SIZE: usize = 12;

/// Encrypt plaintext using AES-256-GCM
///
/// This function performs authenticated encryption, which means it both
/// encrypts the data and generates an authentication tag to detect tampering.
///
/// # Arguments
/// * `key` - 32-byte (256-bit) encryption key from key derivation
/// * `plaintext` - The data to encrypt
///
/// # Returns
/// A tuple of (nonce, ciphertext+tag) where:
/// - nonce: 12-byte random value (must be stored with ciphertext)
/// - ciphertext: Encrypted data with 16-byte authentication tag appended
///
/// # Security Notes
/// - A new random nonce is generated for each encryption
/// - The same key+nonce combination must NEVER be reused
/// - The authentication tag is automatically appended to ciphertext
/// - Tag verification happens automatically during decryption
///
/// # Errors
/// Returns `CryptoError::EncryptionFailed` if:
/// - Key length is not 32 bytes
/// - Encryption operation fails (rare, usually indicates hardware issues)
///
/// # Example
/// ```no_run
/// use filecypter_lib::crypto::{encrypt, SecureBytes};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let key = SecureBytes::new(vec![0u8; 32]);
/// let plaintext = b"Hello, World!";
/// let (_nonce, _ciphertext) = encrypt(&key, plaintext)?;
/// // ciphertext is now encrypted and includes authentication tag
/// # Ok(())
/// # }
/// ```
pub fn encrypt(key: &SecureBytes, plaintext: &[u8]) -> CryptoResult<(Vec<u8>, Vec<u8>)> {
    // Validate key length (AES-256 requires exactly 32 bytes)
    if key.len() != 32 {
        return Err(CryptoError::EncryptionFailed);
    }

    // Generate a random nonce using OS-provided CSPRNG
    // CRITICAL: Nonces must be unique for each encryption with the same key
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut nonce_bytes)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Create the AES-256-GCM cipher instance with our key
    let cipher = Aes256Gcm::new_from_slice(key.as_slice())
        .map_err(|_| CryptoError::EncryptionFailed)?;

    // Perform the encryption
    // This produces: ciphertext || tag (tag is automatically appended)
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    // Return both nonce and ciphertext (both are needed for decryption)
    Ok((nonce_bytes.to_vec(), ciphertext))
}

/// Decrypt ciphertext using AES-256-GCM
///
/// This function decrypts data and verifies the authentication tag.
/// If the tag doesn't match (data was tampered with or wrong key),
/// decryption fails and returns an error.
///
/// # Arguments
/// * `key` - 32-byte (256-bit) decryption key (must match encryption key)
/// * `nonce` - 12-byte nonce used during encryption
/// * `ciphertext` - Encrypted data with authentication tag appended
///
/// # Returns
/// The original plaintext if decryption and authentication succeed
///
/// # Security Notes
/// - Authentication tag is automatically verified
/// - If tag verification fails, an error is returned (data was tampered with)
/// - Wrong password results in tag verification failure
/// - Timing-safe comparison prevents timing attacks
///
/// # Errors
/// Returns `CryptoError::InvalidPassword` if:
/// - Wrong password (key) was used
/// - Data has been tampered with
/// - Authentication tag is invalid
///
/// Returns `CryptoError::DecryptionFailed` if:
/// - Key or nonce length is invalid
/// - Ciphertext is too short (missing tag)
///
/// # Example
/// ```no_run
/// use filecypter_lib::crypto::{decrypt, SecureBytes};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let key = SecureBytes::new(vec![0u8; 32]);
/// let nonce = vec![1u8; 12];
/// let ciphertext = vec![0u8; 16];
/// let _plaintext = decrypt(&key, &nonce, &ciphertext)?;
/// # Ok(())
/// # }
/// ```
pub fn decrypt(key: &SecureBytes, nonce: &[u8], ciphertext: &[u8]) -> CryptoResult<Vec<u8>> {
    // Validate key length
    if key.len() != 32 {
        return Err(CryptoError::DecryptionFailed);
    }

    // Validate nonce length
    if nonce.len() != NONCE_SIZE {
        return Err(CryptoError::DecryptionFailed);
    }

    // Convert nonce to the correct type
    let nonce = Nonce::from_slice(nonce);

    // Create the AES-256-GCM cipher instance with our key
    let cipher = Aes256Gcm::new_from_slice(key.as_slice())
        .map_err(|_| CryptoError::DecryptionFailed)?;

    // Perform the decryption
    // This automatically verifies the authentication tag
    // If the tag doesn't match, this returns an error
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::InvalidPassword)?; // Most likely wrong password

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Hello, FileCypter! This is a test message.";

        // Encrypt
        let (nonce, ciphertext) = encrypt(&key, plaintext).unwrap();

        // Verify nonce size
        assert_eq!(nonce.len(), NONCE_SIZE);

        // Verify ciphertext is longer than plaintext (includes tag)
        assert!(ciphertext.len() > plaintext.len());

        // Decrypt
        let decrypted = decrypt(&key, &nonce, &ciphertext).unwrap();

        // Verify plaintext matches
        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_different_nonces_produce_different_ciphertext() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Same message";

        let (nonce1, ciphertext1) = encrypt(&key, plaintext).unwrap();
        let (nonce2, ciphertext2) = encrypt(&key, plaintext).unwrap();

        // Nonces should be different
        assert_ne!(nonce1, nonce2);

        // Ciphertexts should be different (due to different nonces)
        assert_ne!(ciphertext1, ciphertext2);

        // But both should decrypt to the same plaintext
        let decrypted1 = decrypt(&key, &nonce1, &ciphertext1).unwrap();
        let decrypted2 = decrypt(&key, &nonce2, &ciphertext2).unwrap();
        assert_eq!(decrypted1, decrypted2);
    }

    #[test]
    fn test_wrong_key_fails_decryption() {
        let key1 = SecureBytes::new(vec![1u8; 32]);
        let key2 = SecureBytes::new(vec![2u8; 32]);
        let plaintext = b"Secret message";

        let (nonce, ciphertext) = encrypt(&key1, plaintext).unwrap();

        // Try to decrypt with wrong key
        let result = decrypt(&key2, &nonce, &ciphertext);

        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::InvalidPassword)));
    }

    #[test]
    fn test_tampered_ciphertext_fails_decryption() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Original message";

        let (nonce, mut ciphertext) = encrypt(&key, plaintext).unwrap();

        // Tamper with the ciphertext
        ciphertext[0] ^= 0xFF;

        // Decryption should fail due to authentication tag mismatch
        let result = decrypt(&key, &nonce, &ciphertext);

        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::InvalidPassword)));
    }

    #[test]
    fn test_tampered_tag_fails_decryption() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Original message";

        let (nonce, mut ciphertext) = encrypt(&key, plaintext).unwrap();

        // Tamper with the last byte (part of the authentication tag)
        let last_idx = ciphertext.len() - 1;
        ciphertext[last_idx] ^= 0xFF;

        // Decryption should fail
        let result = decrypt(&key, &nonce, &ciphertext);

        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_nonce_fails_decryption() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Test message";

        let (_correct_nonce, ciphertext) = encrypt(&key, plaintext).unwrap();
        let wrong_nonce = vec![0u8; NONCE_SIZE]; // Different nonce

        // Decryption with wrong nonce should fail
        let result = decrypt(&key, &wrong_nonce, &ciphertext);

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_key_length() {
        let short_key = SecureBytes::new(vec![1u8; 16]); // Too short
        let plaintext = b"Test";

        let result = encrypt(&short_key, plaintext);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_nonce_length() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let wrong_nonce = vec![1u8; 16]; // Wrong size
        let ciphertext = vec![1u8; 32];

        let result = decrypt(&key, &wrong_nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"";

        let (nonce, ciphertext) = encrypt(&key, plaintext).unwrap();

        // Even empty plaintext should have a tag
        assert!(ciphertext.len() >= 16);

        let decrypted = decrypt(&key, &nonce, &ciphertext).unwrap();
        assert_eq!(decrypted.len(), 0);
    }

    #[test]
    fn test_large_plaintext() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = vec![0xAAu8; 1024 * 1024]; // 1 MB

        let (nonce, ciphertext) = encrypt(&key, &plaintext).unwrap();
        let decrypted = decrypt(&key, &nonce, &ciphertext).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_ciphertext_includes_tag() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Test message";

        let (_, ciphertext) = encrypt(&key, plaintext).unwrap();

        // Ciphertext should be plaintext length + 16 bytes (tag)
        assert_eq!(ciphertext.len(), plaintext.len() + 16);
    }
}
