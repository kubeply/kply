# Least-Privilege RBAC

Kply should run with the smallest Kubernetes permissions required for the
workflow a team enables. Start with read-only inspection. Add sandbox mutation
only in namespaces where Kply may create temporary resources. Add Gateway API
route mutation only after the platform owner has approved temporary
`HTTPRoute` creation.

These examples use a dedicated namespace named `kply-sessions` and service
account named `kply-agent`. Adjust names before applying them.

## Read-Only Inspection

Use this role for commands that inspect workloads and sessions without creating
or deleting resources, such as `kply session list`, `kply session status`,
`kply report show`, and read-only app discovery.

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: kply-agent
  namespace: kply-sessions
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: kply-read-only
  namespace: kply-sessions
rules:
  - apiGroups: [""]
    resources: ["pods", "services"]
    verbs: ["get", "list", "watch"]
  - apiGroups: ["apps"]
    resources: ["deployments", "statefulsets"]
    verbs: ["get", "list", "watch"]
  - apiGroups: ["gateway.networking.k8s.io"]
    resources: ["gateways", "httproutes"]
    verbs: ["get", "list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: kply-read-only
  namespace: kply-sessions
subjects:
  - kind: ServiceAccount
    name: kply-agent
    namespace: kply-sessions
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: kply-read-only
```

Gateway classes are cluster-scoped. Grant this only when read-only route
capability detection needs to list `GatewayClass` resources.

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: kply-gatewayclass-read-only
rules:
  - apiGroups: ["gateway.networking.k8s.io"]
    resources: ["gatewayclasses"]
    verbs: ["get", "list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: kply-gatewayclass-read-only
subjects:
  - kind: ServiceAccount
    name: kply-agent
    namespace: kply-sessions
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: kply-gatewayclass-read-only
```

## Sandbox-Only Sessions

Use this role for `mutation_mode: sandbox-only` when Kply may create, annotate,
check, list, and delete only session-owned sandbox `Deployment` and `Service`
resources in one namespace. This is enough for `--route-strategy none`,
`preview`, and `preview-service`; it does not allow temporary edge route
mutation.

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: kply-sandbox-only
  namespace: kply-sessions
rules:
  - apiGroups: [""]
    resources: ["pods"]
    verbs: ["get", "list", "watch"]
  - apiGroups: [""]
    resources: ["services"]
    verbs: ["create", "delete", "get", "list", "patch"]
  - apiGroups: ["apps"]
    resources: ["deployments"]
    verbs: ["create", "delete", "get", "list", "patch"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: kply-sandbox-only
  namespace: kply-sessions
subjects:
  - kind: ServiceAccount
    name: kply-agent
    namespace: kply-sessions
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: kply-sandbox-only
```

Kply labels generated resources with `kply.dev/managed-by=kply` and
`kply.dev/session-id=<session-id>`. Kubernetes RBAC cannot restrict `delete` by
label selector. Use a dedicated namespace, admission policy, or both if the
cluster contains resources that Kply must never delete.

## Route Mutation

Add this role only for `mutation_mode: route-mutation` and route strategies
that create temporary Gateway API `HTTPRoute` resources. Keep it separate from
the sandbox role so teams can enable sandbox creation without edge route
mutation.

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: kply-route-mutation
  namespace: kply-sessions
rules:
  - apiGroups: ["gateway.networking.k8s.io"]
    resources: ["httproutes"]
    verbs: ["create", "delete", "get", "list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: kply-route-mutation
  namespace: kply-sessions
subjects:
  - kind: ServiceAccount
    name: kply-agent
    namespace: kply-sessions
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: kply-route-mutation
```

Do not grant write access to production `Ingress`, existing production
`HTTPRoute`, Secret, ConfigMap, or broad workload resources unless a separate
platform policy requires and constrains that access. Kply session mutation is
designed around temporary sandbox resources, not general cluster administration.
