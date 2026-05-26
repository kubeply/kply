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

## Compatibility

Exit codes are part of the CLI contract. Changes to these meanings must update
this document, tests, snapshots, and release notes in the same pull request.

## Demo Commands

`kply demo doctor` checks local prerequisites for the manual Kind demo without
mutating a cluster.

The command verifies that the ecommerce demo fixture files exist and that
`kind`, `kubectl`, and one supported container runtime command are available on
`PATH`. Supported runtime commands are `docker`, `podman`, and `nerdctl`.

Doctor results use exit code `0` when every check passes and exit code `1` when
one or more prerequisites are missing. Missing prerequisites are blocking
results, not usage errors.
