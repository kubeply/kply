# Release

Kply uses semver and `cargo-dist` for binary releases.

## Versioning

- Patch releases fix bugs or improve docs/tests without changing the CLI
  contract.
- Minor releases add commands, output fields, routing adapters, or checks in a
  backward-compatible way.
- Major releases may change command semantics, JSON output contracts, or
  session lifecycle behavior.

## Process

1. Update workspace package version in `Cargo.toml`.
2. Update snapshots when output changes intentionally.
3. Draft release notes from
   [`docs/release-notes-template.md`](release-notes-template.md).
4. Run validation:

   ```bash
   cargo fmt --all -- --check
   cargo check --all-targets --all-features --locked
   cargo clippy --all-targets --all-features --locked -- -D warnings
   cargo test --all-targets --all-features --locked
   cargo deny check
   ```

5. Create a semver tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

Pull requests run `dist plan` for release-shape validation. Semver tag pushes
build archives, the shell installer, SHA-256 checksums, and GitHub artifact
attestations through the pinned `cargo-dist` workflow.

## First-Release Checklist

Before tagging the first public binary release:

- Confirm `dist-workspace.toml` releases only `kply-cli`.
- Confirm Linux, portable Linux, and macOS targets are present in
  `dist-workspace.toml`.
- Confirm the shell installer, SHA-256 checksums, and GitHub artifact
  attestations are enabled.
- Confirm `.github/workflows/release.yml` is generated from the pinned
  `cargo-dist` version and keeps the least-privilege permission override.
- Confirm `README.md` contains the final install command and upgrade path.
- Confirm `docs/release-notes-template.md` has been copied into the GitHub
  release notes draft and edited for the tagged version.
- Confirm one local archive has been unpacked and smoke-tested before the
  release is announced.
