# First Release Scope

Kply `v0.1.0` is the first public evaluation release. It is intended to make
the agent-safe Kubernetes session workflow inspectable from a released binary,
not to act as a complete production safety layer.

## Release Goal

The first release should let users evaluate whether Kply's CLI contract,
configuration model, Kubernetes inspection, dry-run session planning, generated
manifests, local demo, and release packaging are coherent enough to build on.

The useful outcome is concrete feedback against real commands and reports:
missing fields, confusing output, weak defaults, unsupported workload shapes,
and integration gaps.

## In Scope

- Stable `kply --version` and `kply --version --json` output.
- Deterministic config validation through `kply config validate`.
- Read-only application inspection for Kubernetes workloads and related graph
  metadata.
- Dry-run session planning for a target app, workload, image, route strategy,
  and policy.
- Generated sandbox manifests that can be reviewed before apply.
- Clearly marked experimental live session apply and cleanup behavior where it
  already exists.
- Runtime checks that report current evidence without turning evidence into a
  deployment approval.
- Local Kind demo documentation and demo commands for a bounded ecommerce
  fixture.
- GitHub Action usage for released `kply` binaries.
- Binary release packaging through `cargo-dist`, including archives, shell
  installer, SHA-256 checksums, and GitHub artifact attestations.

## Out Of Scope

- Automatic promotion of application changes.
- Broad production route mutation.
- Replacing deployment platforms such as Argo, Flux, Harness, or GitHub
  Actions.
- Reading Kubernetes Secret values.
- Hosted team policy, audit, or reporting.
- Multi-cluster orchestration.
- Long-running controller behavior inside the cluster.
- A stable `1.0.0` JSON contract.

## Release Bar

Before tagging `v0.1.0`, the repository must show that the scoped release
surface is covered by docs, tests, CI, and release packaging:

- CLI version output is stable.
- Config validation is stable.
- Read-only app inspection is available.
- Dry-run session planning is available.
- Generated manifest output is available.
- Any live apply behavior is marked as experimental.
- Secret value reads remain forbidden.
- CI passes.
- Release packaging passes.
- GitHub Action usage with released `kply` binaries is verified.
- Local demo docs are present.
- Completed roadmap milestones are reflected in the roadmap.
- Known limitations and security assumptions are documented.

## Version Output Requirement

The first release must keep version output stable because installers, GitHub
Actions, and agents use it to verify which binary is running.

The text form is exactly:

```text
kply 0.1.0
```

The JSON form is exactly an object with `name` and `version` fields:

```json
{
  "name": "kply",
  "version": "0.1.0"
}
```

Do not add status, build metadata, target triples, or other fields to
`kply --version --json` before `v0.1.0`. Additive version metadata can be
reconsidered in a later minor release with snapshot changes and release notes.

## Config Validation Requirement

The first release must keep `kply config validate` deterministic and
machine-readable before any Kubernetes access happens.

The default valid config text output is exactly:

```text
kply config validate
Config is valid.
```

The valid JSON form is exactly an object with `status: "valid"` and an empty
`errors` array:

```json
{
  "errors": [],
  "status": "valid"
}
```

Invalid config JSON must keep the same top-level shape with
`status: "invalid"` and a deterministic `errors` array of field-scoped strings.
Do not add warning, hint, path, or remediation fields before `v0.1.0`.

## Read-Only App Inspection Requirement

The first release must let users inspect configured app targets without
mutating Kubernetes resources. `kply app inspect <app>` loads and validates
configuration, then renders only the selected app contract.

The text form must include these stable lines in this order:

```text
kply app inspect <app>
name: <app>
namespace: <namespace>
workload: <workload>
service: <service>
route_strategy: <strategy>
default_image: <image-or-none>
```

The JSON form must remain an object with these fields:

- `name`
- `namespace`
- `workload`
- `workload_kind`
- `service`
- `route_strategy`
- `default_image`

Do not make `kply app inspect` create, update, delete, or patch Kubernetes
resources in `v0.1.0`. Future live discovery for this command must stay
read-only and preserve deterministic JSON output.

## Dry-Run Session Planning Requirement

The first release must let users plan a sandbox session without touching the
cluster. `kply session plan <app>` loads and validates configuration, applies
CLI overrides, evaluates configured policy, and renders the planned session
contract. It must not create, update, delete, or patch Kubernetes resources.

The text form must include these stable sections in this order:

```text
kply session plan <app>
id: <session-id>
name: <session-name>
workload: <namespace>/<kind>/<name>
image: <image>
planned_resources: <count>
planned_labels: <count>
planned_annotations: <count>
planned_checks: <count>
planned_cleanup_steps: <count>
required_permissions: <count>
unsupported_feature_warnings: <count>
risk_notes: <count>
route_selector: <selector-or-none>
policy_operations: <count>
status: planned
ttl: <duration>
```

The `ttl` line appears only when a session lifetime is configured or passed on
the command line. Resource, label, annotation, check, cleanup, permission,
warning, and risk detail lines must remain deterministic within their sections.

The JSON form must remain an object with these fields:

- `id`
- `name`
- `workload`
- `image`
- `ttl`
- `planned_resources`
- `planned_labels`
- `planned_annotations`
- `planned_checks`
- `planned_cleanup_steps`
- `required_permissions`
- `unsupported_feature_warnings`
- `risk_notes`
- `route_selector`
- `policy`
- `status`

`status` must be `planned` for successful dry-run output in `v0.1.0`.
Mutation belongs behind explicit apply-oriented commands, not behind
`kply session plan`.

## Generated Manifest Output Requirement

The first release must let users review generated sandbox manifests before
anything is applied to a cluster. `kply session manifests <app>` builds from
the same validated dry-run plan as `kply session plan <app>` and renders
deterministic Kubernetes objects for review.

The text form must include these stable lines in this order:

```text
kply session manifests <app>
session_id: <session-id>
manifests: <count>
  manifest: <kind> <namespace>/<name>
```

The JSON form must remain an object with these fields:

- `app`
- `session_id`
- `status`
- `manifests`

Each manifest entry must include `kind`, `namespace`, `name`, and `object`.
The nested `object` is the Kubernetes object that would be applied by an
apply-oriented command. `status` must be `generated` for successful output in
`v0.1.0`.

The `--yaml` form must render the generated Kubernetes objects as a
multi-document YAML stream without the JSON wrapper. It is intended for human
review, diffing, and manual tooling experiments, not as a production promotion
signal.

## Experimental Apply Requirement

The first release may expose apply-oriented commands, but they must not be
presented as a complete production safety layer.

`kply session create <app>` without `--apply` remains a dry-run command.
Successful text and JSON output must keep `status: planned`,
`mutation: not_applied`, and `apply: false`.

`kply session create <app> --apply` is experimental in `v0.1.0`. Successful
live mutation output and live mutation errors must include
`apply_stage: experimental` in text output or `apply_stage: "experimental"` in
JSON output. This command can create sandbox Deployment and Service resources,
record session state in annotations, and leave pending route resources for
later cleanup or manual review.

`kply route apply <session>` remains a guarded placeholder in `v0.1.0`. Even
with `--confirm-route-mutation`, it must report `status: not_implemented`,
`mutation: not_applied`, and `apply: false` until a real route mutation adapter
is implemented and tested.

## No Secret Value Reads Requirement

The first release must never read Kubernetes Secret values. Kply may model
Secret names and references as metadata for graph output, risk notes, policy
checks, and RBAC planning, but it must not fetch or print Secret contents.

`cargo xtask check-no-secret-content-reads` is part of the release gate. It
must scan product crate source trees for direct Kubernetes `Secret` API usage,
typed Secret content field access, and `.data` or `.string_data` reads on
Secret-like values.

If future work needs Secret contents, it requires a separate design, explicit
policy, tests, documentation, and a release note before any implementation
lands. Do not add a bypass to the guard for `v0.1.0`.

## CI Passing Requirement

The first release must be cut from a commit where the GitHub Actions `ci`
workflow passes on the release branch before the tag is created.

The `ci` workflow is the required repository quality gate for `v0.1.0`. It
must run on pull requests, merge queues, and pushes to `main`; keep
`contents: read`; lint workflows with `actionlint`; install the pinned Rust
toolchain; run formatting, check, clippy, all workspace tests, fixture helper
tests, cargo-deny, and every release-gate `cargo xtask` check.

`cargo xtask check-ci-workflow` is part of the release gate. It pins the
required CI triggers, permissions, actions, and commands so CI coverage cannot
silently drift away from the first-release bar.

## Release Packaging Passing Requirement

The first release must be cut from a commit where the GitHub Actions `Release`
workflow passes its pull-request `plan` job before the tag is created. A passing
plan proves the checked-in `cargo-dist` workflow can resolve the release shape
without building or publishing artifacts from a pull request.

Local validation must also run `dist plan --output-format=json --no-local-paths`
and verify that the generated plan releases only `kply-cli`. The plan must not
include `xtask` or other workspace helper crates as release artifacts.

`cargo xtask check-release-planning` remains part of the release gate. It pins
the `cargo-dist` version, released package set, target matrix, shell installer,
SHA-256 checksums, GitHub artifact attestations, pull-request planning path,
semver tag release path, and the absence of direct `dist publish` commands.

The semver tag workflow must pass before announcing `v0.1.0`. Tag runs are the
only path that may build archives, generate the shell installer and checksums,
upload artifacts, attest them, and create the GitHub Release.

## Local Demo Docs Requirement

The first release must include local demo documentation that lets a developer
evaluate Kply without cloud credentials or production cluster access.

The required demo docs are:

- [Local Kind Demo](demo-kind.md), covering prerequisites, Kind cluster setup,
  `kply demo doctor`, `kply demo install`, `kply demo reset`, `kply demo
  teardown`, baseline fixture installation, backend variant switching,
  inspection, route planning, route no-op apply, and cleanup.
- [Coding Agent Demo Guide](demo-agent.md), covering the safe prompt,
  namespace boundary, allowed commands, forbidden Secret value reads, backend
  variant repair path, expected agent output, reset, and current limitations.
- [Demo fixture README](../fixtures/demo/ecommerce-basic/README.md), linking
  the fixture shape back to the human and agent demo guides.

`cargo xtask check-demo-docs` is part of the release gate. It pins the README
local demo link, the Kind and agent guides, the walkthrough script reference,
the demo fixture path, the dedicated `kply-demo` namespace, and the current
route apply limitation so the local demo cannot silently disappear from
`v0.1.0`.
