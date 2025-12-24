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

/**
 * Safe error messages to display to users.
 * Maps error keywords to user-friendly messages.
 * This prevents leaking sensitive system information in error messages.
 */
const SAFE_ERROR_MESSAGES: Record<string, string> = {
  InvalidPassword: 'Incorrect password or corrupted file',
  FileNotFound: 'File could not be accessed',
  FileTooLarge: 'File is too large for this operation. Use streaming for large files.',
  TooManyFiles: 'Too many files selected for batch operation',
  InvalidPath: 'Invalid file path',
  permission: 'Permission denied - unable to access file',
  default: 'Operation failed - please try again',
};

/**
 * Sanitize error messages for user display.
 * Prevents information leakage by mapping backend errors to safe messages.
 */
function sanitizeErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    const msg = error.message;
    // Check for known error keywords
    for (const [key, safeMsg] of Object.entries(SAFE_ERROR_MESSAGES)) {
      if (key !== 'default' && msg.includes(key)) {
        return safeMsg;
      }
    }
  }
  return SAFE_ERROR_MESSAGES['default'] ?? 'Operation failed';
}

/**
 * Composable for file encryption/decryption operations
 *
 * Provides reactive state management and validation logic
 * for encrypt and decrypt workflows.
 *
 * Features:
 * - Automatic streaming encryption for large files (>10MB)
 * - Form validation
 * - Status message handling
 *
 * @returns {Object} Object containing state, validation, and operation methods
 */
export function useFileOps() {
  const {
    encryptFile,
    decryptFile,
    encryptFileStreamed,
    decryptFileStreamed,
    checkUseStreaming,
  } = useTauri();

  // Track whether file is large enough for streaming
  const useStreaming = ref(false);

  // ========== Reactive State ==========
  // ref() makes primitive values reactive in Vue

  const inputPath = ref('');
  const outputPath = ref('');
  const password = ref('');
  const isProcessing = ref(false);
  const statusMessage = ref('');
  const statusType = ref<StatusType>('info');

  // ========== Computed Properties ==========
  // computed() creates derived reactive values
  // These automatically update when their dependencies change

  /**
   * Check if the form is valid and ready to submit
   *
   * Requirements:
   * - Input file must be selected
   * - Output path must be set
   * - Password must be at least 8 characters (recommended minimum)
   * - Not currently processing
   */
  const isFormValid = computed(() => {
    return (
      inputPath.value.length > 0 &&
      outputPath.value.length > 0 &&
      password.value.length >= 8 &&
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
    return password.value.length >= 8;
  });

  // ========== Helper Methods ==========

  /**
   * Set the input file path and auto-suggest output path
   *
   * When a user selects an input file, we automatically suggest
   * an output filename by adding/removing the .encrypted extension.
   * Also checks if streaming encryption should be used for large files.
   *
   * @param path - Selected input file path
   * @param isEncrypt - Whether this is for encryption (add .encrypted) or decryption (remove it)
   */
  async function setInputPath(path: string, isEncrypt: boolean) {
    inputPath.value = path;

    // Check if we should use streaming for this file
    useStreaming.value = await checkUseStreaming(path);

    // Auto-suggest output filename
    if (isEncrypt) {
      // For encryption: add .encrypted extension
      outputPath.value = path + '.encrypted';
    } else {
      // For decryption: remove .encrypted extension if present
      if (path.endsWith('.encrypted')) {
        outputPath.value = path.slice(0, -10); // Remove '.encrypted'
      } else {
        outputPath.value = path + '.decrypted';
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
  function showStatus(message: string, type: StatusType, duration = 5000) {
    statusMessage.value = message;
    statusType.value = type;

    // Auto-hide success messages after duration
    if (type === 'success' && duration > 0) {
      setTimeout(() => {
        statusMessage.value = '';
      }, duration);
    }
  }

  /**
   * Clear the status message
   */
  function clearStatus() {
    statusMessage.value = '';
  }

  /**
   * Reset all form fields
   *
   * Used after successful operation or when switching tabs.
   */
  function resetForm() {
    inputPath.value = '';
    outputPath.value = '';
    password.value = '';
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

  // ========== Main Operations ==========

  /**
   * Perform file encryption
   *
   * This is the main encryption workflow:
   * 1. Validate inputs
   * 2. Show processing state
   * 3. Call Rust backend via Tauri IPC (streaming for large files)
   * 4. Handle success/error
   * 5. Clear password for security
   *
   * @returns Promise<boolean> True if successful, false otherwise
   */
  async function performEncrypt(): Promise<boolean> {
    if (!isFormValid.value) {
      showStatus('Please fill in all fields correctly', 'error');
      return false;
    }

    try {
      isProcessing.value = true;
      const mode = useStreaming.value ? 'Streaming encryption' : 'Encrypting file';
      showStatus(`${mode}... This may take a moment.`, 'info', 0);

      // Call Rust backend
      // Uses streaming for large files (>10MB) to avoid memory issues
      const result = useStreaming.value
        ? await encryptFileStreamed(inputPath.value, outputPath.value, password.value)
        : await encryptFile(inputPath.value, outputPath.value, password.value);

      // Success!
      showStatus(result, 'success');
      clearPassword(); // Clear password for security

      return true;
    } catch (error) {
      // Handle errors from Rust backend with sanitized messages
      const errorMessage = sanitizeErrorMessage(error);
      showStatus(errorMessage, 'error', 0);
      return false;
    } finally {
      isProcessing.value = false;
    }
  }

  /**
   * Perform file decryption
   *
   * Similar workflow to encryption:
   * 1. Validate inputs
   * 2. Show processing state
   * 3. Call Rust backend (tries regular decryption first, then streaming if format error)
   * 4. Handle success/error (wrong password will fail here)
   * 5. Clear password
   *
   * @returns Promise<boolean> True if successful, false otherwise
   */
  async function performDecrypt(): Promise<boolean> {
    if (!isFormValid.value) {
      showStatus('Please fill in all fields correctly', 'error');
      return false;
    }

    try {
      isProcessing.value = true;
      showStatus('Decrypting file... This may take a moment.', 'info', 0);

      let result: string;

      // Try regular decryption first, fall back to streaming if format error
      try {
        result = await decryptFile(inputPath.value, outputPath.value, password.value);
      } catch (error) {
        // If format error indicates streaming format (version 2), try streaming decryption
        const errorMsg = error instanceof Error ? error.message : String(error);
        if (errorMsg.includes('version') || errorMsg.includes('format')) {
          showStatus('Trying streaming decryption...', 'info', 0);
          result = await decryptFileStreamed(inputPath.value, outputPath.value, password.value);
        } else {
          throw error;
        }
      }

      // Success!
      showStatus(result, 'success');
      clearPassword(); // Clear password for security

      return true;
    } catch (error) {
      // Handle errors (most commonly: wrong password) with sanitized messages
      const errorMessage = sanitizeErrorMessage(error);
      showStatus(errorMessage, 'error', 0);
      return false;
    } finally {
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
    isProcessing,
    statusMessage,
    statusType,
    useStreaming, // Whether streaming mode will be used (for large files)

    // Computed
    isFormValid,
    isPasswordValid,

    // Methods
    setInputPath,
    setOutputPath,
    setPassword,
    showStatus,
    clearStatus,
    resetForm,
    clearPassword,
    performEncrypt,
    performDecrypt,
  };
}
