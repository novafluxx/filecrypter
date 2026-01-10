// composables/useTauri.ts - Tauri API Wrapper
//
// This composable provides type-safe wrappers around Tauri's invoke() function
// and file dialog API. It handles the communication between the Vue frontend
// and the Rust backend.
//
// Tauri IPC (Inter-Process Communication):
// - Frontend calls invoke('command_name', { args }) to call Rust functions
// - Backend executes the command and returns a result
// - TypeScript provides type safety for arguments and return values

import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { CryptoResponse, BatchResult } from '../types/crypto';

/**
 * Composable for Tauri-specific operations
 *
 * This provides a clean abstraction layer over Tauri APIs,
 * making it easy to call Rust commands from Vue components.
 *
 * @returns {Object} Object containing Tauri command wrappers
 */
export function useTauri() {
  /**
   * Encrypt a file using the Rust backend
   *
   * Calls the 'encrypt_file' Tauri command which:
   * 1. Reads the input file
   * 2. Derives encryption key from password using Argon2id
   * 3. Encrypts with AES-256-GCM
   * 4. Writes encrypted file
   *
   * @param inputPath - Path to the file to encrypt
   * @param outputPath - Path where encrypted file will be saved
   * @param password - User's password
   * @param allowOverwrite - Allow overwriting existing files (default: false)
   * @returns Promise resolving to message + resolved output path
   * @throws Error if encryption fails (wrong path, permission denied, etc.)
   */
  async function encryptFile(
    inputPath: string,
    outputPath: string,
    password: string,
    allowOverwrite = false
  ): Promise<CryptoResponse> {
    try {
      // invoke() is Tauri's IPC mechanism - it calls the Rust function
      // The command name 'encrypt_file' matches the #[command] in Rust
      const result = await invoke<CryptoResponse>('encrypt_file', {
        inputPath,   // Maps to input_path parameter in Rust
        outputPath,  // Maps to output_path parameter in Rust
        password,    // Maps to password parameter in Rust
        allowOverwrite,
      });
      return result;
    } catch (error) {
      // Tauri errors are serialized from Rust CryptoError enum
      throw new Error(`Encryption failed: ${error}`);
    }
  }

  /**
   * Decrypt an encrypted file using the Rust backend
   *
   * Calls the 'decrypt_file' Tauri command which:
   * 1. Reads the encrypted file
   * 2. Parses file format (salt, nonce, ciphertext)
   * 3. Derives decryption key from password + salt
   * 4. Decrypts and verifies authentication tag
   * 5. Writes decrypted file
   *
   * @param inputPath - Path to the encrypted file
   * @param outputPath - Path where decrypted file will be saved
   * @param password - User's password (must match encryption password)
   * @param allowOverwrite - Allow overwriting existing files (default: false)
   * @returns Promise resolving to message + resolved output path
   * @throws Error if decryption fails (wrong password, corrupted file, etc.)
   */
  async function decryptFile(
    inputPath: string,
    outputPath: string,
    password: string,
    allowOverwrite = false
  ): Promise<CryptoResponse> {
    try {
      const result = await invoke<CryptoResponse>('decrypt_file', {
        inputPath,
        outputPath,
        password,
        allowOverwrite,
      });
      return result;
    } catch (error) {
      // Error will include descriptive message from Rust
      throw new Error(`Decryption failed: ${error}`);
    }
  }

  /**
   * Open a file picker dialog for selecting a file to encrypt/decrypt
   *
   * Uses Tauri's dialog plugin to show native OS file picker.
   * This provides a better UX than manual path entry.
   *
   * @param title - Dialog window title
   * @param filters - File type filters (optional)
   * @returns Promise resolving to selected file path, or null if cancelled
   */
  async function selectFile(
    title: string,
    filters?: Array<{ name: string; extensions: string[] }>
  ): Promise<string | null> {
    try {
      // open() shows the native file picker dialog
      // Returns file path as string, or null if user cancelled
      const selected = await open({
        title,
        multiple: false,  // Only allow single file selection
        directory: false, // Select files, not directories
        filters,          // Optional file type filters
      });

      // selected is either a string (path) or null (cancelled)
      return selected as string | null;
    } catch (error) {
      console.error('File selection error:', error);
      return null;
    }
  }

  /**
   * Open a save dialog for choosing where to save encrypted/decrypted file
   *
   * @param title - Dialog window title
   * @param defaultPath - Suggested filename/path
   * @param filters - File type filters (optional)
   * @returns Promise resolving to selected save path, or null if cancelled
   */
  async function selectSavePath(
    title: string,
    defaultPath?: string,
    filters?: Array<{ name: string; extensions: string[] }>
  ): Promise<string | null> {
    try {
      // save() shows the native save file dialog
      const selected = await save({
        title,
        defaultPath, // Pre-fill the filename field
        filters,
      });

      return selected;
    } catch (error) {
      console.error('Save path selection error:', error);
      return null;
    }
  }

  /**
   * Open a file picker dialog for selecting multiple files
   *
   * @param title - Dialog window title
   * @param filters - File type filters (optional)
   * @returns Promise resolving to array of file paths, or empty array if cancelled
   */
  async function selectMultipleFiles(
    title: string,
    filters?: Array<{ name: string; extensions: string[] }>
  ): Promise<string[]> {
    try {
      const selected = await open({
        title,
        multiple: true,   // Allow multiple file selection
        directory: false,
        filters,
      });

      if (!selected) return [];
      // open() returns string[] when multiple is true
      return Array.isArray(selected) ? selected : [selected];
    } catch (error) {
      console.error('Multi-file selection error:', error);
      return [];
    }
  }

  /**
   * Open a directory picker dialog
   *
   * @param title - Dialog window title
   * @returns Promise resolving to directory path, or null if cancelled
   */
  async function selectDirectory(title: string): Promise<string | null> {
    try {
      const selected = await open({
        title,
        directory: true,
        multiple: false,
      });

      return selected as string | null;
    } catch (error) {
      console.error('Directory selection error:', error);
      return null;
    }
  }

  /**
   * Batch encrypt multiple files using the same password
   *
   * @param inputPaths - Array of file paths to encrypt
   * @param outputDir - Directory where encrypted files will be saved
   * @param password - Password for encryption
   * @param allowOverwrite - Allow overwriting existing files (default: false)
   * @returns Promise resolving to BatchResult
   */
  async function batchEncrypt(
    inputPaths: string[],
    outputDir: string,
    password: string,
    allowOverwrite = false
  ): Promise<BatchResult> {
    try {
      const result = await invoke<BatchResult>('batch_encrypt', {
        inputPaths,
        outputDir,
        password,
        allowOverwrite,
      });
      return result;
    } catch (error) {
      throw new Error(`Batch encryption failed: ${error}`);
    }
  }

  /**
   * Batch decrypt multiple files using the same password
   *
   * @param inputPaths - Array of encrypted file paths
   * @param outputDir - Directory where decrypted files will be saved
   * @param password - Password for decryption
   * @param allowOverwrite - Allow overwriting existing files (default: false)
   * @returns Promise resolving to BatchResult
   */
  async function batchDecrypt(
    inputPaths: string[],
    outputDir: string,
    password: string,
    allowOverwrite = false
  ): Promise<BatchResult> {
    try {
      const result = await invoke<BatchResult>('batch_decrypt', {
        inputPaths,
        outputDir,
        password,
        allowOverwrite,
      });
      return result;
    } catch (error) {
      throw new Error(`Batch decryption failed: ${error}`);
    }
  }

  // Return the public API
  return {
    encryptFile,
    decryptFile,
    selectFile,
    selectSavePath,
    selectMultipleFiles,
    selectDirectory,
    batchEncrypt,
    batchDecrypt,
  };
}
