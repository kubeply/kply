# Implementation Roadmap

This roadmap tracks the work needed to turn `kply` from an open-source Rust
scaffold into a useful CLI for safe Kubernetes sessions for AI coding agents.

The order is intentional:

1. Keep the project trustworthy before adding behavior.
2. Define the session contract before touching Kubernetes.
3. Build local and dry-run flows before cluster mutation.
4. Add Kubernetes execution behind narrow adapters.
5. Add routing, verification, cleanup, and reporting only when the lower layers
   are stable.
6. Treat agent usability, auditability, and failure behavior as product
   features, not later polish.

Every implementation task should be small enough to land independently with
focused tests and snapshot coverage.

## Current Status

Completed work:

- Milestones 1 through 18 are implemented and validated in the repository.
- Milestone 19 items 1 through 15 are implemented:
  - first-release scope.
  - stable version output.
  - stable config validation.
  - read-only app inspection.
  - dry-run session planning.
  - generated manifest output.
  - experimental live apply marking.
  - no Secret value reads.
  - CI passing.
  - release packaging passing.
  - local demo docs.
  - roadmap status for completed milestones.
  - known limitations.
  - security assumptions.
  - `v0.1.0` tag.
- Milestone 20 item 1 is implemented:
  - feedback issue templates.
- Milestone 20 item 2 is implemented:
  - security policy.
- Milestone 20 item 3 is implemented:
  - roadmap issue template.
- Milestone 20 item 4 is implemented:
  - top-level `kply doctor` readiness checks for config, kubeconfig, and
    `kubectl`.
- Milestone 20 item 5 is implemented:
  - opt-in anonymized `kply doctor --capability-report` output.
- Milestone 20 item 6 is implemented:
  - terminal agent usage docs for Codex, Claude Code, Cursor, and generic
    agents.
- Milestone 20 item 7 is implemented:
  - local demo terminal cast and replay guide.
- Milestone 20 item 8 is implemented:
  - paired `infra-bench` Kubernetes tasks comparing raw `kubectl` rollouts with
    `kply`-bounded sandbox workflows.
- Milestone 20 item 10 is implemented:
  - missing route adapter requests are classified in the routing issue template
    and triaged through `docs/feedback-triage.md`.
- Milestone 20 item 11 is implemented:
  - repeated policy needs are classified in the session planning issue template
    and triaged through `docs/feedback-triage.md`.
- Milestone 20 item 12 is implemented:
  - repeated app graph failures are classified in the Kubernetes discovery issue
    template and triaged through `docs/feedback-triage.md`.
- Milestone 20 item 13 is implemented:
  - repeated check failures are classified in the agent workflow issue template
    and triaged through `docs/feedback-triage.md`.
- Milestone 20 item 14 is implemented:
  - repeated feedback has an OpenSpec conversion gate in the roadmap issue
    template and `docs/feedback-triage.md`.
- Milestone 9 item 12 is implemented:
  - scripted local demo walkthrough for inspect, plan, sandbox create, checks,
    and cleanup.
- Milestone 9 item 13 is implemented:
  - demo manifests and app config stay inside the dedicated `kply-demo`
    namespace, enforced by `crates/kply-cli/tests/demo_manifests.rs`.
- Milestone 9 item 14 is implemented:
  - `kply demo teardown` deletes only labeled demo Deployments and Services,
    enforced by CLI snapshots for the generated `kubectl` command.
- Milestone 9 item 15 is implemented:
  - the local demo has a replayable terminal cast and guide at
    `docs/demo-terminal-cast.md`.

Remaining adoption and feedback work:

- Milestone 20 item 9: add examples from real user feedback only when
  permission exists.
- Milestone 20 item 15: keep commercial/enterprise features separate from
  open-source trust features.

## Locked Decisions

These decisions are fixed unless implementation proves they are wrong.

1. The public tool is `kply`; Kubeply remains the company and organization
   brand.
2. The first interface is CLI-first. MCP may be added later as an adapter over
   the same core, not as the primary implementation surface.
3. The first product primitive is a session: a bounded, auditable workspace for
   an agent to inspect or test a Kubernetes-related change.
4. The open-source repository must remain inspectable and safe to run near
   Kubernetes. Avoid hidden side effects and implicit cluster mutation.
5. Default commands must be dry-run or read-only until explicit apply semantics
   are implemented and tested.
6. Production traffic must not depend on a `kply` proxy in the first usable
   release.
7. Use existing traffic layers first: Gateway API, ingress controllers, service
   mesh, or fallback preview routing. Do not build a full service mesh.
8. Keep crate boundaries aligned with responsibility:
   - `kply-core`: session model and state transitions.
   - `kply-config`: configuration parsing and validation.
   - `kply-k8s`: Kubernetes discovery and mutation adapters.
   - `kply-routing`: route planning and routing adapters.
   - `kply-checks`: verification checks and check reports.
   - `kply-cli`: command parsing and human/agent output.
   - `kply-test`: shared fixtures, snapshots, and CLI helpers.
9. Use `insta` snapshots for CLI output, JSON reports, plans, check output,
   and generated Kubernetes manifests.
10. Keep JSON output deterministic and stable enough for agents to consume.
    Breaking JSON changes are allowed before `1.0.0`, but must be visible in
    snapshot diffs.
11. Lock exit codes early:
    - `0`: command completed successfully.
    - `1`: session or check produced a blocking result.
    - `2`: usage, config, auth, or input error.
    - `3`: unexpected internal error.
12. Do not read Kubernetes `Secret` values by default. Secret references may be
    inspected as metadata, but secret contents require explicit future design.
13. Do not run destructive cleanup outside resources labeled and owned by a
    `kply` session.
14. Session resources must carry stable labels and owner metadata so cleanup is
    deterministic.
15. Support local Kind demos before asking users to install anything in a real
    cluster.
16. Keep `cargo deny`, clippy, fmt, and tests blocking from the beginning.
17. Release binaries should be distributed through `cargo-dist`; users should
    not need a Rust toolchain to install `kply`.
18. The first GitHub Action must run a released binary, not compile Rust in
    user workflows.
19. Avoid broad product claims in docs until the matching workflow exists.
20. Roadmap milestones may be implemented one by one; do not skip ahead to
    cluster mutation before session contracts and dry-run reports are stable.

## Milestone 1: Repository Hygiene And Placeholder Baseline

Goal: preserve a clean, trustworthy scaffold before implementation starts.

1. Keep every crate source file documented with a one-line module docstring.
   Enforce this with `cargo xtask check-module-docs`.
2. Keep placeholder marker types in non-CLI crates until their roadmap work
   starts. Enforce product crate placeholders with
   `cargo xtask check-placeholders`.
3. Keep `README.md`, `docs/architecture.md`, and `docs/product.md` explicit
   that behavior is placeholder-only. Enforce this with
   `cargo xtask check-placeholder-docs`.
4. Keep `AGENTS.md`, `CONTRIBUTING.md`, and `crates/README.md` aligned with the
   workspace crate inventory. Enforce this with
   `cargo xtask check-crate-inventory-docs`.
5. Keep the Apache-2.0 license and notice files. Enforce this with
   `cargo xtask check-license-files`.
6. Keep the local Rust toolchain pinned through `rust-toolchain.toml`. Enforce
   this with `cargo xtask check-toolchain-pin`.
7. Keep `cargo deny` configuration strict enough for an open-source CLI.
   Enforce this with `cargo xtask check-deny-config`.
8. Keep release planning config present but avoid publishing a binary until the
   first useful workflow exists. Enforce this with
   `cargo xtask check-release-planning`.
9. Keep a top-level roadmap link from `README.md`. Enforce this with
   `cargo xtask check-readme-roadmap-link`.
10. Keep a short “not implemented yet” note in each public-facing doc that
    mentions future sessions. Enforce this with
    `cargo xtask check-future-session-docs`.

Acceptance criteria:

- The repository builds with placeholders only.
- `cargo fmt`, `cargo check`, `cargo clippy`, `cargo test`, and `cargo deny`
  pass.
- Public docs do not imply Kubernetes behavior exists before it does.

## Milestone 2: Test Harness And Fixtures

Goal: make future CLI, config, manifest, and Kubernetes behavior easy to verify.

1. Create fixture directories:
   - `fixtures/cli/`
   - `fixtures/config/`
   - `fixtures/manifests/`
   - `fixtures/k8s-responses/`
   - `fixtures/reports/`
   - `fixtures/demo/`
   Enforce this with `cargo xtask check-fixture-directories`.
2. Define fixture naming:
   - CLI fixtures: `cli/<behavior-name>/`
   - config fixtures: `config/<case-name>/kply.yaml`
   - manifest fixtures: `manifests/<workload-shape>/`
   - report fixtures: `reports/<workflow-name>/`
   Enforce this with `cargo xtask check-fixture-naming-docs`.
3. Extend `kply-test` with helpers for:
   - resolving fixture paths.
   - invoking the `kply` binary.
   - normalizing JSON output.
   - normalizing timestamps and generated ids.
   - normalizing Kubernetes object names.
   - normalizing absolute paths.
4. Add snapshot helpers for:
   - CLI text output.
   - CLI JSON output.
   - generated Kubernetes manifests.
   - check reports.
   - route plans.
5. Add fixture docs explaining when to use snapshots versus direct assertions.
   Enforce this with `cargo xtask check-fixture-testing-docs`.
6. Add test helpers for temporary directories.
7. Add test helpers for fake kubeconfig paths.
8. Add test helpers for checking exit codes.
9. Add test helpers for comparing stable JSON object ordering.
10. Add CI coverage for fixture-backed tests.

Acceptance criteria:

- New behavior can be tested without ad hoc path or JSON normalization.
- Snapshot output is stable across machines.
- `kply-test` remains dev/test-only.

## Milestone 3: CLI Contract

Goal: define the first stable user and agent-facing command shape without
implementing Kubernetes behavior.

1. Document CLI exit codes.
2. Add `kply --version`.
3. Add `kply --version --json`.
4. Add `kply help` snapshots.
5. Add command groups without behavior:
   - `kply session`
   - `kply app`
   - `kply config`
   - `kply cluster`
   - `kply report`
6. Add `--json` as a global flag where output is supported.
7. Add `--quiet` for scripts and agents.
8. Add `--verbose` for local debugging.
9. Add `--no-color` for deterministic output.
10. Add consistent error rendering for usage errors.
11. Add JSON error output for agent consumption.
12. Add CLI tests for every top-level command and flag.
13. Define command naming rules in `docs/architecture.md`.
14. Add shell completion generation as a later placeholder command if useful.
15. Avoid hidden aliases until the primary command surface is stable.

Acceptance criteria:

- Agents and humans can discover the command surface.
- Every global flag has snapshot coverage.
- Exit code behavior is documented and tested.

## Milestone 4: Session Domain Model

Goal: define the core session contract before integrating with Kubernetes.

1. Add `SessionId`.
2. Add `SessionName`.
3. Add `SessionStatus` with initial states:
   - `Planned`
   - `Preparing`
   - `Active`
   - `Verifying`
   - `Blocked`
   - `Ready`
   - `CleanedUp`
   - `Failed`
4. Add `WorkloadRef` with namespace, kind, and name.
5. Add `ImageRef` for proposed sandbox images.
6. Add `RouteSelector` for future test traffic routing.
7. Add `SessionPolicy` for allowed operations.
8. Add `SessionPlan` for dry-run output.
9. Add `SessionReport` for final report output.
10. Add `SessionEvent` for audit history.
11. Add serde support for JSON output.
12. Add state transition validation.
13. Add tests for valid transitions.
14. Add tests for invalid transitions.
15. Add snapshot tests for session plans and reports.
16. Document which fields are stable before `1.0.0`.

Acceptance criteria:

- Session state can be modeled without Kubernetes dependencies.
- Invalid lifecycle transitions are rejected in `kply-core`.
- JSON snapshots define the first agent-readable contract.

## Milestone 5: Configuration Model

Goal: define how projects describe apps and safe session defaults.

1. Choose `kply.yaml` as the canonical config filename.
2. Add explicit `--config <path>`.
3. Add config discovery from the current directory upward.
4. Add `--no-config`.
5. Define top-level config fields:
   - `version`
   - `apps`
   - `routing`
   - `checks`
   - `policies`
6. Define app config fields:
   - `name`
   - `namespace`
   - `workload`
   - `service`
   - `default_image`
   - `route_strategy`
7. Add schema versioning.
8. Add clear validation errors.
9. Add JSON output for resolved config.
10. Add `kply config show`.
11. Add `kply config validate`.
12. Add fixture tests for valid configs.
13. Add fixture tests for invalid configs.
14. Add snapshot tests for error messages.
15. Document config precedence.

Acceptance criteria:

- Config parsing is deterministic and well-tested.
- Invalid configs fail before any Kubernetes access.
- Agents can inspect resolved config through JSON.

## Milestone 6: Read-Only Kubernetes Discovery

Goal: inspect cluster state safely before supporting any mutation.

1. Add Kubernetes client dependency in `kply-k8s`.
2. Load kubeconfig using standard Kubernetes conventions.
3. Add `kply cluster info`.
4. Add `kply app list`.
5. Add `kply app inspect <app>`.
6. Discover Deployments.
7. Discover Services.
8. Discover Pods owned by a workload.
9. Discover Ingress objects where available.
10. Discover Gateway API resources where available.
11. Discover basic rollout status.
12. Discover readiness and liveness probe metadata.
13. Discover resource requests and limits.
14. Do not read secret contents.
15. Add fake Kubernetes response fixtures.
16. Add integration tests that do not require a live cluster.
17. Add optional live-cluster tests gated by environment variables.
18. Add clear errors for missing kubeconfig, forbidden access, and missing
    workload.

Acceptance criteria:

- Users can inspect an app without mutating the cluster.
- Read-only commands work with least-privilege permissions.
- Secret values are never printed.

## Milestone 7: App Graph Model

Goal: turn raw Kubernetes objects into a useful app-level graph.

1. Add `AppGraph` to `kply-core`.
2. Model workload-to-pod ownership.
3. Model workload-to-service selection.
4. Model service-to-route references.
5. Model probe facts.
6. Model container image facts.
7. Model resource facts.
8. Model config and secret references as metadata.
9. Add confidence levels for inferred relationships.
10. Add warnings for ambiguous service selectors.
11. Add warnings for missing routes.
12. Add warnings for missing probes.
13. Add `kply app graph <app> --json`.
14. Add human output for graph summaries.
15. Add snapshot tests for common Kubernetes shapes.

Acceptance criteria:

- `kply` can explain what it believes an app consists of.
- Ambiguity is surfaced rather than hidden.
- The graph model is independent from raw Kubernetes client types.

## Milestone 8: Dry-Run Session Planning

Goal: create useful session plans without creating resources.

1. Add `kply session plan <app>`.
2. Accept `--image <image>`.
3. Accept `--namespace <namespace>` override.
4. Accept `--ttl <duration>`.
5. Accept `--route-strategy <strategy>`.
6. Produce a `SessionPlan`.
7. Include planned Kubernetes resources.
8. Include planned labels and annotations.
9. Include planned checks.
10. Include planned cleanup steps.
11. Include required permissions.
12. Include unsupported feature warnings.
13. Include risk notes for database or secret references.
14. Add text output snapshots.
15. Add JSON output snapshots.
16. Add error tests for missing image, missing app, and invalid TTL.

Acceptance criteria:

- Users can understand what `kply` would do before cluster mutation.
- Session plans are deterministic and reviewable.
- Plans identify permissions and risks explicitly.

## Milestone 9: Local Kind Demo

Goal: let developers and investors try the concept locally without touching a
real cluster.

1. Add a local demo fixture with frontend, backend, and simple backing service.
2. Add a broken backend variant.
3. Add a fixed backend variant.
4. Add Kind cluster setup docs.
5. Add `kply demo doctor`.
6. Add `kply demo install`.
7. Add `kply demo reset`.
8. Add `kply demo teardown`.
9. Keep demo commands explicit and isolated.
10. Add smoke tests for generated demo manifests.
11. Add docs for running the demo with a coding agent.
12. [x] Add a scripted walkthrough:
    - inspect app.
    - plan session.
    - create sandbox.
    - run checks.
    - cleanup.
13. [x] Ensure demo resources use a dedicated namespace.
14. [x] Ensure demo cleanup removes only labeled demo resources.
15. [x] Add screenshots or terminal recordings later, not before the flow works.

Acceptance criteria:

- A developer can run a full local demo without cloud credentials.
- Demo resources are isolated and cleanly removable.
- The demo proves the product concept without production access.

## Milestone 10: Manifest Generation

Goal: generate Kubernetes resources for a sandbox session without applying them.

1. Generate sandbox Deployment manifests.
2. Generate sandbox Service manifests.
3. Generate labels for session ownership.
4. Generate annotations for audit metadata.
5. Preserve selected app labels where safe.
6. Avoid copying unsafe production-only annotations by default.
7. Generate TTL metadata.
8. Generate cleanup selectors.
9. Generate route placeholder manifests where supported.
10. Add `kply session manifests <app>`.
11. Add YAML output.
12. Add JSON output if useful for agents.
13. Add snapshot tests for generated manifests.
14. Add tests for deterministic object names.
15. Add tests for collision handling.

Acceptance criteria:

- Generated resources are reviewable before apply.
- Every generated resource is labeled for ownership and cleanup.
- Manifests are deterministic across runs except for explicit ids.

## Milestone 11: Session Apply And Cleanup

Goal: create and remove sandbox resources safely.

1. Add explicit `kply session create <app>`.
2. Require `--apply` or equivalent explicit mutation flag if command defaults
   remain dry-run.
3. Create sandbox Deployment.
4. Create sandbox Service.
5. Wait for basic readiness.
6. Record session state locally or in cluster metadata.
7. Add `kply session list`.
8. Add `kply session status <session>`.
9. Add `kply session cleanup <session>`.
10. Cleanup only resources labeled with the session id.
11. Add cleanup dry-run mode.
12. Handle partial creation failures.
13. Handle interrupted sessions.
14. Add tests with fake Kubernetes clients.
15. Add gated live-cluster tests in Kind.

Acceptance criteria:

- `kply` can create and clean up sandbox resources.
- Partial failures produce clear recovery instructions.
- Cleanup cannot target unrelated resources.

## Milestone 12: Verification Checks

Goal: run useful checks against a sandbox session.

1. Define check result types:
   - `Passed`
   - `Failed`
   - `Warning`
   - `Skipped`
2. Add pod readiness check.
3. Add rollout availability check.
4. Add service endpoint check.
5. Add HTTP smoke check.
6. Add log fatal-pattern check.
7. Add restart count check.
8. Add resource request sanity check.
9. Add probe existence check.
10. Add timeout handling.
11. Add `kply check run <session>`.
12. Add text report output.
13. Add JSON report output.
14. Add snapshot tests for each check result.
15. Add checks fixture suite.

Acceptance criteria:

- Checks produce stable machine-readable reports.
- Blocking versus warning results are explicit.
- Failed checks do not hide raw evidence needed for debugging.

## Milestone 13: Gateway API Routing Adapter

Goal: support the first Kubernetes-native route isolation path.

1. Add Gateway API resource detection.
2. Model supported route capabilities.
3. Generate temporary `HTTPRoute` manifests.
4. Support header-based routing where available.
5. Support host-based preview routing where available.
6. Add route ownership labels.
7. Add route cleanup behavior.
8. Add `kply route plan <session>`.
9. Add `kply route apply <session>`.
10. Add `kply route cleanup <session>`.
11. Add fixtures for supported Gateway API shapes.
12. Add fixtures for unsupported shapes.
13. Add docs for required Gateway API permissions.
14. Add Kind-compatible demo if feasible.
15. Add clear fallback guidance when Gateway API is unavailable.

Acceptance criteria:

- Gateway API route changes are generated and applied safely.
- Unsupported clusters fail with actionable guidance.
- Normal production routes are not replaced by default.

## Milestone 14: Ingress And Fallback Routing

Goal: support more clusters without requiring Gateway API from day one.

1. Add routing capability detection.
2. Add NGINX Ingress planning if feasible.
3. Add Traefik planning if feasible.
4. Add fallback preview service mode.
5. Add explicit unsupported-route output.
6. Document what each route strategy can and cannot test.
7. Add `--route-strategy auto`.
8. Add `--route-strategy none`.
9. Add `--route-strategy preview-service`.
10. Add strategy-specific warnings.
11. Add fixtures for common ingress annotations.
12. Add tests for route strategy selection.
13. Avoid silently patching production ingress resources.
14. Require explicit confirmation for route mutation.
15. Add cleanup tests for route objects.

Acceptance criteria:

- Users can understand which route strategy `kply` selected and why.
- Fallback mode works without advanced cluster routing.
- Route limitations are explicit in reports.

## Milestone 15: Agent-Friendly Reports

Goal: make `kply` useful to coding agents and humans after a session.

1. Define `SessionReport`.
2. Include session metadata.
3. Include app graph summary.
4. Include created resources.
5. Include route strategy.
6. Include checks and evidence.
7. Include cleanup status.
8. Include limitations.
9. Include recommended next action:
   - promote outside `kply`.
   - fix and retry.
   - inspect manually.
   - cleanup required.
10. Add `kply report show <session>`.
11. Add `kply report export <session> --format json`.
12. Add Markdown report output.
13. Add snapshot tests for text, Markdown, and JSON.
14. Add docs showing how an agent should use the report.
15. Avoid exaggerated “safe to deploy” language before promotion integrations
    exist.

Acceptance criteria:

- Reports are useful as PR comments, terminal output, and agent context.
- Reports include evidence, not only conclusions.
- Cleanup status is always visible.

## Milestone 16: Policy And Permissions

Goal: make safe boundaries explicit and enforceable.

1. Define policy config structure.
2. Add allowed namespaces.
3. Add allowed workload kinds.
4. Add allowed route strategies.
5. Add max session TTL.
6. Add image registry allowlist.
7. Add mutation mode policy:
   - read-only.
   - sandbox-only.
   - route-mutation.
8. Add secret handling policy.
9. Add database-risk warnings as metadata, not deep inspection.
10. Add `kply policy check`.
11. Add policy evaluation to session planning.
12. Add policy evaluation to session apply.
13. Add JSON policy violation output.
14. Add tests for allowed and denied actions.
15. Document least-privilege RBAC examples.

Acceptance criteria:

- Policies block unsafe actions before Kubernetes mutation.
- Policy decisions are explainable.
- Least-privilege examples exist for read-only and sandbox modes.

## Milestone 17: GitHub Action

Goal: make `kply` easy to run in pull request workflows.

1. Add `action.yml`.
2. The action downloads released `kply` binaries.
3. Support `config-path`.
4. Support `app`.
5. Support `image`.
6. Support `mode: plan`.
7. Support `mode: check` later.
8. Upload JSON report as artifact.
9. Optionally write Markdown summary.
10. Do not require a live cluster for plan-only mode.
11. Add action self-test workflow.
12. Add docs for GitHub Actions usage.
13. Add examples for PR comments without extra tokens.
14. Pin action dependencies.
15. Keep action behavior aligned with CLI release versions.

Acceptance criteria:

- Users can run `kply` in CI without installing Rust.
- Plan-only mode works safely in pull requests.
- Action docs are clear about required cluster credentials for live checks.

## Milestone 18: Release Packaging

Goal: ship installable binaries once the first useful plan workflow exists.

1. Finalize `cargo-dist` workflow.
2. Build Linux x86_64.
3. Build Linux aarch64.
4. Build macOS x86_64.
5. Build macOS aarch64.
6. Add portable Linux targets where cleanly supported.
7. Generate shell installer.
8. Generate checksums.
9. Enable GitHub artifact attestations.
10. Add release notes template.
11. Add `docs/release.md` first-release checklist.
12. Add install docs to `README.md`.
13. Add upgrade docs.
14. Add rollback install docs.
15. Smoke-test released archive locally before announcing.

Acceptance criteria:

- Users can install `kply` without a Rust toolchain.
- Release artifacts are checksummed and attested.
- Release docs match the actual install path.

## Milestone 19: First Usable Release

Goal: publish a narrowly useful release without pretending to be a full
production safety layer.

1. Define release scope in `docs/first-release.md`.
2. Require stable CLI version output.
3. Require stable config validation.
4. Require read-only app inspection.
5. Require dry-run session planning.
6. Require generated manifest output.
7. Require placeholder or experimental live apply clearly marked if present.
8. Require no secret value reads.
9. Require CI passing.
10. Require release packaging passing.
11. Require local demo docs.
12. Require roadmap updated with completed milestones.
13. Add known limitations.
14. Add security assumptions.
15. Tag `v0.1.0`.

Acceptance criteria:

- The first release is useful for evaluation and roadmap validation.
- The release does not overclaim production safety.
- Users can file useful issues against concrete behavior.

## Milestone 20: Adoption And Feedback Loop

Goal: turn the open-source CLI into a distribution channel for the larger
Kubeply product direction.

1. Add issue templates for:
   - routing environment.
   - Kubernetes discovery bug.
   - session planning gap.
   - agent workflow request.
2. Add a security policy.
3. Add a roadmap issue template.
4. [x] Add `kply doctor` once enough checks exist.
5. [x] Add anonymized environment capability report, opt-in only.
6. [x] Add docs for using `kply` with Codex, Claude Code, Cursor, and other terminal
   agents.
7. [x] Add a local demo video or terminal cast.
8. [x] Add benchmark tasks in `infra-bench` comparing raw `kubectl` agent use
   with `kply`-bounded workflows.
9. Add examples from real user feedback only when permission exists.
10. [x] Track repeated missing route adapters.
11. [x] Track repeated policy needs.
12. [x] Track repeated app graph failures.
13. [x] Track repeated check failures.
14. [x] Convert repeated feedback into new OpenSpec changes.
15. Keep commercial/enterprise features separate from open-source trust
    features.

Acceptance criteria:

- The repo produces useful feedback about real agent-infra workflows.
- Roadmap updates are driven by user evidence, demos, or benchmark failures.
- Open-source `kply` remains the trusted CLI boundary for future Kubeply work.
