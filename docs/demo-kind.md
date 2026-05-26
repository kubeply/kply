# Local Kind Demo

This guide sets up the ecommerce demo fixture in a local Kind cluster.

Use `kply demo doctor` to validate local prerequisites first. `kply demo
install` can install the baseline fixture after the cluster exists. `kply demo
reset` can restore the baseline fixture after variant testing. `kply demo
teardown` removes the dedicated demo namespace.

Every demo action is an explicit subcommand. Commands operate against the
current Kubernetes context and keep their resources in the `kply-demo`
namespace.

For an agent-oriented workflow, see
[Coding Agent Demo Guide](demo-agent.md).

For a runnable end-to-end walkthrough, use
[`scripts/demo-walkthrough.sh`](../scripts/demo-walkthrough.sh). The script
simulates the sandbox creation step until `kply session create` is implemented.

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

The fixture includes a `kply.yaml` configuration file. Current `kply` commands
are read-only or planning-oriented; session creation is not implemented yet.

```bash
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml config validate
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml app list
cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml session plan checkout --image hashicorp/http-echo:1.0
```

## Cleanup

Remove only the demo namespace resources:

```bash
cargo run --locked --bin kply -- demo teardown
```

The equivalent manual command is:

```bash
kubectl delete namespace kply-demo --ignore-not-found --wait=true --timeout=5m
```

Or remove the whole local Kind cluster:

```bash
kind delete cluster --name kply-demo
```
