# Import

Import passwords from other password managers into your rpass store.

    rpass import --bitwarden <file>

## Bitwarden

1. In Bitwarden, go to **Tools → Export vault** and select **File format: JSON** (unencrypted).
2. Run: `rpass import --bitwarden ~/bitwarden_export.json`

| Flag | Description |
|---|---|
| `--force` | Overwrite existing entries |
| `--json` | JSON output |

Supports individual and organization vaults. Folders and collections become path prefixes. All item types are handled: logins, secure notes, cards, and identities.
