# Import

Import passwords from other password managers into your rpass store.

    rpass import --bitwarden <file>

## Bitwarden

1. In Bitwarden, go to **Tools → Export vault** and select **File format: JSON** (unencrypted).
2. Run: `rpass import --bitwarden ~/bitwarden_export.json`

| Flag | Description |
|---|---|
| `--force` | Overwrite entries with matching names; generated suffixes that already exist require manual resolution |
| `--json` | JSON output |

Supports individual and organization vaults. Folders and collections become path prefixes; line breaks in titles become `-` in filenames. Names that match when lowercased get distinct suffixes even on case-sensitive filesystems, avoiding common collisions on macOS and Windows. All item types are handled: logins, secure notes, cards, and identities. Names that become invalid when used as paths, multiline passwords, and fields that cannot be stored unchanged reject the export before any entry is written. Imported note lines are marked `# rpass-note ` in the raw entry and returned without that marker in JSON `extra_lines`, so URLs and OTP-like text remain notes. If an individual entry cannot be saved, the command reports how many were imported and skipped and exits with an error; check the report before deleting the original export. Retrying without `--force` reports existing entries as conflicts instead of creating duplicates; use `--force` to overwrite entries from the previous attempt after reviewing the report.
