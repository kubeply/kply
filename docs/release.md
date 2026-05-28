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
4. Confirm the release stays inside the
   [`docs/first-release.md`](first-release.md) scope.
5. Confirm the GitHub Actions `ci` workflow is passing on the release branch or
   commit being tagged.
6. Run validation:

   ```bash
   cargo fmt --all -- --check
   cargo check --all-targets --all-features --locked
   cargo clippy --all-targets --all-features --locked -- -D warnings
   cargo test --all-targets --all-features --locked
   cargo test -p kply-test --locked
   cargo deny check
   cargo xtask check-ci-workflow
   ```

7. Create a semver tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

Pull requests run `dist plan` for release-shape validation. Semver tag pushes
build archives, the shell installer, SHA-256 checksums, and GitHub artifact
attestations through the pinned `cargo-dist` workflow.

## Local Archive Smoke Test

After the release workflow uploads artifacts and before announcing the release,
download one archive for the current machine and verify it outside the source
tree:

```bash
version=v0.1.0
target=aarch64-apple-darwin
archive="kply-cli-${target}.tar.xz"
workdir="$(mktemp -d)"

gh release download "$version" \
  --repo kubeply/kply \
  --pattern "$archive" \
  --pattern "${archive}.sha256" \
  --dir "$workdir"

cd "$workdir"
shasum -a 256 --check "${archive}.sha256"
tar -xJf "$archive"
./kply --version
./kply --version --json
./kply help
```

Use the matching host target for the machine being tested:
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, or
`x86_64-unknown-linux-gnu`. Use `aarch64-unknown-linux-musl` or
`x86_64-unknown-linux-musl` only when testing a musl-based system or verifying
portable Linux compatibility.

## First-Release Checklist

Before tagging the first public binary release:

- Confirm `docs/first-release.md` still matches the behavior being released.
- Confirm the GitHub Actions `ci` workflow is green for the release commit.
- Confirm `cargo xtask check-ci-workflow` passes locally.
- Confirm `dist-workspace.toml` releases only `kply-cli`.
- Confirm Linux, portable Linux, and macOS targets are present in
  `dist-workspace.toml`.
- Confirm the shell installer, SHA-256 checksums, and GitHub artifact
  attestations are enabled.
- Confirm `.github/workflows/release.yml` is generated from the pinned
  `cargo-dist` version and keeps the least-privilege permission override.
- Confirm `README.md` contains the final install, upgrade, and rollback paths.
- Confirm `docs/release-notes-template.md` has been copied into the GitHub
  release notes draft and edited for the tagged version.
- Confirm one local archive has passed the local archive smoke test before the
  release is announced.
