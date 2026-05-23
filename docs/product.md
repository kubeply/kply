# Product Direction

Kubeply should not become a generic CD platform. Existing systems such as
Harness, Argo, GitHub Actions, and Flux already own deployment orchestration.

Kubeply focuses on the agent boundary:

> Give AI agents safe Kubernetes sessions instead of raw cluster access.

## Initial Product

Kubeply CLI lets a developer or coding agent create a sandbox session for one
Kubernetes workload and get a clear report about what would happen.

## Expansion

1. CLI dry-run session planning.
2. Local Kind demo with a broken service.
3. Kubernetes sandbox deployment creation.
4. Gateway API route adapter.
5. Runtime checks and cleanup.
6. GitHub Action integration.
7. Team policy, audit, and hosted reporting.
