# Cluster Init Sprint Tasks

- [ ] Define the `kply init --from-cluster` CLI contract.
- [ ] Add read-only Kubernetes discovery for namespaces, Deployments, Services,
      and Service selectors.
- [ ] Match Services to Deployments when selectors are deterministic.
- [ ] Generate a minimal `kply.yaml` with discovered apps.
- [ ] Add `--output <path>` and refuse existing files without `--overwrite`.
- [ ] Add human output with grouped sections, indentation, and color support.
- [ ] Add `--json`, `--quiet`, and `--no-color` behavior.
- [ ] Add tests and snapshots for empty cluster, one app, multiple apps, and
      ambiguous Service selectors.
- [ ] Document the onboarding flow in README and CLI docs.
