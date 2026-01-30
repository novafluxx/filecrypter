// commands/keyfile.rs - Key File Generation Command
//
// This module provides the Tauri command for generating key files.
// Key files are used as a second authentication factor for file encryption.

use tauri::command;

use crate::commands::CryptoResponse;
use crate::error::CryptoResult;

/// Generate a key file containing 32 cryptographically random bytes.
///
/// # Arguments
/// * `output_path` - Path where the key file will be saved
///
/// # Returns
/// A success response with the output path
#[command]
pub async fn generate_key_file(output_path: String) -> CryptoResult<CryptoResponse> {
    log::info!("Generating key file: {}", output_path);

    let path = std::path::Path::new(&output_path);

    crate::crypto::keyfile::generate_key_file(path)?;

    Ok(CryptoResponse {
        message: format!("Key file generated successfully: {}", output_path),
        output_path,
    })
}
