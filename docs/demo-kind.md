# Local Kind Demo

This guide sets up the ecommerce demo fixture in a local Kind cluster.

The current flow uses `kind` and `kubectl` directly. `kply demo doctor`,
`kply demo install`, `kply demo reset`, and `kply demo teardown` are not
implemented yet.

## Prerequisites

- Docker or another Kind-compatible container runtime.
- Kind installed as `kind`.
- Kubectl installed as `kubectl`.
- A checkout of this repository.

## Create The Cluster

```bash
kind create cluster --name kply-demo
kubectl cluster-info --context kind-kply-demo
```

Use the `kind-kply-demo` context for the rest of this guide.

```bash
kubectl config use-context kind-kply-demo
```

## Install The Baseline Fixture

Apply the dedicated namespace first, then the backing service, frontend, and
baseline backend.

```bash
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/namespace.yaml
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/catalog.yaml
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/frontend.yaml
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/backend.yaml
```

Wait for the workloads to become available.

```bash
kubectl -n kply-demo rollout status deployment/catalog-api
kubectl -n kply-demo rollout status deployment/storefront-web
kubectl -n kply-demo rollout status deployment/checkout-api
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
kubectl -n kply-demo rollout status deployment/checkout-api
```

To switch from the broken backend to the fixed backend:

```bash
kubectl delete -f fixtures/demo/ecommerce-basic/manifests/backend-broken.yaml --ignore-not-found
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/backend-fixed.yaml
kubectl -n kply-demo rollout status deployment/checkout-api
```

The broken variant returns an error-shaped checkout response. The fixed variant
returns a healthy response that includes a reachable catalog status.

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
kubectl delete namespace kply-demo --ignore-not-found
```

Or remove the whole local Kind cluster:

```bash
kind delete cluster --name kply-demo
```
