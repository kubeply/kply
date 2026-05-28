# CLI Contract

Kply is CLI-first. Commands must stay predictable for both humans and terminal
driven AI agents.

## Exit Codes

| Code | Name | Meaning |
| --- | --- | --- |
| `0` | success | The command completed successfully. |
| `1` | blocking | A session, check, or report completed but found a blocking result. |
| `2` | usage | The command could not run because of usage, config, auth, or input errors. |
| `3` | internal | The command failed because of an unexpected internal error. |

Blocking results use `1` so scripts and agents can distinguish expected safety
stops from invalid invocations and internal failures.

Usage errors use `2` for invalid flags, invalid arguments, unreadable
configuration, missing credentials, or rejected input.

Internal errors use `3` only when Kply cannot map the failure to a documented
user or infrastructure condition.

## Policy Errors

When `--json` is set, policy denials use `error.code: "policy"` and include a
stable `error.policy_violation` object. Consumers can rely on
`error.policy_violation.reason: "policy_denied"` and an
`error.policy_violation.violations` list. The top-level `error.message` remains
present for terminal agents that only need a human-readable explanation.

## Global Flags

`--config <path>` accepts an explicit project configuration path. The canonical
filename is `kply.yaml`. The configured path is shown in `--verbose` output.

`--no-config` disables future configuration discovery and loading. It conflicts
with `--config <path>` so command intent remains explicit. The current CLI does
not automatically discover config files yet, so `--no-config` is a stable guard
for that future behavior.

## Config Precedence

Config commands resolve configuration in this order:

1. `--config <path>`: load and parse the explicit file. Read or parse failures
   are usage/config errors and exit with code `2`.
2. No config path: use the default in-memory config shape.

The `kply-config` crate can discover the nearest `kply.yaml` from a directory
upward, but automatic discovery is intentionally not wired into CLI behavior
yet. When discovery becomes active, `--config <path>` will remain the highest
precedence input and `--no-config` will force the default in-memory shape.

## Doctor Command

`kply doctor` checks local readiness for normal Kply workflows without mutating
the cluster. It validates the resolved configuration, verifies kubeconfig can
be resolved for the current context, and checks that `kubectl` is available on
`PATH`.

Doctor results use exit code `0` when every check passes and exit code `1` when
one or more checks are missing or invalid. Blocking readiness failures are
reported on stdout so agents can parse the result without treating it as an
unexpected internal error.

## Compatibility

Exit codes are part of the CLI contract. Changes to these meanings must update
this document, tests, snapshots, and release notes in the same pull request.

## Demo Commands

`kply demo` requires an explicit subcommand. Demo commands are intentionally
scoped to the current Kubernetes context and the dedicated `kply-demo`
namespace so the local walkthrough stays separate from non-demo workloads.

`kply demo doctor` checks local prerequisites for the manual Kind demo without
mutating a cluster.

The command verifies that the ecommerce demo fixture files exist and that
`kind`, `kubectl`, and one supported container runtime command are available on
`PATH`. Supported runtime commands are `docker`, `podman`, and `nerdctl`.

Doctor results use exit code `0` when every check passes and exit code `1` when
one or more prerequisites are missing. Missing prerequisites are blocking
results, not usage errors.

`kply demo install` installs the baseline ecommerce fixture into the current
Kubernetes context. It applies only the local demo manifests in this order:
namespace, catalog backing service, frontend, and baseline backend.

After applying manifests, the command waits for the `catalog-api`,
`storefront-web`, and `checkout-api` deployments in the `kply-demo` namespace
to become available. Kubectl failures exit with code `1` because they are
blocking demo readiness results.

`kply demo reset` re-applies the same baseline ecommerce fixture and waits for
the same demo deployments. Use it to return the local demo to the known-good
baseline after switching backend variants.

`kply demo teardown` deletes only labeled demo `Deployment` and `Service`
resources in the dedicated `kply-demo` namespace with `--ignore-not-found`,
waits for deletion, and uses a timeout. It does not delete the namespace or the
Kind cluster itself.

## Report Commands

`kply report show <session>` prints the current terminal-readable report
availability for one sandbox session.

`kply report export <session> --format json` emits machine-readable report
availability for scripts and agents. `--format markdown` emits the same
availability information in a pull-request-friendly format.

The current report commands discover session metadata but do not yet load
persisted full reports. Agents must treat `report: not_available` and
`reason: session_report_persistence_not_implemented` as a conservative fallback,
then run `kply check run <session> --json` for current evidence.

For agent-specific handling and handoff wording, see
[Agent Report Workflow](report-agent.md).
