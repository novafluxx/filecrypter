// events.rs - Tauri Event Definitions
//
// This module defines typed event payloads for frontend communication.
// Events are emitted during long-running operations to provide progress feedback.
//
// Tauri Event System:
// - Events are emitted from Rust using app_handle.emit()
// - Frontend listens using listen() from @tauri-apps/api/event
// - Payloads must implement Serialize to be converted to JSON

use serde::Serialize;

/// Progress event sent during encryption/decryption operations
///
/// This event is emitted at key stages during file processing:
/// - reading: Loading file from disk
/// - deriving_key: Argon2id key derivation (CPU-intensive, ~100-300ms)
/// - encrypting/decrypting: AES-256-GCM crypto operation
/// - complete: Operation finished successfully
///
/// # Frontend Usage
/// ```typescript
/// import { listen } from '@tauri-apps/api/event';
///
/// const unlisten = await listen<ProgressEvent>('crypto-progress', (event) => {
///   console.log(`${event.payload.stage}: ${event.payload.percent}%`);
/// });
/// ```
#[derive(Clone, Serialize, Debug)]
pub struct ProgressEvent {
    /// Current processing stage
    /// Values: "reading", "deriving_key", "encrypting", "decrypting", "complete"
    pub stage: String,

    /// Progress percentage (0-100)
    pub percent: u32,

    /// Human-readable status message for display
    pub message: String,
}

impl ProgressEvent {
    /// Create a new progress event
    ///
    /// # Arguments
    /// * `stage` - Current operation stage
    /// * `percent` - Progress percentage (0-100)
    /// * `message` - Human-readable status message
    ///
    /// # Example
    /// ```no_run
    /// use filecrypter_lib::events::ProgressEvent;
    /// let _event = ProgressEvent::new("deriving_key", 20, "Deriving encryption key...");
    /// ```
    pub fn new(stage: &str, percent: u32, message: &str) -> Self {
        Self {
            stage: stage.to_string(),
            percent,
            message: message.to_string(),
        }
    }

    // Convenience constructors for common lifecycle stages

    /// Create "reading file" progress event
    pub fn reading() -> Self {
        Self::new("reading", 0, "Reading file...")
    }

    /// Create "deriving key" progress event
    pub fn deriving_key() -> Self {
        Self::new(
            "deriving_key",
            20,
            "Deriving encryption key (this may take a moment)...",
        )
    }

    /// Create "complete" progress event for encryption
    pub fn encrypt_complete() -> Self {
        Self::new("complete", 100, "Encryption complete!")
    }

    /// Create "complete" progress event for decryption
    pub fn decrypt_complete() -> Self {
        Self::new("complete", 100, "Decryption complete!")
    }
}

/// Event name constant for crypto progress events
pub const CRYPTO_PROGRESS_EVENT: &str = "crypto-progress";
