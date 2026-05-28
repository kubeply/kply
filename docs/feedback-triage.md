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
