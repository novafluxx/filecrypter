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
│   ├── encrypt.rs         # Single file streaming encryption
│   ├── decrypt.rs         # Single file streaming decryption
│   ├── batch.rs           # Batch encrypt/decrypt operations
│   └── file_utils.rs      # File system utilities
├── crypto/                # Cryptographic implementations
│   ├── mod.rs             # Module exports
│   ├── cipher.rs          # AES-256-GCM encryption/decryption
│   ├── kdf.rs             # Argon2id key derivation
│   ├── secure.rs          # Password and SecureBytes wrappers (zeroization)
│   └── streaming.rs       # Chunked encryption (Version 4 format, all files)
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

**File Format - Version 4 (src-tauri/src/crypto/streaming.rs)**

All files use the Version 4 streaming format:
```
Header (little-endian):
[VERSION:1][SALT_LEN:4][KDF_ALG:1][KDF_MEM_COST:4][KDF_TIME_COST:4]
[KDF_PARALLELISM:4][KDF_KEY_LEN:4][SALT:N][BASE_NONCE:12]
[CHUNK_SIZE:4][TOTAL_CHUNKS:8]

Chunks:
[CHUNK_1_LEN:4][CHUNK_1_CIPHERTEXT+TAG]
[CHUNK_2_LEN:4][CHUNK_2_CIPHERTEXT+TAG]
...
```

**Streaming Encryption Details:**
- Used for all files regardless of size (no threshold)
- Processes files in 1 MB chunks (configurable via `DEFAULT_CHUNK_SIZE`)
- Each chunk has unique nonce: BLAKE3(base_nonce || chunk_index)
- Header authenticated as AAD (Additional Authenticated Data) for every chunk
- Uses temporary files during encryption/decryption for atomic writes
- Temp files protected with restrictive permissions (Unix: 0o600, Windows: ACLs)
- Optimal memory usage (constant 1MB buffer, not proportional to file size)

### IPC Communication

Frontend calls Rust backend via Tauri's `invoke()`:
```typescript
// Frontend (src/composables/useTauri.ts)
await invoke('encrypt_file', { inputPath, outputPath, password })
await invoke('decrypt_file', { inputPath, outputPath, password })
await invoke('batch_encrypt', { inputPaths, outputDir, password })
await invoke('batch_decrypt', { inputPaths, outputDir, password })

// Backend (src-tauri/src/commands/)
#[command]
pub async fn encrypt_file(input_path: String, output_path: String, password: String)
```

**Available Commands:**
- `encrypt_file` / `decrypt_file`: Streaming encryption/decryption (all files, any size)
- `batch_encrypt` / `batch_decrypt`: Process multiple files with progress events
- All commands use streaming internally (Version 4 format)

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

All file operations use streaming (chunked) encryption for consistent behavior and optimal memory usage:

1. **Streaming encryption**: Files are processed in 1MB chunks
   - Commands: `encrypt_file`, `decrypt_file`
   - Used for all file sizes
   - Reduces memory footprint for large files
   - Small files create appropriately-sized chunks
   - Used in `EncryptTab.vue` and `DecryptTab.vue`

2. **Batch mode**: Multiple files encrypted/decrypted sequentially
   - Commands: `batch_encrypt`, `batch_decrypt`
   - Emits progress events to frontend for UI updates
   - No per-file size limit
   - Used in `BatchTab.vue`

## Common Modifications

**Adding New Crypto Algorithms**: Modify `src-tauri/src/crypto/cipher.rs` and update `STREAMING_VERSION` in `src-tauri/src/crypto/streaming.rs`

**Changing Key Derivation Parameters**: Update constants in `src-tauri/src/crypto/kdf.rs` (MEMORY_COST, TIME_COST, PARALLELISM)

**Changing Chunk Size**: Update `DEFAULT_CHUNK_SIZE` constant in `src-tauri/src/crypto/streaming.rs`

**Adding New Tauri Commands**:
1. Create handler in `src-tauri/src/commands/`
2. Export in `src-tauri/src/commands/mod.rs`
3. Register in `src-tauri/src/lib.rs` `invoke_handler![]`
4. Call from frontend via `invoke('command_name', {...})`

**Emitting Progress Events**: Use `emit_progress` from `src-tauri/src/events.rs` to send updates to frontend
