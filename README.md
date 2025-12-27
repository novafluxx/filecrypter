# FileCypter

FileCypter is a cross-platform desktop app for encrypting and decrypting files. The UI is built with Vue 3 and the cryptographic operations live in a Rust/Tauri backend.

## Features
- Password-based encryption using AES-256-GCM.
- Argon2id key derivation with per-file salt and nonce.
- Streaming (chunked) encryption for large files (auto-selected for >10 MB).
- Batch encrypt/decrypt with progress updates.
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

Preview the production build:
```bash
bun run preview
```

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
- Each encryption generates a unique salt and nonce.
- File I/O is handled in Rust; the frontend only invokes commands.
- Single-file operations switch to streaming for large files (default threshold: 10 MB).
- Batch operations are in-memory and enforce a per-file size limit (100 MB).

## Contributing
See `AGENTS.md` for repository guidelines and contribution practices.
