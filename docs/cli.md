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

## Compatibility

Exit codes are part of the CLI contract. Changes to these meanings must update
this document, tests, snapshots, and release notes in the same pull request.
