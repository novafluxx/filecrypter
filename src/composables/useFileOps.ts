// composables/useFileOps.ts - File Operation Logic
//
// This composable manages the state and logic for file encryption/decryption.
// It provides reactive state, computed properties for validation, and
// methods for handling file operations.
//
// Vue Composition API:
// - ref() creates reactive primitive values
// - computed() creates reactive derived values
// - This composable can be used in any component

import { ref, computed } from 'vue';
import type { StatusType } from '../types/crypto';
import { useTauri } from './useTauri';
import {
  MIN_PASSWORD_LENGTH,
  ENCRYPTED_EXTENSION,
  DECRYPTED_EXTENSION,
  STATUS_SUCCESS_TIMEOUT_MS,
  DEFAULT_COMPRESSION_LEVEL,
} from '../constants';
import { sanitizeErrorMessage } from '../utils/errorSanitizer';

/**
 * Composable for file encryption/decryption operations
 *
 * Provides reactive state management and validation logic
 * for encrypt and decrypt workflows.
 *
 * Features:
 * - Streaming encryption/decryption for all files (consistent behavior)
 * - Form validation (password strength, required fields)
 * - Status message handling with sanitized error messages
 * - Progress tracking via Tauri events
 * - Secure password handling (cleared after operations)
 *
 * All file operations use streaming (1MB chunks) regardless of file size,
 * providing optimal memory usage and support for files of any size.
 *
 * @returns {Object} Object containing state, validation, and operation methods
 */
export function useFileOps() {
  const { encryptFile, decryptFile } = useTauri();

  // ========== Reactive State ==========
  // ref() makes primitive values reactive in Vue

  const inputPath = ref('');
  const outputPath = ref('');
  const password = ref('');
  const neverOverwrite = ref(true);
  const compressionEnabled = ref(false); // Compression disabled by default for single files
  const compressionLevel = ref(DEFAULT_COMPRESSION_LEVEL); // ZSTD level 3 (balanced)
  const keyFilePath = ref('');
  const isProcessing = ref(false);
  const statusMessage = ref('');
  const statusType = ref<StatusType>('info');

  // Track status timeout to prevent race conditions
  let statusTimeoutId: ReturnType<typeof setTimeout> | null = null;

  // ========== Computed Properties ==========
  // computed() creates derived reactive values
  // These automatically update when their dependencies change

  /**
   * Check if the encrypt form is valid and ready to submit
   *
   * Requirements:
   * - Input file must be selected
   * - Output path must be set
   * - Password must be at least MIN_PASSWORD_LENGTH characters (recommended minimum)
   * - Not currently processing
   */
  const isEncryptFormValid = computed(() => {
    return (
      inputPath.value.length > 0 &&
      outputPath.value.length > 0 &&
      password.value.length >= MIN_PASSWORD_LENGTH &&
      !isProcessing.value
    );
  });

  /**
   * Check if password meets minimum security requirements
   *
   * Note: This is a basic check. In production, you might want
   * to add a password strength meter (e.g., using zxcvbn library).
   */
  const isPasswordValid = computed(() => {
    return password.value.length >= MIN_PASSWORD_LENGTH;
  });

  /**
   * Check if the decrypt form is valid and ready to submit
   *
   * For decryption, we only require a non-empty password because:
   * - The cryptographic layer validates the password (wrong password = auth failure)
   * - Files may have been encrypted with different password policies
   * - Better UX: crypto error "wrong password" vs frontend validation error
   */
  const isDecryptFormValid = computed(() => {
    return (
      inputPath.value.length > 0 &&
      outputPath.value.length > 0 &&
      password.value.length > 0 &&
      !isProcessing.value
    );
  });

  // ========== Helper Methods ==========

  /**
   * Set the input file path and auto-suggest output path
   *
   * When a user selects an input file, we automatically suggest
   * an output filename by adding/removing the .encrypted extension.
   *
   * For encryption: appends ".encrypted" to the filename
   * For decryption: removes ".encrypted" if present, otherwise appends ".decrypted"
   *
   * @param path - Selected input file path
   * @param isEncrypt - Whether this is for encryption (add .encrypted) or decryption (remove it)
   */
  function setInputPath(path: string, isEncrypt: boolean) {
    inputPath.value = path;

    // Auto-suggest output filename
    if (isEncrypt) {
      // For encryption: add .encrypted extension
      outputPath.value = path + ENCRYPTED_EXTENSION;
    } else {
      // For decryption: remove .encrypted extension if present
      if (path.endsWith(ENCRYPTED_EXTENSION)) {
        outputPath.value = path.slice(0, -ENCRYPTED_EXTENSION.length);
      } else {
        outputPath.value = path + DECRYPTED_EXTENSION;
      }
    }
  }

  /**
   * Manually set the output path
   *
   * Used when user clicks "Change" button to select a different save location.
   *
   * @param path - User-selected output path
   */
  function setOutputPath(path: string) {
    outputPath.value = path;
  }

  /**
   * Set the password
   *
   * @param pwd - User-entered password
   */
  function setPassword(pwd: string) {
    password.value = pwd;
  }

  /**
   * Display a status message to the user
   *
   * @param message - Message text to display
   * @param type - Message type (success, error, info)
   * @param duration - How long to show the message (ms), 0 for permanent
   */
  function showStatus(message: string, type: StatusType, duration = STATUS_SUCCESS_TIMEOUT_MS) {
    // Cancel any pending timeout to prevent it from clearing this new message
    if (statusTimeoutId !== null) {
      clearTimeout(statusTimeoutId);
      statusTimeoutId = null;
    }

    statusMessage.value = message;
    statusType.value = type;

    // Auto-hide success messages after duration
    if (type === 'success' && duration > 0) {
      statusTimeoutId = setTimeout(() => {
        statusMessage.value = '';
        statusTimeoutId = null;
      }, duration);
    }
  }

  /**
   * Clear the status message
   */
  function clearStatus() {
    if (statusTimeoutId !== null) {
      clearTimeout(statusTimeoutId);
      statusTimeoutId = null;
    }
    statusMessage.value = '';
  }

  /**
   * Reset all form fields
   *
   * Used after successful operation or when switching tabs.
   */
  function resetForm() {
    if (statusTimeoutId !== null) {
      clearTimeout(statusTimeoutId);
      statusTimeoutId = null;
    }
    inputPath.value = '';
    outputPath.value = '';
    password.value = '';
    keyFilePath.value = '';
    statusMessage.value = '';
    isProcessing.value = false;
  }

  /**
   * Clear only the password field
   *
   * For security, we clear the password after each operation
   * so it doesn't remain in memory.
   */
  function clearPassword() {
    password.value = '';
  }

  /**
   * Clear the key file path
   */
  function clearKeyFile() {
    keyFilePath.value = '';
  }

  // ========== Main Operations ==========

  /**
   * Perform file encryption
   *
   * This is the main encryption workflow:
   * 1. Validate inputs (password length, file path)
   * 2. Show processing state (status message)
   * 3. Call Rust backend via Tauri IPC (uses streaming for all files)
   * 4. Handle success/error with sanitized error messages
   * 5. Clear password for security
   *
   * The backend uses streaming encryption (1MB chunks) for all files,
   * regardless of size. Progress updates are received via Tauri events.
   *
   * @returns Promise<boolean> True if successful, false otherwise
   */
  async function performEncrypt(): Promise<boolean> {
    if (!isEncryptFormValid.value) {
      showStatus('Please fill in all fields correctly', 'error');
      return false;
    }

    try {
      isProcessing.value = true;
      showStatus('Encrypting file... This may take a moment.', 'info', 0);

      // Call Rust backend (uses streaming for all files)
      const allowOverwrite = !neverOverwrite.value;
      const result = await encryptFile(
        inputPath.value,
        outputPath.value,
        password.value,
        allowOverwrite,
        compressionEnabled.value,
        compressionLevel.value,
        keyFilePath.value || undefined
      );

      // Success!
      showStatus(result.message, 'success');
      outputPath.value = result.output_path;

      return true;
    } catch (error) {
      // Handle errors from Rust backend with sanitized messages
      const errorMessage = sanitizeErrorMessage(error);
      showStatus(errorMessage, 'error', 0);
      return false;
    } finally {
      // Always clear password, even when the operation fails
      clearPassword();
      isProcessing.value = false;
    }
  }

  /**
   * Perform file decryption
   *
   * Similar workflow to encryption:
   * 1. Validate inputs (password length, file path)
   * 2. Show processing state (status message)
   * 3. Call Rust backend via Tauri IPC (uses streaming for all files)
   * 4. Handle success/error (wrong password will fail with authentication error)
   * 5. Clear password for security
   *
   * The backend uses streaming decryption (1MB chunks) for all files,
   * regardless of size. Version 4 and Version 5 formats are supported.
   *
   * @returns Promise<boolean> True if successful, false otherwise
   */
  async function performDecrypt(): Promise<boolean> {
    if (!isDecryptFormValid.value) {
      showStatus('Please fill in all fields correctly', 'error');
      return false;
    }

    try {
      isProcessing.value = true;
      showStatus('Decrypting file... This may take a moment.', 'info', 0);

      const allowOverwrite = !neverOverwrite.value;
      const result = await decryptFile(
        inputPath.value,
        outputPath.value,
        password.value,
        allowOverwrite,
        keyFilePath.value || undefined
      );

      // Success!
      showStatus(result.message, 'success');
      outputPath.value = result.output_path;

      return true;
    } catch (error) {
      // Handle errors (most commonly: wrong password) with sanitized messages
      const errorMessage = sanitizeErrorMessage(error);
      showStatus(errorMessage, 'error', 0);
      return false;
    } finally {
      // Always clear password, even when the operation fails
      clearPassword();
      isProcessing.value = false;
    }
  }

  // Return the public API
  // These will be available to any component that uses this composable
  return {
    // State
    inputPath,
    outputPath,
    password,
    neverOverwrite,
    compressionEnabled,
    compressionLevel,
    keyFilePath,
    isProcessing,
    statusMessage,
    statusType,

    // Computed
    isEncryptFormValid,
    isDecryptFormValid,
    isPasswordValid,

    // Methods
    setInputPath,
    setOutputPath,
    setPassword,
    showStatus,
    clearStatus,
    resetForm,
    clearPassword,
    clearKeyFile,
    performEncrypt,
    performDecrypt,
  };
}
