# Kply

Kply is the Kubeply CLI for giving AI coding agents safe Kubernetes sessions
instead of raw cluster access.

The CLI is the first open-source surface for the larger Kubeply product: a
control boundary where agents can inspect workloads, create sandbox sessions,
run checks, and produce auditable reports before any production change is
promoted.

## Status

Implementation in progress. The workspace now includes real session planning,
Kubernetes discovery, sandbox create/cleanup, early runtime check support, and
Gateway API routing groundwork.

Session mutation commands require explicit `--apply` confirmation.

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

## Roadmap

See [docs/implementation-roadmap.md](docs/implementation-roadmap.md).

## CLI Contract

See [docs/cli.md](docs/cli.md) for command contract notes, including exit
codes.

## Gateway API

See [docs/gateway-api.md](docs/gateway-api.md) for Gateway API routing
permissions and ownership constraints.

## Local Demo

See [docs/demo-kind.md](docs/demo-kind.md) for the current manual Kind setup
guide.

## License

Apache-2.0. See [LICENSE](LICENSE).
