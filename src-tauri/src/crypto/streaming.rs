// crypto/streaming.rs - Streaming Encryption/Decryption (Version 4 & 5 Format)
//
// This module implements chunked file encryption using the Version 4 and 5 formats.
// All files in FileCrypter use this streaming approach, regardless of size,
// for consistent behavior and optimal memory usage.
//
// ## Architecture
//
// Files are processed in chunks (default 1MB) rather than loading entirely
// into memory. This provides:
// - Constant memory usage (1MB buffer) independent of file size
// - Support for files of any size (no upper limit)
// - Consistent behavior across all file sizes
// - Atomic writes via temporary files
//
// ## Security Design
//
// **Nonce Derivation:**
// - Base nonce: 96-bit random value XORed with timestamp
// - Per-chunk nonce: BLAKE3("filecrypter-chunk-nonce-v1" || base_nonce || chunk_index)
// - Each chunk has unique nonce, preventing nonce reuse even if base_nonce repeats
// - Chunk ordering enforced by binding chunk_index into nonce derivation
//
// **Authentication:**
// - Each chunk encrypted with AES-256-GCM (provides both encryption and authentication)
// - 128-bit authentication tag per chunk (detects tampering at chunk granularity)
// - Header authenticated as AAD (Additional Authenticated Data) for every chunk
// - Wrong password or tampering detected immediately on first chunk
//
// **Key Derivation:**
// - Argon2id with parameters stored in header (self-describing format)
// - Unique salt per file ensures different keys for same password
//
// ## File Format (Version 4 - No Compression)
//
// All integer fields are little-endian.
//
// **Header:**
// [VERSION:1] [SALT_LEN:4] [KDF_ALG:1] [KDF_MEM_COST:4] [KDF_TIME_COST:4]
// [KDF_PARALLELISM:4] [KDF_KEY_LEN:4] [SALT:N] [BASE_NONCE:12]
// [CHUNK_SIZE:4] [TOTAL_CHUNKS:8]
//
// ## File Format (Version 5 - With Compression)
//
// **Header:**
// [VERSION:1] [SALT_LEN:4] [KDF_ALG:1] [KDF_MEM_COST:4] [KDF_TIME_COST:4]
// [KDF_PARALLELISM:4] [KDF_KEY_LEN:4] [SALT:N] [BASE_NONCE:12]
// [CHUNK_SIZE:4] [TOTAL_CHUNKS:8]
// [COMPRESSION_ALG:1] [COMPRESSION_LEVEL:1] [ORIGINAL_SIZE:8]
//
// **Chunks:**
// [CHUNK_1_LEN:4] [CHUNK_1_CIPHERTEXT+TAG]
// [CHUNK_2_LEN:4] [CHUNK_2_CIPHERTEXT+TAG]
// ...
//
// **Edge Cases:**
// - Empty files (0 bytes): Represented as 1 chunk with 0 data bytes (still produces auth tag)
// - Last chunk: May be smaller than CHUNK_SIZE (exact length stored per chunk)
//
// ## Atomic Writes
//
// Uses temporary files to ensure atomic operations:
// 1. Create secure temp file in output directory
// 2. Write all encrypted chunks to temp file
// 3. Atomically rename temp to final output (no partial files)
// 4. Temp files have restrictive permissions (Unix: 0o600, Windows: ACLs)

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use rand::{rngs::OsRng, TryRngCore};

use crate::crypto::compression::{
    compress, decompress_with_limit, CompressionAlgorithm, CompressionConfig,
};
use crate::crypto::kdf::{
    derive_key_with_material, derive_key_with_params, generate_salt_with_len, KdfAlgorithm,
    KdfParams,
};
use crate::crypto::keyfile::{combine_password_and_keyfile, hash_key_file};
use crate::crypto::secure::Password;
use crate::error::{CryptoError, CryptoResult};

use crate::security::create_secure_tempfile;

/// Default chunk size: 1 MB
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum allowed chunk size to avoid excessive memory usage during decrypt
const MAX_CHUNK_SIZE: usize = 16 * 1024 * 1024;

// Header field sizes (kept local to streaming; header layout differs from non-streaming).
const VERSION_SIZE: usize = 1;
const SALT_LEN_SIZE: usize = 4;
const KDF_PARAMS_SIZE: usize = 1 + 4 + 4 + 4 + 4;
const HEADER_V4_FIXED_SIZE: usize =
    VERSION_SIZE + SALT_LEN_SIZE + KDF_PARAMS_SIZE + NONCE_SIZE + 4 + 8;

// Version 5/7 adds compression fields: algorithm (1) + level (1) + original_size (8) = 10 bytes
const COMPRESSION_FIELDS_SIZE: usize = 1 + 1 + 8;

/// Streaming file format version (without compression)
pub const STREAMING_VERSION_V4: u8 = 4;

/// Streaming file format version (with compression)
pub const STREAMING_VERSION_V5: u8 = 5;

/// Streaming file format version (no compression, with key file support)
pub const STREAMING_VERSION_V6: u8 = 6;

/// Streaming file format version (with compression and key file support)
pub const STREAMING_VERSION_V7: u8 = 7;

/// Default streaming version for backward compatibility (V4 when no compression)
pub const STREAMING_VERSION: u8 = STREAMING_VERSION_V4;

/// Size of the flags byte added in V6/V7
const FLAGS_SIZE: usize = 1;

/// Flag bit: key file was used during encryption
const FLAG_KEY_FILE_USED: u8 = 0x01;

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
/// This function reads the input file in chunks, optionally compresses each chunk,
/// encrypts each chunk independently with AES-256-GCM, and writes to the output file.
///
/// # Arguments
/// * `input_path` - Path to the plaintext file
/// * `output_path` - Path where encrypted file will be saved
/// * `password` - User's password
/// * `chunk_size` - Size of each chunk in bytes (default: 1MB)
/// * `progress_callback` - Optional callback for progress updates (bytes_processed, total_bytes)
/// * `allow_overwrite` - Allow overwriting existing files (default: false)
/// * `compression` - Optional compression configuration. If provided, uses Version 5/7 format.
/// * `key_file_path` - Optional path to a key file for two-factor encryption.
///   If provided, the key file is hashed and combined with the password before key derivation.
///   This produces Version 6 (no compression) or Version 7 (with compression) format.
///
/// # Returns
/// Ok(()) on success, or CryptoError on failure
#[allow(clippy::too_many_arguments)]
pub fn encrypt_file_streaming<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    password: &Password,
    chunk_size: usize,
    progress_callback: Option<ProgressCallback>,
    allow_overwrite: bool,
    compression: Option<CompressionConfig>,
    key_file_path: Option<&Path>,
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
            "Chunk size {} bytes exceeds maximum {} bytes",
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

    // Hash key file if provided, then derive encryption key
    let use_key_file = key_file_path.is_some();
    let key = if let Some(kf_path) = key_file_path {
        let kf_hash = hash_key_file(kf_path)?;
        let combined = combine_password_and_keyfile(password.as_bytes(), kf_hash.as_slice());
        derive_key_with_material(combined.as_slice(), &salt, &kdf_params)?
    } else {
        derive_key_with_params(password, &salt, &kdf_params)?
    };
    let cipher =
        Aes256Gcm::new_from_slice(key.as_slice()).map_err(|_| CryptoError::EncryptionFailed)?;

    // Generate base nonce using cryptographically secure RNG
    let mut base_nonce = [0u8; NONCE_SIZE];
    let mut rng = OsRng;
    rng.try_fill_bytes(&mut base_nonce)
        .map_err(|_| CryptoError::EncryptionFailed)?;

    // Mix in timestamp as defense-in-depth (belt-and-suspenders approach)
    // OsRng is cryptographically secure and is the primary source of randomness.
    // The timestamp XOR provides additional entropy as a secondary defense against:
    // - Hypothetical RNG state compromise or implementation bugs
    // - Nonce reuse if the same RNG state is restored (e.g., VM snapshots)
    // This is purely supplemental and does NOT replace the CSPRNG requirement.
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
    // Note: Empty files (0 bytes) are represented as 1 chunk with 0 data bytes.
    // This ensures we still produce an AEAD authentication tag, which allows
    // password validation even for empty files (wrong password = tag verification fails).
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

    // Determine version based on compression and key file usage
    let compression_config = compression.unwrap_or_else(CompressionConfig::none);
    let use_compression = compression_config.is_enabled();
    let version = match (use_compression, use_key_file) {
        (false, false) => STREAMING_VERSION_V4,
        (true, false) => STREAMING_VERSION_V5,
        (false, true) => STREAMING_VERSION_V6,
        (true, true) => STREAMING_VERSION_V7,
    };
    let flags = if use_key_file { FLAG_KEY_FILE_USED } else { 0 };
    let max_ciphertext_chunk_len = max_ciphertext_len(
        chunk_size,
        if use_compression {
            Some(compression_config.algorithm)
        } else {
            None
        },
    )?;

    // Write header
    let header = build_header(&HeaderParams {
        version,
        kdf_params: &kdf_params,
        salt: &salt,
        base_nonce: &base_nonce,
        chunk_size,
        total_chunks: total_chunks_u64,
        compression: if use_compression {
            Some(&compression_config)
        } else {
            None
        },
        original_size: file_size,
        flags: if use_key_file { Some(flags) } else { None },
    });
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

        // Compress chunk if compression is enabled
        let data_to_encrypt = if use_compression {
            compress(&buffer[..bytes_to_read], &compression_config)?
        } else {
            buffer[..bytes_to_read].to_vec()
        };

        // Encrypt chunk
        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: &data_to_encrypt,
                    aad: &header,
                },
            )
            .map_err(|_| CryptoError::EncryptionFailed)?;

        if ciphertext.len() > max_ciphertext_chunk_len {
            return Err(CryptoError::FormatError(format!(
                "Encrypted chunk length {} exceeds max {} for chunk_size {}",
                ciphertext.len(),
                max_ciphertext_chunk_len,
                chunk_size
            )));
        }

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
/// * `key_file_path` - Optional path to a key file. Required if the file was encrypted
///   with a key file (V6/V7 format with KEY_FILE_USED flag set).
///
/// # Returns
/// Ok(()) on success, or CryptoError on failure
pub fn decrypt_file_streaming<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    password: &Password,
    progress_callback: Option<ProgressCallback>,
    allow_overwrite: bool,
    key_file_path: Option<&Path>,
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
    if !matches!(
        version[0],
        STREAMING_VERSION_V4 | STREAMING_VERSION_V5 | STREAMING_VERSION_V6 | STREAMING_VERSION_V7
    ) {
        return Err(CryptoError::FormatError(format!(
            "Unsupported file format version: {}",
            version[0]
        )));
    }
    let has_compression = version[0] == STREAMING_VERSION_V5 || version[0] == STREAMING_VERSION_V7;
    let has_flags = version[0] == STREAMING_VERSION_V6 || version[0] == STREAMING_VERSION_V7;

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
            "Invalid chunk size: {} bytes (max {} bytes)",
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

    // Read compression fields for V5/V7
    let (compression_algorithm, compression_level, original_size) = if has_compression {
        let mut alg_byte = [0u8; 1];
        reader.read_exact(&mut alg_byte)?;
        let algorithm = CompressionAlgorithm::from_u8(alg_byte[0])?;

        let mut level_byte = [0u8; 1];
        reader.read_exact(&mut level_byte)?;
        let level = level_byte[0] as i32;

        let mut orig_size_bytes = [0u8; 8];
        reader.read_exact(&mut orig_size_bytes)?;
        let orig_size = u64::from_le_bytes(orig_size_bytes);

        (Some(algorithm), level, orig_size)
    } else {
        (None, 0, 0)
    };

    if has_compression {
        let max_plaintext_size = total_chunks.saturating_mul(chunk_size as u64);
        if original_size > max_plaintext_size {
            return Err(CryptoError::FormatError(format!(
                "Invalid original size: {} bytes (max {} bytes)",
                original_size, max_plaintext_size
            )));
        }
    }

    // Read flags byte for V6/V7
    let flags = if has_flags {
        let mut flags_byte = [0u8; 1];
        reader.read_exact(&mut flags_byte)?;
        flags_byte[0]
    } else {
        0
    };
    let key_file_required = flags & FLAG_KEY_FILE_USED != 0;

    // If the file was encrypted with a key file, ensure one is provided
    if key_file_required && key_file_path.is_none() {
        return Err(CryptoError::KeyFileRequired);
    }

    // Build header for AAD (must match what was used during encryption)
    let compression_config = compression_algorithm.map(|alg| CompressionConfig {
        algorithm: alg,
        level: compression_level,
    });
    let header = build_header(&HeaderParams {
        version: version[0],
        kdf_params: &kdf_params,
        salt: &salt,
        base_nonce: &base_nonce,
        chunk_size,
        total_chunks,
        compression: compression_config.as_ref(),
        original_size,
        flags: if has_flags { Some(flags) } else { None },
    });
    let header_aad = header.as_slice();

    // Derive key (with optional key file)
    let key = if key_file_required {
        let kf_path = key_file_path.unwrap(); // Safe: checked above
        let kf_hash = hash_key_file(kf_path)?;
        let combined = combine_password_and_keyfile(password.as_bytes(), kf_hash.as_slice());
        derive_key_with_material(combined.as_slice(), &salt, &kdf_params)?
    } else {
        derive_key_with_params(password, &salt, &kdf_params)?
    };
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
    let max_ciphertext_chunk_len = max_ciphertext_len(
        chunk_size,
        if has_compression {
            compression_algorithm
        } else {
            None
        },
    )?;
    let mut plaintext_written: u64 = 0;

    for chunk_index in 0..total_chunks {
        // Read chunk length
        let mut chunk_len_bytes = [0u8; 4];
        reader.read_exact(&mut chunk_len_bytes)?;
        let chunk_len = u32::from_le_bytes(chunk_len_bytes) as usize;

        // Strict chunk length validation
        if chunk_len > max_ciphertext_chunk_len {
            return Err(CryptoError::FormatError(format!(
                "Invalid chunk length: {} bytes (max {} for chunk_size {})",
                chunk_len, max_ciphertext_chunk_len, chunk_size
            )));
        }

        // Read encrypted chunk
        let mut ciphertext = vec![0u8; chunk_len];
        reader.read_exact(&mut ciphertext)?;

        // Derive chunk nonce
        let chunk_nonce = derive_chunk_nonce(&base_nonce, chunk_index);
        let nonce = Nonce::from_slice(&chunk_nonce);

        // Decrypt chunk
        let decrypted = cipher
            .decrypt(
                nonce,
                Payload {
                    msg: ciphertext.as_ref(),
                    aad: header_aad,
                },
            )
            .map_err(|_| CryptoError::InvalidPassword)?;

        let expected_plaintext_len = if has_compression {
            let remaining = original_size.saturating_sub(plaintext_written);
            std::cmp::min(chunk_size as u64, remaining) as usize
        } else {
            chunk_size
        };

        // Decompress (or validate) with a hard output size cap.
        let plaintext = if let Some(alg) = compression_algorithm {
            decompress_with_limit(&decrypted, alg, expected_plaintext_len)?
        } else {
            if decrypted.len() > expected_plaintext_len {
                return Err(CryptoError::FormatError(format!(
                    "Decrypted chunk exceeds expected size (max {} bytes)",
                    expected_plaintext_len
                )));
            }
            decrypted
        };

        // Write plaintext
        writer.write_all(&plaintext)?;
        plaintext_written = plaintext_written.saturating_add(plaintext.len() as u64);

        // Track encrypted bytes processed (excludes header and per-chunk length fields).
        bytes_processed += chunk_len as u64;

        // Call progress callback
        if let Some(ref callback) = progress_callback {
            callback(bytes_processed, file_size);
        }
    }

    if has_compression && plaintext_written != original_size {
        return Err(CryptoError::FormatError(format!(
            "Decrypted size mismatch: {} bytes (expected {})",
            plaintext_written, original_size
        )));
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

struct HeaderParams<'a> {
    version: u8,
    kdf_params: &'a KdfParams,
    salt: &'a [u8],
    base_nonce: &'a [u8; NONCE_SIZE],
    chunk_size: usize,
    total_chunks: u64,
    compression: Option<&'a CompressionConfig>,
    original_size: u64,
    /// Flags byte for V6/V7. None for V4/V5.
    flags: Option<u8>,
}

fn build_header(params: &HeaderParams<'_>) -> Vec<u8> {
    let mut capacity = HEADER_V4_FIXED_SIZE + params.salt.len();
    if params.compression.is_some() {
        capacity += COMPRESSION_FIELDS_SIZE;
    }
    if params.flags.is_some() {
        capacity += FLAGS_SIZE;
    }
    let mut header = Vec::with_capacity(capacity);

    // Common header fields (all versions)
    header.push(params.version);
    header.extend_from_slice(&(params.salt.len() as u32).to_le_bytes());
    header.push(params.kdf_params.algorithm.to_u8());
    header.extend_from_slice(&params.kdf_params.memory_cost_kib.to_le_bytes());
    header.extend_from_slice(&params.kdf_params.time_cost.to_le_bytes());
    header.extend_from_slice(&params.kdf_params.parallelism.to_le_bytes());
    header.extend_from_slice(&params.kdf_params.key_length.to_le_bytes());
    header.extend_from_slice(params.salt);
    header.extend_from_slice(params.base_nonce);
    header.extend_from_slice(&(params.chunk_size as u32).to_le_bytes());
    header.extend_from_slice(&params.total_chunks.to_le_bytes());

    // V5/V7 compression fields
    if let Some(config) = params.compression {
        header.push(config.algorithm.to_u8());
        header.push(config.level as u8);
        header.extend_from_slice(&params.original_size.to_le_bytes());
    }

    // V6/V7 flags byte
    if let Some(flags) = params.flags {
        header.push(flags);
    }

    header
}

fn max_ciphertext_len(
    chunk_size: usize,
    compression: Option<CompressionAlgorithm>,
) -> CryptoResult<usize> {
    let max_payload_len = match compression {
        Some(CompressionAlgorithm::Zstd) => zstd_safe::compress_bound(chunk_size),
        _ => chunk_size,
    };
    max_payload_len.checked_add(TAG_SIZE).ok_or_else(|| {
        CryptoError::FormatError("Chunk size too large to compute ciphertext bound".to_string())
    })
}

/// Check if a file should use streaming encryption based on size
///
/// Returns true if the file is larger than the threshold (default: 10MB)
///
/// # Deprecated
/// This function is a legacy utility. As of the current implementation,
/// all files use streaming encryption regardless of size for consistent
/// behavior and optimal memory usage. This function is retained for
/// potential future use cases where size-based decisions may be needed.
#[allow(dead_code)]
pub fn should_use_streaming(file_size: u64, threshold: u64) -> bool {
    file_size > threshold
}

/// Default threshold for automatic streaming (10 MB)
///
/// # Note
/// This constant is retained for potential future use. Currently, all files
/// use streaming encryption regardless of size.
#[allow(dead_code)]
pub const STREAMING_THRESHOLD: u64 = 10 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::kdf::KdfParams;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::NamedTempFile;

    fn test_password() -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        format!("{now_nanos:x}{counter:x}")
    }

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

        // Encrypt (no compression - V4)
        let encrypted_path = temp_dir.path().join("encrypted.bin");
        let password = Password::new(test_password());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            1024, // Small chunk size for testing
            None,
            false,
            None, // No compression
            None, // No key file
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
            None,
        )
        .unwrap();

        // Verify content matches
        let decrypted_content = fs::read(&decrypted_path).unwrap();
        assert_eq!(content, decrypted_content.as_slice());
    }

    #[test]
    fn test_streaming_encrypt_decrypt_with_compression() {
        // Create a temp directory for output files
        let temp_dir = tempfile::tempdir().unwrap();

        // Create a test file with compressible content
        let content = b"Hello, streaming encryption! ".repeat(100);
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), &content).unwrap();

        // Encrypt with compression (V5)
        let encrypted_path = temp_dir.path().join("encrypted_compressed.bin");
        let password = Password::new(test_password());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            1024,
            None,
            false,
            Some(CompressionConfig::default()), // ZSTD level 3
            None,                               // No key file
        )
        .unwrap();

        // Verify encrypted file is V5
        let encrypted_data = fs::read(&encrypted_path).unwrap();
        assert_eq!(encrypted_data[0], STREAMING_VERSION_V5);

        // Decrypt
        let decrypted_path = temp_dir.path().join("decrypted.bin");
        decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &password,
            None,
            false,
            None,
        )
        .unwrap();

        // Verify content matches
        let decrypted_content = fs::read(&decrypted_path).unwrap();
        assert_eq!(content.to_vec(), decrypted_content);
    }

    #[test]
    fn test_streaming_compression_small_chunk_size_roundtrip() {
        // Ensure very small chunk sizes still decrypt correctly with compression enabled.
        let temp_dir = tempfile::tempdir().unwrap();

        let content = b"a";
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), content).unwrap();

        let encrypted_path = temp_dir.path().join("encrypted_small_chunk.bin");
        let password = Password::new(test_password());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            1,
            None,
            false,
            Some(CompressionConfig::default()),
            None, // No key file
        )
        .unwrap();

        let decrypted_path = temp_dir.path().join("decrypted_small_chunk.bin");
        decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &password,
            None,
            false,
            None,
        )
        .unwrap();

        let decrypted_content = fs::read(&decrypted_path).unwrap();
        assert_eq!(content.to_vec(), decrypted_content);
    }

    #[test]
    fn test_streaming_empty_file_roundtrip() {
        // Empty inputs should still authenticate (we store a single empty chunk + tag).
        let temp_dir = tempfile::tempdir().unwrap();

        let input_file = NamedTempFile::new().unwrap(); // Empty by default

        let encrypted_path = temp_dir.path().join("encrypted_empty.bin");
        let password = Password::new(test_password());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            1024,
            None,
            false,
            None,
            None,
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
            None,
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
        let correct_password = Password::new(test_password());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &correct_password,
            1024,
            None,
            false,
            None,
            None,
        )
        .unwrap();

        // Try to decrypt with wrong password
        let decrypted_path = temp_dir.path().join("decrypted.bin");
        let mut wrong_password_value = test_password();
        while wrong_password_value == correct_password.as_str() {
            wrong_password_value = test_password();
        }
        let wrong_password = Password::new(wrong_password_value);
        let result = decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &wrong_password,
            None,
            false,
            None,
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::InvalidPassword)));
    }

    #[test]
    fn test_streaming_empty_password() {
        let input_file = NamedTempFile::new().unwrap();
        let output_file = NamedTempFile::new().unwrap();

        let empty_password = Password::new(String::new());
        let result = encrypt_file_streaming(
            input_file.path(),
            output_file.path(),
            &empty_password,
            DEFAULT_CHUNK_SIZE,
            None,
            false,
            None,
            None,
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
        let header = build_header(&HeaderParams {
            version: STREAMING_VERSION,
            kdf_params: &kdf_params,
            salt: &salt,
            base_nonce: &base_nonce,
            chunk_size: 0,
            total_chunks: 0,
            compression: None,
            original_size: 0,
            flags: None,
        });
        fs::write(&encrypted_path, header).unwrap();

        let password = Password::new(test_password());
        let result =
            decrypt_file_streaming(&encrypted_path, &output_path, &password, None, false, None);
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
        let header = build_header(&HeaderParams {
            version: STREAMING_VERSION,
            kdf_params: &kdf_params,
            salt: &salt,
            base_nonce: &base_nonce,
            chunk_size: MAX_CHUNK_SIZE + 1,
            total_chunks: 0,
            compression: None,
            original_size: 0,
            flags: None,
        });
        fs::write(&encrypted_path, header).unwrap();

        let password = Password::new(test_password());
        let result =
            decrypt_file_streaming(&encrypted_path, &output_path, &password, None, false, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_rejects_v5_chunk_expansion() {
        let temp_dir = tempfile::tempdir().unwrap();
        let encrypted_path = temp_dir.path().join("bad_v5_expand.bin");
        let output_path = temp_dir.path().join("out_v5_expand.bin");

        let chunk_size = 1024;
        let total_chunks = 1u64;
        let original_size = 512u64; // Smaller than actual plaintext.

        let kdf_params = KdfParams::default();
        let salt = vec![1u8; kdf_params.salt_length as usize];
        let base_nonce = [2u8; NONCE_SIZE];
        let compression_config = CompressionConfig::default();

        let header = build_header(&HeaderParams {
            version: STREAMING_VERSION_V5,
            kdf_params: &kdf_params,
            salt: &salt,
            base_nonce: &base_nonce,
            chunk_size,
            total_chunks,
            compression: Some(&compression_config),
            original_size,
            flags: None,
        });

        let password = Password::new(test_password());
        let key = derive_key_with_params(&password, &salt, &kdf_params).unwrap();
        let cipher = Aes256Gcm::new_from_slice(key.as_slice()).unwrap();

        let plaintext = vec![b'A'; chunk_size];
        let compressed = compress(&plaintext, &compression_config).unwrap();

        let chunk_nonce = derive_chunk_nonce(&base_nonce, 0);
        let nonce = Nonce::from_slice(&chunk_nonce);
        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: &compressed,
                    aad: &header,
                },
            )
            .unwrap();

        let mut file_bytes = Vec::new();
        file_bytes.extend_from_slice(&header);
        file_bytes.extend_from_slice(&(ciphertext.len() as u32).to_le_bytes());
        file_bytes.extend_from_slice(&ciphertext);
        fs::write(&encrypted_path, file_bytes).unwrap();

        let result =
            decrypt_file_streaming(&encrypted_path, &output_path, &password, None, false, None);
        assert!(matches!(result, Err(CryptoError::FormatError(_))));
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
        let password = Password::new(test_password());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            chunk_size,
            None,
            false,
            None,
            None,
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
            None,
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

    #[test]
    fn test_streaming_v6_keyfile_roundtrip() {
        // Test V6: no compression + key file
        let temp_dir = tempfile::tempdir().unwrap();
        let content = b"Secret data with key file protection";
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), content).unwrap();

        // Generate a key file
        let key_file_path = temp_dir.path().join("test.key");
        crate::crypto::keyfile::generate_key_file(&key_file_path).unwrap();

        let encrypted_path = temp_dir.path().join("encrypted_v6.bin");
        let password = Password::new(test_password());

        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            1024,
            None,
            false,
            None, // No compression
            Some(key_file_path.as_path()),
        )
        .unwrap();

        // Verify V6 format
        let encrypted_data = fs::read(&encrypted_path).unwrap();
        assert_eq!(encrypted_data[0], STREAMING_VERSION_V6);

        // Decrypt with key file
        let decrypted_path = temp_dir.path().join("decrypted_v6.bin");
        decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &password,
            None,
            false,
            Some(key_file_path.as_path()),
        )
        .unwrap();

        let decrypted_content = fs::read(&decrypted_path).unwrap();
        assert_eq!(content.to_vec(), decrypted_content);
    }

    #[test]
    fn test_streaming_v7_keyfile_compression_roundtrip() {
        // Test V7: compression + key file
        let temp_dir = tempfile::tempdir().unwrap();
        let content = b"Compressible content ".repeat(100);
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), &content).unwrap();

        let key_file_path = temp_dir.path().join("test.key");
        crate::crypto::keyfile::generate_key_file(&key_file_path).unwrap();

        let encrypted_path = temp_dir.path().join("encrypted_v7.bin");
        let password = Password::new(test_password());

        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            1024,
            None,
            false,
            Some(CompressionConfig::default()),
            Some(key_file_path.as_path()),
        )
        .unwrap();

        // Verify V7 format
        let encrypted_data = fs::read(&encrypted_path).unwrap();
        assert_eq!(encrypted_data[0], STREAMING_VERSION_V7);

        // Decrypt with key file
        let decrypted_path = temp_dir.path().join("decrypted_v7.bin");
        decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &password,
            None,
            false,
            Some(key_file_path.as_path()),
        )
        .unwrap();

        let decrypted_content = fs::read(&decrypted_path).unwrap();
        assert_eq!(content.to_vec(), decrypted_content);
    }

    #[test]
    fn test_streaming_keyfile_required_error() {
        // Encrypt with key file, then try to decrypt without it
        let temp_dir = tempfile::tempdir().unwrap();
        let content = b"Secret data";
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), content).unwrap();

        let key_file_path = temp_dir.path().join("test.key");
        crate::crypto::keyfile::generate_key_file(&key_file_path).unwrap();

        let encrypted_path = temp_dir.path().join("encrypted.bin");
        let password = Password::new(test_password());

        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            1024,
            None,
            false,
            None,
            Some(key_file_path.as_path()),
        )
        .unwrap();

        // Try to decrypt without key file -> KeyFileRequired
        let decrypted_path = temp_dir.path().join("decrypted.bin");
        let result = decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &password,
            None,
            false,
            None, // No key file provided
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::KeyFileRequired)));
    }

    #[test]
    fn test_streaming_wrong_keyfile() {
        // Encrypt with one key file, decrypt with different key file
        let temp_dir = tempfile::tempdir().unwrap();
        let content = b"Secret data";
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), content).unwrap();

        let key_file_1 = temp_dir.path().join("key1.key");
        let key_file_2 = temp_dir.path().join("key2.key");
        crate::crypto::keyfile::generate_key_file(&key_file_1).unwrap();
        crate::crypto::keyfile::generate_key_file(&key_file_2).unwrap();

        let encrypted_path = temp_dir.path().join("encrypted.bin");
        let password = Password::new(test_password());

        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &password,
            1024,
            None,
            false,
            None,
            Some(key_file_1.as_path()),
        )
        .unwrap();

        // Decrypt with wrong key file -> InvalidPassword
        let decrypted_path = temp_dir.path().join("decrypted.bin");
        let result = decrypt_file_streaming(
            &encrypted_path,
            &decrypted_path,
            &password,
            None,
            false,
            Some(key_file_2.as_path()), // Wrong key file
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(CryptoError::InvalidPassword)));
    }

    #[test]
    fn test_streaming_v4_v5_backward_compatibility() {
        // Ensure V4/V5 files still decrypt with key_file_path=None
        let temp_dir = tempfile::tempdir().unwrap();
        let content = b"Backward compatibility test";
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), content).unwrap();

        let password = Password::new(test_password());

        // V4 (no compression, no key file)
        let encrypted_v4 = temp_dir.path().join("encrypted_v4.bin");
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_v4,
            &password,
            1024,
            None,
            false,
            None,
            None,
        )
        .unwrap();
        assert_eq!(fs::read(&encrypted_v4).unwrap()[0], STREAMING_VERSION_V4);

        let decrypted_v4 = temp_dir.path().join("decrypted_v4.bin");
        decrypt_file_streaming(&encrypted_v4, &decrypted_v4, &password, None, false, None).unwrap();
        assert_eq!(fs::read(&decrypted_v4).unwrap(), content);

        // V5 (compression, no key file)
        let encrypted_v5 = temp_dir.path().join("encrypted_v5.bin");
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_v5,
            &password,
            1024,
            None,
            false,
            Some(CompressionConfig::default()),
            None,
        )
        .unwrap();
        assert_eq!(fs::read(&encrypted_v5).unwrap()[0], STREAMING_VERSION_V5);

        let decrypted_v5 = temp_dir.path().join("decrypted_v5.bin");
        decrypt_file_streaming(&encrypted_v5, &decrypted_v5, &password, None, false, None).unwrap();
        assert_eq!(fs::read(&decrypted_v5).unwrap(), content);
    }

    // ---------------------------------------------------------------
    // Helper: encrypt test content and return raw encrypted file bytes
    // ---------------------------------------------------------------
    fn encrypt_test_file(content: &[u8], password: &str, chunk_size: usize) -> Vec<u8> {
        let output_dir = tempfile::tempdir().unwrap();
        let input_file = NamedTempFile::new().unwrap();
        fs::write(input_file.path(), content).unwrap();

        let encrypted_path = output_dir.path().join("encrypted.bin");
        let pw = Password::new(password.to_string());
        encrypt_file_streaming(
            input_file.path(),
            &encrypted_path,
            &pw,
            chunk_size,
            None,
            false,
            None,
            None,
        )
        .unwrap();

        fs::read(&encrypted_path).unwrap()
    }

    /// Try to decrypt raw bytes; returns the CryptoResult.
    fn try_decrypt_bytes(data: &[u8], password: &str) -> CryptoResult<Vec<u8>> {
        let temp_dir = tempfile::tempdir().unwrap();
        let enc_path = temp_dir.path().join("tampered.bin");
        fs::write(&enc_path, data).unwrap();

        let dec_path = temp_dir.path().join("decrypted.bin");
        let pw = Password::new(password.to_string());
        decrypt_file_streaming(&enc_path, &dec_path, &pw, None, false, None)?;
        Ok(fs::read(&dec_path).unwrap())
    }

    // ---------------------------------------------------------------
    // Header tampering tests
    // ---------------------------------------------------------------

    #[test]
    fn test_tamper_version_byte() {
        let password = test_password();
        let data = encrypt_test_file(b"hello world", &password, 1024);
        assert_eq!(data[0], STREAMING_VERSION_V4);

        // Set version to an unsupported value
        let mut tampered = data.clone();
        tampered[0] = 99;
        let result = try_decrypt_bytes(&tampered, &password);
        assert!(
            matches!(result, Err(CryptoError::FormatError(ref msg)) if msg.contains("Unsupported file format version")),
            "Expected FormatError for invalid version, got: {:?}",
            result
        );
    }

    #[test]
    fn test_tamper_salt_bytes() {
        let password = test_password();
        let data = encrypt_test_file(b"hello world", &password, 1024);

        // Salt starts at offset 22 (after VERSION:1 + SALT_LEN:4 + KDF_PARAMS:17)
        let salt_offset = VERSION_SIZE + SALT_LEN_SIZE + KDF_PARAMS_SIZE;

        let mut tampered = data.clone();
        tampered[salt_offset] ^= 0xFF; // flip bits in first salt byte
        let result = try_decrypt_bytes(&tampered, &password);
        // Corrupted salt -> different key -> AEAD tag mismatch -> InvalidPassword
        assert!(
            matches!(result, Err(CryptoError::InvalidPassword)),
            "Expected InvalidPassword for corrupted salt, got: {:?}",
            result
        );
    }

    #[test]
    fn test_tamper_base_nonce() {
        let password = test_password();
        let data = encrypt_test_file(b"hello world", &password, 1024);

        // Base nonce follows salt: offset = 22 + salt_len (16 for default)
        let kdf = KdfParams::default();
        let nonce_offset =
            VERSION_SIZE + SALT_LEN_SIZE + KDF_PARAMS_SIZE + kdf.salt_length as usize;

        let mut tampered = data.clone();
        tampered[nonce_offset] ^= 0xFF;
        let result = try_decrypt_bytes(&tampered, &password);
        // Corrupted nonce -> wrong chunk nonces AND wrong AAD -> AEAD failure
        assert!(
            matches!(result, Err(CryptoError::InvalidPassword)),
            "Expected InvalidPassword for corrupted nonce, got: {:?}",
            result
        );
    }

    #[test]
    fn test_tamper_kdf_mem_cost() {
        let password = test_password();
        let data = encrypt_test_file(b"hello world", &password, 1024);

        // KDF mem_cost is at offset 6..10 (after VERSION:1 + SALT_LEN:4 + KDF_ALG:1)
        let mem_cost_offset = VERSION_SIZE + SALT_LEN_SIZE + 1; // 6

        let mut tampered = data.clone();
        // Change mem_cost to a small invalid value (avoids memory allocation)
        let new_val = 1u32;
        tampered[mem_cost_offset..mem_cost_offset + 4].copy_from_slice(&new_val.to_le_bytes());

        let result = try_decrypt_bytes(&tampered, &password);
        // Tampered KDF params -> either rejected by validation (FormatError) or wrong key (InvalidPassword)
        assert!(
            matches!(
                result,
                Err(CryptoError::InvalidPassword) | Err(CryptoError::FormatError(_))
            ),
            "Expected InvalidPassword or FormatError for corrupted KDF mem_cost, got: {:?}",
            result
        );
    }

    #[test]
    fn test_tamper_kdf_time_cost() {
        let password = test_password();
        let data = encrypt_test_file(b"hello world", &password, 1024);

        // KDF time_cost at offset 10..14
        let time_cost_offset = VERSION_SIZE + SALT_LEN_SIZE + 1 + 4; // 10

        let mut tampered = data.clone();
        let orig = u32::from_le_bytes(
            tampered[time_cost_offset..time_cost_offset + 4]
                .try_into()
                .unwrap(),
        );
        let new_val = orig + 1;
        tampered[time_cost_offset..time_cost_offset + 4].copy_from_slice(&new_val.to_le_bytes());

        let result = try_decrypt_bytes(&tampered, &password);
        assert!(
            matches!(result, Err(CryptoError::InvalidPassword)),
            "Expected InvalidPassword for corrupted KDF time_cost, got: {:?}",
            result
        );
    }

    #[test]
    fn test_tamper_chunk_ciphertext() {
        let password = test_password();
        let data = encrypt_test_file(b"hello world", &password, 1024);

        // Flip a byte in the ciphertext (last byte of the file, part of chunk data)
        let mut tampered = data.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0xFF;
        let result = try_decrypt_bytes(&tampered, &password);
        assert!(
            matches!(result, Err(CryptoError::InvalidPassword)),
            "Expected InvalidPassword for corrupted ciphertext, got: {:?}",
            result
        );
    }

    #[test]
    fn test_tamper_chunk_length_field() {
        let password = test_password();
        let data = encrypt_test_file(b"hello world", &password, 1024);

        // The chunk length field is right after the header.
        // Header size for V4 = HEADER_V4_FIXED_SIZE + salt_len
        let kdf = KdfParams::default();
        let header_size = HEADER_V4_FIXED_SIZE + kdf.salt_length as usize;

        let mut tampered = data.clone();
        // Set chunk length to something huge (but within file bounds won't match)
        tampered[header_size] = 0xFF;
        tampered[header_size + 1] = 0xFF;
        let result = try_decrypt_bytes(&tampered, &password);
        // Either FormatError (invalid chunk length) or Io (unexpected EOF)
        assert!(
            result.is_err(),
            "Expected error for corrupted chunk length, got: {:?}",
            result
        );
        match result {
            Err(CryptoError::FormatError(_)) | Err(CryptoError::Io(_)) => {} // expected
            other => panic!("Expected FormatError or Io, got: {:?}", other),
        }
    }

    // ---------------------------------------------------------------
    // Truncated file tests
    // ---------------------------------------------------------------

    #[test]
    fn test_truncated_empty_file() {
        let password = test_password();
        let result = try_decrypt_bytes(&[], &password);
        assert!(
            matches!(result, Err(CryptoError::Io(_))),
            "Expected Io error for empty file, got: {:?}",
            result
        );
    }

    #[test]
    fn test_truncated_mid_header_version_only() {
        // File contains only the version byte, nothing else
        let password = test_password();
        let result = try_decrypt_bytes(&[STREAMING_VERSION_V4], &password);
        assert!(
            matches!(result, Err(CryptoError::Io(_))),
            "Expected Io error for truncated header (version only), got: {:?}",
            result
        );
    }

    #[test]
    fn test_truncated_mid_header_partial_kdf() {
        let password = test_password();
        let data = encrypt_test_file(b"test data", &password, 1024);

        // Truncate in the middle of the KDF parameters (e.g., 10 bytes in)
        let truncated = &data[..10];
        let result = try_decrypt_bytes(truncated, &password);
        assert!(
            matches!(result, Err(CryptoError::Io(_))),
            "Expected Io error for truncation mid-KDF params, got: {:?}",
            result
        );
    }

    #[test]
    fn test_truncated_mid_header_before_nonce() {
        let password = test_password();
        let data = encrypt_test_file(b"test data", &password, 1024);

        // Truncate just before the base nonce (after salt)
        let kdf = KdfParams::default();
        let nonce_offset =
            VERSION_SIZE + SALT_LEN_SIZE + KDF_PARAMS_SIZE + kdf.salt_length as usize;
        let truncated = &data[..nonce_offset];
        let result = try_decrypt_bytes(truncated, &password);
        assert!(
            matches!(result, Err(CryptoError::Io(_))),
            "Expected Io error for truncation before nonce, got: {:?}",
            result
        );
    }

    #[test]
    fn test_truncated_header_complete_but_no_chunks() {
        let password = test_password();
        let data = encrypt_test_file(b"test data", &password, 1024);

        // Truncate right at end of header (no chunk data at all)
        let kdf = KdfParams::default();
        let header_size = HEADER_V4_FIXED_SIZE + kdf.salt_length as usize;
        let truncated = &data[..header_size];
        let result = try_decrypt_bytes(truncated, &password);
        // Will try to read chunk length field and fail with Io (UnexpectedEof)
        assert!(
            matches!(result, Err(CryptoError::Io(_))),
            "Expected Io error for header-only file, got: {:?}",
            result
        );
    }

    #[test]
    fn test_truncated_mid_chunk_data() {
        let password = test_password();
        let data = encrypt_test_file(b"hello world", &password, 1024);

        // Truncate in the middle of the chunk ciphertext (remove last 5 bytes)
        let truncated = &data[..data.len() - 5];
        let result = try_decrypt_bytes(truncated, &password);
        // read_exact for chunk ciphertext will fail with UnexpectedEof
        assert!(
            matches!(result, Err(CryptoError::Io(_))),
            "Expected Io error for truncation mid-chunk, got: {:?}",
            result
        );
    }

    #[test]
    fn test_truncated_between_chunks() {
        let password = test_password();
        // Create multi-chunk file: 3 chunks of 64 bytes each
        let content: Vec<u8> = (0..192).map(|i| (i % 256) as u8).collect();
        let data = encrypt_test_file(&content, &password, 64);

        // Find where second chunk starts and truncate there
        let kdf = KdfParams::default();
        let header_size = HEADER_V4_FIXED_SIZE + kdf.salt_length as usize;

        // Read first chunk length to find boundary
        let chunk1_len =
            u32::from_le_bytes(data[header_size..header_size + 4].try_into().unwrap()) as usize;
        let after_chunk1 = header_size + 4 + chunk1_len;

        // Truncate right after first chunk (before second chunk's length field)
        let truncated = &data[..after_chunk1];
        let result = try_decrypt_bytes(truncated, &password);
        assert!(
            matches!(result, Err(CryptoError::Io(_))),
            "Expected Io error for truncation between chunks, got: {:?}",
            result
        );
    }

    #[test]
    fn test_truncated_at_chunk_length_field() {
        let password = test_password();
        let content: Vec<u8> = (0..192).map(|i| (i % 256) as u8).collect();
        let data = encrypt_test_file(&content, &password, 64);

        let kdf = KdfParams::default();
        let header_size = HEADER_V4_FIXED_SIZE + kdf.salt_length as usize;

        // Read first chunk length
        let chunk1_len =
            u32::from_le_bytes(data[header_size..header_size + 4].try_into().unwrap()) as usize;
        let chunk2_len_offset = header_size + 4 + chunk1_len;

        // Truncate in the middle of the second chunk's length field (2 of 4 bytes)
        let truncated = &data[..chunk2_len_offset + 2];
        let result = try_decrypt_bytes(truncated, &password);
        assert!(
            matches!(result, Err(CryptoError::Io(_))),
            "Expected Io error for truncation at chunk length field, got: {:?}",
            result
        );
    }
}
