// constants.ts - Centralized configuration values
//
// This file contains magic values used throughout the frontend codebase.
// Centralizing these values makes them easier to maintain and update.

/** Minimum required password length for encryption */
export const MIN_PASSWORD_LENGTH = 8;

/** File extension added to encrypted files */
export const ENCRYPTED_EXTENSION = '.encrypted';

/** File extension added when decrypting non-.encrypted files */
export const DECRYPTED_EXTENSION = '.decrypted';

/** Duration in ms to show success status messages before auto-hiding */
export const STATUS_SUCCESS_TIMEOUT_MS = 5000;

/** Default ZSTD compression level (balanced speed/ratio) */
export const DEFAULT_COMPRESSION_LEVEL = 3;

