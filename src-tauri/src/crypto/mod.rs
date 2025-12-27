// crypto/mod.rs - Cryptography Module
//
// This module provides all cryptographic operations for FileCypter.
// It exports a clean API for file encryption and decryption.

mod cipher;
mod format;
mod kdf;
mod secure;
pub mod streaming;

// Re-export the main types and functions for easy access
pub use cipher::{decrypt, encrypt};
pub use format::EncryptedFile;
pub use kdf::{derive_key, generate_salt};
pub use secure::{Password, SecureBytes};
pub use streaming::{
    decrypt_file_streaming, encrypt_file_streaming, should_use_streaming, DEFAULT_CHUNK_SIZE,
    STREAMING_THRESHOLD,
};
