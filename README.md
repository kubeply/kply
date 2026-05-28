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

## Install

Install the latest released binary with the shell installer:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/kubeply/kply/releases/latest/download/kply-cli-installer.sh \
  | sh
```

The installer places `kply` under `CARGO_HOME` when that environment variable is
set, otherwise under the default cargo home directory.

Release archives are also published for Linux, portable Linux, and macOS on
x86_64 and aarch64. Each release includes SHA-256 checksums and GitHub artifact
attestations.

## Upgrade

Review the release notes before upgrading, especially for CLI output contract,
config schema, RBAC, routing, or generated Kubernetes resource changes.

Upgrade to the latest released binary by rerunning the installer:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/kubeply/kply/releases/latest/download/kply-cli-installer.sh \
  | sh
```

Verify the installed version:

```bash
kply --version
```

## Rollback

Rollback to a known-good release by installing from its release tag. Replace
`v0.1.0` with the version listed in the release notes or deployment record:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/kubeply/kply/releases/download/v0.1.0/kply-cli-installer.sh \
  | sh
```

Verify the rollback installed the expected version before running cluster
workflows:

```bash
kply --version
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

## Roadmap

See [docs/implementation-roadmap.md](docs/implementation-roadmap.md).

## CLI Contract

See [docs/cli.md](docs/cli.md) for command contract notes, including exit
codes.

## Gateway API

See [docs/gateway-api.md](docs/gateway-api.md) for Gateway API routing
permissions, ownership constraints, and fallback guidance when Gateway API is
unavailable.

## RBAC

See [docs/rbac.md](docs/rbac.md) for least-privilege Kubernetes RBAC examples
for read-only inspection, sandbox-only sessions, and optional route mutation.

## GitHub Actions

See [docs/github-actions.md](docs/github-actions.md) for running Kply plan
reports in pull-request workflows.

## Local Demo

See [docs/demo-kind.md](docs/demo-kind.md) for the current manual Kind setup
guide.

## Security

See [SECURITY.md](SECURITY.md) for private vulnerability reporting.

## License

Apache-2.0. See [LICENSE](LICENSE).
