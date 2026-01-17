# Changelog

All notable changes to FileCrypter will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] - 2026-01-17



### Features

- **ui:** Open file picker from encrypt input ([1ee5d32](https://github.com/novafluxx/filecrypter/commit/1ee5d329a6fa9b0456b3c16e1ac15ab5e7bc703c))


### Documentation

- Minor change to user documentation ([39c99b6](https://github.com/novafluxx/filecrypter/commit/39c99b69080bd0e897b1659a0325478cc37e6454))
- Add lint command and naive-ui to CLAUDE.md, unify commit guidelines ([83a5d59](https://github.com/novafluxx/filecrypter/commit/83a5d5954effd4d44c02ff347d03a76a68ecb99e))


### CI

- **release:** Customize release asset names for clarity ([a1db861](https://github.com/novafluxx/filecrypter/commit/a1db861657110966732bf1b4c163f976db98743e))
- **changelog:** Remove emojis and add commit/PR links ([2ff89bb](https://github.com/novafluxx/filecrypter/commit/2ff89bb922111767141ba572a389ec3304292b5a))
- Split CI into rust/frontend jobs and add ESLint ([af6e45d](https://github.com/novafluxx/filecrypter/commit/af6e45d5c9f722566d4438e0ad79cd941b3abad3))
## [0.1.3] - 2026-01-16



### Features

- **ui:** Migrate UI components to Naive UI ([2709da4](https://github.com/novafluxx/filecrypter/commit/2709da49c1728f75f53d960d10b50f13fb1bd453)) in [#18](https://github.com/novafluxx/filecrypter/pull/18)


### Dependencies

- Updating dependencies ([2640099](https://github.com/novafluxx/filecrypter/commit/2640099b347d235a771b975621d28062ebc7325d))


### CI

- **codeql:** Fix Rust analysis build mode ([8feb8d5](https://github.com/novafluxx/filecrypter/commit/8feb8d56ba3bd724f00a0f727c38b262c6e434a8))
## [0.1.2] - 2026-01-14



### Features

- Add Settings tab with persistent user preferences ([8c24eb5](https://github.com/novafluxx/filecrypter/commit/8c24eb50b68a61ebc16711c13a74c0652cd26784)) in [#14](https://github.com/novafluxx/filecrypter/pull/14)
- **ui:** Add mobile bottom navigation for iOS/Android ([c34687a](https://github.com/novafluxx/filecrypter/commit/c34687a3b4b0eaa8640d4e17349da0afb4751874)) in [#16](https://github.com/novafluxx/filecrypter/pull/16)
- **ui:** Native desktop-like UI behavior ([5317ff5](https://github.com/novafluxx/filecrypter/commit/5317ff5b94270e6c0319b8604d60be9dc915f6ad)) in [#17](https://github.com/novafluxx/filecrypter/pull/17)


### Dependencies

- Dependency updates ([592daad](https://github.com/novafluxx/filecrypter/commit/592daad2e2d8d81cec04a1a1fe5fbc208d4b1dd0))


### Miscellaneous

- Added permissions to ci workflow ([bd81890](https://github.com/novafluxx/filecrypter/commit/bd818903a18cad37117f545d58ef075027c2cc08))
- Updating CLAUDE.md for accuracy ([8141c53](https://github.com/novafluxx/filecrypter/commit/8141c533625ad855c0099f499626290f0df1e837))
- Updated with the new icon ([a5b028e](https://github.com/novafluxx/filecrypter/commit/a5b028e3e7a025233913232e3880b78002dce24c))
- Attempts at fixing icon transparency ([bc76876](https://github.com/novafluxx/filecrypter/commit/bc76876fb2599deee08089271fa41b5f4b675d3c))
- Removed claude settings.json ([29fc17a](https://github.com/novafluxx/filecrypter/commit/29fc17a7584499490a806c6895cc3ba109d6ba14))


### Refactoring

- **frontend:** Extract shared utilities and improve code quality ([b1368a8](https://github.com/novafluxx/filecrypter/commit/b1368a81be0cb990e2f195ee8ad1d4bba9d29a40))


### Documentation

- Updating user help ([8d45e5c](https://github.com/novafluxx/filecrypter/commit/8d45e5cf2970227a73a01b91cf1fc42f3f7b6736))


### CI

- Add path filters and concurrency control ([c7566d4](https://github.com/novafluxx/filecrypter/commit/c7566d4a56c1c38158e2fe32320f7475020e8b00))
- Add optimized CodeQL workflow with path filtering ([1f15685](https://github.com/novafluxx/filecrypter/commit/1f156857faac4a807f6cf2a8a141dcb48a2d8ff5))
- **release:** Add macOS Apple Silicon builds to release workflow ([2168ea1](https://github.com/novafluxx/filecrypter/commit/2168ea18e18783557612fd5c082f6000c0d85b25))
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


### Dependencies

- **ci:** Optimize workflows with caching and remove macOS builds ([eda4d2b](https://github.com/novafluxx/filecrypter/commit/eda4d2b957f0e902a0cd859f2de855b393b4cc7a))
- Update Rust dependencies ([85e90c2](https://github.com/novafluxx/filecrypter/commit/85e90c2d6e66720009d9c9a882945622cd510fc7))


### Miscellaneous

- **ci:** Update git-cliff to v2.11.0 ([3b3af7e](https://github.com/novafluxx/filecrypter/commit/3b3af7efae67ad032b7a02a7fa1e4c855d41ffef))
- **release:** Build only AppImage for Linux releases ([ae521ef](https://github.com/novafluxx/filecrypter/commit/ae521efdac376feb63af92e04354dfc570bfddf5))
- **release:** Build only NSIS installer for Windows ([f0380a3](https://github.com/novafluxx/filecrypter/commit/f0380a3e25bbd965786d3298462c31ddc5c8858e))
- Simplify bundle config and remove DMG background ([0a6b6de](https://github.com/novafluxx/filecrypter/commit/0a6b6de2386f15ceb679c8c96d89b4a05d26e44f))


### Refactoring

- **ci:** Remove redundant frontend build step ([249301f](https://github.com/novafluxx/filecrypter/commit/249301f6cd47d241be7ebf4a3c8ac7cf190213e8))
- **ci:** Simplify to Rust-only validation ([e01960f](https://github.com/novafluxx/filecrypter/commit/e01960fa6ebfb8152a7de1757e8c5da211112c74))


### Documentation

- Improve backend comment accuracy and clarity ([46d671d](https://github.com/novafluxx/filecrypter/commit/46d671d82d1d0807f30c8f7088df25625430ad59)) in [#9](https://github.com/novafluxx/filecrypter/pull/9)
- Add commit conventions to CLAUDE.md ([7174950](https://github.com/novafluxx/filecrypter/commit/71749504e30c50e5ab932618cd63f3f521840fd3))
<!-- generated by git-cliff -->
