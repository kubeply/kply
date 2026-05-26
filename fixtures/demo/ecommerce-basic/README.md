# Ecommerce Basic Demo

This fixture describes the first local Kind demo shape for Kply.

It models a small ecommerce stack:

- `storefront-web`: frontend service.
- `checkout-api`: backend service intended for sandbox updates.
- `catalog-api`: simple backing service used by the backend.

All Kubernetes objects live in the dedicated `kply-demo` namespace and carry
`app.kubernetes.io/part-of: kply-demo` labels so future demo install and cleanup
commands can stay scoped.

The manifests are fixtures only. Demo install, reset, teardown, and live Kind
workflow commands are not implemented yet.

## Variants

Backend variant manifests are mutually exclusive fixture inputs for future demo
reset and repair flows.

- `manifests/backend.yaml`: baseline backend fixture.
- `manifests/backend-broken.yaml`: intentionally broken backend response for
  future agent repair and verification flows.
- `manifests/backend-fixed.yaml`: repaired backend response for future agent
  verification and promotion flows.
