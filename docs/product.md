# Product Direction

Kply should not become a generic CD platform. Existing systems such as
Harness, Argo, GitHub Actions, and Flux already own deployment orchestration.

Kply focuses on the Kubeply agent boundary:

> Give AI agents safe Kubernetes sessions instead of raw cluster access.

## Initial Product Hypothesis

Kply CLI should eventually let a developer or coding agent create a sandbox
session for one Kubernetes workload and get a clear report about what would
happen. This is a roadmap hypothesis, not implemented behavior; the current
repository remains placeholder-only until the roadmap starts landing.
Sessions are not implemented yet.

## Expansion

1. Session contract definition.
2. Local Kind demo with a broken service.
3. Kubernetes sandbox deployment creation.
4. Gateway API route adapter.
5. Runtime checks and cleanup.
6. GitHub Action integration.
7. Team policy, audit, and hosted reporting.
