// crypto/streaming.rs - Streaming Encryption/Decryption
//
// This module implements chunked file encryption for large files.
// Instead of loading the entire file into memory, it processes
// files in chunks (default 1MB), encrypting each chunk independently.
//
// Security considerations:
// - Each chunk uses a unique nonce derived from (base_nonce, chunk_index) via BLAKE3
// - AES-GCM authentication tag verifies each chunk's integrity
// - Chunk ordering is enforced by binding chunk_index into the nonce derivation
//
// File format (version 4):
// - KDF parameters are stored in the header so each file is self-describing.
// - The header is authenticated as AAD for every chunk.
// All integer fields are little-endian.
// [VERSION:1] [SALT_LEN_LE:4] [KDF_ALG:1] [KDF_MEM_COST:4] [KDF_TIME_COST:4]
// [KDF_PARALLELISM:4] [KDF_KEY_LEN:4] [SALT:N] [BASE_NONCE:12] [CHUNK_SIZE_LE:4] [TOTAL_CHUNKS_LE:8]
// [CHUNK_1_LEN_LE:4] [CHUNK_1_CIPHERTEXT+TAG] [CHUNK_2_LEN_LE:4] [CHUNK_2_CIPHERTEXT+TAG] ...
//
// Each chunk's nonce is derived via:
// BLAKE3("filecrypter-chunk-nonce-v1" || base_nonce || chunk_index)
// For version 4, the header bytes are authenticated as AAD for every chunk.

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use rand::{rngs::OsRng, TryRngCore};
use tempfile::NamedTempFile;

use crate::crypto::kdf::{derive_key_with_params, generate_salt_with_len, KdfAlgorithm, KdfParams};
use crate::crypto::secure::Password;
use crate::error::{CryptoError, CryptoResult};

#[cfg(windows)]
use crate::security::set_owner_only_dacl;

/// Default chunk size: 1 MB
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum allowed chunk size to avoid excessive memory usage during decrypt
const MAX_CHUNK_SIZE: usize = 16 * 1024 * 1024;

/// Streaming file format version
pub const STREAMING_VERSION: u8 = 4;

/// Nonce size for AES-GCM (96 bits = 12 bytes)
const NONCE_SIZE: usize = 12;

/// AES-GCM authentication tag size
const TAG_SIZE: usize = 16;

/// Maximum allowed chunks (~10TB at 1MB chunks)
const MAX_CHUNKS: u64 = 10_000_000;


/// Progress callback type for streaming operations
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Encrypt a file using streaming (chunked) encryption
///
/// This function reads the input file in chunks, encrypts each chunk
/// independently with AES-256-GCM, and writes to the output file.
///
/// # Arguments
/// * `input_path` - Path to the plaintext file
/// * `output_path` - Path where encrypted file will be saved
/// * `password` - User's password
/// * `chunk_size` - Size of each chunk in bytes (default: 1MB)
/// * `progress_callback` - Optional callback for progress updates (bytes_processed, total_bytes)
/// * `allow_overwrite` - Allow overwriting existing files (default: false)
///
/// # Returns
/// Ok(()) on success, or CryptoError on failure
pub fn encrypt_file_streaming<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    password: &Password,
    chunk_size: usize,
    progress_callback: Option<ProgressCallback>,
    allow_overwrite: bool,
) -> CryptoResult<()> {
    if password.is_empty() {
        return Err(CryptoError::FormatError(
            "Password cannot be empty".to_string(),
        ));
    }

    let chunk_size = if chunk_size == 0 {
        DEFAULT_CHUNK_SIZE
    } else {
        chunk_size
    };

    if chunk_size > MAX_CHUNK_SIZE {
        return Err(CryptoError::FormatError(format!(
            "Chunk size too large: {} bytes (max {})",
            chunk_size, MAX_CHUNK_SIZE
        )));
    }

    // Open input file and get size
    let input_file = File::open(input_path.as_ref())?;
    let file_size = input_file.metadata()?.len();
    let mut reader = BufReader::new(input_file);

    // Create a secure temp file in the output directory.
    // We only rename to the final output path after the full write completes.
    let output_path = output_path.as_ref();
    let output_parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp_file = create_secure_tempfile(output_parent)?;
    let mut writer = BufWriter::new(temp_file.as_file_mut());

    // Generate salt and derive key
    let kdf_params = KdfParams::default();
    let salt = generate_salt_with_len(kdf_params.salt_length as usize)?;
    let key = derive_key_with_params(password, &salt, &kdf_params)?;
    let cipher =
        Aes256Gcm::new_from_slice(key.as_slice()).map_err(|_| CryptoError::EncryptionFailed)?;

    // Generate base nonce using cryptographically secure RNG
    // Combined with timestamp to prevent nonce reuse even if RNG has low entropy
    let mut base_nonce = [0u8; NONCE_SIZE];
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut base_nonce)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    // Mix in timestamp as additional entropy source
    // This prevents nonce reuse even if RNG fails or has low entropy
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| CryptoError::EncryptionFailed)?
        .as_nanos() as u64;

    for (i, byte) in timestamp.to_le_bytes().iter().enumerate() {
        if i < NONCE_SIZE {
            base_nonce[i] ^= byte;
        }
    }

    // Calculate total chunks
    // Note: Empty files (0 bytes) are represented as 1 chunk with 0 data bytes, so we still
    // produce a single AEAD tag. This lets decryption validate the password (and header AAD)
    // even for empty inputs.
    let total_chunks_u64 = if file_size == 0 {
        1u64
    } else {
        (file_size / chunk_size as u64)
            + if file_size % chunk_size as u64 != 0 {
                1
            } else {
                0
            }
    };

    // Validate chunk count to prevent creating files that can't be decrypted
    if total_chunks_u64 > MAX_CHUNKS {
        return Err(CryptoError::FormatError(format!(
            "File too large for encryption: {} chunks (max {})",
            total_chunks_u64, MAX_CHUNKS
        )));
    }

    // Write header
    let header = build_header(
        STREAMING_VERSION,
        &kdf_params,
        &salt,
        &base_nonce,
        chunk_size,
        total_chunks_u64,
    );
    writer.write_all(&header)?;

    // Process chunks
    let mut buffer = vec![0u8; chunk_size];
    let mut bytes_processed: u64 = 0;

    for chunk_index in 0..total_chunks_u64 {
        let remaining = file_size.saturating_sub(chunk_index * chunk_size as u64);
        let bytes_to_read = std::cmp::min(chunk_size as u64, remaining) as usize;

        if bytes_to_read > 0 {
            reader.read_exact(&mut buffer[..bytes_to_read])?;
        }

        // Derive a per-chunk nonce deterministically from (base_nonce, chunk_index).
        let chunk_nonce = derive_chunk_nonce(&base_nonce, chunk_index);
        let nonce = Nonce::from_slice(&chunk_nonce);

        // Encrypt chunk
        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: &buffer[..bytes_to_read],
                    aad: &header,
                },
            )
            .map_err(|_| CryptoError::EncryptionFailed)?;

        // Write chunk: [length:4][ciphertext+tag]
        writer.write_all(&(ciphertext.len() as u32).to_le_bytes())?;
        writer.write_all(&ciphertext)?;

        bytes_processed += bytes_to_read as u64;

        // Call progress callback
        if let Some(ref callback) = progress_callback {
            callback(bytes_processed, file_size);
        }
    }

    writer.flush()?;
    drop(writer);

    if allow_overwrite && output_path.exists() {
        fs::remove_file(output_path).map_err(CryptoError::Io)?;
    }

    if let Err(err) = temp_file.persist(output_path) {
        let _ = fs::remove_file(err.file.path());
        return Err(CryptoError::Io(err.error));
    }

    Ok(())
}

/// Decrypt a file using streaming (chunked) decryption
///
/// This function reads the encrypted file in chunks, decrypts each chunk
/// independently, and writes the plaintext to the output file.
///
/// # Arguments
/// * `input_path` - Path to the encrypted file
/// * `output_path` - Path where decrypted file will be saved
/// * `password` - User's password
/// * `progress_callback` - Optional callback for progress updates
/// * `allow_overwrite` - Allow overwriting existing files (default: false)
///
/// # Returns
/// Ok(()) on success, or CryptoError on failure
pub fn decrypt_file_streaming<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    password: &Password,
    progress_callback: Option<ProgressCallback>,
    allow_overwrite: bool,
) -> CryptoResult<()> {
    if password.is_empty() {
        return Err(CryptoError::FormatError(
            "Password cannot be empty".to_string(),
        ));
    }

    // Open input file
    let input_file = File::open(input_path.as_ref())?;
    let file_size = input_file.metadata()?.len();
    let mut reader = BufReader::new(input_file);

    // Read and verify version
    let mut version = [0u8; 1];
    reader.read_exact(&mut version)?;
    if version[0] != STREAMING_VERSION {
        return Err(CryptoError::FormatError(
            "Unsupported file format".to_string(),
        ));
    }

    // Read salt length
    let mut salt_len_bytes = [0u8; 4];
    reader.read_exact(&mut salt_len_bytes)?;
    let salt_len = u32::from_le_bytes(salt_len_bytes) as usize;

    // Read KDF parameters
    let mut alg_byte = [0u8; 1];
    reader.read_exact(&mut alg_byte)?;
    let algorithm = KdfAlgorithm::from_u8(alg_byte[0])?;

    let mut mem_cost_bytes = [0u8; 4];
    reader.read_exact(&mut mem_cost_bytes)?;
    let memory_cost_kib = u32::from_le_bytes(mem_cost_bytes);

    let mut time_cost_bytes = [0u8; 4];
    reader.read_exact(&mut time_cost_bytes)?;
    let time_cost = u32::from_le_bytes(time_cost_bytes);

    let mut parallelism_bytes = [0u8; 4];
    reader.read_exact(&mut parallelism_bytes)?;
    let parallelism = u32::from_le_bytes(parallelism_bytes);

    let mut key_len_bytes = [0u8; 4];
    reader.read_exact(&mut key_len_bytes)?;
    let key_length = u32::from_le_bytes(key_len_bytes);

    let kdf_params = KdfParams {
        algorithm,
        memory_cost_kib,
        time_cost,
        parallelism,
        key_length,
        salt_length: salt_len as u32,
    };
    kdf_params.validate()?;

    let mut salt = vec![0u8; salt_len];
    reader.read_exact(&mut salt)?;

    // Read base nonce
    let mut base_nonce = [0u8; NONCE_SIZE];
    reader.read_exact(&mut base_nonce)?;

    // Read chunk size and total chunks
    let mut chunk_size_bytes = [0u8; 4];
    reader.read_exact(&mut chunk_size_bytes)?;
    let chunk_size = u32::from_le_bytes(chunk_size_bytes) as usize;

    if chunk_size == 0 || chunk_size > MAX_CHUNK_SIZE {
        return Err(CryptoError::FormatError(format!(
            "Invalid chunk size: {} bytes (max {})",
            chunk_size, MAX_CHUNK_SIZE
        )));
    }

    let mut total_chunks_bytes = [0u8; 8];
    reader.read_exact(&mut total_chunks_bytes)?;
    let total_chunks = u64::from_le_bytes(total_chunks_bytes);

    // Validate chunk count to prevent DoS attacks
    if total_chunks > MAX_CHUNKS {
        return Err(CryptoError::FormatError("File too large".to_string()));
    }

    let header = build_header(
        version[0],
        &kdf_params,
        &salt,
        &base_nonce,
        chunk_size,
        total_chunks,
    );
    let header_aad = header.as_slice();

    // Derive key
    let key = derive_key_with_params(password, &salt, &kdf_params)?;
    let cipher =
        Aes256Gcm::new_from_slice(key.as_slice()).map_err(|_| CryptoError::EncryptionFailed)?;

    // Create a secure temp file in the output directory.
    // We only rename to the final output path after the full write completes.
    let output_path = output_path.as_ref();
    let output_parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp_file = create_secure_tempfile(output_parent)?;
    let mut writer = BufWriter::new(temp_file.as_file_mut());

    // Process chunks
    let mut bytes_processed: u64 = 0;

    for chunk_index in 0..total_chunks {
        // Read chunk length
        let mut chunk_len_bytes = [0u8; 4];
        reader.read_exact(&mut chunk_len_bytes)?;
        let chunk_len = u32::from_le_bytes(chunk_len_bytes) as usize;

        // Strict chunk length validation
        // For version 4, we know the exact chunk_size from header
        // Maximum valid chunk: chunk_size + TAG_SIZE (no extra tolerance needed)
        let max_valid_chunk = chunk_size + TAG_SIZE;
        if chunk_len > max_valid_chunk {
            return Err(CryptoError::FormatError(format!(
                "Invalid chunk length: {} bytes (max {} for chunk_size {})",
                chunk_len, max_valid_chunk, chunk_size
            )));
        }

        // Read encrypted chunk
        let mut ciphertext = vec![0u8; chunk_len];
        reader.read_exact(&mut ciphertext)?;

        // Derive chunk nonce
        let chunk_nonce = derive_chunk_nonce(&base_nonce, chunk_index);
        let nonce = Nonce::from_slice(&chunk_nonce);

        // Decrypt chunk
        let plaintext = cipher
            .decrypt(
                nonce,
                Payload {
                    msg: ciphertext.as_ref(),
                    aad: header_aad,
                },
            )
            .map_err(|_| CryptoError::InvalidPassword)?;

        // Write plaintext
        writer.write_all(&plaintext)?;

        // Track encrypted bytes processed (excludes header and per-chunk length fields).
        bytes_processed += chunk_len as u64;

        // Call progress callback
        if let Some(ref callback) = progress_callback {
            callback(bytes_processed, file_size);
        }
    }

    writer.flush()?;
    drop(writer);

    if allow_overwrite && output_path.exists() {
        fs::remove_file(output_path).map_err(CryptoError::Io)?;
    }

    if let Err(err) = temp_file.persist(output_path) {
        let _ = fs::remove_file(err.file.path());
        return Err(CryptoError::Io(err.error));
    }

    Ok(())
}

/// Derive a unique nonce for each chunk using BLAKE3
///
/// Uses BLAKE3 as a KDF to derive cryptographically unique nonces for each chunk.
/// This provides proper domain separation and prevents nonce collisions.
fn derive_chunk_nonce(base_nonce: &[u8; NONCE_SIZE], chunk_index: u64) -> [u8; NONCE_SIZE] {
    // Use BLAKE3 to derive unique nonces for each chunk
    // This provides cryptographic separation between chunk nonces
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"filecrypter-chunk-nonce-v1"); // Domain separation
    hasher.update(base_nonce);
    hasher.update(&chunk_index.to_le_bytes());

    let hash = hasher.finalize();
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&hash.as_bytes()[..NONCE_SIZE]);
    nonce
}

fn build_header(
    version: u8,
    kdf_params: &KdfParams,
    salt: &[u8],
    base_nonce: &[u8; NONCE_SIZE],
    chunk_size: usize,
    total_chunks: u64,
) -> Vec<u8> {
    let mut header = Vec::with_capacity(1 + 4 + 1 + 4 + 4 + 4 + 4 + salt.len() + NONCE_SIZE + 4 + 8);
    header.push(version);
    header.extend_from_slice(&(salt.len() as u32).to_le_bytes());
    header.push(kdf_params.algorithm.to_u8());
    header.extend_from_slice(&kdf_params.memory_cost_kib.to_le_bytes());
    header.extend_from_slice(&kdf_params.time_cost.to_le_bytes());
    header.extend_from_slice(&kdf_params.parallelism.to_le_bytes());
    header.extend_from_slice(&kdf_params.key_length.to_le_bytes());
    header.extend_from_slice(salt);
    header.extend_from_slice(base_nonce);
    header.extend_from_slice(&(chunk_size as u32).to_le_bytes());
    header.extend_from_slice(&total_chunks.to_le_bytes());
    header
}

fn create_secure_tempfile(parent: &Path) -> CryptoResult<NamedTempFile> {
    let temp_file = NamedTempFile::new_in(parent).map_err(CryptoError::Io)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = temp_file
            .as_file()
            .metadata()
            .map_err(CryptoError::Io)?
            .permissions();
        perms.set_mode(0o600);
        fs::set_permissions(temp_file.path(), perms).map_err(CryptoError::Io)?;
    }

    #[cfg(windows)]
    {
        if let Err(err) = set_owner_only_dacl(temp_file.path()) {
            let _ = fs::remove_file(temp_file.path());
            return Err(CryptoError::Io(err.into()));
        }
    }

    Ok(temp_file)
}

/// Check if a file should use streaming encryption based on size
///
/// Returns true if the file is larger than the threshold (default: 10MB)
pub fn should_use_streaming(file_size: u64, threshold: u64) -> bool {
    file_size > threshold
}

/// Default threshold for automatic streaming (10 MB)
pub const STREAMING_THRESHOLD: u64 = 10 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::kdf::KdfParams;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_derive_chunk_nonce() {
        let base = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

        // BLAKE3-based derivation: all nonces should be unique and unpredictable
        let nonce0 = derive_chunk_nonce(&base, 0);
        let nonce1 = derive_chunk_nonce(&base, 1);
        let nonce2 = derive_chunk_nonce(&base, 2);

        // All nonces should be different from base and each other
        assert_ne!(nonce0, base);
        assert_ne!(nonce1, base);
        assert_ne!(nonce2, base);
        assert_ne!(nonce0, nonce1);
        assert_ne!(nonce1, nonce2);
        assert_ne!(nonce0, nonce2);

        // Same inputs should produce same output (deterministic)
        let nonce0_again = derive_chunk_nonce(&base, 0);
        assert_eq!(nonce0, nonce0_again);
    }

    #[test]
    fn test_streaming_encrypt_decrypt_roundtrip() {
        // Create a temp directory for output files (avoids sharing violations on Windows)
        let temp_dir = tempfile::tempdir().unwrap();

        // Create a test file with some content
        let content = b"Hello, streaming encryption! This is test content.";
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), content).unwrap();

        // Encrypt
        let encrypted_path = temp_dir.path().join("encrypted.bin");
        let password = Password::new("test_password".to_string());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            1024, // Small chunk size for testing
            None,
            false,
        )
        .unwrap();

        // Verify encrypted file is different
        let encrypted_data = fs::read(&encrypted_path).unwrap();
        assert_ne!(encrypted_data, content);

        // Decrypt
        let decrypted_path = temp_dir.path().join("decrypted.bin");
        decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &password,
            None,
            false,
        )
        .unwrap();

        // Verify content matches
        let decrypted_content = fs::read(&decrypted_path).unwrap();
        assert_eq!(content, decrypted_content.as_slice());
    }

    #[test]
    fn test_streaming_empty_file_roundtrip() {
        // Empty inputs should still authenticate (we store a single empty chunk + tag).
        let temp_dir = tempfile::tempdir().unwrap();

        let input_file = NamedTempFile::new().unwrap(); // Empty by default

        let encrypted_path = temp_dir.path().join("encrypted_empty.bin");
        let password = Password::new("test_password".to_string());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            1024,
            None,
            false,
        )
        .unwrap();

        // Encrypted file should contain at least header + length + tag.
        let encrypted_data = fs::read(&encrypted_path).unwrap();
        assert!(!encrypted_data.is_empty());

        let decrypted_path = temp_dir.path().join("decrypted_empty.bin");
        decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &password,
            None,
            false,
        )
        .unwrap();

        let decrypted_data = fs::read(&decrypted_path).unwrap();
        assert!(decrypted_data.is_empty());
    }

    #[test]
    fn test_streaming_wrong_password() {
        // Create a temp directory for output files (avoids sharing violations on Windows)
        let temp_dir = tempfile::tempdir().unwrap();

        // Create and encrypt a file
        let content = b"Secret data";
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), content).unwrap();

        let encrypted_path = temp_dir.path().join("encrypted.bin");
        let correct_password = Password::new("correct_password".to_string());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &correct_password,
            1024,
            None,
            false,
        )
        .unwrap();

        // Try to decrypt with wrong password
        let decrypted_path = temp_dir.path().join("decrypted.bin");
        let wrong_password = Password::new("wrong_password".to_string());
        let result = decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &wrong_password,
            None,
            false,
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::InvalidPassword)));
    }

    #[test]
    fn test_streaming_empty_password() {
        let input_file = NamedTempFile::new().unwrap();
        let output_file = NamedTempFile::new().unwrap();

        let empty_password = Password::new("".to_string());
        let result = encrypt_file_streaming(
            input_file.path(),
            output_file.path(),
            &empty_password,
            DEFAULT_CHUNK_SIZE,
            None,
            false,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_rejects_zero_chunk_size_header() {
        let temp_dir = tempfile::tempdir().unwrap();
        let encrypted_path = temp_dir.path().join("bad_zero_chunk.bin");
        let output_path = temp_dir.path().join("out_zero_chunk.bin");

        let kdf_params = KdfParams::default();
        let salt = vec![0u8; kdf_params.salt_length as usize];
        let base_nonce = [0u8; NONCE_SIZE];
        let header = build_header(STREAMING_VERSION, &kdf_params, &salt, &base_nonce, 0, 0);
        fs::write(&encrypted_path, header).unwrap();

        let password = Password::new("test_password".to_string());
        let result = decrypt_file_streaming(&encrypted_path, &output_path, &password, None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_rejects_large_chunk_size_header() {
        let temp_dir = tempfile::tempdir().unwrap();
        let encrypted_path = temp_dir.path().join("bad_large_chunk.bin");
        let output_path = temp_dir.path().join("out_large_chunk.bin");

        let kdf_params = KdfParams::default();
        let salt = vec![0u8; kdf_params.salt_length as usize];
        let base_nonce = [0u8; NONCE_SIZE];
        let header = build_header(
            STREAMING_VERSION,
            &kdf_params,
            &salt,
            &base_nonce,
            MAX_CHUNK_SIZE + 1,
            0,
        );
        fs::write(&encrypted_path, header).unwrap();

        let password = Password::new("test_password".to_string());
        let result = decrypt_file_streaming(&encrypted_path, &output_path, &password, None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_multi_chunk() {
        // Create a temp directory for output files (avoids sharing violations on Windows)
        let temp_dir = tempfile::tempdir().unwrap();

        // Create a file that spans multiple chunks
        let chunk_size = 1024;
        let num_chunks = 5;
        let content: Vec<u8> = (0..chunk_size * num_chunks)
            .map(|i| (i % 256) as u8)
            .collect();

        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), &content).unwrap();

        // Encrypt
        let encrypted_path = temp_dir.path().join("encrypted.bin");
        let password = Password::new("multi_chunk_test".to_string());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            chunk_size,
            None,
            false,
        )
        .unwrap();

        // Decrypt
        let decrypted_path = temp_dir.path().join("decrypted.bin");
        decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &password,
            None,
            false,
        )
        .unwrap();

        // Verify
        let decrypted_content = fs::read(&decrypted_path).unwrap();
        assert_eq!(content, decrypted_content);
    }

    #[test]
    fn test_should_use_streaming() {
        assert!(!should_use_streaming(1024, STREAMING_THRESHOLD)); // 1KB - no
        assert!(!should_use_streaming(10 * 1024 * 1024, STREAMING_THRESHOLD)); // 10MB exactly - no
        assert!(should_use_streaming(
            10 * 1024 * 1024 + 1,
            STREAMING_THRESHOLD
        )); // 10MB + 1 - yes
        assert!(should_use_streaming(100 * 1024 * 1024, STREAMING_THRESHOLD)); // 100MB - yes
    }
}
