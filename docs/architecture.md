# Architecture

Kply starts as a CLI-first scaffold for a future safety layer around AI agents
working near Kubernetes.

## Actors

```text
AI coding agent / human
        |
        | runs kply commands
        v
kply CLI
        |
        | creates scoped sessions and checks
        v
Kubernetes API / routing layer
        |
        v
future sandbox workloads, temporary routes, reports, cleanup
```

The first interface is the CLI because agents already operate terminals well.
MCP can be added later as another adapter over the same core.

## CLI Command Naming

Command names are part of the agent-facing contract. Agents are the primary
automation audience, while humans must still be able to inspect and approve the
same commands. Names must stay boring, predictable, and easy to discover from
`kply --help`.

Rules:

- Use lowercase kebab-case for multi-word commands and flags.
- Prefer nouns for command groups, such as `session`, `app`, `config`,
  `cluster`, and `report`.
- Prefer explicit verbs for subcommands that perform actions, such as `show`,
  `validate`, `plan`, `start`, `verify`, and `cleanup`.
- Keep mutation verbs explicit for destructive commands, such as
  `session delete` or `resource destroy`.
- Reserve plan or preview commands for non-mutating output by default. If a
  plan command can later perform the planned change, the canonical confirmation
  flag is `--apply`, for example `session plan --apply`.
- Avoid hidden aliases until the primary command surface is stable.
- Do not reuse a command name for different resource concepts. Reusing verbs is
  fine when the resource group stays clear: `session show` and `app show` are
  acceptable, but two unrelated resource concepts should not both be named
  `session`.
- Keep JSON field names aligned with command names when the command produces
  machine-readable output. Convert kebab-case command terms to snake_case JSON
  keys, for example `route-header` becomes `route_header`.

## Future Session

A session is the expected core primitive. It will represent a bounded attempt
to test a change against Kubernetes-like reality without giving an agent direct
production mutation access.

Real session planning and Kubernetes execution are now implemented. Runtime
checks are starting to land, and Gateway API routing groundwork has started.

The current `kply-core` session domain model defines the first pre-`1.0.0`
agent-readable JSON contract. This contract is provisional and may change
before `1.0.0` when the roadmap requires it. Intentional changes must update
snapshots in the same pull request.

The current `kply-core` app graph model defines the first pre-`1.0.0` graph
contract. It is independent from raw Kubernetes client types and currently
contains the root workload, Pods directly owned by that workload, and Services
that select that workload, route objects that reference those Services, and
container probe, image, resource, config, and secret metadata facts, plus
confidence metadata for inferred relationships and graph warnings. Future
roadmap tasks will add more warning variants.

Current provisional pre-`1.0.0` app graph fields:

- `workload`: root workload object with `namespace`, `kind`, and `name`.
- `owned_pods`: list of Pod references owned by the root workload, serialized
  in deterministic order.
- `selecting_services`: list of Service references selecting the root workload,
  serialized in deterministic order.
- `service_routes`: list of Service-to-route reference edges, serialized in
  deterministic order.
- `probe_facts`: list of container probe facts, each identifying a container
  and indicating readiness, liveness, and startup probe presence, serialized
  in deterministic order.
- `image_facts`: list of container image facts, each identifying a container
  and the configured image reference, serialized in deterministic order.
- `resource_facts`: list of container resource facts, each identifying a
  container and configured CPU and memory request or limit quantities,
  serialized in deterministic order.
- `config_references`: list of container-to-ConfigMap metadata references,
  serialized in deterministic order.
- `secret_references`: list of container-to-Secret metadata references. These
  entries identify Secret names only; Secret contents are never part of the
  graph contract. The list is serialized in deterministic order.
- `relationship_confidences`: list of confidence metadata entries for inferred
  graph relationships. Each entry contains a typed `relationship` object and a
  `confidence` level of `low`, `medium`, or `high`, serialized in
  deterministic order.
- `warnings`: list of graph-building warnings, serialized in deterministic
  order. Current warnings include `ambiguous_service_selector`, which names the
  Service and candidate workloads when a selector is not specific enough, and
  `missing_route`, which names a selected Service with no discovered route
  reference, and `missing_probes`, which names a container and enumerates probe
  kinds not discovered for it.

Current provisional pre-`1.0.0` session plan fields:

- `id`: session identifier string.
- `name`: human-readable session name string.
- `workload`: target workload object with `namespace`, `kind`, and `name`.
- `image`: proposed sandbox image reference string.
- `ttl`: optional compact duration string for the planned session lifetime,
  such as `30m`; serialized as `null` when unset.
- `planned_resources`: list of Kubernetes resources Kply expects a future
  session to create for the sandbox, deduplicated and serialized in
  deterministic order.
- `planned_labels`: list of metadata key/value pairs Kply expects to apply as
  labels to session-owned resources, deduplicated and serialized in
  deterministic order.
- `planned_annotations`: list of metadata key/value pairs Kply expects to
  apply as annotations to session-owned resources, deduplicated and serialized
  in deterministic order.
- `planned_checks`: list of verification checks Kply expects a future session
  to run against sandbox resources, deduplicated and serialized in
  deterministic order.
- `planned_cleanup_steps`: list of cleanup operations Kply expects a future
  session to run when removing sandbox resources, deduplicated and serialized
  in execution order.
- `required_permissions`: list of Kubernetes-style permissions Kply expects
  the caller or future controller to need for the planned session, deduplicated
  and serialized in deterministic order.
- `unsupported_feature_warnings`: list of plan features Kply can describe but
  cannot execute yet, deduplicated and serialized in deterministic order.
- `risk_notes`: list of sensitive infrastructure references that require
  manual review or policy handling, deduplicated and serialized in
  deterministic order.
- `route_selector`: always serialized as a nullable field; it is a test
  traffic selector object when configured and `null` otherwise.
- `policy`: allowed operation policy.
- `status`: session lifecycle status string.

Current provisional pre-`1.0.0` workload fields:

- `namespace`: Kubernetes namespace string.
- `kind`: Kubernetes resource kind string such as `Deployment` or
  `StatefulSet`.
- `name`: Kubernetes resource name string.

Current provisional pre-`1.0.0` planned Kubernetes resource fields:

- `namespace`: Kubernetes namespace string.
- `kind`: Kubernetes resource kind string such as `Deployment`, `Service`, or
  `HTTPRoute`.
- `name`: Kubernetes resource name string.

Current provisional pre-`1.0.0` planned metadata fields:

- `key`: Kubernetes metadata key string.
- `value`: Kubernetes metadata value string.

Current provisional pre-`1.0.0` planned check fields:

- `name`: stable planned check name string.
- `target`: resource, image, or route target string the check will verify.

Current provisional pre-`1.0.0` executed check result statuses:

- `passed`: the check completed successfully and found no blocking condition.
- `failed`: the check completed and found a blocking condition.
- `warning`: the check completed and found a non-blocking concern.
- `skipped`: the check did not run because prerequisites were missing or
  disabled.

Current provisional pre-`1.0.0` planned cleanup step fields:

- `action`: stable planned cleanup action string.
- `target`: resource target string the cleanup action will remove.

Current provisional pre-`1.0.0` required permission fields:

- `api_group`: Kubernetes API group string; the core API group is serialized as
  an empty string.
- `resource`: Kubernetes resource plural string.
- `verbs`: deduplicated list of Kubernetes RBAC verbs required for that
  resource, serialized in deterministic order.

Gateway API routing permissions are documented in
[gateway-api.md](gateway-api.md). Session plans currently include
`gateway.networking.k8s.io/httproutes` with `create`, `delete`, and `get` when
temporary routes may be part of the sandbox session.

Current provisional pre-`1.0.0` routing capability detection fields:

- `gateway_api`: Gateway API resource and temporary `HTTPRoute` capability
  details.
- `ingress`: Ingress inventory detected in the target namespace, including
  Ingress names, class names, hostnames, and backend Service names. Ingress
  planning is detected as future work until controller-specific adapters land.
- `preview_service_available`: boolean indicating that direct preview Service
  checks can be considered as an explicit fallback.
- `candidate_strategies`: deterministic list of route strategies with enough
  detected input to consider, currently `gateway_api`, `ingress`, and
  `preview_service`.
  This list is informational only; consumers must evaluate `limitations` before
  selecting a strategy as executable.
- `limitations`: stable limitation codes explaining unavailable or incomplete
  routing paths.

Current provisional pre-`1.0.0` NGINX Ingress planning behavior:

- `kply-routing` can generate a session-owned ingress-nginx canary `Ingress`
  manifest from an existing source `Ingress`.
- The source `Ingress` must declare an IngressClass name of `nginx`,
  `ingress-nginx`, or `nginx-ingress`.
- Only header-selected sandbox traffic is planned. The generated manifest adds
  ingress-nginx canary annotations for the configured header name and value.
- Host/path rules are mirrored from the source `Ingress`, but backends point to
  the sandbox Service. Production `Ingress` resources are not patched.

Current provisional pre-`1.0.0` unsupported feature warning fields:

- `feature`: stable unsupported feature identifier.
- `reason`: stable reason code explaining why the feature is not executable
  yet.

Current provisional pre-`1.0.0` risk note fields:

- `category`: stable risk category such as `database` or `secret`.
- `severity`: stable risk severity such as `warning`.
- `target`: resource, config value, or metadata reference the note applies to.
- `reason`: stable reason code explaining why the target requires review.

Current provisional pre-`1.0.0` route selector fields:

`route_selector` is a single tagged object. Header and host selectors are
mutually exclusive alternatives, selected by the `kind` field.

- `kind`: selector type string, currently either `header` or `host`.
- `name`: header name string, present when `kind` is `header`.
- `value`: header value string, present when `kind` is `header`.
- `hostname`: host name string, present when `kind` is `host`.

Unknown `kind` values are rejected. Implementations must reject cross-variant
or extra fields: when `kind` is `header`, only `kind`, `name`, and `value` are
allowed; when `kind` is `host`, only `kind` and `hostname` are allowed.

Current provisional pre-`1.0.0` policy fields:

- `allowed_operations`: list of operation strings such as `inspect`, `plan`,
  `prepare`, `route`, `verify`, `cleanup`, and `promote`. Serialization follows
  the canonical `SessionOperation::all()` declaration order so JSON snapshots
  are comparable, but agents must treat the values as a set of allowed
  operations rather than an execution sequence. Agents should check membership
  in `allowed_operations`; they must not infer execution order. Duplicate
  operations are invalid in the current parser and must not be emitted.

Current provisional pre-`1.0.0` status values:

- `planned`: session has been modeled but no sandbox work has started.
- `preparing`: sandbox resources are being prepared.
- `active`: sandbox resources are available for agent or check traffic.
- `verifying`: checks are running against the session.
- `blocked`: verification or policy found a blocking result.
- `ready`: verification found the session ready for a future promotion step.
- `cleaned_up`: session-owned resources have been cleaned up.
- `failed`: planning, preparation, verification, or cleanup failed.

Current provisional pre-`1.0.0` session report fields:

- `plan`: embedded full session plan object with all session plan fields above.
- `status`: reportable final status string, one of `blocked`, `ready`,
  `cleaned_up`, or `failed`.

Current provisional pre-`1.0.0` session event fields:

- `session_id`: session identifier string.
- `sequence`: monotonically increasing audit event sequence integer.
- `kind`: event kind string.
- `status`: stored lifecycle status string for agent convenience. It must
  always match the status implied by `kind`; deserialization rejects mismatches.

Current event `kind` to `status` mappings are one-to-one:

- `planned` implies `planned`.
- `preparing` implies `preparing`.
- `active` implies `active`.
- `verifying` implies `verifying`.
- `blocked` implies `blocked`.
- `ready` implies `ready`.
- `cleaned_up` implies `cleaned_up`.
- `failed` implies `failed`.

Both `kind` and `status` are stored so agents can filter by `status` without
understanding every event `kind`. The current mappings are identity mappings;
future event kinds may map to broader statuses, for example a `check_failed`
kind could imply `blocked`. Deserialization validates `kind` and `status`
together so contract changes fail closed during evolution.

## Crate Direction

```text
kply-cli
  -> kply-config
  -> kply-checks
  -> kply-routing
  -> kply-k8s
  -> kply-core
```

Core does not depend on Kubernetes client libraries or CLI output.

## Future Configuration

Kply project configuration uses `kply.yaml` as the canonical filename.

The current provisional config schema version is `1`. This binary accepts
schema versions from `1` through the current version.

The top-level configuration model has these sections:

- `version`: config schema version.
- `apps`: application targets.
- `routing`: routing defaults.
- `checks`: verification checks.
- `policies`: safety policies.

Application config entries define these fields:

- `name`: Kply app name used by humans and agents.
- `namespace`: Kubernetes namespace containing the app.
- `workload`: Kubernetes workload name.
- `workload_kind`: optional Kubernetes workload kind such as `Deployment`,
  `StatefulSet`, or `DaemonSet`, defaulting to `Deployment`.
- `service`: Kubernetes service name used for routed traffic.
- `default_image`: optional default sandbox image.
- `route_strategy`: requested sandbox route strategy.

Config validation reports deterministic field-scoped errors before any future
Kubernetes access. Current validation covers unsupported schema versions and
required app fields.

Resolved config JSON serializes the top-level model with `apps`, `checks`, and
`policies` as arrays, `routing` as an object, `version` as a number, and route
strategies as stable snake_case strings.

`kply config show` renders the currently resolved config model. Like
`validate`, it uses `resolved_config()` to load a file specified with
`--config`, or the default empty config shape if not provided.

`kply config validate` validates the currently resolved config model from a
file specified with `--config`, or the default config shape if not provided.

Current CLI config precedence is:

1. An explicit `--config <path>` is loaded with `load_config_path()` through
   `resolved_config()`. Load or parse failures are reported as usage/config
   errors and exit with code `2`.
2. Without `--config`, config commands use the default in-memory config shape.

The `kply-config` crate includes upward discovery for the nearest `kply.yaml`,
but automatic discovery is intentionally not wired into CLI resolution yet.
`--no-config` already conflicts with `--config` and is reserved as the explicit
opt-out once discovery becomes active.

## Current Workflow

The current workflow is intentionally bounded:

1. Preserve crate boundaries.
2. Keep real session planning and Kubernetes execution guarded by explicit
   `--apply` confirmation.
3. Keep tests and CI green.
4. Keep routing changes scoped to Gateway API adapter roadmap milestones.

Runtime checks are starting to land on top of the implemented session
planning/execution path. Routing changes are now scoped to Gateway API adapter
roadmap milestones.
