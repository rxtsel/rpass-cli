# rpass

`rpass` is a native, cross-platform backend for password-store compatible
repositories.

It keeps the existing `pass`/password-store format:

```text
~/.password-store/
  example/login.gpg
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
- Git is optional and only required for `rpass git ...` workflows.

### macOS

- GnuPG 2.x from a package manager or installer.
- `gpg` should be available in `PATH`.
- Git is optional and only required for `rpass git ...` workflows.

### Linux

- GnuPG 2.x from your distribution packages.
- `gpg` should be available in `PATH`.
- Git is optional and only required for `rpass git ...` workflows.

## Store Directory

`rpass` resolves the store directory in this order:

1. `--store-dir <PATH>`
2. `PASSWORD_STORE_DIR`
3. `~/.password-store`

## Status

`rpass` can initialize, list, search, show, generate, insert, edit, remove,
move, and run Git commands for password-store entries using external GnuPG. It
also supports TOTP generation from `otpauth://` lines.

Commands such as clipboard support are intentionally not implemented yet.

## Commands

```bash
rpass -h                                      # show help
rpass list                                    # list entries
rpass search example                          # search entries
rpass show example/login                      # show an entry explicitly
rpass init alice@example.com                  # initialize .gpg-id recipients
rpass generate example/login                  # generate and save a 14-character password
rpass insert example/login                    # insert a password interactively
rpass edit example/login                      # edit or create an entry
rpass rm example/login                        # remove an entry
rpass mv example/login archive/login          # move or rename an entry
rpass git status                              # run git inside the store
rpass git init                                # initialize store Git history
rpass otp example/login                       # generate an OTP code
rpass doctor                                  # check local setup
```

`init` creates the store if needed and writes `.gpg-id` recipients. Use
`--path <subfolder>` or `-p <subfolder>` for directory-level recipients.

`generate` writes to the store by default. Use `--dry-run` to print a generated
password or passphrase without opening the store, requiring `.gpg-id`, or calling
GPG. Use `--length <N>` with `--dry-run` when no entry name is provided.

`insert` prompts for a password and confirmation when run in an interactive
terminal. Use `--echo` to show input, `--multiline` to read the full entry until
EOF, and `--force` to overwrite an existing entry. In multiline mode, the first
line is the password and additional lines are metadata.

`rpass git <args...>` passes arguments to Git using the password store as the
repository. `rpass git init` also stages the current store and creates the same
initial commit used by `pass`. When the store is a Git repository, write
commands automatically create matching commits. Use `rpass git --json <args...>`
for structured stdout, stderr, and exit code output.

Most read commands support `--json` for integrations. Commands that decrypt
entries also support `--passphrase-stdin` for non-interactive integrations:

```bash
printf 'gpg-passphrase\n' | rpass show example/login --json --passphrase-stdin
printf 'gpg-passphrase\n' | rpass otp example/login --json --passphrase-stdin
```

Run `rpass <command> --help` for command-specific flags.

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

- write support is limited to `generate`, `insert`, `edit`, `rm`, and `mv`;
- Git integration is explicit through `rpass git <args...>`;
- changing recipients with `init` does not re-encrypt existing entries yet;
- shell completion, clipboard, and QR code are not implemented;
- unsupported `pass` flags are rejected instead of ignored;
- JSON output is an `rpass` integration contract, not part of the original
  `pass` CLI.
