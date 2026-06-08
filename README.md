# rpass

`rpass` is a native, cross-platform backend for password-store compatible
repositories.

It keeps the existing `pass`/password-store format:

```text
~/.password-store/
  personal/openai.com.gpg
  .gpg-id
```

Decrypted entries keep the usual first-line password format:

```text
password
username: alice
url: https://example.com
otpauth://totp/...
```

## Why

- Use password-store repositories from Windows, macOS, and Linux.
- Provide stable JSON output for launchers such as Raycast and Vicinae.
- Avoid Bash-specific behavior.
- Keep compatibility with existing `.gpg` entries and `.gpg-id` files.

## Installation

Prebuilt binaries and installers are available on the GitHub Releases page.

Install from crates.io with:

```bash
cargo install rpass-cli
```

The crates.io package is `rpass-cli`; the installed binary is `rpass`.

## Requirements

### Windows

- Gpg4win or GnuPG 2.x.
- `rpass` detects common GnuPG install paths automatically.
- You can also set `PASSWORD_STORE_GPG` to a specific `gpg.exe`.

### macOS

- GnuPG 2.x from a package manager or installer.
- `gpg` should be available in `PATH`.

### Linux

- GnuPG 2.x from your distribution packages.
- `gpg` should be available in `PATH`.

## Store Directory

`rpass` resolves the store directory in this order:

1. `--store-dir <PATH>`
2. `PASSWORD_STORE_DIR`
3. `~/.password-store`

## Current Scope

`rpass` reads existing password-store repositories and decrypts existing `.gpg`
entries with external GnuPG. It also supports inserting new entries with
external GnuPG encryption.

Write commands such as `generate`, `rm`, `mv`, and store initialization are
intentionally not implemented yet.

## Commands

```bash
rpass list
rpass list --json
rpass search openai
rpass search openai --json
rpass show personal/openai.com
rpass show personal/openai.com --json
rpass show personal/openai.com --json --passphrase-stdin
rpass insert personal/openai.com
rpass insert --echo personal/openai.com
printf 'password\nusername: alice\n' | rpass insert --multiline personal/openai.com
rpass insert --force personal/openai.com
rpass edit personal/openai.com
rpass otp personal/openai.com
rpass otp personal/openai.com --json
rpass otp personal/openai.com --json --passphrase-stdin
rpass doctor
rpass doctor --json
```

`insert` prompts for a password and confirmation when run in an interactive
terminal. Use `--echo` to show input, `--multiline` to read the full entry until
EOF, and `--force` to overwrite an existing entry. In multiline mode, the first
line is the password and additional lines are metadata.

`--passphrase-stdin` reads a single passphrase from standard input and passes it
to GnuPG through loopback pinentry. It is intended for integrations that cannot
show the native GPG pinentry UI.

## JSON Contract

Commands that accept `--json` follow this contract:

- exit code `0`: stdout contains one complete JSON value and stderr is empty;
- non-zero exit code: stderr contains one JSON error object and stdout is empty.

Error responses use this shape:

```json
{
  "error": {
    "code": "gpg_decrypt_failed",
    "message": "gpg failed to decrypt entry: ..."
  }
}
```

## Password-store Compatibility

Supported behavior:

- entries are addressed without the `.gpg` suffix;
- decrypted first line is the password;
- `name: value` metadata lines are preserved in JSON fields;
- `otpauth://` lines are used for TOTP generation;
- unknown lines are preserved as `extra_lines`;
- store directory is resolved from `--store-dir`, `PASSWORD_STORE_DIR`, then
  `~/.password-store`.

Known differences from `pass`:

- write support is limited to `insert`;
- shell completion, clipboard, QR code, Git, and edit workflows are not
  implemented;
- unsupported `pass` flags are rejected instead of ignored;
- JSON output is an `rpass` integration contract, not part of the original
  `pass` CLI.

## Releases

Prebuilt binaries and installers are available on the GitHub Releases page.
See `CHANGELOG.md` for release notes.

## Diagnostics

Run:

```bash
rpass doctor
```

It checks the store directory, `.gpg-id`, and GnuPG availability.
