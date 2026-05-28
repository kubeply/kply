# Contributing

Kply is early, but the engineering bar should be high from the first commit.

## Local Checks

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo xtask check-crate-inventory-docs
cargo xtask check-deny-config
cargo xtask check-fixture-directories
cargo xtask check-fixture-naming-docs
cargo xtask check-fixture-testing-docs
cargo xtask check-future-session-docs
cargo xtask check-issue-templates
cargo xtask check-known-limitations-docs
cargo xtask check-license-files
cargo xtask check-module-docs
cargo xtask check-placeholder-docs
cargo xtask check-placeholders
cargo xtask check-readme-roadmap-link
cargo xtask check-release-planning
cargo xtask check-security-assumptions-docs
cargo xtask check-toolchain-pin
```

## Testing

- Use unit tests for core models and state transitions.
- Use integration tests for CLI behavior.
- Use `insta` snapshots for structured command output.
- Keep test fixtures focused and deterministic.

Optional live Kubernetes tests are read-only and skipped unless explicitly
enabled:

```bash
KPLY_LIVE_K8S_TESTS=1 KPLY_LIVE_K8S_NAMESPACE=default cargo test -p kply-k8s --test live_cluster --locked
```

Live tests use standard kubeconfig resolution. Keep `KPLY_LIVE_K8S_NAMESPACE`
scoped to a namespace where listing Deployments and Services is acceptable.

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
