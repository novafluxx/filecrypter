// crypto/compression.rs - Compression Module
//
// This module provides ZSTD compression/decompression for FileCrypter.
// Compression is applied before encryption (compress-then-encrypt) to
// reduce file size while maintaining security.
//
// ## Design Decisions
//
// **Algorithm: ZSTD (Zstandard)**
// - Modern compression algorithm with excellent speed/ratio tradeoff
// - Level 3 provides ~70% reduction at ~100 MB/s compression speed
// - Streaming API for efficient memory usage
//
// **Compress-Then-Encrypt (CTE)**
// - Encrypted data is indistinguishable from random and cannot be compressed
// - Compression must happen before encryption to be effective
// - Safe here because FileCrypter has no adaptive chosen-plaintext oracle
//
// ## Security Considerations
//
// - Compressed size may leak information about plaintext patterns
// - This is acceptable for file encryption (no compression oracle attacks)
// - AES-GCM authentication prevents tampering with compressed data

use std::io::{BufReader, Cursor, Read};

use crate::error::{CryptoError, CryptoResult};

/// Compression algorithms supported by FileCrypter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionAlgorithm {
    /// No compression (backward compatible with Version 4)
    None = 0x00,
    /// ZSTD compression
    Zstd = 0x01,
}

impl CompressionAlgorithm {
    /// Convert from u8 byte (from file header)
    pub fn from_u8(value: u8) -> CryptoResult<Self> {
        match value {
            0x00 => Ok(CompressionAlgorithm::None),
            0x01 => Ok(CompressionAlgorithm::Zstd),
            _ => Err(CryptoError::FormatError(format!(
                "Unknown compression algorithm: 0x{:02x}",
                value
            ))),
        }
    }

    /// Convert to u8 byte (for file header)
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Default ZSTD compression level (balanced speed/ratio)
pub const DEFAULT_COMPRESSION_LEVEL: i32 = 3;

/// Configuration for compression operations
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Compression algorithm to use
    pub algorithm: CompressionAlgorithm,
    /// Compression level (0-22 for ZSTD, 3 is recommended)
    pub level: i32,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Zstd,
            level: DEFAULT_COMPRESSION_LEVEL,
        }
    }
}

impl CompressionConfig {
    /// Create a new compression config with ZSTD at the specified level
    pub fn new(level: i32) -> Self {
        Self {
            algorithm: CompressionAlgorithm::Zstd,
            level,
        }
    }

    /// Create a config for no compression
    pub fn none() -> Self {
        Self {
            algorithm: CompressionAlgorithm::None,
            level: 0,
        }
    }

    /// Check if compression is enabled
    pub fn is_enabled(&self) -> bool {
        self.algorithm != CompressionAlgorithm::None
    }
}

/// Compress data using ZSTD
///
/// # Arguments
/// * `data` - Raw data to compress
/// * `level` - Compression level (1-22, default 3)
///
/// # Returns
/// Compressed data as Vec<u8>
pub fn compress_zstd(data: &[u8], level: i32) -> CryptoResult<Vec<u8>> {
    zstd::encode_all(data, level)
        .map_err(|e| CryptoError::FormatError(format!("Compression failed: {}", e)))
}

/// Decompress ZSTD-compressed data
///
/// # Arguments
/// * `data` - Compressed data
///
/// # Returns
/// Decompressed data as Vec<u8>
pub fn decompress_zstd(data: &[u8]) -> CryptoResult<Vec<u8>> {
    zstd::decode_all(data)
        .map_err(|e| CryptoError::FormatError(format!("Decompression failed: {}", e)))
}

/// Decompress ZSTD-compressed data with a hard output size limit
///
/// # Arguments
/// * `data` - Compressed data
/// * `max_size` - Maximum allowed decompressed size in bytes
///
/// # Returns
/// Decompressed data as Vec<u8>
pub fn decompress_zstd_with_limit(data: &[u8], max_size: usize) -> CryptoResult<Vec<u8>> {
    let cursor = Cursor::new(data);
    let mut decoder = zstd::Decoder::new(BufReader::new(cursor))
        .map_err(|e| CryptoError::FormatError(format!("Failed to create decompressor: {}", e)))?;
    let mut output = Vec::with_capacity(std::cmp::min(max_size, 64 * 1024));
    let mut buffer = [0u8; 8192];

    loop {
        let read = decoder
            .read(&mut buffer)
            .map_err(|e| CryptoError::FormatError(format!("Decompression failed: {}", e)))?;
        if read == 0 {
            break;
        }
        if output.len() + read > max_size {
            return Err(CryptoError::FormatError(format!(
                "Decompressed data exceeds expected size (max {} bytes)",
                max_size
            )));
        }
        output.extend_from_slice(&buffer[..read]);
    }

    Ok(output)
}

/// Compress data using the specified algorithm
///
/// # Arguments
/// * `data` - Raw data to compress
/// * `config` - Compression configuration
///
/// # Returns
/// Compressed data (or original data if compression disabled)
pub fn compress(data: &[u8], config: &CompressionConfig) -> CryptoResult<Vec<u8>> {
    match config.algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),
        CompressionAlgorithm::Zstd => compress_zstd(data, config.level),
    }
}

/// Decompress data using the specified algorithm
///
/// # Arguments
/// * `data` - Compressed data
/// * `algorithm` - Algorithm used for compression
///
/// # Returns
/// Decompressed data
pub fn decompress(data: &[u8], algorithm: CompressionAlgorithm) -> CryptoResult<Vec<u8>> {
    match algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),
        CompressionAlgorithm::Zstd => decompress_zstd(data),
    }
}

/// Decompress data using the specified algorithm with a hard output size limit
///
/// # Arguments
/// * `data` - Compressed data
/// * `algorithm` - Algorithm used for compression
/// * `max_size` - Maximum allowed decompressed size in bytes
///
/// # Returns
/// Decompressed data
pub fn decompress_with_limit(
    data: &[u8],
    algorithm: CompressionAlgorithm,
    max_size: usize,
) -> CryptoResult<Vec<u8>> {
    match algorithm {
        CompressionAlgorithm::None => {
            if data.len() > max_size {
                return Err(CryptoError::FormatError(format!(
                    "Decompressed data exceeds expected size (max {} bytes)",
                    max_size
                )));
            }
            Ok(data.to_vec())
        }
        CompressionAlgorithm::Zstd => decompress_zstd_with_limit(data, max_size),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn create_encoder<W: Write>(writer: W, level: i32) -> CryptoResult<zstd::Encoder<'static, W>> {
        zstd::Encoder::new(writer, level)
            .map_err(|e| CryptoError::FormatError(format!("Failed to create compressor: {}", e)))
    }

    fn create_decoder<R: Read>(
        reader: R,
    ) -> CryptoResult<zstd::Decoder<'static, std::io::BufReader<R>>> {
        zstd::Decoder::new(reader)
            .map_err(|e| CryptoError::FormatError(format!("Failed to create decompressor: {}", e)))
    }

    #[test]
    fn test_compression_algorithm_roundtrip() {
        assert_eq!(
            CompressionAlgorithm::from_u8(0x00).unwrap(),
            CompressionAlgorithm::None
        );
        assert_eq!(
            CompressionAlgorithm::from_u8(0x01).unwrap(),
            CompressionAlgorithm::Zstd
        );
        assert!(CompressionAlgorithm::from_u8(0xFF).is_err());
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original = b"Hello, this is test data for compression! ".repeat(100);
        let config = CompressionConfig::default();

        let compressed = compress(&original, &config).unwrap();
        let decompressed = decompress(&compressed, config.algorithm).unwrap();

        assert_eq!(original.to_vec(), decompressed);
        // Compressed should be smaller
        assert!(compressed.len() < original.len());
    }

    #[test]
    fn test_no_compression() {
        let original = b"Test data";
        let config = CompressionConfig::none();

        let result = compress(original, &config).unwrap();
        assert_eq!(original.to_vec(), result);

        let decompressed = decompress(&result, CompressionAlgorithm::None).unwrap();
        assert_eq!(original.to_vec(), decompressed);
    }

    #[test]
    fn test_compression_ratio() {
        // Highly compressible data
        let data = b"AAAAAAAAAA".repeat(1000);
        let compressed = compress_zstd(&data, 3).unwrap();

        // Should achieve significant compression
        assert!(compressed.len() < data.len() / 2);
    }

    #[test]
    fn test_empty_data() {
        let empty: &[u8] = &[];
        let config = CompressionConfig::default();

        let compressed = compress(empty, &config).unwrap();
        let decompressed = decompress(&compressed, config.algorithm).unwrap();

        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_streaming_encoder() {
        let data = b"Test data for streaming compression".repeat(100);
        let mut output = Vec::new();

        {
            let mut encoder = create_encoder(&mut output, 3).unwrap();
            encoder.write_all(&data).unwrap();
            encoder.finish().unwrap();
        }

        // Verify it can be decompressed
        let decompressed = decompress_zstd(&output).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_streaming_decoder() {
        let original = b"Test data for streaming decompression".repeat(100);
        let compressed = compress_zstd(&original, 3).unwrap();

        let cursor = std::io::Cursor::new(compressed);
        let mut decoder = create_decoder(cursor).unwrap();
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();

        assert_eq!(original.to_vec(), decompressed);
    }

    #[test]
    fn test_decompress_with_limit_rejects_oversize() {
        let original = b"0123456789".repeat(100);
        let compressed = compress_zstd(&original, 3).unwrap();
        let result =
            decompress_with_limit(&compressed, CompressionAlgorithm::Zstd, original.len() - 1);
        assert!(result.is_err());
    }
}
