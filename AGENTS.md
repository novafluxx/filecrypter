# Repository Guidelines

## Project Structure & Module Organization
- `src/` holds the Vue 3 frontend (entry: `src/main.ts`, root UI: `src/App.vue`).
- `src/components/` contains tab UI components and shared widgets (e.g., `EncryptTab.vue`, `DecryptTab.vue`, `BatchTab.vue`, `ProgressBar.vue`).
- `src/composables/` has shared frontend logic (file ops, progress, theme, drag/drop, password strength, Tauri IPC).
- `src/types/` stores TypeScript type definitions.
- `src-tauri/` is the Rust/Tauri backend (commands, crypto, security, errors).
- `src-tauri/src/commands/` defines IPC handlers (batch, streaming, file utils), and `src-tauri/src/crypto/` holds AES/Argon2 + streaming implementations.

## Build, Test, and Development Commands
- `bun install` installs frontend dependencies.
- `bun run dev` starts the Vite dev server on port 5173.
- `bun run build` runs type checking (`vue-tsc`) and builds the frontend.
- `bun run preview` serves the production build locally.
- `bun run tauri:dev` launches the full Tauri app with hot reload.
- `bun run tauri:build` creates a production desktop build.
- `cd src-tauri && cargo test` runs Rust unit + integration tests.
- `cd src-tauri && cargo clippy` runs the Rust linter.

## Coding Style & Naming Conventions
- TypeScript: use 2-space indentation; prefer Composition API patterns in `src/`.
- Vue files: PascalCase component filenames (e.g., `EncryptTab.vue`).
- Rust: follow standard `rustfmt` defaults; modules are `snake_case`.
- IPC command names are `snake_case` (e.g., `encrypt_file`, `decrypt_file`).

## Testing Guidelines
- Rust unit tests live in `#[cfg(test)]` blocks within backend modules.
- Rust integration tests live in `src-tauri/tests/`.
- Frontend: no test framework is configured yet.
- Preferred test command: `cd src-tauri && cargo test`.

## Commit & Pull Request Guidelines
- Follow the Conventional Commits format documented in `CLAUDE.md` (e.g., `feat:`, `fix:`, `docs:`).
- Keep PRs focused; include a clear description and any relevant screenshots for UI changes.
- If modifying cryptography or file I/O, mention how the change affects security or performance.

## Security & Configuration Notes
- Encryption uses AES-256-GCM with Argon2id; keep salts and nonces unique per file.
- Tauri file I/O stays in Rust; avoid moving sensitive operations to the frontend.
