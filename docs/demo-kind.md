# Local Kind Demo

This guide sets up the ecommerce demo fixture in a local Kind cluster.

Use `kply demo doctor` to validate local prerequisites first. `kply demo
install` can install the baseline fixture after the cluster exists. `kply demo
reset` can restore the baseline fixture after variant testing. `kply demo
teardown` removes only labeled demo workloads and services.

Every demo action is an explicit subcommand. Commands operate against the
current Kubernetes context and keep their resources in the `kply-demo`
namespace.

For an agent-oriented workflow, see
[Coding Agent Demo Guide](demo-agent.md).

For a runnable end-to-end walkthrough, use
[`scripts/demo-walkthrough.sh`](../scripts/demo-walkthrough.sh). The script
inspects the demo app, plans a sandbox session, creates the sandbox with
`kply session create --apply`, runs `kply check run`, verifies the sandbox
Service, and removes the session with `kply session cleanup --apply`.

For a short replayable terminal cast of the same bounded workflow, see
[Local Demo Terminal Cast](demo-terminal-cast.md).

## Prerequisites

- Docker or another Kind-compatible container runtime.
- Kind installed as `kind`.
- Kubectl installed as `kubectl`.
- A checkout of this repository.

## Create The Cluster

Check local prerequisites before creating the cluster:

```bash
cargo run --locked --bin kply -- demo doctor
```

```bash
kind create cluster --name kply-demo
kubectl cluster-info --context kind-kply-demo
```

Use the `kind-kply-demo` context for the rest of this guide.

```bash
kubectl config use-context kind-kply-demo
```

## Run Optional Live Kind Tests

The live Kind mutation test is disabled unless explicitly requested. It creates
an isolated namespace, creates Kply-labeled sandbox resources, patches session
state annotations, lists cleanup candidates, deletes those resources, and then
removes the namespace.

```bash
KPLY_LIVE_KIND_TESTS=1 \
KPLY_LIVE_KIND_NAMESPACE=kply-live-kind \
cargo test -p kply-k8s --test live_cluster live_kind_session_resources_create_patch_list_and_cleanup_when_enabled --locked
```

## Install The Baseline Fixture

Install the dedicated namespace, backing service, frontend, and baseline
backend with Kply:

```bash
cargo run --locked --bin kply -- demo install
```

The command applies the baseline manifests and waits for the demo deployments
to roll out. The equivalent manual commands are:

```bash
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/namespace.yaml
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/catalog.yaml
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/frontend.yaml
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/backend.yaml
```

Wait for the workloads to become available.

```bash
kubectl -n kply-demo rollout status --timeout=5m deployment/catalog-api
kubectl -n kply-demo rollout status --timeout=5m deployment/storefront-web
kubectl -n kply-demo rollout status --timeout=5m deployment/checkout-api
```

## Check The Backend Response

Port-forward the checkout backend service in one terminal:

```bash
kubectl -n kply-demo port-forward service/checkout-api 18080:8080
```

Then call it from another terminal:

```bash
curl http://127.0.0.1:18080
```

The baseline backend should return a healthy checkout response.

## Switch Backend Variants

Backend variants are mutually exclusive because they use the same Kubernetes
resource names. Delete the current backend variant before applying another one.

To switch from the baseline backend to the broken backend:

```bash
kubectl delete -f fixtures/demo/ecommerce-basic/manifests/backend.yaml --ignore-not-found
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/backend-broken.yaml
kubectl -n kply-demo rollout status --timeout=5m deployment/checkout-api
```

To switch from the broken backend to the fixed backend:

```bash
kubectl delete -f fixtures/demo/ecommerce-basic/manifests/backend-broken.yaml --ignore-not-found
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/backend-fixed.yaml
kubectl -n kply-demo rollout status --timeout=5m deployment/checkout-api
```

The broken variant returns an error-shaped checkout response. The fixed variant
returns a healthy response that includes a reachable catalog status.

Return to the baseline fixture with Kply:

```bash
cargo run --locked --bin kply -- demo reset
```

## Inspect With Kply

The fixture includes a `kply.yaml` configuration file. Mutation commands still
require explicit `--apply` confirmation.

```bash
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml config validate
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml app list
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml app inspect checkout
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml session plan checkout --image hashicorp/http-echo:1.0
```

## Walk Through A Sandbox Session

The scripted walkthrough uses `nginx:1.27-alpine` as a small HTTP candidate
image because generated sandbox Services currently target port `80`. It keeps
the production `checkout-api` fixture separate and creates session-owned
resources named from the deterministic `checkout-plan` session id.

```bash
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml session plan checkout --image nginx:1.27-alpine --namespace kply-demo --route-strategy preview-service
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml session create checkout --image nginx:1.27-alpine --namespace kply-demo --route-strategy preview-service --apply
cargo run --locked --bin kply -- check run checkout-plan --namespace kply-demo
kubectl -n kply-demo port-forward service/checkout-plan-service 18080:80
curl http://127.0.0.1:18080
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml session cleanup checkout-plan --namespace kply-demo --dry-run
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml session cleanup checkout-plan --namespace kply-demo --apply
```

This is the current local version of the inspect, plan, create sandbox, run
checks, and cleanup flow. It validates sandbox resource creation and cleanup in
the disposable Kind cluster; it does not promote traffic or replace the
production demo backend.

## Plan Temporary Routing

The current Gateway API route commands are Kind-compatible because they only
render deterministic route plans and guarded no-op mutation reports. They do
not require Gateway API CRDs or a Gateway controller yet.

```bash
cargo run --locked --bin kply -- route plan checkout-plan --namespace kply-demo
cargo run --locked --bin kply -- route apply checkout-plan --namespace kply-demo
cargo run --locked --bin kply -- route cleanup checkout-plan --namespace kply-demo
```

`route apply` currently reports `status: not_implemented` and
`mutation: not_applied`. This keeps the local demo runnable on a plain Kind
cluster while the live Gateway API mutation adapter is still under
implementation.

## Cleanup

Remove only the labeled demo resources:

```bash
cargo run --locked --bin kply -- demo teardown
```

The equivalent manual command is:

```bash
kubectl -n kply-demo delete deployment,service --selector app.kubernetes.io/part-of=kply-demo --ignore-not-found --wait=true --timeout=5m
```

Or remove the whole local Kind cluster:

```bash
kind delete cluster --name kply-demo
```
