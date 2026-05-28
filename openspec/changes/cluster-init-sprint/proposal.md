# Cluster Init Sprint

## Why

The current CLI can validate config and inspect app contracts, but a new user
must write `kply.yaml` before Kply shows meaningful app-level value. That makes
the first experience feel empty on real clusters.

This sprint gives humans and agents a starting map from the cluster itself.

## What Changes

- Add a read-only `kply init --from-cluster` workflow.
- Discover candidate applications from Kubernetes Deployments and Services.
- Generate a starter `kply.yaml` instead of requiring manual app config first.
- Render grouped, indented, color-aware human output.
- Preserve deterministic JSON output for agents.
- Refuse to overwrite an existing config unless the user passes `--overwrite`.

## Non-Goals

- No sandbox resource creation.
- No route mutation.
- No Secret value reads.
- No automatic production deploy or promotion.
- No large replacement roadmap.

## Success

A user with a working kubeconfig can run one command and get a useful starter
config plus clear next commands within 10 minutes.
