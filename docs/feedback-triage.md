# Feedback Triage

This guide keeps adoption feedback useful without turning isolated anecdotes
into roadmap commitments.

## Missing Route Adapters

Track a missing route adapter when a routing issue or roadmap request shows that
Kply cannot model a real user routing stack with Gateway API, ingress-nginx,
Traefik, preview Service, or no-route mode.

Use the `routing` and `feedback` labels for the first report. Add or keep the
`roadmap` label only when the report includes sanitized evidence and a concrete
requested adapter behavior.

A missing route adapter becomes repeated feedback when one of these is true:

1. Three separate users or organizations request the same adapter family.
2. Two separate user reports and one benchmark or local demo failure point at
   the same missing adapter.
3. One user report shows a production-blocking route stack that cannot be
   represented safely by any existing strategy, and a maintainer can reproduce
   the limitation with sanitized fixtures.

Do not include Secret values, credentials, private hostnames, or unredacted
customer data in public tracking. Do not count those reports until the reporter
provides a sanitized version. Do not add customer examples to public docs unless
explicit permission exists.

When the repeated threshold is met, open a roadmap issue with evidence type
`repeated route adapter request`. Include:

- the adapter family, such as service mesh, cloud load balancer, custom ingress,
  or Gateway API implementation.
- the current Kply fallback and why it is insufficient.
- the minimum safe route behavior needed for an agent workflow.
- the required cleanup and ownership boundary.
- links to sanitized issues, benchmark runs, demo failures, or fixtures.

Convert the roadmap issue into an OpenSpec change only after the required route
mutation, cleanup, RBAC, and fallback behavior are clear enough to test.

## Policy Needs

Track a policy need when a session planning issue or agent workflow request
shows that Kply cannot express, evaluate, or explain a safety boundary that
humans need before allowing an agent to inspect, plan, check, or apply a
Kubernetes workflow.

Use the `session-planning` and `feedback` labels for the first report. Add or
keep the `roadmap` label only when the report includes sanitized evidence and a
concrete policy behavior, warning, or blocking rule.

A policy need becomes repeated feedback when one of these is true:

1. Three separate users or organizations request the same policy boundary.
2. Two separate user reports and one benchmark or local demo failure point at
   the same missing policy behavior.
3. One user report shows a production-blocking policy requirement that cannot be
   represented safely by existing config fields, and a maintainer can reproduce
   the limitation with sanitized fixtures.

Do not include Secret values, credentials, private hostnames, admission payloads,
or unredacted customer policy documents in public tracking. Do not count those
reports until the reporter provides a sanitized version. Do not add customer
examples to public docs unless explicit permission exists.

When the repeated threshold is met, open a roadmap issue with evidence type
`repeated policy need`. Include:

- the policy boundary, such as namespace allowlists, mutation modes, Secret
  handling, RBAC assumptions, admission controls, database access, or approval
  requirements.
- the current Kply behavior and why the warning, config field, or blocking
  result is insufficient.
- the minimum deterministic decision an agent needs before continuing.
- the expected CLI text, JSON field, or report evidence.
- links to sanitized issues, benchmark runs, demo failures, or fixtures.

Convert the roadmap issue into an OpenSpec change only after the required
policy input, evaluation timing, failure output, and test fixture are clear
enough to verify.

## App Graph Failures

Track an app graph failure when a Kubernetes discovery bug shows that Kply
models the wrong workload, Service, route, owner reference, selector, probe,
config reference, Secret metadata reference, or dependency relationship for an
application.

Use the `discovery` and `feedback` labels for the first report. Add or keep the
`roadmap` label only when the report includes sanitized input, current Kply
output, and the expected graph relationship or confidence level.

An app graph failure becomes repeated feedback when one of these is true:

1. Three separate users or organizations report the same missing graph relationship.
2. Two separate user reports and one benchmark or local demo failure point at
   the same incorrect app graph behavior.
3. One user report shows a production-blocking graph mistake that can cause an
   agent to plan against the wrong workload, Service, route, or dependency, and
   a maintainer can reproduce the limitation with sanitized fixtures.

Do not include Secret values, credentials, private hostnames, Pod logs, or
unredacted customer manifests in public tracking. Do not count those reports
until the reporter provides a sanitized version. Do not add customer examples
to public docs unless explicit permission exists.

When the repeated threshold is met, open a roadmap issue with evidence type
`repeated app graph failure`. Include:

- the missed or incorrect graph relationship.
- the Kubernetes object shapes involved, such as Deployment, Service, Ingress,
  Gateway API route, owner reference, selector, probe, ConfigMap reference, or
  Secret metadata reference.
- the current Kply graph output and why it is unsafe or insufficient for an
  agent workflow.
- the minimum deterministic graph evidence needed before planning a session.
- links to sanitized issues, benchmark runs, demo failures, or fixtures.

Convert the roadmap issue into an OpenSpec change only after the discovery
input, graph edge semantics, confidence output, and fixture set are clear enough
to test.

## Check Failures

Track a check failure when an agent workflow request shows that Kply ran,
skipped, omitted, or explained a verification check in a way that could mislead
an agent or human about sandbox health, service reachability, route behavior, or
deployment readiness.

Use the `agent-workflow` and `feedback` labels for the first report. Add or keep
the `roadmap` label only when the report includes sanitized commands, check
output, expected evidence, and the safety impact of the missing or misleading
verification.

A check failure becomes repeated feedback when one of these is true:

1. Three separate users or organizations report the same check failure mode.
2. Two separate user reports and one benchmark or local demo failure point at
   the same missing or misleading check behavior.
3. One user report shows a production-blocking verification gap that can cause
   an agent to proceed with false confidence, and a maintainer can reproduce the
   limitation with sanitized fixtures.

Do not include Secret values, credentials, private hostnames, response bodies
with customer data, or unredacted logs in public tracking. Do not count those
reports until the reporter provides a sanitized version. Do not add customer
examples to public docs unless explicit permission exists.

When the repeated threshold is met, open a roadmap issue with evidence type
`repeated check failure`. Include:

- the check name or missing check category.
- the current Kply check status, evidence, skipped reason, or missing output.
- why the result is unsafe, misleading, incomplete, or insufficient for an
  agent workflow.
- the minimum deterministic check evidence needed before an agent continues.
- links to sanitized issues, benchmark runs, demo failures, or fixtures.

Convert the roadmap issue into an OpenSpec change only after the check input,
status semantics, evidence schema, and fixture set are clear enough to test.

## OpenSpec Conversion

Convert repeated feedback into a new OpenSpec change only after a maintainer can
write testable requirements from sanitized evidence. Do not start implementation
from an anecdote, a private customer artifact, or a broad product wish.

Before opening an OpenSpec change, the roadmap issue must include:

- evidence type, such as `repeated route adapter request`, `repeated policy
  need`, `repeated app graph failure`, or `repeated check failure`.
- links to sanitized issues, benchmark runs, demo failures, or fixtures.
- the affected requirement or the new requirement that should be added under
  `openspec/specs/`.
- at least one acceptance scenario written as a concrete WHEN and THEN outcome.
- the fixture, snapshot, or integration test boundary that will prove the
  behavior.
- the explicit non-goals so the change does not become a broad platform rewrite.

The OpenSpec change should stay scoped to one capability family. Split separate
route adapters, policy semantics, graph relationships, and check behaviors into
separate changes unless the same fixture and acceptance scenario require them
together.

Keep the roadmap issue open until the OpenSpec change links back to the
sanitized evidence and states how public documentation will avoid exposing
customer details.
