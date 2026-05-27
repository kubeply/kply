# Coding Agent Demo Guide

This guide shows how to hand the local Kind demo to a coding agent without
giving it access to a production cluster.

Current `kply` behavior is limited to demo setup, read-only inspection, and
session planning. Session creation, sandbox routing, automated checks, and
promotion are not implemented yet.

## Boundary

Use a disposable Kind cluster and the `kind-kply-demo` context. All demo
resources must stay in the `kply-demo` namespace.

Allowed agent actions for this demo:

- run `kply demo doctor`, `kply demo install`, `kply demo reset`, and
  `kply demo teardown`.
- run read-only `kply` commands with
  `--config fixtures/demo/ecommerce-basic/kply.yaml`.
- run `kubectl get`, `kubectl describe`, `kubectl logs`, and `kubectl port-forward`
  inside the `kply-demo` namespace.
- apply only the demo backend variant manifests listed in this guide.

Do not allow the agent to change Kubernetes resources outside `kply-demo`.

## Prepare The Demo

For a scripted local run, use:

```bash
scripts/demo-walkthrough.sh
```

The script simulates sandbox creation by applying the fixed backend variant
until `kply session create` is implemented.

Create the local cluster and install the baseline fixture:

```bash
cargo run --locked --bin kply -- demo doctor
kind create cluster --name kply-demo
kubectl config use-context kind-kply-demo
cargo run --locked --bin kply -- demo install
```

Inject the broken backend variant:

```bash
kubectl delete -f fixtures/demo/ecommerce-basic/manifests/backend.yaml --ignore-not-found
kubectl apply -f fixtures/demo/ecommerce-basic/manifests/backend-broken.yaml
kubectl -n kply-demo rollout status --timeout=5m deployment/checkout-api
```

Start a port-forward in a separate terminal:

```bash
kubectl -n kply-demo port-forward service/checkout-api 18080:8080
```

## Give The Agent This Prompt

```text
You are working in the kply repository on a disposable Kind cluster.

Goal: diagnose the broken checkout backend in the local Kply demo and propose
the safest repair path.

Boundaries:
- Use Kubernetes context kind-kply-demo only.
- Do not touch resources outside the kply-demo namespace.
- Do not read Kubernetes Secret values.
- Do not create new cluster resources except by applying one of the demo
  backend variant manifests in fixtures/demo/ecommerce-basic/manifests/.
- Prefer kply commands before raw kubectl when kply exposes the needed view.

Useful commands:
- cargo run --locked --bin kply -- demo doctor
- cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml config validate
- cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml app list
- cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml app inspect checkout
- cargo run --locked --bin kply -- --config fixtures/demo/ecommerce-basic/kply.yaml session plan checkout --image hashicorp/http-echo:1.0
- cargo run --locked --bin kply -- route plan checkout-plan --namespace kply-demo
- cargo run --locked --bin kply -- route apply checkout-plan --namespace kply-demo
- cargo run --locked --bin kply -- route cleanup checkout-plan --namespace kply-demo
- kubectl -n kply-demo get deployment,service,pod
- kubectl -n kply-demo logs deployment/checkout-api
- curl http://127.0.0.1:18080

Expected outcome:
- Explain what is broken.
- Show the kply plan output you used.
- Show the route plan output and note that route apply is currently a guarded
  no-op.
- Apply fixtures/demo/ecommerce-basic/manifests/backend-fixed.yaml only if you
  need to verify the repair.
- Verify the checkout response returns healthy JSON.
- Leave a short report of commands run and remaining limitations.
```

## Verify And Reset

The broken backend returns an error-shaped response:

```bash
curl http://127.0.0.1:18080
```

The repaired backend should return a healthy response that includes catalog
reachability. After the agent finishes, reset the fixture or remove it:

```bash
cargo run --locked --bin kply -- demo reset
cargo run --locked --bin kply -- demo teardown
```

## What This Proves Today

The current demo proves that an agent can use a constrained CLI contract,
deterministic fixture manifests, and an isolated namespace to inspect and plan
around a Kubernetes change.

It does not yet prove live sandbox routing, automated verification, rollback,
or promotion. Those flows are future roadmap work.
