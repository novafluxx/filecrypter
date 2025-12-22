# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FileCypter is a cross-platform desktop file encryption application built with Tauri. It uses Vue 3 for the frontend and Rust for the cryptographic backend, providing secure password-based file encryption using industry-standard algorithms.

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
cargo clippy                   # Run linter
```

## Architecture

### Frontend Structure (Vue 3 Composition API)

- **src/main.ts**: Application entry point
- **src/App.vue**: Root component with tab navigation
- **src/components/**: Tab components (EncryptTab.vue, DecryptTab.vue)
- **src/composables/**: Reusable logic
  - `useFileOps.ts`: State management for encryption/decryption workflows
  - `useTauri.ts`: Wrapper for Tauri IPC commands
- **src/types/**: TypeScript type definitions

### Backend Structure (Rust)

```
src-tauri/src/
├── lib.rs                  # Main entry point, registers commands
├── commands/               # Tauri IPC command handlers
│   ├── mod.rs             # Exports encrypt_file, decrypt_file
│   ├── encrypt.rs         # File encryption workflow
│   └── decrypt.rs         # File decryption workflow
├── crypto/                # Cryptographic implementations
│   ├── mod.rs             # Module exports
│   ├── cipher.rs          # AES-256-GCM encryption/decryption
│   ├── kdf.rs             # Argon2id key derivation
│   ├── format.rs          # Binary file format serialization
│   └── secure.rs          # Password and SecureBytes wrappers (zeroization)
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

### IPC Communication

Frontend calls Rust backend via Tauri's `invoke()`:
```typescript
// Frontend (src/composables/useTauri.ts)
await invoke('encrypt_file', { inputPath, outputPath, password })

// Backend (src-tauri/src/commands/encrypt.rs)
#[command]
pub async fn encrypt_file(input_path: String, output_path: String, password: String)
```

### Security Practices

- **Password Handling**: Passwords are wrapped in `Password` type and zeroized after use (src-tauri/src/crypto/secure.rs)
- **Unique Salts**: Each encryption generates a new random salt, ensuring different keys for same password
- **Authentication**: AES-GCM provides both encryption and authentication (detects tampering)
- **Error Messages**: Generic error messages prevent information leakage (wrong password → "Invalid password or corrupted file")

## Working with Tauri

- Frontend runs on port 5173 (Vite dev server)
- Tauri expects this fixed port (configured in vite.config.ts)
- File dialogs use `@tauri-apps/plugin-dialog` (not native browser dialogs)
- All file I/O happens in Rust backend for security

## Testing

- **Rust**: Comprehensive unit tests in each module (`#[cfg(test)]` blocks)
- **Frontend**: No test framework currently configured
- Run Rust tests before committing: `cd src-tauri && cargo test`

## File Operations

Files are loaded entirely into memory during encryption/decryption. For large files (>100MB), consider implementing streaming encryption by modifying:
- `src-tauri/src/commands/encrypt.rs` (currently uses `fs::read()`)
- `src-tauri/src/commands/decrypt.rs` (currently uses `fs::read()`)

## Common Modifications

**Adding New Crypto Algorithms**: Modify `src-tauri/src/crypto/cipher.rs` and update `VERSION` in `format.rs`

**Changing Key Derivation Parameters**: Update constants in `src-tauri/src/crypto/kdf.rs` (MEMORY_COST, TIME_COST, PARALLELISM)

**Adding New Tauri Commands**:
1. Create handler in `src-tauri/src/commands/`
2. Export in `src-tauri/src/commands/mod.rs`
3. Register in `src-tauri/src/lib.rs` `invoke_handler![]`
4. Call from frontend via `invoke('command_name', {...})`
