# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FileCrypter is a cross-platform file encryption application built with Tauri v2. It uses Vue 3 for the frontend and Rust for the cryptographic backend, providing secure password-based file encryption using industry-standard algorithms. The project is currently desktop-first (macOS, Windows). Mobile (iOS/Android) is a future goal and not part of the actively maintained workflow yet.

## Tech Stack

- **Frontend**: Vue 3 (Composition API) + TypeScript + Vite + PrimeVue 4
- **Backend**: Rust + Tauri v2
- **Cryptography**: AES-256-GCM encryption with Argon2id key derivation
- **Package Manager**: pnpm (for frontend; version pinned by `packageManager` in `package.json`), Cargo (for Rust)

## Development Commands

### Frontend Development
```bash
pnpm install --frozen-lockfile # Install dependencies (pnpm switches to the pinned version automatically)
pnpm run dev                   # Start Vite dev server (port 5173)
pnpm run build                 # Build frontend with TypeScript checking
pnpm run preview               # Preview production build
pnpm run lint                  # Run ESLint on frontend code
```

### Tauri Development (Desktop)
```bash
pnpm run tauri:dev             # Run in development mode (hot reload)
pnpm run tauri:build           # Build production executable
```

Use `pnpm exec tauri <args>` for direct Tauri CLI commands.

### Rust Testing
```bash
cd src-tauri
cargo test                     # Run all tests
cargo test --lib               # Run library tests only
cargo test <test_name>         # Run specific test
cargo clippy                   # Run linter
```

### CI-equivalent checks
`.github/workflows/ci.yml` is path-filtered (only the Rust and/or frontend jobs affected by a change run). To match it locally:
```bash
pnpm exec vue-tsc --noEmit                                   # Frontend type check
pnpm run lint                                                # Frontend lint
cd src-tauri && cargo test --locked --all-features --lib --tests
cd src-tauri && cargo fmt --check                            # PRs only
cd src-tauri && cargo clippy --locked --all-features -- -D warnings   # PRs only
```

Releases come from the manually triggered `.github/workflows/release.yml`. It calculates the version from the commit log via git-cliff, builds signed macOS aarch64 and Windows x64 artifacts, commits the version and changelog updates, generates `latest.json`, and creates a draft GitHub release. Don't bump versions or edit `CHANGELOG.md` by hand.

## Architecture

### Frontend Structure (Vue 3 Composition API)

- **src/App.vue**: Root component with adaptive navigation (desktop-first with mobile-aware UI scaffolding), PrimeVue ConfirmDialog, and Tabs
- **src/components/**: Tab UI components (`EncryptTab`, `DecryptTab`, `BatchTab`, `SettingsTab`, `HelpTab`) and widgets
  - `BottomNav.vue`: Mobile-oriented bottom navigation scaffold with icons
- **src/composables/**: Shared logic (file ops, Tauri IPC, progress, theme, drag-drop, platform detection)
  - `usePlatform.ts`: Detects iOS/Android vs desktop for conditional UI rendering
- **src/types/**: TypeScript type definitions

### Backend Structure (Rust)

`src-tauri/src/` is layered:
- `lib.rs` registers plugins and IPC commands (`main.rs` just delegates to it).
- `commands/` holds thin IPC handlers (one file per operation, plus `archive.rs` for the TAR+ZSTD batch mode). Shared validation lives in `command_utils.rs` and `file_utils.rs`.
- `crypto/` holds the actual cryptography. `streaming.rs` owns the file format and is used by every encrypt/decrypt path. `keyfile.rs` builds the KDF input as `password || BLAKE3(key_file)`. `secure.rs` provides zeroizing wrappers.
- `security/` holds the platform temp-file protections. `events.rs` handles progress events and `error.rs` the error types.

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

Four format versions are written and read. The version byte is selected from (compression, key file):

| Version | Compression | Key file | Extra header fields |
|---------|-------------|----------|---------------------|
| 4 | no | no | — |
| 5 | yes | no | compression fields |
| 6 | no | yes | flags byte |
| 7 | yes | yes | compression fields + flags byte |

```
Header (little-endian):
[VERSION:1][SALT_LEN:4][KDF_ALG:1][KDF_MEM_COST:4][KDF_TIME_COST:4]
[KDF_PARALLELISM:4][KDF_KEY_LEN:4][SALT:N][BASE_NONCE:12]
[CHUNK_SIZE:4][TOTAL_CHUNKS:8]
[COMPRESSION_ALG:1][COMPRESSION_LEVEL:1][ORIGINAL_SIZE:8]   (V5/V7 only)
[FLAGS:1]                                                   (V6/V7 only; 0x01 = key file used)

Chunks:
[CHUNK_1_LEN:4][CHUNK_1_CIPHERTEXT+TAG]   (plaintext compressed before encryption in V5/V7)
[CHUNK_2_LEN:4][CHUNK_2_CIPHERTEXT+TAG]
...
```

KDF parameters are stored in the header and validated against min/max bounds on decrypt (see `kdf.rs`), so changing the default constants doesn't break old files. Any header layout change must keep decrypt support for all existing versions.

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
- `batch_encrypt_archive` / `batch_decrypt_archive`: Archive-mode batch operations
- `generate_key_file`: Create key files for optional two-factor encryption

### Mobile (Future Goal, Not Maintained)

iOS/Android are not part of the dev, CI, or release workflow. The UI already has scaffolding for them: `usePlatform.ts` exposes a cached `isMobile` flag, and `App.vue` uses it to switch between top tabs (desktop) and `BottomNav.vue` (mobile). There is also safe-area/viewport CSS. Keep the updater desktop-only. `src-tauri/gen/` is generated and gitignored. When mobile work resumes, see the playbook in `AGENTS.md` and reconfirm the steps against current Tauri mobile docs.

### Security Notes

- Passwords wrapped in `Password` type and zeroized after use (`src-tauri/src/crypto/secure.rs`)
- On Windows, temp files use ACLs to restrict access to current user only (`src-tauri/src/security/windows_acl.rs`)
- Archive extraction (`commands/archive.rs`) enforces decompression-ratio and absolute size caps against zip bombs, and validates entry paths. Preserve these checks when touching archive code.
- Keep the Tauri CSP in `src-tauri/tauri.conf.json` restrictive. Change it deliberately only when new asset, network, or IPC requirements are introduced.
- Desktop window is 700×680 (min 500×500). Keep default sizing compatible with 1366×768 displays.
- `AGENTS.md` mirrors much of this file for other agents. Update both when changing commands, CI, or architecture notes.

## Working with Tauri

- Frontend runs on port 5173 (Vite dev server)
- Tauri expects this fixed port (configured in vite.config.ts)
- File dialogs use `@tauri-apps/plugin-dialog` (not native browser dialogs)
- Platform detection uses `@tauri-apps/plugin-os` for mobile vs desktop UI
- All file I/O happens in Rust backend for security
- Events flow from Rust → Frontend for progress updates during batch operations

### Auto-Updater (Desktop Only)

The app includes automatic update checking for desktop platforms using Tauri's updater plugin.

**Configuration (`src-tauri/tauri.conf.json`):**
- Update endpoint: GitHub releases (`/releases/latest/download/latest.json`)
- Code signing: Updates are verified with a public key
- Windows install mode: Passive (minimal user interaction)

**Implementation:**
- `src/composables/useUpdater.ts`: Reactive composable providing update state, download progress, and install/dismiss methods
- `src/components/UpdateNotification.vue`: Banner UI that appears when an update is available

**Behavior:**
- Checks for updates 2 seconds after app launch (non-blocking)
- Shows notification banner with version info when update available
- Tracks download progress (0-100%) during installation
- Auto-relaunches app after update installation
- If mobile targets are enabled in the future, update checks should remain desktop-only (app stores handle mobile updates)

**Dependencies:**
- `@tauri-apps/plugin-updater`: Update checking and installation
- `@tauri-apps/plugin-process`: App relaunch after update
- Desktop-only: These plugins are excluded from mobile builds via conditional compilation in `Cargo.toml`

## Testing

- **Rust**: Unit tests in `#[cfg(test)]` blocks, integration tests in `src-tauri/tests/`
- **Frontend**: No test framework currently configured
- Run Rust tests before committing: `cd src-tauri && cargo test`

## Common Modifications

**Adding New Crypto Algorithms or Header Fields**: Modify `src-tauri/src/crypto/cipher.rs`, then add a new `STREAMING_VERSION_V*` constant in `src-tauri/src/crypto/streaming.rs`. Wire it into both the version-selection `match` in the encrypt path and the version check/parsing in the decrypt path. Keep V4–V7 decryptable.

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
