/**
 * Error Sanitization Utility
 *
 * This module provides error message sanitization to prevent information leakage.
 * Backend errors are mapped to user-friendly messages that don't expose
 * sensitive system information.
 */

/**
 * Safe error messages to display to users.
 * Maps error keywords to user-friendly messages.
 * This prevents leaking sensitive system information in error messages.
 */
export const SAFE_ERROR_MESSAGES: Record<string, string> = {
  InvalidPassword: 'Incorrect password or corrupted file',
  FileNotFound: 'File could not be accessed',
  FileTooLarge: 'File is too large for this operation',
  TooManyFiles: 'Too many files selected for batch operation',
  InvalidPath: 'Invalid file path',
  'encrypted with a key file': 'This file was encrypted with a key file — please provide it to decrypt',
  'Key file error': 'Key file is invalid or could not be read',
  permission: 'Permission denied - unable to access file',
  default: 'Operation failed - please try again',
};

/**
 * Sanitize error messages for user display.
 * Prevents information leakage by mapping backend errors to safe messages.
 *
 * This function handles both Error instances and other types by converting
 * them to strings before checking for known error keywords.
 *
 * @param error - The error to sanitize (can be Error instance, string, or any other type)
 * @returns A safe, user-friendly error message
 */
export function sanitizeErrorMessage(error: unknown): string {
  // Convert error to string to handle both Error instances and other types
  const errStr = error instanceof Error ? error.message : String(error);

  // Check for known error keywords
  for (const [key, safeMsg] of Object.entries(SAFE_ERROR_MESSAGES)) {
    if (key !== 'default' && errStr.includes(key)) {
      return safeMsg;
    }
  }

  return SAFE_ERROR_MESSAGES['default'] ?? 'Operation failed';
}
