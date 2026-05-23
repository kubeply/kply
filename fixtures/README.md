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

## Snapshot Versus Direct Assertions

Use snapshots when the output is a structured artifact that humans need to
review as a whole:

- CLI text and JSON output
- generated Kubernetes manifests
- check reports
- route plans

Use direct assertions when the behavior has a small, stable contract:

- fixture path resolution
- exit codes
- single field validation
- normalization helper output

Prefer direct assertions for invariants and snapshots for reviewable artifacts.
