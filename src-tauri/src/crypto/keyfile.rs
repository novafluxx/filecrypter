// crypto/keyfile.rs - Key File Support
//
// This module provides key file operations for two-factor file encryption.
// A key file adds a second authentication factor beyond the password.
//
// Design:
// - Key file contents are hashed with BLAKE3 to produce 32 bytes
// - The hash is concatenated with password bytes before Argon2id key derivation
// - This means: key_material = password_bytes || blake3(key_file)
// - Argon2id then derives the final encryption key from key_material + salt
//
// Security:
// - Key files are streamed in 8KB chunks (constant memory usage)
// - Empty files and files >10MB are rejected
// - Generated key files contain 32 cryptographically random bytes

use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;

use rand::{rngs::OsRng, TryRngCore};

use crate::crypto::secure::SecureBytes;
use crate::error::{CryptoError, CryptoResult};

/// Maximum key file size (10 MB)
const MAX_KEY_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Buffer size for streaming key file hashing (8 KB)
const HASH_BUFFER_SIZE: usize = 8 * 1024;

/// Size of generated key files (32 bytes of random data)
const GENERATED_KEY_FILE_SIZE: usize = 32;

/// Hash a key file's contents using BLAKE3 to produce 32 bytes.
///
/// The file is streamed in 8KB chunks for constant memory usage.
///
/// # Arguments
/// * `path` - Path to the key file
///
/// # Returns
/// A `SecureBytes` containing the 32-byte BLAKE3 hash
///
/// # Errors
/// - Empty files are rejected
/// - Files larger than 10MB are rejected
/// - I/O errors during reading
pub fn hash_key_file(path: &Path) -> CryptoResult<SecureBytes> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    if file_size == 0 {
        return Err(CryptoError::KeyFileError(
            "Key file is empty".to_string(),
        ));
    }

    if file_size > MAX_KEY_FILE_SIZE {
        return Err(CryptoError::KeyFileError(format!(
            "Key file is too large ({} bytes, maximum {} bytes)",
            file_size, MAX_KEY_FILE_SIZE
        )));
    }

    // Reject non-regular files
    if !metadata.file_type().is_file() {
        return Err(CryptoError::KeyFileError(
            "Key file must be a regular file".to_string(),
        ));
    }

    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; HASH_BUFFER_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(SecureBytes::new(hash.as_bytes().to_vec()))
}

/// Generate a key file containing 32 cryptographically random bytes.
///
/// # Arguments
/// * `path` - Path where the key file will be created
///
/// # Errors
/// - I/O errors during writing
/// - RNG failure
pub fn generate_key_file(path: &Path) -> CryptoResult<()> {
    let mut key_data = [0u8; GENERATED_KEY_FILE_SIZE];
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut key_data)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    let mut file = File::create(path)?;
    file.write_all(&key_data)?;
    file.flush()?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    Ok(())
}

/// Combine password bytes and key file hash into a single key material buffer.
///
/// The result is `password_bytes || key_file_hash` which is then fed into
/// Argon2id key derivation.
///
/// # Arguments
/// * `password_bytes` - Raw password bytes
/// * `key_file_hash` - 32-byte BLAKE3 hash of the key file
///
/// # Returns
/// A `SecureBytes` containing the concatenated key material
pub fn combine_password_and_keyfile(
    password_bytes: &[u8],
    key_file_hash: &[u8],
) -> SecureBytes {
    let mut combined = Vec::with_capacity(password_bytes.len() + key_file_hash.len());
    combined.extend_from_slice(password_bytes);
    combined.extend_from_slice(key_file_hash);
    SecureBytes::new(combined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_key_file_consistency() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"test key file content").unwrap();

        let hash1 = hash_key_file(file.path()).unwrap();
        let hash2 = hash_key_file(file.path()).unwrap();

        assert_eq!(hash1.as_slice(), hash2.as_slice());
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_hash_key_file_different_content_different_hash() {
        let file1 = NamedTempFile::new().unwrap();
        let file2 = NamedTempFile::new().unwrap();
        fs::write(file1.path(), b"content A").unwrap();
        fs::write(file2.path(), b"content B").unwrap();

        let hash1 = hash_key_file(file1.path()).unwrap();
        let hash2 = hash_key_file(file2.path()).unwrap();

        assert_ne!(hash1.as_slice(), hash2.as_slice());
    }

    #[test]
    fn test_hash_key_file_rejects_empty() {
        let file = NamedTempFile::new().unwrap();
        // File is empty by default

        let result = hash_key_file(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_hash_key_file_rejects_oversize() {
        let file = NamedTempFile::new().unwrap();
        // Write >10MB
        let data = vec![0u8; (MAX_KEY_FILE_SIZE + 1) as usize];
        fs::write(file.path(), &data).unwrap();

        let result = hash_key_file(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too large"));
    }

    #[test]
    fn test_generate_key_file_produces_32_bytes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test.key");

        generate_key_file(&path).unwrap();

        let data = fs::read(&path).unwrap();
        assert_eq!(data.len(), 32);
    }

    #[test]
    fn test_generate_key_file_unique() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path1 = temp_dir.path().join("key1.key");
        let path2 = temp_dir.path().join("key2.key");

        generate_key_file(&path1).unwrap();
        generate_key_file(&path2).unwrap();

        let data1 = fs::read(&path1).unwrap();
        let data2 = fs::read(&path2).unwrap();

        assert_ne!(data1, data2);
    }

    #[test]
    fn test_combine_password_and_keyfile() {
        let password = b"password123";
        let key_hash = [42u8; 32];

        let combined = combine_password_and_keyfile(password, &key_hash);

        assert_eq!(combined.len(), password.len() + 32);
        assert_eq!(&combined.as_slice()[..password.len()], password);
        assert_eq!(&combined.as_slice()[password.len()..], &key_hash);
    }
}
