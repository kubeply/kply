# Kply

Kply is the Kubeply CLI for giving AI coding agents safe Kubernetes sessions
instead of raw cluster access.

The CLI is the first open-source surface for the larger Kubeply product: a
control boundary where agents can inspect workloads, create sandbox sessions,
run checks, and produce auditable reports before any production change is
promoted.

## Status

Early scaffold. The current code intentionally contains placeholders only:
workspace structure, crate boundaries, basic CLI entrypoint, tests, CI, release
planning, and OpenSpec context.

Sessions are not implemented yet.

## Product Primitive

A future Kply session is expected to be a temporary, scoped workspace for an
agent:

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

## Roadmap

See [docs/implementation-roadmap.md](docs/implementation-roadmap.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
