# Cluster Init Sprint Tasks

- [x] Define the `kply init --from-cluster` CLI contract.
- [x] Add read-only Kubernetes discovery for namespaces, Deployments, Services,
      and Service selectors.
- [x] Match Services to Deployments when selectors are deterministic.
- [x] Generate a minimal `kply.yaml` with discovered apps.
- [x] Add `--output <path>` and refuse existing files without `--overwrite`.
- [x] Add human output with grouped sections, indentation, and color support.
- [x] Add `--json`, `--quiet`, and `--no-color` behavior.
- [x] Add tests and snapshots for empty cluster, one app, multiple apps, and
      ambiguous Service selectors.
- [x] Document the onboarding flow in README and CLI docs.
