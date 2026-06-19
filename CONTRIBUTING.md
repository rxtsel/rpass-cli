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

For dependency advisory checks, install `cargo-audit` if needed and run:

```bash
cargo install cargo-audit --locked
cargo audit
```

## Branching Model

Use trunk-based development with short-lived branches.

- `main` is the stable branch and should stay green.
- Use `feat/...` for features.
- Use `fix/...` for bug fixes.
- Use `docs/...` for documentation.
- Use `ci/...` for CI and release automation.
- `release-plz-*` branches are managed by release-plz.

Do not use a long-lived `develop` branch or Git Flow. Keep branches focused,
open a pull request, wait for CI, then merge into `main`.

Recommended flow:

```bash
git checkout main
git pull --ff-only
git checkout -b fix/example-bug
# make changes
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
git commit -m "fix(scope): summary"
git push -u origin fix/example-bug
```

Prefer squash merging normal pull requests so the final commit message is a
clean Conventional Commit. Make the pull request title match the intended commit
message.

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
- release-plz branch names use timestamps, such as `release-plz-2026-06-07T22-46-17Z`.
- Normal development pushes do not create releases.
- Merging a release pull request creates the `vX.Y.Z` tag.
- cargo-dist builds release assets and creates or updates the GitHub Release.
- release-plz publishes to crates.io automatically after creating the tag.

Review the release pull request before merging. Edit changelog entries if the
automated summary is unclear.

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
