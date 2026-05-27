# Kubernetes Response Fixtures

This directory contains captured or synthetic Kubernetes API responses.

Fixtures use `k8s-responses/<api-shape>/` directories. Each JSON file should be
the response body for one Kubernetes list request and must avoid Secret values.
Secret names may appear as metadata references, such as Ingress TLS
`secretName` fields.

Current fixtures:

- `read-only-app/`: synthetic shop namespace responses for read-only app
  discovery covering Deployments, Services, Pods, Ingress, GatewayClass,
  Gateway, and HTTPRoute.
- `gateway-api-supported/`: synthetic Gateway API responses with HTTP and HTTPS
  listeners plus existing HTTPRoutes that should model as route-capable.
- `gateway-api-unsupported-missing-class/`: synthetic Gateway API responses
  where Gateways exist but no GatewayClass resources are discoverable.
- `gateway-api-unsupported-tcp-only/`: synthetic Gateway API responses where
  discovered Gateways only expose TCP listeners.
