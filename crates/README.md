# Crates

The workspace is split by product responsibility, not by implementation phase.
Some crates are thin on purpose so future work lands in the correct boundary.

```text
kply-cli      command-line interface for humans and agents
kply-core     session domain model, audit events, state transitions
kply-config   kply.yaml parsing and validation
kply-k8s      Kubernetes API adapters
kply-routing  route adapter traits and implementations
kply-checks   runtime checks and report generation
kply-test     integration test helpers
xtask            repository automation tasks
```
