# Security Policy

Kply is an early CLI for safer Kubernetes sessions around AI coding agents. The
security policy exists to keep vulnerability reports private, actionable, and
separate from public product feedback.

## Supported Versions

Security reports are accepted for the latest released `v0.1.x` version and the
current `main` branch. Older unreleased commits may be considered when they
affect the current release line.

## Reporting A Vulnerability

Report vulnerabilities through
[GitHub private vulnerability reporting](https://github.com/kubeply/kply/security/advisories/new).
Do not open a public issue for a vulnerability.

Include as much of the following as possible:

- affected Kply version or commit SHA
- operating system and architecture
- command, config shape, or Kubernetes resource shape involved
- impact and expected attacker capability
- minimal reproduction steps with sensitive data removed
- whether Kubernetes Secret values, kubeconfigs, tokens, private hostnames, or
  tenant data may be exposed

## Security Scope

Examples of in-scope reports:

- Kply reads or prints Kubernetes Secret values.
- Kply mutates resources without an explicit apply or confirmation boundary.
- Kply generates unsafe RBAC, route, sandbox, cleanup, or report behavior that
  contradicts documented safety assumptions.
- Release artifacts, checksums, installers, or attestations do not match the
  documented release process.
- Agent-facing output could cause a reasonable terminal agent to perform a
  dangerous Kubernetes operation.

Examples that usually belong in public issue templates instead:

- missing route adapters
- unsupported Kubernetes workload shapes
- confusing but non-security CLI output
- feature requests for policy, checks, reports, or agent workflows

## Handling

Kply has no paid bug bounty program. Reports are reviewed on a best-effort
basis. Valid reports may result in a private advisory, patch release, public
changelog entry, or documentation update depending on impact.

We may ask for a reduced reproduction. Do not send live credentials, Secret
values, production kubeconfigs, private keys, or customer data.
