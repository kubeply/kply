# Product Direction

Kply should not become a generic CD platform. Existing systems such as
Harness, Argo, GitHub Actions, and Flux already own deployment orchestration.

Kply focuses on the Kubeply agent boundary:

> Give AI agents safe Kubernetes sessions instead of raw cluster access.

## Initial Product Hypothesis

Kply CLI should let a developer or coding agent create a sandbox session for
one Kubernetes workload and get a clear report about what happened. This is a
roadmap hypothesis, partially implemented behavior: session create/cleanup has
started, runtime checks are landing, and routing remains placeholder-only until
its roadmap milestone starts.

## Expansion

1. Session contract definition.
2. Local Kind demo with a broken service.
3. Kubernetes sandbox deployment creation.
4. Gateway API route adapter.
5. Runtime checks and cleanup.
6. GitHub Action integration.
7. Team policy, audit, and hosted reporting.
