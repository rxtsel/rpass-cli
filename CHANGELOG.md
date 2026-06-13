# Changelog

All notable changes to this project will be documented in this file.

This project uses [Conventional Commits](https://www.conventionalcommits.org/)
and release automation with release-plz.

## [0.1.10](https://github.com/rxtsel/rpass-cli/compare/v0.1.9...v0.1.10) - 2026-06-13

### 🚀 Features

- *(cli)* Add pass-compatible git command

- *(cli)* Auto-commit store writes


### 🐛 Bug Fixes

- *(cli)* Allow git init on empty stores



## [0.1.9](https://github.com/rxtsel/rpass-cli/compare/v0.1.8...v0.1.9) - 2026-06-12

### 🚀 Features

- *(cli)* Add generate dry-run



## [0.1.8](https://github.com/rxtsel/rpass-cli/compare/v0.1.7...v0.1.8) - 2026-06-12

### 📚 Documentation

- *(readme)* Simplify command examples



## [0.1.7](https://github.com/rxtsel/rpass-cli/compare/v0.1.6...v0.1.7) - 2026-06-12

### 🚀 Features

- *(cli)* Add mv command



## [0.1.6](https://github.com/rxtsel/rpass-cli/compare/v0.1.5...v0.1.6) - 2026-06-12

### 📚 Documentation

- *(cli)* Group generate help options


### ⚙️ Miscellaneous Tasks

- Move dependency audit to dedicated workflow



## [0.1.5](https://github.com/rxtsel/rpass-cli/compare/v0.1.4...v0.1.5) - 2026-06-12

### 🚀 Features

- *(cli)* Add rm command



## [0.1.4](https://github.com/rxtsel/rpass-cli/compare/v0.1.3...v0.1.4) - 2026-06-12

### Added

- Add `edit` support for creating and updating entries with `$EDITOR`.
- Add `generate` support for random passwords and memorable passphrases.
- Add `rpass <entry>` as a `pass`-compatible shorthand for `show`.

### Fixed

- Preserve existing encrypted entries if GPG encryption fails during writes.
- Remove plaintext `--passphrase` arguments in favor of `--passphrase-stdin`.
- Fix Windows editor handling for temporary edit files.

### Documentation

- Refresh README command examples and current feature scope.

### Maintenance

- Add dependency advisory checks to CI.
- Update release metadata for the renamed `rpass-cli` package.

## [0.1.3](https://github.com/rxtsel/rpass/compare/v0.1.2...v0.1.3) - 2026-06-07

### Other

- *(workflows)* cancel stale runs
- *(release)* use pat for release-plz

## [0.1.2](https://github.com/rxtsel/rpass/compare/v0.1.1...v0.1.2) - 2026-06-07

### Other

- *(contributing)* document branching model

## [0.1.1](https://github.com/rxtsel/rpass/compare/v0.1.0...v0.1.1) - 2026-06-07

### Other

- *(contributing)* document release workflow
- *(release)* build cargo-dist assets

## [0.1.0] - 2026-06-07

### Added

- Add read-only password-store compatible commands: `list`, `search`, `show`,
  `otp`, and `doctor`.
- Add stable JSON output for read-only commands and structured JSON errors.
- Add external GnuPG decrypt support with loopback passphrase input through
  `--passphrase-stdin`.
- Add TOTP generation from `otpauth://` entry lines.
- Add cross-platform Rust CI for Linux, macOS, and Windows.

### Fixed

- Fix first decrypt after locked GPG agent when integrations provide passphrase
  over stdin.
- Prevent decrypted GPG output from leaking directly to stdout before JSON
  serialization.
- Treat successful GPG decrypts with empty stdout as explicit errors.

### Documentation

- Document read-only scope, JSON contract, password-store compatibility, and
  known differences from `pass`.
