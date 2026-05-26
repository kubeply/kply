# Ecommerce Basic Demo

This fixture describes the first local Kind demo shape for Kply.

It models a small ecommerce stack:

- `storefront-web`: frontend service.
- `checkout-api`: backend service intended for sandbox updates.
- `catalog-api`: simple backing service used by the backend.

All Kubernetes objects live in the dedicated `kply-demo` namespace and carry
`app.kubernetes.io/part-of: kply-demo` labels so demo install, reset, and
teardown commands can stay scoped.

The manifests are local demo fixtures. Session creation, sandbox routing,
automated checks, and promotion are not implemented yet.

See [../../../docs/demo-kind.md](../../../docs/demo-kind.md) for the current
manual Kind setup guide and
[../../../docs/demo-agent.md](../../../docs/demo-agent.md) for the coding agent
workflow.

## Variants

Backend variant manifests are mutually exclusive fixture inputs for future demo
reset and repair flows.

- `manifests/backend.yaml`: baseline backend fixture.
- `manifests/backend-broken.yaml`: intentionally broken backend response for
  future agent repair and verification flows.
- `manifests/backend-fixed.yaml`: repaired backend response for future agent
  verification and promotion flows.
