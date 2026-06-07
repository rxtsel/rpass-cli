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

## Commands

```bash
rpass list
rpass list --json
rpass search openai
rpass search openai --json
rpass show personal/openai.com
rpass show personal/openai.com --json
rpass otp personal/openai.com
rpass otp personal/openai.com --json
rpass doctor
rpass doctor --json
```

## Diagnostics

Run:

```bash
rpass doctor
```

It checks the store directory, `.gpg-id`, and GnuPG availability.
