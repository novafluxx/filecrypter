// crypto/mod.rs - Cryptography Module
//
// This module provides all cryptographic operations for FileCrypter.
// It exports a clean API for file encryption and decryption.

mod cipher;
pub mod compression;
mod kdf;
pub mod keyfile;
mod secure;
pub mod streaming;

// Re-export the main types and functions for easy access
pub use cipher::{decrypt, encrypt};
pub use compression::{compress, decompress, CompressionAlgorithm, CompressionConfig};
pub use kdf::{
    derive_key, derive_key_with_material, derive_key_with_params, generate_salt,
    generate_salt_with_len, KdfAlgorithm, KdfParams,
};
pub use keyfile::{combine_password_and_keyfile, generate_key_file, hash_key_file};
pub use secure::{Password, SecureBytes};
pub use streaming::{decrypt_file_streaming, encrypt_file_streaming, DEFAULT_CHUNK_SIZE};
