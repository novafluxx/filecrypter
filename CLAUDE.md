# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FileCrypter is a cross-platform file encryption application built with Tauri v2. It uses Vue 3 for the frontend and Rust for the cryptographic backend, providing secure password-based file encryption using industry-standard algorithms. The app supports desktop (macOS, Windows, Linux) and mobile (iOS, Android) platforms.

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

### Tauri Development (Desktop)
```bash
bun run tauri:dev              # Run in development mode (hot reload)
bun run tauri:build            # Build production executable
```

### iOS Development
```bash
# Prerequisites (one-time setup):
# 1. Install Xcode from Mac App Store and launch it once
# 2. rustup target add aarch64-apple-ios aarch64-apple-ios-sim
# 3. brew install cocoapods

bun tauri ios init             # Initialize iOS project (one-time)
bun tauri ios dev              # Run in iOS Simulator
bun tauri ios dev --device     # Run on physical device (requires signing)
bun tauri ios build --open     # Build and open in Xcode
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

- **src/App.vue**: Root component with adaptive navigation (top tabs on desktop, bottom nav on mobile)
- **src/components/**: Tab UI components (`EncryptTab`, `DecryptTab`, `BatchTab`, `SettingsTab`, `HelpTab`) and widgets
  - `BottomNav.vue`: Mobile-only bottom navigation bar with icons
- **src/composables/**: Shared logic (file ops, Tauri IPC, progress, theme, drag-drop, platform detection)
  - `usePlatform.ts`: Detects iOS/Android vs desktop for conditional UI rendering
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
│   ├── compression.rs     # ZSTD compression for optional file size reduction
│   ├── kdf.rs             # Argon2id key derivation
│   ├── secure.rs          # Password and SecureBytes wrappers (zeroization)
│   └── streaming.rs       # Chunked encryption (Version 4/5 format, all files)
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

**File Formats (src-tauri/src/crypto/streaming.rs)**

Two file format versions are supported:

**Version 4 (No Compression):**
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

**Version 5 (With Compression):**
```
Header (little-endian):
[VERSION:1][SALT_LEN:4][KDF_ALG:1][KDF_MEM_COST:4][KDF_TIME_COST:4]
[KDF_PARALLELISM:4][KDF_KEY_LEN:4][SALT:N][BASE_NONCE:12]
[CHUNK_SIZE:4][TOTAL_CHUNKS:8]
[COMPRESSION_ALG:1][COMPRESSION_LEVEL:1][ORIGINAL_SIZE:8]

Chunks:
[CHUNK_1_LEN:4][CHUNK_1_CIPHERTEXT+TAG]  (compressed before encryption)
...
```

**Compression (src-tauri/src/crypto/compression.rs):**
- Algorithm: ZSTD (Zstandard) level 3 (balanced speed/ratio)
- Strategy: Compress-then-encrypt (data is compressed before encryption)
- Single file mode: Optional (disabled by default, checkbox to enable)
- Batch mode: Always enabled for all files
- Typical reduction: ~70% for text/documents, less for already-compressed formats

**Streaming Encryption Details:**
- Used for all files regardless of size (no threshold)
- Processes files in 1 MB chunks (configurable via `DEFAULT_CHUNK_SIZE`)
- Each chunk has unique nonce: BLAKE3("filecrypter-chunk-nonce-v1" || base_nonce || chunk_index)
- Header authenticated as AAD (Additional Authenticated Data) for every chunk
- Uses temporary files during encryption/decryption for atomic writes
- Temp files protected with restrictive permissions (Unix: 0o600, Windows: ACLs)
- Optimal memory usage (constant 1MB buffer, not proportional to file size)

### IPC Commands

Frontend calls Rust via `invoke()` in `src/composables/useTauri.ts`:
- `encrypt_file` / `decrypt_file`: Single file streaming encryption/decryption
- `batch_encrypt` / `batch_decrypt`: Multiple files with progress events

### Mobile Architecture

The app uses platform-aware navigation that adapts to the device:

**Platform Detection (`src/composables/usePlatform.ts`)**
- Uses `@tauri-apps/plugin-os` to detect the current platform
- Returns `isMobile` ref (true for iOS/Android, false for desktop)
- State is cached globally to avoid repeated API calls

**Adaptive Navigation**
- **Desktop**: Traditional top tab bar in `App.vue`
- **Mobile**: Bottom navigation bar (`BottomNav.vue`) with icons for thumb-friendly access
- Navigation is conditionally rendered based on `isMobile` state

**Mobile-Specific CSS**
- **Dynamic Viewport Height (`100dvh`)**: Used instead of `100vh` because iOS Safari's address bar collapses when scrolling. With `100vh`, the bottom navigation would be hidden behind the address bar on initial load. `100dvh` adjusts to the actual visible viewport, ensuring the bottom nav is always accessible. Falls back to `100vh` for browsers that don't support `dvh`.
- **Safe Area Insets**: Bottom nav includes `env(safe-area-inset-bottom)` padding to avoid overlap with the home indicator on notched devices (iPhone X and later).
- **Flex Overflow Fix**: `min-height: 0` on `.tab-panels` is required for flex children with `overflow-y: auto` to properly constrain their height and enable scrolling.

### Security Notes

- Passwords wrapped in `Password` type and zeroized after use (`src-tauri/src/crypto/secure.rs`)
- On Windows, temp files use ACLs to restrict access to current user only (`src-tauri/src/security/windows_acl.rs`)

## Working with Tauri

- Frontend runs on port 5173 (Vite dev server)
- Tauri expects this fixed port (configured in vite.config.ts)
- File dialogs use `@tauri-apps/plugin-dialog` (not native browser dialogs)
- Platform detection uses `@tauri-apps/plugin-os` for mobile vs desktop UI
- All file I/O happens in Rust backend for security
- Events flow from Rust → Frontend for progress updates during batch operations

### iOS-Specific Notes

- The `src-tauri/gen/apple/` directory is generated by `tauri ios init` and should not be committed
- iOS icons are stored in `src-tauri/icons/ios/` and must be manually copied to `src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset/` after init
- Physical device testing requires an Apple Developer account and code signing configured in Xcode
- The iOS Simulator requires Xcode to be fully installed and launched at least once

## Testing

- **Rust**: Unit tests in `#[cfg(test)]` blocks, integration tests in `src-tauri/tests/`
- **Frontend**: No test framework currently configured
- Run Rust tests before committing: `cd src-tauri && cargo test`

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

## Commit Conventions

Use [Conventional Commits](https://www.conventionalcommits.org/) format. These prefixes are parsed by git-cliff to generate changelogs and auto-calculate version bumps:

| Prefix | Category | Version Bump |
|--------|----------|--------------|
| `feat:` | Features | Minor |
| `fix:` | Bug Fixes | Patch |
| `docs:` | Documentation | Patch |
| `perf:` | Performance | Patch |
| `refactor:` | Refactoring | Patch |
| `test:` | Testing | Patch |
| `build:` | Build | Patch |
| `ci:` | CI | Patch |
| `chore:` | Miscellaneous | Patch |
| `revert:` | Reverts | Patch |

**Breaking changes**: Add `!` after the prefix (e.g., `feat!:`) or include `BREAKING CHANGE:` in the commit body → Major version bump

**Scopes** (optional): Add context in parentheses, e.g., `feat(crypto):`, `fix(ui):`
