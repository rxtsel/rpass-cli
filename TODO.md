# rpass TODO

`rpass` is a native, cross-platform backend for password-store compatible
repositories. The goal is compatibility with the password-store format first,
not full command-for-command parity with the original `pass` CLI.

## Product Principles

- Preserve the password-store file format:
  - entries are stored as `<entry-name>.gpg`;
  - recipients are discovered from `.gpg-id`;
  - decrypted content keeps the password on the first line;
  - additional metadata stays as plain text lines such as `username: alice`,
    `url: https://example.com`, and `otpauth://...`.
- Prefer stable JSON output for integrations such as Raycast and Vicinae.
- Use external GnuPG in the first implementation phase.
- Keep future OpenPGP-native support optional and behind a clear abstraction.
- Treat compatibility with existing stores as a product requirement.

## Engineering Principles

- Use conventional commits.
- Keep changes small, testable, and focused.
- Prefer standard CLI conventions for command names, flags, aliases, exit codes,
  streams, and help output.
- Prefer intention-revealing functions over inline policy conditions.
- Split multi-step procedures into small functions with one responsibility.
- Treat code smells as blockers during implementation and review.
- Prefer Rust idioms over imported architecture ceremony.
- Introduce design patterns only when they solve present coupling, variation,
  orchestration, or readability problems.
- Make intentional patterns visible in names when doing so improves clarity.
- Avoid speculative abstractions, duplicate code, dead code, and comments that
  restate the implementation.
- Use tests as design pressure, especially for parsing, path handling,
  compatibility behavior, and error cases.

## Phase 1: Read-Only Backend

Goal: provide reliable read-only access to existing password-store compatible
repositories with structured JSON output.

### CLI Surface

- [x] Keep commands and flags aligned with common CLI standards unless there is
  a clear password-store compatibility reason to differ.
- [x] `rpass list`
- [x] `rpass list --json`
- [x] `rpass show <entry>`
- [x] `rpass show <entry> --json`
- [x] `rpass otp <entry>`
- [x] `rpass otp <entry> --json`
- [x] `rpass search <query>`
- [x] `rpass search <query> --json`
- [x] `rpass doctor`
- [x] `rpass doctor --json`

### Store Discovery

- [x] Resolve the store directory from `PASSWORD_STORE_DIR`.
- [x] Fall back to the platform-appropriate default store path.
- [x] Validate that the selected store exists.
- [x] Return structured errors for missing, unreadable, or invalid stores.

### Entry Listing

- [x] Recursively discover `.gpg` files.
- [x] Convert file paths into password-store entry names.
- [x] Ignore non-entry files such as `.gpg-id` and Git metadata.
- [x] Sort entries deterministically.
- [x] Add fixtures for nested stores and edge-case entry names.

### GPG Decryption

- [x] Add a narrow GPG adapter for invoking external `gpg`.
- [x] Keep command construction isolated and testable.
- [x] Surface decryption failures as structured application errors.
- [x] Avoid leaking decrypted secrets into logs or debug output.

### Entry Parsing

- [x] Parse the first line as the password.
- [x] Parse common metadata fields such as `username`, `login`, `email`, and
  `url`.
- [x] Detect `otpauth://` lines.
- [x] Preserve unrecognized lines for compatibility.
- [x] Add table-driven tests for common password-store entry shapes.

### OTP

- [x] Parse TOTP data from `otpauth://` URIs.
- [x] Generate current TOTP codes.
- [x] Return remaining validity seconds in JSON output.
- [x] Add deterministic tests using fixed timestamps.

### JSON Contract

- [x] Define stable response structs.
- [x] Define stable error structs.
- [x] Keep JSON field names explicit and integration-friendly.
- [x] Add snapshot-style tests for JSON output.

## Phase 2: Compatible Writes

Goal: add write operations while preserving interoperability with `pass`, iOS,
Android Password Store, and existing Git-based stores.

- [x] `rpass insert <entry>`
- [x] `rpass edit <entry>`
- [x] `rpass generate <entry> <length>`
- [x] `rpass rm <entry>`
- [ ] `rpass mv <old-entry> <new-entry>`
- [x] Resolve recipients from the nearest `.gpg-id`.
- [x] Encrypt with external `gpg`.
- [x] Preserve directory-level recipient behavior.
- [x] Add compatibility fixtures for encrypted writes.

## Phase 3: Git Integration

Goal: provide explicit, predictable Git commands for password-store workflows.

- [ ] `rpass git status`
- [ ] `rpass git pull`
- [ ] `rpass git push`
- [ ] `rpass git log`
- [ ] Keep Git integration optional.
- [ ] Return structured errors for missing Git repositories.

## Phase 4: Initialization And Store Management

Goal: support creation and maintenance of password-store compatible stores.

- [ ] `rpass init <key-id>`
- [ ] `rpass recipients`
- [ ] `rpass recipients add <key-id>`
- [ ] `rpass recipients remove <key-id>`
- [ ] Support multiple stores only after the single-store model is stable.

## Phase 5: Compatibility Hardening

Goal: earn stronger compatibility claims through automated evidence.

- [ ] Build a fixture suite from real-world password-store layouts.
- [ ] Test behavior against the original `pass` where practical.
- [x] Document known differences from `pass`.
- [x] Add cross-platform CI for Windows, macOS, and Linux.
- [ ] Decide which `pass` CLI flags are intentionally unsupported.
- [ ] Add Debian package release asset.
- [ ] Add AppImage release asset.
- [ ] Add AUR packaging for Arch users.

## Near-Term Technical Notes

- Start with a small application core and a thin CLI layer.
- Keep filesystem, GPG, and clock access behind narrow boundaries for testing.
- Prefer explicit domain types for entry names, store paths, parsed entries,
  OTP secrets, and JSON responses.
- Avoid primitive obsession around paths and entry names once behavior grows.
- Do not add a facade, strategy, command, or observer unless the code has a
  concrete variation point that benefits from it.
