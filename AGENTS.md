# Repository Guidelines

## Project Structure & Module Organization
- `src/` holds the Vue 3 frontend (entry: `src/main.ts`, root UI: `src/App.vue`).
- `src/main.ts` configures PrimeVue 4 (Aura preset + app theme overrides) and registers `ConfirmationService`.
- `src/components/` contains tab UI components (`EncryptTab.vue`, `DecryptTab.vue`, `BatchTab.vue`, `SettingsTab.vue`, `HelpTab.vue`) plus navigation and shared UI (`BottomNav.vue`, `UpdateNotification.vue`, `ChangelogAction.vue`, `CryptoOperationForm.vue`, `KeyFileSection.vue`, `PasswordSection.vue`, `OverwriteCheckbox.vue`, `ProgressBar.vue`, `PasswordStrengthMeter.vue`, `StatusMessage.vue`).
- `src/composables/` has shared frontend logic (`useCryptoOperation`, `useFileOps`, `useProgress`, `useTheme`, `useDragDrop`, `usePasswordStrength`, `useTauri`, `usePlatform`, `useSettings`, `useUpdater`, `useVersion`, `useSettingsSync`).
- `src/utils/` keeps shared helpers; `src/constants.ts` and `src/shared.css` define shared frontend constants and global/shared styles.
- `src/types/` stores TypeScript type definitions.
- `src-tauri/` is the Rust/Tauri backend (`src/main.rs` delegates to `src/lib.rs`, which registers plugins and IPC commands).
- `src-tauri/src/commands/` defines IPC handlers and helpers (`encrypt.rs`, `decrypt.rs`, `batch.rs`, `archive.rs`, `keyfile.rs`, `file_utils.rs`, `command_utils.rs`), and `src-tauri/src/crypto/` holds AES/Argon2 + streaming implementations.
- `src-tauri/src/security/` contains security checks/platform abstractions, while `src-tauri/src/error.rs` and `src-tauri/src/events.rs` centralize error and event types.

## Build, Test, and Development Commands
- `corepack enable` enables the pinned pnpm version declared in `package.json`.
- `pnpm install --frozen-lockfile` installs frontend dependencies and matches CI.
- `pnpm run dev` starts the Vite dev server on port 5173.
- `pnpm run build` runs type checking (`vue-tsc`) and builds the frontend.
- `pnpm run preview` serves the production build locally.
- `pnpm run lint` runs ESLint on the frontend.
- `pnpm exec vue-tsc --noEmit` runs the frontend type check used in CI.
- `pnpm exec tauri <subcommand>` runs the Tauri CLI directly.
- `pnpm run tauri:dev` launches the full Tauri app with hot reload.
- `pnpm run tauri:build` creates a production desktop build.
- Mobile support is a future goal and not part of the currently maintained build/dev workflow; treat mobile commands as experimental.
- `cd src-tauri && cargo test` runs Rust unit + integration tests.
- `cd src-tauri && cargo clippy` runs the Rust linter.
- `cd src-tauri && cargo fmt --check` matches the PR formatting check in CI.
- `cd src-tauri && cargo clippy --locked --all-features -- -D warnings` matches the PR lint command in CI.
- `cd src-tauri && cargo test --locked --all-features --lib --tests` matches the Rust test command in CI.

## Mobile Workflow Notes (Future Goal / Experimental)
- Use `pnpm exec tauri <args>` for direct Tauri CLI commands.
- One-time mobile project setup: `pnpm exec tauri android init` and `pnpm exec tauri ios init`.
- Android prerequisites: Android Studio SDK/NDK and JDK 17+, plus Rust targets (`aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android`).
- iOS prerequisites (macOS only): Xcode + CocoaPods, plus Rust targets (`aarch64-apple-ios`, `aarch64-apple-ios-sim`).
- Common dev commands: `pnpm exec tauri android dev`, `pnpm exec tauri ios dev`, and optional `--open` to launch Android Studio/Xcode.
- Device/LAN workflow: use `--host` and set `TAURI_DEV_HOST=<LAN_IP>` so the mobile target can reach the Vite dev server.
- Common build commands: `pnpm exec tauri android build --apk` and `pnpm exec tauri ios build --open`.

## Coding Style & Naming Conventions
- TypeScript: use 2-space indentation; prefer Composition API patterns in `src/`.
- Vue files: PascalCase component filenames (e.g., `EncryptTab.vue`).
- Frontend UI stack is PrimeVue 4; prefer PrimeVue components/services over adding alternate UI frameworks.
- Rust: follow standard `rustfmt` defaults; modules are `snake_case`.
- IPC command names are `snake_case` (e.g., `encrypt_file`, `decrypt_file`).

## Testing Guidelines
- Rust unit tests live in `#[cfg(test)]` blocks within backend modules.
- Rust integration tests live in `src-tauri/tests/`.
- Frontend: no test framework is configured yet.
- Preferred test command: `cd src-tauri && cargo test`.
- CI runs `pnpm install --frozen-lockfile`, `pnpm exec vue-tsc --noEmit`, and `pnpm run lint` for frontend changes, plus `cargo test --locked --all-features --lib --tests` for Rust changes.
- On pull requests, CI also runs `cargo fmt --check` and `cargo clippy --locked --all-features -- -D warnings` for Rust changes.

## Commit & Pull Request Guidelines
- Follow the Conventional Commits format documented in `CLAUDE.md` (e.g., `feat:`, `fix:`, `docs:`).
- Keep PRs focused; include a clear description and any relevant screenshots for UI changes.
- If modifying cryptography or file I/O, mention how the change affects security or performance.
- Releases are published from the manual GitHub Actions workflow in `.github/workflows/release.yml`; it calculates the version, updates version files/changelog, builds signed artifacts, and creates a draft GitHub release.

## Security & Configuration Notes
- Encryption uses AES-256-GCM with Argon2id; keep salts and nonces unique per file.
- Tauri file I/O stays in Rust; avoid moving sensitive operations to the frontend.
- Desktop window config is currently resizable with `minWidth: 500` and `minHeight: 500`; keep default window sizing compatible with 1366×768 displays.
