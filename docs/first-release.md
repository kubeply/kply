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
