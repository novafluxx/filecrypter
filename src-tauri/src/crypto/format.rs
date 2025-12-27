// crypto/format.rs - Encrypted File Format Serialization/Deserialization
//
// This module defines the binary file format for encrypted files and provides
// functions to serialize and deserialize encrypted data.
//
// File Format Specification (Version 1):
// ┌─────────────────────────────────────────────────────────────────┐
// │ Byte 0       │ VERSION (1 byte)                                 │
// │ Bytes 1-4    │ SALT_LENGTH (4 bytes, big-endian u32)            │
// │ Bytes 5...N  │ SALT (variable length, typically 16 bytes)       │
// │ Bytes N+1... │ NONCE (12 bytes for AES-GCM)                     │
// │ Bytes ...EOF │ CIPHERTEXT + AUTHENTICATION_TAG (variable length)│
// └─────────────────────────────────────────────────────────────────┘
//
// Design Decisions:
// - Version byte allows future format upgrades without breaking compatibility
// - Big-endian for cross-platform compatibility (network byte order)
// - Variable salt length for flexibility (though currently fixed at 16 bytes)
// - Nonce is stored before ciphertext (standard practice)
// - Authentication tag is appended to ciphertext by AES-GCM

use crate::error::{CryptoError, CryptoResult};

/// Current file format version
const VERSION: u8 = 1;

/// Nonce size for AES-GCM (12 bytes = 96 bits is standard)
const NONCE_SIZE: usize = 12;

/// Minimum authentication tag size (AES-GCM uses 16 bytes)
const MIN_TAG_SIZE: usize = 16;

/// Represents an encrypted file with all necessary decryption metadata
///
/// This structure contains everything needed to decrypt a file:
/// - Salt: Used with password to derive the encryption key
/// - Nonce: Initialization vector for AES-GCM (must be unique per encryption)
/// - Ciphertext: The encrypted data plus authentication tag
#[derive(Debug)]
pub struct EncryptedFile {
    /// Salt used for key derivation (typically 16 bytes)
    pub salt: Vec<u8>,

    /// Nonce/IV for AES-GCM encryption (always 12 bytes)
    pub nonce: Vec<u8>,

    /// Encrypted data with authentication tag appended (variable length)
    pub ciphertext: Vec<u8>,
}

impl EncryptedFile {
    /// Serialize the encrypted file to binary format
    ///
    /// Creates a byte vector containing all components in the correct order
    /// for storage on disk.
    ///
    /// # Returns
    /// A byte vector ready to be written to a file
    ///
    /// # Format
    /// `[VERSION][SALT_LEN][SALT][NONCE][CIPHERTEXT+TAG]`
    ///
    /// # Example
    /// ```no_run
    /// use filecypter_lib::crypto::EncryptedFile;
    /// let encrypted = EncryptedFile {
    ///     salt: vec![1, 2, 3],
    ///     nonce: vec![4; 12],
    ///     ciphertext: vec![5; 32],
    /// };
    /// let _bytes = encrypted.serialize();
    /// // bytes now contains the full file format
    /// ```
    pub fn serialize(&self) -> Vec<u8> {
        // Calculate total size needed for the serialized data
        let salt_len = self.salt.len() as u32;
        let total_size = 1 // version
            + 4 // salt length field
            + self.salt.len()
            + NONCE_SIZE
            + self.ciphertext.len();

        // Pre-allocate the exact size needed (optimization)
        let mut buffer = Vec::with_capacity(total_size);

        // 1. Write version byte
        buffer.push(VERSION);

        // 2. Write salt length as 4-byte big-endian integer
        buffer.extend_from_slice(&salt_len.to_be_bytes());

        // 3. Write salt bytes
        buffer.extend_from_slice(&self.salt);

        // 4. Write nonce (always 12 bytes for AES-GCM)
        buffer.extend_from_slice(&self.nonce);

        // 5. Write ciphertext + authentication tag
        buffer.extend_from_slice(&self.ciphertext);

        buffer
    }

    /// Deserialize binary data into an EncryptedFile structure
    ///
    /// Parses the binary file format and extracts all components,
    /// validating the format along the way.
    ///
    /// # Arguments
    /// * `data` - Raw bytes read from an encrypted file
    ///
    /// # Returns
    /// An `EncryptedFile` structure if the format is valid
    ///
    /// # Errors
    /// - `FormatError` if the file is too small or corrupted
    /// - `InvalidVersion` if the version byte doesn't match
    ///
    /// # Example
    /// ```no_run
    /// use filecypter_lib::crypto::EncryptedFile;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let file_bytes = std::fs::read("file.encrypted")?;
    /// let _encrypted = EncryptedFile::deserialize(&file_bytes)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn deserialize(data: &[u8]) -> CryptoResult<Self> {
        // Minimum size check: version(1) + salt_len(4) + nonce(12) + tag(16)
        let min_size = 1 + 4 + NONCE_SIZE + MIN_TAG_SIZE;
        if data.len() < min_size {
            return Err(CryptoError::FormatError(format!(
                "File too small (expected at least {} bytes, got {})",
                min_size,
                data.len()
            )));
        }

        let mut pos = 0;

        // 1. Read and validate version byte
        let version = data[pos];
        pos += 1;

        if version != VERSION {
            return Err(CryptoError::InvalidVersion);
        }

        // 2. Read salt length (4 bytes, big-endian)
        let salt_len_bytes: [u8; 4] = data[pos..pos + 4]
            .try_into()
            .map_err(|_| CryptoError::FormatError("Failed to read salt length".to_string()))?;
        let salt_len = u32::from_be_bytes(salt_len_bytes) as usize;
        pos += 4;

        // Validate salt length is reasonable (prevent allocation attacks)
        if salt_len > 1024 {
            return Err(CryptoError::FormatError(format!(
                "Salt length too large ({} bytes)",
                salt_len
            )));
        }

        // 3. Verify we have enough bytes for salt + nonce + minimal ciphertext
        if data.len() < pos + salt_len + NONCE_SIZE + MIN_TAG_SIZE {
            return Err(CryptoError::FormatError(
                "File truncated or corrupted".to_string(),
            ));
        }

        // 4. Read salt
        let salt = data[pos..pos + salt_len].to_vec();
        pos += salt_len;

        // 5. Read nonce (always 12 bytes)
        let nonce = data[pos..pos + NONCE_SIZE].to_vec();
        pos += NONCE_SIZE;

        // 6. Read remaining data as ciphertext (includes authentication tag)
        let ciphertext = data[pos..].to_vec();

        // Validate ciphertext has at least the authentication tag
        if ciphertext.len() < MIN_TAG_SIZE {
            return Err(CryptoError::FormatError(
                "Ciphertext too small (missing authentication tag)".to_string(),
            ));
        }

        Ok(Self {
            salt,
            nonce,
            ciphertext,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let original = EncryptedFile {
            salt: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            nonce: vec![1; NONCE_SIZE],
            ciphertext: vec![42; 64], // 64 bytes including tag
        };

        let serialized = original.serialize();
        let deserialized = EncryptedFile::deserialize(&serialized).unwrap();

        assert_eq!(original.salt, deserialized.salt);
        assert_eq!(original.nonce, deserialized.nonce);
        assert_eq!(original.ciphertext, deserialized.ciphertext);
    }

    #[test]
    fn test_serialize_format() {
        let encrypted = EncryptedFile {
            salt: vec![1, 2],
            nonce: vec![3; 12],
            ciphertext: vec![4; 20],
        };

        let bytes = encrypted.serialize();

        // Check version byte
        assert_eq!(bytes[0], VERSION);

        // Check salt length (big-endian)
        let salt_len = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        assert_eq!(salt_len, 2);

        // Check salt starts at byte 5
        assert_eq!(&bytes[5..7], &[1, 2]);

        // Check nonce starts after salt
        assert_eq!(&bytes[7..19], &[3; 12]);

        // Check ciphertext starts after nonce
        assert_eq!(&bytes[19..], &[4; 20]);
    }

    #[test]
    fn test_deserialize_too_small() {
        let data = vec![1, 2, 3]; // Way too small

        let result = EncryptedFile::deserialize(&data);
        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::FormatError(_))));
    }

    #[test]
    fn test_deserialize_wrong_version() {
        let mut data = vec![0; 100]; // Enough bytes
        data[0] = 99; // Wrong version

        let result = EncryptedFile::deserialize(&data);
        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::InvalidVersion)));
    }

    #[test]
    fn test_deserialize_truncated_file() {
        let encrypted = EncryptedFile {
            salt: vec![1; 16],
            nonce: vec![2; 12],
            ciphertext: vec![3; 32],
        };

        let mut bytes = encrypted.serialize();
        bytes.truncate(bytes.len() - 5); // Truncate some bytes

        let result = EncryptedFile::deserialize(&bytes);
        // Should either fail format check or produce incorrect ciphertext
        // This is acceptable as decryption will fail anyway
        if let Ok(parsed) = result {
            assert!(parsed.ciphertext.len() < 32);
        }
    }

    #[test]
    fn test_deserialize_massive_salt_length() {
        let mut data = vec![0; 1000];
        data[0] = VERSION;
        // Set salt length to unreasonably large value
        data[1..5].copy_from_slice(&(100000u32).to_be_bytes());

        let result = EncryptedFile::deserialize(&data);
        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::FormatError(_))));
    }

    #[test]
    fn test_serialize_size_calculation() {
        let encrypted = EncryptedFile {
            salt: vec![1; 16],
            nonce: vec![2; 12],
            ciphertext: vec![3; 48],
        };

        let bytes = encrypted.serialize();

        // Expected: 1 (version) + 4 (salt_len) + 16 (salt) + 12 (nonce) + 48 (ciphertext)
        assert_eq!(bytes.len(), 1 + 4 + 16 + 12 + 48);
    }

    #[test]
    fn test_empty_ciphertext_rejected() {
        let data = {
            let encrypted = EncryptedFile {
                salt: vec![1; 16],
                nonce: vec![2; 12],
                ciphertext: vec![3; 10], // Less than MIN_TAG_SIZE
            };
            encrypted.serialize()
        };

        let result = EncryptedFile::deserialize(&data);
        assert!(result.is_err());
    }
}
