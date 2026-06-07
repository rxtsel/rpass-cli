# Changelog

All notable changes to this project will be documented in this file.

This project uses [Conventional Commits](https://www.conventionalcommits.org/)
and release automation with release-plz.

## [0.1.3](https://github.com/rxtsel/rpass/compare/v0.1.2...v0.1.3) - 2026-06-07

### Other

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
