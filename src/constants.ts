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

// ============================================================================
// Theme Constants
// ============================================================================
// These values are used by both CSS variables (in App.vue :root) and
// Naive UI theme overrides. Keep them in sync when modifying colors.

/** System font stack used throughout the app */
export const FONT_FAMILY = "system-ui, -apple-system, 'Segoe UI', 'Roboto', 'Ubuntu', 'Cantarell', 'Noto Sans', sans-serif";

/** Light theme accent colors */
export const LIGHT_THEME = {
  accent: '#0066cc',
  accentHover: '#0077ee',
  accentPressed: '#0055aa',
} as const;

/** Dark theme accent colors */
export const DARK_THEME = {
  accent: '#4a9eff',
  accentHover: '#5fb0ff',
  accentPressed: '#3d8ee8',
} as const;
