# Gateway API Routing

Kply uses Gateway API as the first Kubernetes-native path for sandbox traffic
isolation. The current CLI can plan route resources and cleanup selectors. Route
mutation remains guarded while the adapter is completed.

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
