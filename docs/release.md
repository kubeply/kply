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
3. Run validation:

   ```bash
   cargo fmt --all -- --check
   cargo check --all-targets --all-features --locked
   cargo clippy --all-targets --all-features --locked -- -D warnings
   cargo test --all-targets --all-features --locked
   cargo deny check
   ```

4. Create a semver tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

The release workflow currently runs `dist plan`. Once the first public binary is
ready, regenerate the full `cargo-dist` release workflow and keep pinned actions.
