# Repository Guidelines

## Project Structure & Module Organization
- `src/` holds the Vue 3 frontend (entry: `src/main.ts`, root UI: `src/App.vue`).
- `src/components/` contains tab UI components (`EncryptTab.vue`, `DecryptTab.vue`, `BatchTab.vue`, `SettingsTab.vue`, `HelpTab.vue`) plus shared widgets (`ProgressBar.vue`, `PasswordStrengthMeter.vue`, `StatusMessage.vue`, etc.).
- `src/composables/` has shared frontend logic (`useCryptoOperation`, `useFileOps`, `useProgress`, `useTheme`, `useDragDrop`, `usePasswordStrength`, `useTauri`, `usePlatform`, `useSettings`, `useUpdater`, `useVersion`, `useSettingsSync`).
- `src/utils/` keeps shared helpers; `src/constants.ts` and `src/shared.css` define global constants and styles.
- `src/types/` stores TypeScript type definitions.
- `src-tauri/` is the Rust/Tauri backend (`src/main.rs` delegates to `src/lib.rs`, which registers plugins and IPC commands).
- `src-tauri/src/commands/` defines IPC handlers and helpers (`encrypt.rs`, `decrypt.rs`, `batch.rs`, `archive.rs`, `keyfile.rs`, `file_utils.rs`, `command_utils.rs`), and `src-tauri/src/crypto/` holds AES/Argon2 + streaming implementations.
- `src-tauri/src/security/` contains security checks/platform abstractions, while `src-tauri/src/error.rs` and `src-tauri/src/events.rs` centralize error and event types.

## Build, Test, and Development Commands
- `bun install` installs frontend dependencies.
- `bun run dev` starts the Vite dev server on port 5173.
- `bun run build` runs type checking (`vue-tsc`) and builds the frontend.
- `bun run preview` serves the production build locally.
- `bun run lint` runs ESLint on the frontend.
- `bun run tauri` runs the Tauri CLI directly (`tauri <subcommand>`).
- `bun run tauri:dev` launches the full Tauri app with hot reload.
- `bun run tauri:build` creates a production desktop build.
- Mobile support is a future goal and not part of the currently maintained build/dev workflow; treat mobile commands as experimental.
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
