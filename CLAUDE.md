# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FileCrypter is a cross-platform desktop file encryption application built with Tauri. It uses Vue 3 for the frontend and Rust for the cryptographic backend, providing secure password-based file encryption using industry-standard algorithms.

## Tech Stack

- **Frontend**: Vue 3 (Composition API) + TypeScript + Vite
- **Backend**: Rust + Tauri v2
- **Cryptography**: AES-256-GCM encryption with Argon2id key derivation
- **Package Manager**: Bun (for frontend), Cargo (for Rust)

## Development Commands

### Frontend Development
```bash
bun install                    # Install dependencies
bun run dev                    # Start Vite dev server (port 5173)
bun run build                  # Build frontend with TypeScript checking
bun run preview                # Preview production build
```

### Tauri Development
```bash
bun run tauri:dev              # Run in development mode (hot reload)
bun run tauri:build            # Build production executable
```

### Rust Testing
```bash
cd src-tauri
cargo test                     # Run all tests
cargo test --lib               # Run library tests only
cargo test <test_name>         # Run specific test
cargo clippy                   # Run linter
```

## Architecture

### Frontend Structure (Vue 3 Composition API)

- **src/main.ts**: Application entry point
- **src/App.vue**: Root component with tab navigation (Encrypt, Decrypt, Batch)
- **src/components/**: UI components
  - `EncryptTab.vue`: Single file encryption interface
  - `DecryptTab.vue`: Single file decryption interface
  - `BatchTab.vue`: Batch encryption/decryption interface
  - `ProgressBar.vue`: Progress indicator for batch operations
  - `PasswordStrengthMeter.vue`: Visual password strength feedback
- **src/composables/**: Reusable logic
  - `useFileOps.ts`: Core file operation state management
  - `useTauri.ts`: Type-safe Tauri IPC command wrappers
  - `useProgress.ts`: Progress tracking for batch operations
  - `usePasswordStrength.ts`: Password strength calculation
  - `useDragDrop.ts`: Drag-and-drop file handling
  - `useTheme.ts`: Dark/light theme management
  - `usePasswordVisibility.ts`: Password field toggle logic
- **src/types/**: TypeScript type definitions

### Backend Structure (Rust)

```
src-tauri/src/
├── lib.rs                  # Main entry point, registers commands
├── main.rs                 # Desktop binary entry
├── commands/               # Tauri IPC command handlers
│   ├── mod.rs             # Exports all commands
│   ├── encrypt.rs         # Single file encryption (in-memory)
│   ├── decrypt.rs         # Single file decryption (in-memory)
│   ├── batch.rs           # Batch encrypt/decrypt operations
│   ├── streaming.rs       # Streaming encrypt/decrypt for large files
│   └── file_utils.rs      # File system utilities
├── crypto/                # Cryptographic implementations
│   ├── mod.rs             # Module exports
│   ├── cipher.rs          # AES-256-GCM encryption/decryption
│   ├── kdf.rs             # Argon2id key derivation
│   ├── format.rs          # Binary file format serialization
│   ├── secure.rs          # Password and SecureBytes wrappers (zeroization)
│   └── streaming.rs       # Chunked encryption for large files
├── security/              # Platform-specific security
│   ├── mod.rs             # Security module exports
│   └── windows_acl.rs     # Windows ACL protection for temp files
├── events.rs              # Event system for progress updates
└── error.rs               # Custom error types
```

### Cryptographic Design

**Key Derivation (src-tauri/src/crypto/kdf.rs)**
- Algorithm: Argon2id (hybrid mode for GPU/side-channel resistance)
- Parameters: 64 MiB memory, 3 iterations, 4 threads (OWASP 2025 recommendations)
- Output: 256-bit key for AES-256
- Performance: ~100-300ms on modern CPUs (intentionally slow for security)

**Encryption (src-tauri/src/crypto/cipher.rs)**
- Algorithm: AES-256-GCM (authenticated encryption)
- Nonce: 96-bit random (generated per encryption, never reused)
- Tag: 128-bit authentication tag (prevents tampering)
- Each encryption generates unique salt and nonce

**File Format (src-tauri/src/crypto/format.rs)**
```
[VERSION:1byte][SALT_LEN:4bytes][SALT:variable][NONCE:12bytes][CIPHERTEXT+TAG:variable]
```

**Streaming Encryption (src-tauri/src/crypto/streaming.rs)**
- Auto-selected for files >10 MB (configurable via `STREAMING_THRESHOLD`)
- Processes files in 1 MB chunks (configurable via `DEFAULT_CHUNK_SIZE`)
- Uses temporary files during encryption/decryption
- On Windows, temp files are protected with ACLs to prevent other processes from reading them
- Reduces memory usage for large files

### IPC Communication

Frontend calls Rust backend via Tauri's `invoke()`:
```typescript
// Frontend (src/composables/useTauri.ts)
await invoke('encrypt_file', { inputPath, outputPath, password })
await invoke('encrypt_file_streamed', { inputPath, outputPath, password })
await invoke('batch_encrypt', { inputPaths, outputDir, password })

// Backend (src-tauri/src/commands/)
#[command]
pub async fn encrypt_file(input_path: String, output_path: String, password: String)
```

**Available Commands:**
- `encrypt_file` / `decrypt_file`: In-memory operations for files <10MB
- `encrypt_file_streamed` / `decrypt_file_streamed`: Chunked operations for large files
- `batch_encrypt` / `batch_decrypt`: Process multiple files with progress events
- `check_use_streaming`: Determine if file should use streaming based on size
- `get_streaming_threshold`: Get the current streaming threshold (10MB)

### Security Practices

- **Password Handling**: Passwords are wrapped in `Password` type and zeroized after use (src-tauri/src/crypto/secure.rs)
- **Unique Salts**: Each encryption generates a new random salt, ensuring different keys for same password
- **Authentication**: AES-GCM provides both encryption and authentication (detects tampering)
- **Error Messages**: Generic error messages prevent information leakage (wrong password → "Invalid password or corrupted file")
- **Temp File Security**: On Windows, temp files use ACLs to restrict access to current user only
- **Memory Safety**: Sensitive data (keys, passwords) is zeroized after use

## Working with Tauri

- Frontend runs on port 5173 (Vite dev server)
- Tauri expects this fixed port (configured in vite.config.ts)
- File dialogs use `@tauri-apps/plugin-dialog` (not native browser dialogs)
- All file I/O happens in Rust backend for security
- Events flow from Rust → Frontend for progress updates during batch operations

## Testing

- **Rust**: Comprehensive unit tests in each module (`#[cfg(test)]` blocks)
- **Frontend**: No test framework currently configured
- Run Rust tests before committing: `cd src-tauri && cargo test`

## File Operations

The app automatically chooses between two encryption modes:

1. **In-memory mode** (files <10MB): Entire file is loaded into memory, encrypted, and written out
   - Commands: `encrypt_file`, `decrypt_file`
   - Used in `EncryptTab.vue` and `DecryptTab.vue`

2. **Streaming mode** (files ≥10MB): File is processed in 1MB chunks
   - Commands: `encrypt_file_streamed`, `decrypt_file_streamed`
   - Automatically selected based on file size
   - Reduces memory footprint for large files

3. **Batch mode**: Multiple files encrypted/decrypted sequentially
   - Commands: `batch_encrypt`, `batch_decrypt`
   - Emits progress events to frontend for UI updates
   - Enforces per-file size limit of 100MB for batch operations
   - Used in `BatchTab.vue`

## Common Modifications

**Adding New Crypto Algorithms**: Modify `src-tauri/src/crypto/cipher.rs` and update `VERSION` in `format.rs`

**Changing Key Derivation Parameters**: Update constants in `src-tauri/src/crypto/kdf.rs` (MEMORY_COST, TIME_COST, PARALLELISM)

**Changing Streaming Threshold**: Update `STREAMING_THRESHOLD` constant in `src-tauri/src/crypto/streaming.rs`

**Adding New Tauri Commands**:
1. Create handler in `src-tauri/src/commands/`
2. Export in `src-tauri/src/commands/mod.rs`
3. Register in `src-tauri/src/lib.rs` `invoke_handler![]`
4. Call from frontend via `invoke('command_name', {...})`

**Emitting Progress Events**: Use `emit_progress` from `src-tauri/src/events.rs` to send updates to frontend
