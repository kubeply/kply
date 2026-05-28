# Commercial Boundary

Kply is the open-source trust boundary for local agent-infra sessions. Commercial
or enterprise services may build on its evidence later, but the open-source CLI
must remain useful, inspectable, and safe without a hosted dependency.

## Open-Source Trust Features

These features belong in the open-source CLI and local Rust crates:

- deterministic text and JSON output.
- configuration validation.
- read-only Kubernetes inspection.
- sandbox resource planning, apply, and cleanup.
- route planning, route adapters, and explicit no-route fallbacks.
- runtime checks that report evidence.
- local reports and session artifacts.
- release artifacts, checksums, and attestations.
- docs for safe agent use from terminal-based coding agents.

Open-source safety behavior must not require telemetry, billing, hosted auth,
team approval, or an external Kubeply service.

## Commercial Candidates

These are commercial or enterprise candidates, not assumptions for the
open-source trust boundary:

- hosted policy service.
- team approval workflows.
- centralized audit retention.
- hosted reporting dashboards.
- organization and user management.
- billing, licensing, and entitlements.
- fleet management or a multi-cluster control plane.

Commercial features may consume local reports or exported evidence from `kply`,
but local session safety must continue to work when those services are absent.

## Product Rules

1. Public docs must not imply hosted policy, team approval, audit retention, or
   reporting are included in the open-source CLI.
2. New commercial roadmap items must state which behavior remains local and
   open-source.
3. OpenSpec changes for enterprise features must include explicit non-goals for
   the CLI trust boundary.
4. Default `kply` commands must not introduce telemetry, billing, or hosted auth
   in the safety path.
5. Hosted products can improve collaboration and retention, but they must not
   become prerequisites for local agent session safety.
