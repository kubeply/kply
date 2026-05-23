# Architecture

Kubeply starts as a CLI-first safety layer for AI agents working near
Kubernetes.

## Actors

```text
AI coding agent / human
        |
        | runs kubeply commands
        v
kubeply CLI
        |
        | creates scoped sessions and checks
        v
Kubernetes API / routing layer
        |
        v
sandbox workloads, temporary routes, reports, cleanup
```

The first interface is the CLI because agents already operate terminals well.
MCP can be added later as another adapter over the same core.

## Session

A session is the core primitive. It represents a bounded attempt to test a
change against Kubernetes-like reality without giving an agent direct production
mutation access.

Initial session fields:

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
kubeply-cli
  -> kubeply-config
  -> kubeply-checks
  -> kubeply-routing
  -> kubeply-k8s
  -> kubeply-core
```

Core does not depend on Kubernetes client libraries or CLI output.

## First Workflow

The first workflow is dry-run session planning:

1. Accept target workload and proposed image.
2. Build a session plan.
3. Render human or JSON output.
4. Snapshot-test output for agent compatibility.

Kubernetes execution will be added behind adapters once the session contract is
stable.
