# Architecture

Kply starts as a CLI-first scaffold for a future safety layer around AI agents
working near Kubernetes.

## Actors

```text
AI coding agent / human
        |
        | runs kply commands
        v
kply CLI placeholder
        |
        | creates scoped sessions and checks
        v
Kubernetes API / routing layer
        |
        v
future sandbox workloads, temporary routes, reports, cleanup
```

The first interface is the CLI because agents already operate terminals well.
MCP can be added later as another adapter over the same core.

## CLI Command Naming

Command names are part of the agent-facing contract. They must stay boring,
predictable, and easy to discover from `kply --help`.

Rules:

- Use lowercase kebab-case for multi-word commands and flags.
- Prefer nouns for command groups, such as `session`, `app`, `config`,
  `cluster`, and `report`.
- Prefer explicit verbs for subcommands once behavior exists, such as `show`,
  `validate`, `plan`, `start`, `verify`, and `cleanup`.
- Keep mutation verbs explicit for user-facing destructive commands, such as
  `session delete` or `resource destroy`.
- Reserve plan or preview commands for non-mutating output by default. If a
  plan command can later perform the planned change, the canonical confirmation
  flag is `--apply`, for example `session plan --apply`.
- Avoid hidden aliases until the primary command surface is stable.
- Do not reuse a command name for different resource types.
- Keep JSON field names aligned with command names when the command produces
  machine-readable output.

## Future Session

A session is the expected core primitive. It will represent a bounded attempt
to test a change against Kubernetes-like reality without giving an agent direct
production mutation access.

Sessions are not implemented yet.

Candidate session fields:

- `id`
- `workload`
- `namespace`
- `image`
- `route_header`
- `status`
- `checks`
- `created_at`

## Crate Direction

```text
kply-cli
  -> kply-config
  -> kply-checks
  -> kply-routing
  -> kply-k8s
  -> kply-core
```

Core does not depend on Kubernetes client libraries or CLI output.

## Current Workflow

The current workflow is intentionally minimal:

1. Preserve crate boundaries.
2. Print placeholder CLI output.
3. Keep tests and CI green.

Real session planning and Kubernetes execution will be added only after the
roadmap is defined.
