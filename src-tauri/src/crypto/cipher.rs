// crypto/cipher.rs - AES-256-GCM Encryption and Decryption (Test Utilities)
//
// This module provides standalone encrypt/decrypt functions used exclusively
// in tests. Production encryption/decryption goes through the streaming module
// (crypto/streaming.rs) which processes files in chunks.
//
// AES-256-GCM Properties:
// - Encryption: AES in counter mode with 256-bit keys
// - Authentication: GMAC (Galois Message Authentication Code)
// - Nonce: 96 bits (12 bytes) - must be unique for each encryption
// - Tag: 128 bits (16 bytes) - verifies data hasn't been tampered

#[cfg(test)]
mod tests {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
    use rand::{rngs::OsRng, TryRngCore};

    use crate::crypto::secure::SecureBytes;
    use crate::error::{CryptoError, CryptoResult};

    /// Nonce size for AES-GCM (12 bytes = 96 bits is the standard)
    const NONCE_SIZE: usize = 12;

    /// Encrypt plaintext using AES-256-GCM
    fn encrypt(key: &SecureBytes, plaintext: &[u8]) -> CryptoResult<(Vec<u8>, Vec<u8>)> {
        if key.len() != 32 {
            return Err(CryptoError::EncryptionFailed);
        }

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        let mut rng = OsRng;
        rng.try_fill_bytes(&mut nonce_bytes)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(key.as_slice())
            .map_err(|_| CryptoError::EncryptionFailed)?;

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        Ok((nonce_bytes.to_vec(), ciphertext))
    }

    /// Decrypt ciphertext using AES-256-GCM
    fn decrypt(key: &SecureBytes, nonce: &[u8], ciphertext: &[u8]) -> CryptoResult<Vec<u8>> {
        if key.len() != 32 {
            return Err(CryptoError::DecryptionFailed);
        }

        if nonce.len() != NONCE_SIZE {
            return Err(CryptoError::DecryptionFailed);
        }

        let nonce = Nonce::from_slice(nonce);

        let cipher = Aes256Gcm::new_from_slice(key.as_slice())
            .map_err(|_| CryptoError::DecryptionFailed)?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::InvalidPassword)?;

        Ok(plaintext)
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Hello, FileCrypter! This is a test message.";

        let (nonce, ciphertext) = encrypt(&key, plaintext).unwrap();

        assert_eq!(nonce.len(), NONCE_SIZE);
        assert!(ciphertext.len() > plaintext.len());

        let decrypted = decrypt(&key, &nonce, &ciphertext).unwrap();
        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_different_nonces_produce_different_ciphertext() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Same message";

        let (nonce1, ciphertext1) = encrypt(&key, plaintext).unwrap();
        let (nonce2, ciphertext2) = encrypt(&key, plaintext).unwrap();

        assert_ne!(nonce1, nonce2);
        assert_ne!(ciphertext1, ciphertext2);

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

        let result = decrypt(&key2, &nonce, &ciphertext);
        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::InvalidPassword)));
    }

    #[test]
    fn test_tampered_ciphertext_fails_decryption() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Original message";

        let (nonce, mut ciphertext) = encrypt(&key, plaintext).unwrap();
        ciphertext[0] ^= 0xFF;

        let result = decrypt(&key, &nonce, &ciphertext);
        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::InvalidPassword)));
    }

    #[test]
    fn test_tampered_tag_fails_decryption() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Original message";

        let (nonce, mut ciphertext) = encrypt(&key, plaintext).unwrap();
        let last_idx = ciphertext.len() - 1;
        ciphertext[last_idx] ^= 0xFF;

        let result = decrypt(&key, &nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_nonce_fails_decryption() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"Test message";

        let (_correct_nonce, ciphertext) = encrypt(&key, plaintext).unwrap();
        let wrong_nonce = vec![0u8; NONCE_SIZE];

        let result = decrypt(&key, &wrong_nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_key_length() {
        let short_key = SecureBytes::new(vec![1u8; 16]);
        let plaintext = b"Test";

        let result = encrypt(&short_key, plaintext);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_nonce_length() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let wrong_nonce = vec![1u8; 16];
        let ciphertext = vec![1u8; 32];

        let result = decrypt(&key, &wrong_nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let key = SecureBytes::new(vec![42u8; 32]);
        let plaintext = b"";

        let (nonce, ciphertext) = encrypt(&key, plaintext).unwrap();
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
        assert_eq!(ciphertext.len(), plaintext.len() + 16);
    }
}
