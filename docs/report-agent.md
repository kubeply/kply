# Agent Report Workflow

Kply reports are the agent-facing handoff after a sandbox session. Agents should
use them to summarize what happened, what evidence was observed, and what a
human should do next.

## Commands

Use `kply report show <session>` for terminal-readable status:

```sh
kply report show checkout-plan --namespace shop
```

Use JSON export when the next step is another tool, script, or agent context:

```sh
kply report export checkout-plan --namespace shop --format json
```

Use Markdown export when the next step is a pull request comment, issue comment,
or handoff note:

```sh
kply report export checkout-plan --namespace shop --format markdown
```

If `--namespace` is omitted, Kply reads the current Kubernetes context to find
the default namespace. Agents should pass `--namespace` when the session
namespace is known so failures are easier to diagnose.

## Agent Handling

Agents should treat report output as evidence, not as permission to deploy.

Required handling:

1. Capture the report output exactly before summarizing it.
2. Preserve `session_id`, `namespace`, `session_status`, `report`, and `reason`
   fields in the handoff.
3. If `report` is `not_available`, explain that persistent report loading is
   not implemented yet and fall back to `kply check run <session> --json`.
4. If a future report includes checks, quote the failing or warning checks by
   name and include their evidence.
5. If cleanup is required or failed, make cleanup the next action before any
   promotion discussion.

Agents must not turn Kply evidence into deployment approval, production
readiness, or promotion approval. Until promotion integrations exist, the
strongest allowed wording is:

> Kply did not report a blocking condition in the available evidence. A human
> still needs to review and promote outside Kply.

## Current Report State

The current CLI report surface is intentionally conservative. `report show` and
`report export` can discover session metadata, but persisted full reports are
not loaded yet. Successful report commands may therefore return:

```text
report: not_available
reason: session_report_persistence_not_implemented
```

When that happens, agents should run the verification command for current
session evidence:

```sh
kply check run checkout-plan --namespace shop --json
```

Then include both outputs in the handoff:

- the report availability output.
- the check summary and evidence.
- the next action, phrased conservatively.

## Handoff Template

```md
## Kply Session Report

- Session: `<session_id>`
- Namespace: `<namespace>`
- Session status: `<session_status>`
- Report status: `<report>`
- Reason: `<reason>`

## Evidence

- Checks: `<check summary or unavailable>`
- Cleanup: `<cleanup status or unavailable>`
- Limitations: `<known limitations or unavailable>`

## Recommended Next Action

<cleanup required | fix and retry | inspect manually | human promotion outside Kply>
```
