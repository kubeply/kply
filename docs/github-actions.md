# GitHub Actions

Use the Kply GitHub Action to produce a pull-request plan report without
installing Rust in the workflow.

The first supported action mode is `plan`. It reads `kply.yaml`, validates the
configured app, writes a JSON plan report, uploads that report as an artifact,
and can append a concise Markdown summary to the workflow run. Plan mode runs
without live cluster credentials.

`mode: check` is reserved for a later live-check runtime. When live checks are
implemented, those workflows will need explicit Kubernetes credentials and
least-privilege RBAC for the target cluster.

## Plan Workflow

```yaml
name: kply-plan

on:
  pull_request:

permissions:
  contents: read

jobs:
  plan:
    runs-on: ubuntu-22.04
    steps:
      - name: Checkout
        uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5 # v4
        with:
          persist-credentials: false

      - name: Run Kply plan
        uses: kubeply/kply@<REPLACE_WITH_COMMIT_SHA> # v0.1.0
        with:
          version: v0.1.0
          config-path: kply.yaml
          app: checkout
          image: ghcr.io/acme/checkout:${{ github.sha }}
          mode: plan
          output-path: kply-report.json
          artifact-name: kply-report
          write-summary: "true"
          github-token: ${{ github.token }}
```

Use a pinned Kply release version for the `version` input and pin the action
reference to a full commit SHA, optionally annotated with the release tag.
`version: latest` is accepted, but pinning keeps pull-request results
reproducible.

## Inputs

| Input | Required | Default | Description |
| --- | --- | --- | --- |
| `version` | no | `latest` | Kply release version to install. |
| `config-path` | no | `kply.yaml` | Path to the project config file. |
| `app` | conditional | none | Configured app name to plan. Required when not inferable from config; omission may cause failures in current action versions. |
| `image` | no | none | Candidate image reference for the sandbox workload. |
| `mode` | no | `plan` | Action mode. Only `plan` is supported today. |
| `output-path` | no | `kply-report.json` | Workspace-local JSON report path. |
| `artifact-name` | no | `kply-report` | Uploaded artifact name. |
| `write-summary` | no | `true` | Append a Markdown plan summary to the workflow run. |
| `github-token` | no | none | Token used to read release metadata. |

## Outputs

| Output | Description |
| --- | --- |
| `kply-path` | Absolute path to the installed `kply` binary. |
| `kply-version` | Version reported by the installed binary. |
| `config-path` | Resolved absolute config path. |
| `app` | Validated app name. |
| `image` | Validated image override, when provided. |
| `mode` | Executed action mode. |
| `output-path` | Absolute JSON report path. |
| `artifact-name` | Uploaded artifact name. |
| `write-summary` | Whether the Markdown summary was written. |

## Optional PR Comment

The action already writes a workflow summary and uploads the JSON report. If a
repository also wants a pull-request comment, use the default `github.token`;
no extra secret is required.

Add `pull-requests: write` to the workflow permissions:

```yaml
permissions:
  contents: read
  pull-requests: write
```

Then add a comment step after `Run Kply plan`:

```yaml
      - name: Comment Kply plan
        if: github.event.pull_request.head.repo.full_name == github.repository
        env:
          GH_TOKEN: ${{ github.token }}
          PR_URL: ${{ github.event.pull_request.html_url }}
        shell: bash
        run: |
          set -euo pipefail

          cat > /tmp/kply-pr-comment.md <<'EOF'
          ## Kply plan

          The Kply plan report was generated for this pull request.

          - JSON artifact: `kply-report`
          - Workflow summary: available on the Kply plan job
          EOF

          gh pr comment "${PR_URL}" --body-file /tmp/kply-pr-comment.md
```

The `if` guard skips comments on fork pull requests, where the default token is
usually read-only. Remove the guard only if the repository has a reviewed fork
workflow policy.

## Cluster Access

Plan workflows do not require `KUBECONFIG`, cloud credentials, or a live
Kubernetes API. The action deliberately runs plan-mode CLI calls with a
missing kubeconfig path so accidental cluster access fails instead of being
silently used.

Do not add production cluster credentials to plan-only workflows. Reserve
cluster credentials for future live-check workflows, and scope those
credentials to the permissions documented in [RBAC](rbac.md).
