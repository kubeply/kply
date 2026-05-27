# Gateway API Routing

Kply uses Gateway API as the first Kubernetes-native path for sandbox traffic
isolation. The current CLI can plan route resources and cleanup selectors. Route
mutation remains guarded while the adapter is completed.

## When Gateway API Is Unavailable

Gateway API routing is unavailable when the cluster cannot list
`GatewayClass`, `Gateway`, or `HTTPRoute` resources, or when no HTTP-compatible
Gateway can accept a temporary route. Treat that as an explicit routing
limitation, not as permission to patch production traffic resources directly.

Use one of these fallback paths:

| Situation | Recommended fallback | What it can prove |
| --- | --- | --- |
| Local Kind demo without Gateway API CRDs | Run `kply route plan`, `kply route apply`, and `kply route cleanup` as dry-run/no-op commands | The session id, ownership labels, route names, and cleanup selectors are deterministic |
| Cluster has no Gateway API controller | Use session manifests plus `kubectl port-forward` or a temporary preview Service for agent-only checks | The sandbox workload starts and answers direct test traffic |
| Cluster has Gateway API CRDs but no compatible HTTP Gateway | Ask the platform owner to expose an HTTP Gateway for sandbox routes, or choose a future fallback strategy such as ingress or preview service | Whether the cluster can support routed sandbox traffic after platform setup |
| Production route mutation is not approved | Keep route mutation disabled and run read-only checks plus direct sandbox checks | The proposed workload can be inspected and smoke-tested without touching live routes |

Do not silently fall back to editing existing production `Ingress`, `HTTPRoute`,
or Service resources. Fallback routing must be explicit because each option
tests a different part of the system. Direct preview traffic can validate the
sandbox workload, but it does not prove that the real edge path, host rules,
headers, TLS, authentication, or middleware will behave the same way.

## Route Strategy Test Scope

Each route strategy validates a different boundary. Treat a passing result as
evidence for that boundary only.

| Strategy | What it can test | What it cannot prove |
| --- | --- | --- |
| Gateway API temporary `HTTPRoute` | Gateway API CRDs, HTTP-compatible Gateway attachment, host/header matching, Service forwarding to the sandbox backend, and ownership-scoped cleanup metadata | Non-Gateway ingress controller behavior, service mesh policies outside the Gateway path, external DNS propagation, or production authentication layers that are not on the selected Gateway listener |
| ingress-nginx canary `Ingress` | Existing ingress-nginx host/path rules, header-selected canary routing, Service forwarding to the sandbox backend, and generated canary annotations | Gateway API behavior, other ingress controllers, middleware not attached to the source Ingress, DNS changes, or traffic that does not carry the selected header |
| Traefik `IngressRoute` | Traefik rule matching with mirrored host/path constraints, header-selected sandbox routing, and Service forwarding to the sandbox backend | ingress-nginx canary behavior, Gateway API behavior, Traefik middleware behavior not represented in the generated route, DNS changes, or traffic that does not carry the selected header |
| Preview Service fallback | Sandbox workload creation, sandbox Service selection, pod readiness, Service endpoints, and direct agent smoke checks | Edge routing, host rules, request headers at the ingress layer, TLS termination, authentication, authorization, rate limits, CDN behavior, or production middleware |

Route strategy outputs must keep limitations explicit. A successful preview
Service check is useful for fast feedback, but it is not equivalent to proving a
production edge route.

## Required Permissions

Read-only route discovery needs permission to list Gateway API resources:

| API group | Resource | Scope | Verbs |
| --- | --- | --- | --- |
| `gateway.networking.k8s.io` | `gatewayclasses` | cluster | `get`, `list` |
| `gateway.networking.k8s.io` | `gateways` | namespace | `get`, `list` |
| `gateway.networking.k8s.io` | `httproutes` | namespace | `get`, `list` |

Future route mutation needs permission to create and remove temporary
`HTTPRoute` objects in the sandbox namespace:

| API group | Resource | Scope | Verbs |
| --- | --- | --- | --- |
| `gateway.networking.k8s.io` | `httproutes` | namespace | `create`, `delete`, `get` |

Session plans currently include the mutating `httproutes` requirement because a
planned sandbox session can include a temporary `HTTPRoute`. Route cleanup must
only delete routes carrying Kply ownership labels, but Kubernetes RBAC cannot
restrict deletion by label selector by itself. Use a dedicated namespace,
dedicated service account, or admission policy when granting delete access.

## Route Ownership

Kply-generated route plans use ownership labels so cleanup can target only
session-owned routes:

- `kply.dev/managed-by=kply`
- `kply.dev/session-id=<session-id>`

Do not grant Kply broad production-route ownership unless the cluster has a
separate policy layer enforcing these labels.
