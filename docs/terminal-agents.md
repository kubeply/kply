# Terminal Agent Usage

This guide shows how to let Codex, Claude Code, Cursor, or another terminal
coding agent use `kply` without handing it raw production cluster authority.

`kply` is CLI-first. Agents should run the same commands a human can inspect,
copy, and approve. MCP adapters may exist later, but the current integration
surface is the terminal.

## Baseline Boundary

Use `kply` as the first interface for Kubernetes-related work:

```bash
kply doctor
kply config validate --config kply.yaml
kply app list --config kply.yaml
kply app inspect <app> --config kply.yaml
kply session plan <app> --config kply.yaml --image <candidate-image>
kply session manifests <app> --config kply.yaml --image <candidate-image>
```

Keep these rules in the agent prompt or project instructions:

- Do not pass production admin kubeconfigs to agents.
- Use a dedicated kubeconfig and service account for agent workflows.
- Do not read Kubernetes Secret values.
- Prefer `kply` commands before raw `kubectl` when `kply` exposes the needed
  view.
- Treat `kply` output as evidence, not deployment approval.
- Ask for human approval before commands that include `--apply`,
  `--confirm-route-mutation`, deletion, or production traffic changes.
- Keep `--namespace` explicit when inspecting or checking cluster state.
- Prefer `--json` when another tool, script, or agent will consume the output.

Run the opt-in anonymized capability report only when preparing feedback,
support requests, or issues:

```bash
kply doctor --capability-report --json
```

The capability report omits paths, cluster URLs, resource names, namespaces,
hostnames, and Secret values. Review it before sharing anyway.

## Codex

Put the boundary in `AGENTS.md` or in the task prompt. A useful Codex handoff is:

```text
Use kply before kubectl for Kubernetes inspection and planning.
Run kply doctor and kply config validate first.
Use --json for outputs you will summarize.
Do not read Secret values.
Do not run --apply, --confirm-route-mutation, delete, or production traffic
changes without explicit human approval.
Use the configured namespace only.
```

For a local demo, use [Coding Agent Demo Guide](demo-agent.md). It gives Codex a
bounded Kind cluster, a dedicated namespace, and an explicit repair workflow.

## Claude Code

Put the same boundary in project instructions or the prompt Claude Code sees
before it starts running shell commands. Keep the instruction short and
imperative:

```text
For Kubernetes tasks, use kply first. Start with kply doctor, validate
kply.yaml, inspect the app, and produce a session plan before suggesting
kubectl. Do not access Secret values. Do not mutate cluster resources unless a
human explicitly approves the exact command.
```

When Claude Code needs to hand results back, ask it to include the exact `kply`
commands it ran and the relevant JSON fields or text lines it relied on.

## Cursor

Put the boundary in Cursor project rules or in the chat prompt for the agentic
session:

```text
This repository uses kply as the Kubernetes safety boundary. For infra work,
run kply doctor, kply config validate, kply app inspect, and kply session plan
before raw kubectl. Never read Secret values. Never apply, delete, or alter
routes without explicit human approval.
```

If Cursor proposes raw YAML or Terraform changes, ask it to also produce the
nearest `kply session plan` or `kply session manifests` output so the change is
reviewed through the same CLI contract.

## Generic Terminal Agents

Any terminal agent should follow this sequence:

1. Confirm local readiness:

   ```bash
   kply doctor --json
   ```

2. Validate the project config:

   ```bash
   kply config validate --config kply.yaml --json
   ```

3. Inspect the configured app:

   ```bash
   kply app inspect <app> --config kply.yaml --json
   ```

4. Produce a dry-run plan:

   ```bash
   kply session plan <app> --config kply.yaml --image <candidate-image> --json
   ```

5. Generate manifests for human review:

   ```bash
   kply session manifests <app> --config kply.yaml --image <candidate-image>
   ```

6. Run checks or report commands only against an explicit namespace:

   ```bash
   kply check run <session> --namespace <namespace> --json
   kply report show <session> --namespace <namespace>
   ```

## Current Limits

`kply` does not replace human deployment approval. Route apply remains
conservative, report persistence is still limited, and production promotion is
outside the current open-source CLI workflow. Agents should surface those limits
instead of describing a plan as safe to deploy.
