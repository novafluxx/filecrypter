# Changelog

All notable changes to FileCrypter will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.13] - 2026-03-14



### Bug Fixes

- **crypto:** Address crypto review findings ([f4f6589](https://github.com/novafluxx/filecrypter/commit/f4f6589f719c9bff8e5f1044915baedd8127164b))
- **crypto:** Address tier-2 defense-in-depth review findings ([ce30ce6](https://github.com/novafluxx/filecrypter/commit/ce30ce6639cab2a2c4160242020ed237aa6287f4))
- **deps:** Add esbuild as explicit dependency for Vite 8 ([035772c](https://github.com/novafluxx/filecrypter/commit/035772cabd0555b850dac5762de63415a097930a))
- **crypto:** Zeroize KDF output buffer on error path ([3dfe0bf](https://github.com/novafluxx/filecrypter/commit/3dfe0bf474ecdabd1742ac6ffb5c93157b22eb00))
- **ci:** Use correct Windows NSIS artifact names in release workflow ([6863495](https://github.com/novafluxx/filecrypter/commit/6863495a0b1f06f545997887fc0a646aa41f661e))
- **ci:** Verify main hasn't moved before committing release ([594b566](https://github.com/novafluxx/filecrypter/commit/594b5666e176274fda958e6a8a8593681d47fb61))


### Refactoring

- **crypto:** Restrict unbounded decompression to test scope and document compression side-channel ([8bbd340](https://github.com/novafluxx/filecrypter/commit/8bbd34065c11edebb76471deda5ce57dd2483dba))
- **ci:** Restructure release workflow to build before committing ([c0f69c2](https://github.com/novafluxx/filecrypter/commit/c0f69c248779d547793cbbdf9ac9715d985d4def))
## [1.0.12] - 2026-03-05



### Bug Fixes

- Disable no-useless-assignment ESLint rule for Vue SFC false positives ([5c07004](https://github.com/novafluxx/filecrypter/commit/5c07004123188f6d7e4ce1df4c1356814e440bf2))


### Refactoring

- **ui:** Improve code reuse, patterns, and correctness across frontend ([2b4df4c](https://github.com/novafluxx/filecrypter/commit/2b4df4c72d991b030f610599dccc1c9c9c6aced3))


### Documentation

- Clarify compression behavior in single and batch modes ([f62b19f](https://github.com/novafluxx/filecrypter/commit/f62b19fbb54e5955fc4257f735757ac1aaebf5ff))
- Fix inaccurate comments in App.vue and useFileOps.ts ([6d79840](https://github.com/novafluxx/filecrypter/commit/6d79840df84bd9cc96813a0b5581bd4440ec5340))
## [1.0.11] - 2026-02-28



### Refactoring

- **backend:** Remove dead code, optimize decryption buffer, fix minor issues ([43d0f96](https://github.com/novafluxx/filecrypter/commit/43d0f9644d8e844c3117be92ae540e16e6a5bf3e))
## [1.0.10] - 2026-02-21



### Bug Fixes

- **build:** Update Vite config to use Tauri v2 env variables ([a5245f1](https://github.com/novafluxx/filecrypter/commit/a5245f13447049dc03d64a71e7e6324bfdf2cb49))
- **ui:** Reduce oversized font sizes to standard desktop scale ([3ead577](https://github.com/novafluxx/filecrypter/commit/3ead57741f9d5ae82e0483564d7252c56bfb29f5))
## [1.0.9] - 2026-02-20



### Bug Fixes

- **ui:** Fix pointer cursor on checkboxes and tab overflow arrows ([4366403](https://github.com/novafluxx/filecrypter/commit/4366403b5e1be3189b800acaef6885ac26a27e2f))
## [1.0.8] - 2026-02-20



### Features

- **ui:** Migrate from naive-ui to PrimeVue 4 ([df1285d](https://github.com/novafluxx/filecrypter/commit/df1285ded903503fccffe2e05ab3ca3b5e66c312))


### Bug Fixes

- **ui:** Fix batch mode switch regression and duplicate checkbox IDs ([91522a0](https://github.com/novafluxx/filecrypter/commit/91522a04d0648626e8cea2f70f27c306b67d8a74))
- **ui:** Make window resizable and restore 768p-safe default height ([22d6d61](https://github.com/novafluxx/filecrypter/commit/22d6d61791f7cfc88c98a4054ff881e3ec4ba306))


### Documentation

- Update AGENTS.md for PrimeVue migration and window config ([edb3ddc](https://github.com/novafluxx/filecrypter/commit/edb3ddc20b518b026872d28a7bf1b542ce3983b4))
## [1.0.6] - 2026-02-09



### Documentation

- Align desktop-first platform documentation ([dc2f515](https://github.com/novafluxx/filecrypter/commit/dc2f515b9aab34819ff3f112a32cff1d1937e1a9))
- Add tauri v2 mobile workflow reference ([422b8f3](https://github.com/novafluxx/filecrypter/commit/422b8f35f33bf30cc6f4eed9605553d1d5215183))
## [1.0.5] - 2026-02-08



### Features

- Add changelog header action and align button styles ([6e0dcc6](https://github.com/novafluxx/filecrypter/commit/6e0dcc69055bf9ba5e7c70f170dc047f9a3ea4b9))


### Bug Fixes

- Make encrypt file picker button-only ([6b0d460](https://github.com/novafluxx/filecrypter/commit/6b0d4608f4d879e5df7ffe530b152dcf3619dfd9))
- Make desktop header divider full width ([206fe00](https://github.com/novafluxx/filecrypter/commit/206fe00525d5131d6f2063d9dde56601bae2b96c))
## [1.0.4] - 2026-02-07



### Bug Fixes

- **security:** IPC security hardening ([0b0ce6d](https://github.com/novafluxx/filecrypter/commit/0b0ce6d1724cb377e7f28fcaa0ad5e1bceb4cba9)) in [#26](https://github.com/novafluxx/filecrypter/pull/26)
- Harden password input handling ([9448920](https://github.com/novafluxx/filecrypter/commit/9448920c6c312edcb1bdbbc88749a662a9e0b9a1)) in [#27](https://github.com/novafluxx/filecrypter/pull/27)
## [1.0.3] - 2026-02-07



### Bug Fixes

- **deps:** Sync bun.lock with package.json ([902b0d0](https://github.com/novafluxx/filecrypter/commit/902b0d0288592d15ccbd817926dd097e52379e73))
- **deps:** Update time crate to 0.3.47 to fix CVE-2026-25727 ([499ce18](https://github.com/novafluxx/filecrypter/commit/499ce18a3a769e5055a3b70b7015287841f36a04))
- Address code review findings across frontend and backend ([967e8ad](https://github.com/novafluxx/filecrypter/commit/967e8adbf8634652b355f5dcb273084c7c387691)) in [#25](https://github.com/novafluxx/filecrypter/pull/25)
## [1.0.0] - 2026-01-31



### Features

- **crypto:** Add optional key file support for two-factor encryption ([7405b5e](https://github.com/novafluxx/filecrypter/commit/7405b5ed420c76aa38bcaff5b552b7fdf0d49673)) in [#24](https://github.com/novafluxx/filecrypter/pull/24)
## [0.2.0] - 2026-01-29



### Features

- **ui:** Add "What's new" link to update notification banner ([674eab4](https://github.com/novafluxx/filecrypter/commit/674eab4c76ace086a23c3548313520d8541abf44))


### Bug Fixes

- **ci:** Fixed release notes issue when release notes contain quotes ([5e9051f](https://github.com/novafluxx/filecrypter/commit/5e9051fefdc444e825b59bb7ec7e8a74331e313a))
- **ci:** Strip 'v' prefix from manual version override input ([f682d63](https://github.com/novafluxx/filecrypter/commit/f682d635bcdfd9fd2478ba341a9bb889f30126d2))
## [0.1.18] - 2026-01-28



### Bug Fixes

- Sync bun.lock with package.json ([50d1d67](https://github.com/novafluxx/filecrypter/commit/50d1d674fe0f7bee848b100faae6eb10ff12a5ec))
- **ui:** Prevent white flash on app startup ([997f587](https://github.com/novafluxx/filecrypter/commit/997f587782d4970aea30c36d7d4a1b4d84c52ec3))
- **ui:** Prevent white flash on app startup ([185f6de](https://github.com/novafluxx/filecrypter/commit/185f6de257548f22c8cc77369aeb0bf2faace6be))
## [0.1.17] - 2026-01-27



### Bug Fixes

- **security:** Update windows-sys 0.61 API compatibility ([d474210](https://github.com/novafluxx/filecrypter/commit/d474210465c8446f99ebd7928b56c77979ede5a4))


### Refactoring

- **security:** Replace windows-acl crate with direct windows-sys calls ([bf641de](https://github.com/novafluxx/filecrypter/commit/bf641de04952051324bcf3e493d3ea2dd81aa714))
## [0.1.16] - 2026-01-25



### Refactoring

- **ui:** Extract reusable form components and add settings sync ([a5eb149](https://github.com/novafluxx/filecrypter/commit/a5eb149291e030b71801e2648059ba62c2769b6a))
- Extract shared utilities for encrypt/decrypt commands ([0337722](https://github.com/novafluxx/filecrypter/commit/0337722ba08f8be7a6395361cb91d12b3281c716)) in [#23](https://github.com/novafluxx/filecrypter/pull/23)
## [0.1.15] - 2026-01-23



### Bug Fixes

- **progress:** Prevent listener leak and race condition with KeepAlive ([373476b](https://github.com/novafluxx/filecrypter/commit/373476b1e8aa4b8ce4805e89f1ddccec4ad3767f))
- **status:** Cancel pending timeout before showing new status message ([3a521d8](https://github.com/novafluxx/filecrypter/commit/3a521d81c8d5b016623e1623b0e32d21c95e3574))
- **updater:** Wait for platform detection before update check ([2ca9fa0](https://github.com/novafluxx/filecrypter/commit/2ca9fa02543c414a0f7fb5eee4604d08012b0284))
- **validation:** Make decrypt password validation consistent with batch mode ([2cad8b5](https://github.com/novafluxx/filecrypter/commit/2cad8b5cb293d15025c961ff009e02e3190c2a32))
- **security:** Sanitize markdown HTML output with DOMPurify ([b8ce55a](https://github.com/novafluxx/filecrypter/commit/b8ce55a9b610a710fe417a6cf8f20e3fbabfb086))
- **path:** Use Tauri path API for cross-platform path joining ([b5126a3](https://github.com/novafluxx/filecrypter/commit/b5126a3fa72074659afe71163b508844bdf44718))


### Refactoring

- **validation:** Rename isFormValid to isEncryptFormValid ([1ca5bf0](https://github.com/novafluxx/filecrypter/commit/1ca5bf0fbc30640cbb8c00ebaf4a4e37b925eb33))


### Documentation

- Update user guide for archive mode ([6c0f6e1](https://github.com/novafluxx/filecrypter/commit/6c0f6e12b5f861193b9dc7c5ff91cee58dbd8982))
## [0.1.14] - 2026-01-22



### Features

- **batch:** Add archive mode for batch encryption ([d713b39](https://github.com/novafluxx/filecrypter/commit/d713b39281597b5fa94624380bfa6cd59c297ca3)) in [#22](https://github.com/novafluxx/filecrypter/pull/22)


### Bug Fixes

- **ci:** Fix YAML syntax in pre-release workflow ([65caf9f](https://github.com/novafluxx/filecrypter/commit/65caf9f2a62ce75a0d2bab62c9019dce8b4241a9))
- **ci:** Support immutable releases in release workflows ([13dca8d](https://github.com/novafluxx/filecrypter/commit/13dca8d0bee6978fc3299b10f590ef3797ef6d80))
- **ci:** Fix version handling between pre-release and release workflows ([1567a17](https://github.com/novafluxx/filecrypter/commit/1567a173aaba4705c8c4df13555a0e05feb97f6c))
- **ci:** Handle retry scenarios where version is already bumped ([5eb47d0](https://github.com/novafluxx/filecrypter/commit/5eb47d0e6799613a3d52a94fb3115dc2fe9091eb))
- **ci:** Handle retry scenarios in release workflow ([cc9c5db](https://github.com/novafluxx/filecrypter/commit/cc9c5db6416ee4d89819e9297da22b3103f7a6d0))
## [0.1.13] - 2026-01-21



### Bug Fixes

- Resolve drag & drop conflicts between tabs ([fddcc4a](https://github.com/novafluxx/filecrypter/commit/fddcc4aed7b3170ef6d58faed93fdb02bcade421)) in [#21](https://github.com/novafluxx/filecrypter/pull/21)
## [0.1.11] - 2026-01-21



### Features

- **ui:** Display app version in desktop header ([4e3f37a](https://github.com/novafluxx/filecrypter/commit/4e3f37a41e7f94ee7425bb021982618d66e77126))


### Documentation

- Add macOS installation instructions for unsigned builds ([b34b660](https://github.com/novafluxx/filecrypter/commit/b34b660af0df0af331003ccef2b7024c622a34ac))
## [0.1.10] - 2026-01-20



### Bug Fixes

- **build:** Add app target for macOS updater artifacts, remove appimage ([d68d1c7](https://github.com/novafluxx/filecrypter/commit/d68d1c702e0a6f92bf77ef60828b8f162156b8af))


### Documentation

- Add Android development and auto-updater sections to CLAUDE.md ([fe7cf6d](https://github.com/novafluxx/filecrypter/commit/fe7cf6dc49327157c7c52ef9b4480219f85cca07))
## [0.1.9] - 2026-01-20



### Features

- Add Tauri auto-updater support ([ddb4807](https://github.com/novafluxx/filecrypter/commit/ddb48071469cc27bedb529d87d475c74a1e9cf79)) in [#19](https://github.com/novafluxx/filecrypter/pull/19)
## [0.1.8] - 2026-01-20



### Bug Fixes

- **settings:** Sync default settings changes to Encrypt/Decrypt tabs in real-time ([accab0f](https://github.com/novafluxx/filecrypter/commit/accab0f5c29b174a67223250b0b5f004fdfd1124))
- **mobile:** Disable pinch-to-zoom and configure touch actions ([93dbd8f](https://github.com/novafluxx/filecrypter/commit/93dbd8f85bd503a8966f5f0c9886bb8897891b28))
## [0.1.7] - 2026-01-18



### Bug Fixes

- **ui:** Clicking the "File to Encrypt:" label no longer opens the file picker dialog. Whoops. ([77c66fc](https://github.com/novafluxx/filecrypter/commit/77c66fcf3f999f5a9689a7fd613ab45e947a8b54))
## [0.1.6] - 2026-01-17



### Bug Fixes

- No more flashing in the UI when you change tabs ([346cc51](https://github.com/novafluxx/filecrypter/commit/346cc5159bc950a619902b30abe4c9c0a6fe2a9b))


### ui

- Removed title banner along top of the app. It was serving no purpose. ([4454097](https://github.com/novafluxx/filecrypter/commit/445409711a8fe436d819c288b7896ee41a3c29eb))
- More ui changes, bold tab names, better looking cards in each tab ([50f0f9c](https://github.com/novafluxx/filecrypter/commit/50f0f9c018b35546564d4519b6e9b39476d1ca1a))
## [0.1.5] - 2026-01-17



### Bug Fixes

- **ci:** Correct tauri-action asset naming parameter ([d5cbfb7](https://github.com/novafluxx/filecrypter/commit/d5cbfb713214e1e8259ad4148a2c15e3c433c727))
## [0.1.4] - 2026-01-17



### Features

- **ui:** Open file picker from encrypt input ([1ee5d32](https://github.com/novafluxx/filecrypter/commit/1ee5d329a6fa9b0456b3c16e1ac15ab5e7bc703c))


### Documentation

- Minor change to user documentation ([39c99b6](https://github.com/novafluxx/filecrypter/commit/39c99b69080bd0e897b1659a0325478cc37e6454))
- Add lint command and naive-ui to CLAUDE.md, unify commit guidelines ([83a5d59](https://github.com/novafluxx/filecrypter/commit/83a5d5954effd4d44c02ff347d03a76a68ecb99e))
## [0.1.3] - 2026-01-16



### Features

- **ui:** Migrate UI components to Naive UI ([2709da4](https://github.com/novafluxx/filecrypter/commit/2709da49c1728f75f53d960d10b50f13fb1bd453)) in [#18](https://github.com/novafluxx/filecrypter/pull/18)
## [0.1.2] - 2026-01-14



### Features

- Add Settings tab with persistent user preferences ([8c24eb5](https://github.com/novafluxx/filecrypter/commit/8c24eb50b68a61ebc16711c13a74c0652cd26784)) in [#14](https://github.com/novafluxx/filecrypter/pull/14)
- **ui:** Add mobile bottom navigation for iOS/Android ([c34687a](https://github.com/novafluxx/filecrypter/commit/c34687a3b4b0eaa8640d4e17349da0afb4751874)) in [#16](https://github.com/novafluxx/filecrypter/pull/16)
- **ui:** Native desktop-like UI behavior ([5317ff5](https://github.com/novafluxx/filecrypter/commit/5317ff5b94270e6c0319b8604d60be9dc915f6ad)) in [#17](https://github.com/novafluxx/filecrypter/pull/17)


### Refactoring

- **frontend:** Extract shared utilities and improve code quality ([b1368a8](https://github.com/novafluxx/filecrypter/commit/b1368a81be0cb990e2f195ee8ad1d4bba9d29a40))


### Documentation

- Updating user help ([8d45e5c](https://github.com/novafluxx/filecrypter/commit/8d45e5cf2970227a73a01b91cf1fc42f3f7b6736))
## [0.1.1] - 2026-01-11



### Bug Fixes

- **ci:** Improve version auto-calculation with better tag fetching and fallbacks ([6104b21](https://github.com/novafluxx/filecrypter/commit/6104b211a14c0f6521ead00e516ac54441559a66))
## [0.1.0] - 2026-01-11



### Features

- **security:** Implement Windows DACL for file permissions ([470c8f8](https://github.com/novafluxx/filecrypter/commit/470c8f8803a8a5344584d25a0c170a706ce07236)) in [#3](https://github.com/novafluxx/filecrypter/pull/3)
- Transform UI to desktop-first design with improved readability ([d6de622](https://github.com/novafluxx/filecrypter/commit/d6de622ea3a82e90b874846610e597b5348a0ef7))
- **ci:** Improve release workflow with automated versioning and changelog ([9776c46](https://github.com/novafluxx/filecrypter/commit/9776c46fd2ef648b78a237a284929af354a98a6d))
- **ci:** Auto-version releases, Windows-only builds, changelog in release notes ([8273424](https://github.com/novafluxx/filecrypter/commit/8273424211405d1ae9316b6fbb7528c3af470896))


### Bug Fixes

- Correct spelling of "FileCrypter" across multiple files ([8703f46](https://github.com/novafluxx/filecrypter/commit/8703f46707ef69ab90d3d432cbe46b712e579573))
- Address 4 high-priority code review issues ([c90e77a](https://github.com/novafluxx/filecrypter/commit/c90e77a449f52f6b4bd2708c43d71b8325db6af0)) in [#4](https://github.com/novafluxx/filecrypter/pull/4)
- **ci:** Replace git-cliff-action with direct CLI installation ([6816334](https://github.com/novafluxx/filecrypter/commit/6816334787b0ce455ca2b1183511601def39669e))
- **ci:** Add Linux system dependencies for cargo check ([8853b5b](https://github.com/novafluxx/filecrypter/commit/8853b5b89b0f31b2523d50751dde1f036c937b0d))
- Memory leak in frontend and optimizing code comments ([7b70695](https://github.com/novafluxx/filecrypter/commit/7b7069505090eba1ce5d6863c35a8ad53cce69bd))


### Refactoring

- **ci:** Remove redundant frontend build step ([249301f](https://github.com/novafluxx/filecrypter/commit/249301f6cd47d241be7ebf4a3c8ac7cf190213e8))
- **ci:** Simplify to Rust-only validation ([e01960f](https://github.com/novafluxx/filecrypter/commit/e01960fa6ebfb8152a7de1757e8c5da211112c74))


### Documentation

- Improve backend comment accuracy and clarity ([46d671d](https://github.com/novafluxx/filecrypter/commit/46d671d82d1d0807f30c8f7088df25625430ad59)) in [#9](https://github.com/novafluxx/filecrypter/pull/9)
- Add commit conventions to CLAUDE.md ([7174950](https://github.com/novafluxx/filecrypter/commit/71749504e30c50e5ab932618cd63f3f521840fd3))
<!-- generated by git-cliff -->
