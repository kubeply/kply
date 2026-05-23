# Kply

Kply is the Kubeply CLI for giving AI coding agents safe Kubernetes sessions
instead of raw cluster access.

The CLI is the first open-source surface for the larger Kubeply product: a
control boundary where agents can inspect workloads, create sandbox sessions,
run checks, and produce auditable reports before any production change is
promoted.

## Status

Early scaffold. The current code establishes the Rust workspace, CLI shape,
session model, tests, CI, release packaging, and OpenSpec context. Kubernetes
execution adapters are intentionally thin until the first workflow is validated.

## Example

```bash
kply session create backend-api \
  --namespace shop \
  --image ghcr.io/acme/backend:agent-fix-123 \
  --route-header x-kply-session \
  --route-value fix-123 \
  --dry-run \
  --json
```

## Product Primitive

A Kply session is a temporary, scoped workspace for an agent:

- target workload
- proposed image or config change
- sandbox deployment and service
- optional route rule for agent/test traffic
- runtime checks
- cleanup plan
- audit report

## Development

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
```

## License

Apache-2.0. See [LICENSE](LICENSE).
