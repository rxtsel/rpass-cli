# Contributing

## Development

Use the Nix dev shell or an equivalent Rust toolchain with `cargo`, `rustfmt`,
and `clippy`.

```bash
nix develop
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Commits

Use Conventional Commits:

```text
fix(gpg): capture loopback passphrase output
feat(cli): add command
ci(release): build cargo-dist assets
docs(readme): document compatibility
```

Keep commits small and focused. Use lowercase commit messages so commitlint can
validate them if enabled later.

## Release Process

Releases use release-plz, cargo-dist, and Conventional Commits.

- release-plz maintains release pull requests on pushes to `main`.
- Normal development pushes do not create releases.
- Merging a release pull request creates the `vX.Y.Z` tag.
- cargo-dist builds release assets and creates or updates the GitHub Release.
- Publishing to crates.io is disabled for now.

Release assets currently include:

- macOS Intel archive;
- macOS Apple Silicon archive;
- Linux x86_64 archive;
- Linux ARM64 archive;
- Linux musl x86_64 archive;
- Windows zip;
- Windows MSI;
- shell installer;
- PowerShell installer;
- checksums;
- source tarball.

For the first release or for rebuilding assets for an existing tag, run the
release workflow manually:

```bash
gh workflow run release.yml --ref main -f tag=v0.1.0
gh run watch
```

## Changelog

`CHANGELOG.md` is the user-facing release history. Keep it aligned with
release-plz output and edit release PR changelog entries before merging when
needed.
