[![Tests](https://github.com/novafluxx/filecrypter/actions/workflows/ci.yml/badge.svg)](https://github.com/novafluxx/filecrypter/actions/workflows/ci.yml)
[![CodeQL](https://github.com/novafluxx/filecrypter/actions/workflows/codeql.yml/badge.svg)](https://github.com/novafluxx/filecrypter/actions/workflows/codeql.yml)
# FileCrypter

FileCrypter is a cross-platform desktop app for encrypting and decrypting files. The UI is built with Vue 3 and the cryptographic operations live in a Rust/Tauri backend.

## End-User Guide
See `README_USER.md` for end-user instructions (how to encrypt/decrypt, batch mode, troubleshooting).

## Features
- Password-based encryption using AES-256-GCM.
- Argon2id key derivation with per-file salt.
- Streaming (chunked) encryption/decryption for all files (default 1MB chunks).
- Optional ZSTD compression (reduces file size by ~70% for text/documents).
- Batch encrypt/decrypt with progress updates (compression enabled by default).
- Native file dialogs via Tauri.

## Tech Stack
- Frontend: Vue 3 + TypeScript + Vite
- Backend: Rust + Tauri v2
- Package manager: Bun (frontend), Cargo (backend)

## Project Structure
- `src/` frontend app (entry: `src/main.ts`, root: `src/App.vue`)
- `src/components/` UI tabs and panels
- `src/composables/` shared frontend logic
- `src/types/` TypeScript types
- `src-tauri/` Rust/Tauri backend (commands + crypto modules)

## Getting Started
Prerequisites:
- Bun installed for frontend tooling
- Rust toolchain for the Tauri backend

Install dependencies:
```bash
bun install
```

Run the frontend in the browser:
```bash
bun run dev
```

Run the full desktop app:
```bash
bun run tauri:dev
```

Build:
```bash
bun run build
bun run tauri:build
```

Preview the production frontend build in a browser:
```bash
bun run preview
```

Platform note:
- `bun run tauri:build` builds bundles for the host OS only (Linux builds on Linux, Windows builds on Windows, macOS builds on macOS).
- Cross-building for other OSes typically requires building on that OS with its toolchain/SDK.

## Testing & Linting
Run Rust tests (unit + integration):
```bash
cd src-tauri
cargo test
```

Run the Rust linter:
```bash
cd src-tauri
cargo clippy
```

## Security Notes
- Encryption uses AES-256-GCM with Argon2id key derivation.
- Each encryption generates a unique salt and base nonce; per-chunk nonces are derived with BLAKE3 and the chunk index.
- File I/O is handled in Rust; the frontend only invokes commands.
- All file operations use streaming (chunked) encryption/decryption with atomic writes (temp file + rename).
- Compression uses compress-then-encrypt strategy (industry standard used by SSH, TLS).
- Batch operations validate max file count (1000) and continue on per-file failures.

## File Format
FileCrypter uses two file format versions:
- **Version 4**: Standard encrypted format (no compression)
- **Version 5**: Encrypted format with ZSTD compression metadata

Version 5 extends Version 4 with compression fields in the header. Both versions are fully supported for decryption, ensuring backward compatibility.

## Contributing
See `AGENTS.md` for repository guidelines and contribution practices.
