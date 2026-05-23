# Crates

The workspace is split by product responsibility, not by implementation phase.
Some crates are thin on purpose so future work lands in the correct boundary.

```text
kubeply-cli      command-line interface for humans and agents
kubeply-core     session domain model, audit events, state transitions
kubeply-config   kubeply.yaml parsing and validation
kubeply-k8s      Kubernetes API adapters
kubeply-routing  route adapter traits and implementations
kubeply-checks   runtime checks and report generation
kubeply-test     integration test helpers
xtask            repository automation tasks
```
