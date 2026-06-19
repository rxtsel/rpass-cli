# rpass

[![Crates.io Version](https://img.shields.io/crates/v/rpass-cli)](https://crates.io/crates/rpass-cli)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/rpass-cli)](https://crates.io/crates/rpass-cli)
[![CI](https://img.shields.io/github/actions/workflow/status/rxtsel/rpass-cli/rust.yml?branch=main)](https://github.com/rxtsel/rpass-cli/actions)
[![License](https://img.shields.io/github/license/rxtsel/rpass-cli)](LICENSE)

**rpass** is a cross-platform [`pass`](https://www.passwordstore.org/)-compatible CLI for managing GPG-encrypted secrets. It reads and writes password-store repositories on Linux, macOS, and Windows, and provides stable JSON output for launchers, plugins, and scripts.

```text
rpass show example/login
rpass list --json
rpass otp example/login --json
rpass generate example/login
rpass git status
```

## Features

- **Works everywhere** — Linux, macOS, Windows (no Bash dependency)
- **password-store compatible** — reads and writes existing `.gpg` entries and `.gpg-id` files
- **JSON output** — structured responses for Raycast, Vicinae, and custom integrations
- **TOTP support** — generate one-time codes from `otpauth://` lines
- **Git integration** — explicit `rpass git <args>` commands; write commands auto-commit
- **Shell completions** — Bash, Zsh, Fish, PowerShell
- **Non-interactive** — `--passphrase-stdin` for headless environments
- **External GPG** — no embedded crypto; uses your existing GnuPG installation

## Installation

### Prebuilt binaries

Download from the [GitHub Releases](https://github.com/rxtsel/rpass-cli/releases) page. Archives, MSI (Windows), shell installers, and checksums are available for all platforms.

### crates.io

```bash
cargo install rpass-cli
```

The crates.io package is `rpass-cli`; the installed binary is `rpass`.

## Requirements

- **GnuPG 2.x** (`gpg`) — install via Gpg4win (Windows), Homebrew (macOS), or your system package manager (Linux)
- **Git** — optional, only needed for `rpass git ...`

## Quick Start

```bash
rpass init alice@example.com              # initialize a store
rpass list                                # list entries
rpass show example/login                  # show an entry
rpass generate example/login              # generate a 14-character password
rpass insert example/login                # insert a password
rpass edit example/login                  # edit an entry
rpass rm example/login                    # remove an entry
rpass mv example/login archive/login      # move/rename an entry
rpass otp example/login                   # generate a TOTP code
rpass git status                          # run git inside the store
rpass doctor                              # check your local setup
```

## Store Directory

`rpass` resolves the store in this order:

1. `--store-dir <PATH>`
2. `PASSWORD_STORE_DIR`
3. `~/.password-store`

## JSON Output

Most read commands accept `--json`. On success, stdout contains the JSON value and stderr is empty. On error, the exit code is non-zero and stderr contains:

```json
{
  "error": {
    "code": "gpg_decrypt_failed",
    "message": "gpg failed to decrypt entry: ..."
  }
}
```

Non-interactive workflows:

```bash
printf 'gpg-passphrase\n' | rpass show example/login --json --passphrase-stdin
printf 'gpg-passphrase\n' | rpass otp example/login --json --passphrase-stdin
```

## Shell Completions

```bash
rpass completions bash >> ~/.bashrc
rpass completions zsh > "${fpath[1]}/_rpass"
rpass completions fish > ~/.config/fish/completions/rpass.fish
rpass completions powershell >> $PROFILE
```

## password-store Compatibility

**Supported:**
- Entries addressed without the `.gpg` suffix
- First decrypted line is the password
- `name: value` metadata lines preserved in JSON
- `otpauth://` lines for TOTP
- Unknown lines preserved as `extra_lines`
- Directory-level recipients with `.gpg-id`

**Known differences from `pass`:**
- `generate`, `insert`, `edit`, `rm`, and `mv` for writes
- Git is explicit (`rpass git <args>`) rather than automatic
- Changing recipients with `init` does not re-encrypt existing entries
- Clipboard and QR codes are not implemented
- Unsupported `pass` flags are rejected instead of ignored

## License

[MIT](LICENSE)
