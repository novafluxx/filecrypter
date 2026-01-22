// types/crypto.ts - TypeScript Interfaces for Crypto Operations
//
// These interfaces define the structure of data exchanged between
// the Vue frontend and the Rust backend via Tauri IPC.

/**
 * Response from encryption/decryption operations
 *
 * The Rust backend returns a message plus the resolved output path.
 */
export interface CryptoResponse {
  message: string;
  output_path: string;
}

/**
 * Status message types for UI feedback
 */
export type StatusType = 'success' | 'error' | 'info';

/**
 * Application state for encrypt/decrypt operations
 */
export interface FileOperationState {
  inputPath: string;
  outputPath: string;
  password: string;
  neverOverwrite: boolean;
  isProcessing: boolean;
  statusMessage: string;
  statusType: StatusType;
}

/**
 * Result for a single file in batch operation
 */
export interface FileResult {
  input_path: string;
  output_path: string | null;
  success: boolean;
  error: string | null;
}

/**
 * Result of a batch operation
 */
export interface BatchResult {
  files: FileResult[];
  success_count: number;
  failed_count: number;
}

/**
 * Progress event for batch operations
 */
export interface BatchProgress {
  current_file: string;
  file_index: number;
  total_files: number;
  stage: string;
  percent: number;
}

/**
 * Progress event for archive operations
 */
export interface ArchiveProgress {
  phase: string; // "archiving", "encrypting", "decrypting", "extracting", "complete"
  current_file: string | null;
  files_processed: number;
  total_files: number;
  percent: number;
}

/**
 * Result of an archive encrypt/decrypt operation
 */
export interface ArchiveResult {
  output_path: string;
  file_count: number;
  success: boolean;
  error: string | null;
}

/**
 * Batch mode type
 */
export type BatchMode = 'individual' | 'archive';
