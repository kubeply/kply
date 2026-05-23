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
