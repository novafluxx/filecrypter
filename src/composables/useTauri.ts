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
import type { CryptoResponse, BatchResult, ArchiveResult } from '../types/crypto';

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
   * 2. Optionally compresses the data using ZSTD
   * 3. Derives encryption key from password using Argon2id
   * 4. Encrypts with AES-256-GCM
   * 5. Writes encrypted file
   *
   * @param inputPath - Path to the file to encrypt
   * @param outputPath - Path where encrypted file will be saved
   * @param password - User's password
   * @param allowOverwrite - Allow overwriting existing files (default: false)
   * @param compressionEnabled - Enable ZSTD compression (default: false)
   * @param compressionLevel - ZSTD compression level 1-22 (default: 3)
   * @returns Promise resolving to message + resolved output path
   * @throws Error if encryption fails (wrong path, permission denied, etc.)
   */
  async function encryptFile(
    inputPath: string,
    outputPath: string,
    password: string,
    allowOverwrite = false,
    compressionEnabled = false,
    compressionLevel = 3,
    keyFilePath?: string
  ): Promise<CryptoResponse> {
    try {
      // invoke() is Tauri's IPC mechanism - it calls the Rust function
      // The command name 'encrypt_file' matches the #[command] in Rust
      const result = await invoke<CryptoResponse>('encrypt_file', {
        inputPath,   // Maps to input_path parameter in Rust
        outputPath,  // Maps to output_path parameter in Rust
        password,    // Maps to password parameter in Rust
        allowOverwrite,
        compressionEnabled,
        compressionLevel,
        keyFilePath: keyFilePath || null,
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
    allowOverwrite = false,
    keyFilePath?: string
  ): Promise<CryptoResponse> {
    try {
      const result = await invoke<CryptoResponse>('decrypt_file', {
        inputPath,
        outputPath,
        password,
        allowOverwrite,
        keyFilePath: keyFilePath || null,
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
    allowOverwrite = false,
    keyFilePath?: string
  ): Promise<BatchResult> {
    try {
      const result = await invoke<BatchResult>('batch_encrypt', {
        inputPaths,
        outputDir,
        password,
        allowOverwrite,
        keyFilePath: keyFilePath || null,
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
    allowOverwrite = false,
    keyFilePath?: string
  ): Promise<BatchResult> {
    try {
      const result = await invoke<BatchResult>('batch_decrypt', {
        inputPaths,
        outputDir,
        password,
        allowOverwrite,
        keyFilePath: keyFilePath || null,
      });
      return result;
    } catch (error) {
      throw new Error(`Batch decryption failed: ${error}`);
    }
  }

  /**
   * Batch encrypt multiple files into a single encrypted archive
   *
   * Creates a compressed TAR archive from all files, then encrypts it as a single unit.
   *
   * @param inputPaths - Array of file paths to include in the archive
   * @param outputDir - Directory where the encrypted archive will be saved
   * @param password - Password for encryption
   * @param archiveName - Optional custom name for the archive (without extension)
   * @param allowOverwrite - Allow overwriting existing files (default: false)
   * @returns Promise resolving to ArchiveResult
   */
  async function batchEncryptArchive(
    inputPaths: string[],
    outputDir: string,
    password: string,
    archiveName?: string,
    allowOverwrite = false,
    keyFilePath?: string
  ): Promise<ArchiveResult> {
    try {
      const result = await invoke<ArchiveResult>('batch_encrypt_archive', {
        inputPaths,
        outputDir,
        password,
        archiveName: archiveName || null,
        allowOverwrite,
        keyFilePath: keyFilePath || null,
      });
      return result;
    } catch (error) {
      throw new Error(`Archive encryption failed: ${error}`);
    }
  }

  /**
   * Decrypt an encrypted archive and extract its contents
   *
   * Decrypts a .tar.zst.encrypted file and extracts all files to the output directory.
   *
   * @param inputPath - Path to the encrypted archive file
   * @param outputDir - Directory where extracted files will be saved
   * @param password - Password for decryption
   * @param allowOverwrite - Allow overwriting existing files (default: false)
   * @returns Promise resolving to ArchiveResult
   */
  async function batchDecryptArchive(
    inputPath: string,
    outputDir: string,
    password: string,
    allowOverwrite = false,
    keyFilePath?: string
  ): Promise<ArchiveResult> {
    try {
      const result = await invoke<ArchiveResult>('batch_decrypt_archive', {
        inputPath,
        outputDir,
        password,
        allowOverwrite,
        keyFilePath: keyFilePath || null,
      });
      return result;
    } catch (error) {
      throw new Error(`Archive decryption failed: ${error}`);
    }
  }

  /**
   * Generate a new key file with 32 random bytes
   *
   * @param outputPath - Path where the key file will be saved
   * @returns Promise resolving to success message
   */
  async function generateKeyFile(outputPath: string): Promise<CryptoResponse> {
    try {
      const result = await invoke<CryptoResponse>('generate_key_file', {
        outputPath,
      });
      return result;
    } catch (error) {
      throw new Error(`Key file generation failed: ${error}`);
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
    batchEncryptArchive,
    batchDecryptArchive,
    generateKeyFile,
  };
}
