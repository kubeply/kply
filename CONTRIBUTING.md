# Contributing

Kply is early, but the engineering bar should be high from the first commit.

## Local Checks

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo xtask check-module-docs
cargo xtask check-placeholders
```

## Testing

- Use unit tests for core models and state transitions.
- Use integration tests for CLI behavior.
- Use `insta` snapshots for structured command output.
- Keep test fixtures focused and deterministic.

## Crate Boundaries

- `kply-core`: domain model, session state, audit events.
- `kply-config`: project and cluster config parsing.
- `kply-k8s`: Kubernetes discovery and mutation adapters.
- `kply-routing`: Gateway, ingress, mesh, and fallback routing adapters.
- `kply-checks`: runtime verification checks.
- `kply-cli`: command parsing and user/agent-facing output.
- `kply-test`: shared test helpers.

## Release

Releases are expected to use `cargo-dist` and semver tags after the first
usable binary exists.
