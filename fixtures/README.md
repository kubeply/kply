# Fixtures

Fixture directories keep future CLI, config, manifest, Kubernetes response,
report, and demo data stable enough for snapshots and direct assertions.

## Naming

- CLI fixtures use `cli/<behavior-name>/`.
- Config fixtures use `config/<case-name>/kply.yaml`.
- Manifest fixtures use `manifests/<workload-shape>/`.
- Kubernetes response fixtures use `k8s-responses/<api-shape>/`.
- Report fixtures use `reports/<workflow-name>/`.
- Demo fixtures use `demo/<scenario-name>/`.
