#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

CLUSTER_NAME="${KPLY_DEMO_CLUSTER:-kply-demo}"
CONTEXT_NAME="kind-${CLUSTER_NAME}"
NAMESPACE="kply-demo"
CONFIG_PATH="fixtures/demo/ecommerce-basic/kply.yaml"
BASELINE_BACKEND="fixtures/demo/ecommerce-basic/manifests/backend.yaml"
BROKEN_BACKEND="fixtures/demo/ecommerce-basic/manifests/backend-broken.yaml"
FIXED_BACKEND="fixtures/demo/ecommerce-basic/manifests/backend-fixed.yaml"
CHECKOUT_PORT="${KPLY_DEMO_PORT:-18080}"

PORT_FORWARD_PID=""
RUN_FULL_CLEANUP_ON_EXIT=1
DEMO_CONTEXT_ACTIVE=0

cleanup_port_forward() {
  if [[ -n "${PORT_FORWARD_PID}" ]] && kill -0 "${PORT_FORWARD_PID}" 2>/dev/null; then
    kill "${PORT_FORWARD_PID}" 2>/dev/null || true
    wait "${PORT_FORWARD_PID}" 2>/dev/null || true
  fi
}

cleanup_demo_on_exit() {
  cleanup_port_forward
  if [[ "${RUN_FULL_CLEANUP_ON_EXIT}" == "1" ]]; then
    local current_context=""
    current_context="$(kubectl config current-context 2>/dev/null || true)"
    if [[ "${DEMO_CONTEXT_ACTIVE}" != "1" ]] || [[ "${current_context}" != "${CONTEXT_NAME}" ]]; then
      printf 'skipping automatic demo teardown on exit: current context is %s (expected %s)\n' \
        "${current_context:-<none>}" "${CONTEXT_NAME}" >&2
      return
    fi
    kubectl delete -f "${BROKEN_BACKEND}" --ignore-not-found >/dev/null 2>&1 || true
    kubectl delete -f "${FIXED_BACKEND}" --ignore-not-found >/dev/null 2>&1 || true
    kply demo teardown >/dev/null 2>&1 || true
  fi
}

trap cleanup_demo_on_exit EXIT

step() {
  printf '\n==> %s\n' "$1"
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'missing required command: %s\n' "$1" >&2
    exit 1
  fi
}

kply() {
  cargo run --locked --bin kply -- "$@"
}

wait_for_checkout() {
  local url="http://127.0.0.1:${CHECKOUT_PORT}"
  local connect_timeout=1
  local max_time=2

  for _ in $(seq 1 30); do
    if curl --fail --silent --show-error \
      --connect-timeout "${connect_timeout}" \
      --max-time "${max_time}" \
      "${url}" >/tmp/kply-demo-checkout-response.json; then
      cat /tmp/kply-demo-checkout-response.json
      printf '\n'
      return 0
    fi
    sleep 1
  done

  printf 'checkout service did not become reachable at %s\n' "${url}" >&2
  exit 1
}

require_command cargo
require_command curl
require_command kind
require_command kubectl

step "Check local demo prerequisites"
kply demo doctor

step "Create or reuse the local Kind cluster"
if ! kind get clusters | grep -Fx "${CLUSTER_NAME}" >/dev/null; then
  kind create cluster --name "${CLUSTER_NAME}"
fi
kubectl config use-context "${CONTEXT_NAME}"
DEMO_CONTEXT_ACTIVE=1

step "Install the baseline demo"
kply demo install

step "Inject the broken backend variant"
kubectl delete -f "${BASELINE_BACKEND}" --ignore-not-found
kubectl apply -f "${BROKEN_BACKEND}"
kubectl -n "${NAMESPACE}" rollout status --timeout=5m deployment/checkout-api

step "Inspect the app with Kply"
kply --config "${CONFIG_PATH}" config validate
kply --config "${CONFIG_PATH}" app list
kply --config "${CONFIG_PATH}" app inspect checkout

step "Plan the future sandbox session"
kply --config "${CONFIG_PATH}" session plan checkout --image hashicorp/http-echo:1.0

step "Plan temporary Gateway API routing"
kply route plan checkout-plan --namespace "${NAMESPACE}"
kply route apply checkout-plan --namespace "${NAMESPACE}"
kply route cleanup checkout-plan --namespace "${NAMESPACE}"

step "Create the simulated sandbox candidate"
printf 'kply session create is not implemented yet; applying the fixed demo backend variant as a local stand-in.\n'
kubectl delete -f "${BROKEN_BACKEND}" --ignore-not-found
kubectl apply -f "${FIXED_BACKEND}"
kubectl -n "${NAMESPACE}" rollout status --timeout=5m deployment/checkout-api

step "Run the demo check"
kubectl -n "${NAMESPACE}" port-forward "service/checkout-api" "${CHECKOUT_PORT}:8080" >/tmp/kply-demo-port-forward.log 2>&1 &
PORT_FORWARD_PID="$!"
sleep 2
if ! kill -0 "${PORT_FORWARD_PID}" 2>/dev/null; then
  printf 'port-forward failed to start; see /tmp/kply-demo-port-forward.log\n' >&2
  cat /tmp/kply-demo-port-forward.log >&2
  exit 1
fi
wait_for_checkout | grep -Eq '"status"[[:space:]]*:[[:space:]]*"ok"'

step "Cleanup"
cleanup_port_forward
PORT_FORWARD_PID=""
kubectl delete -f "${FIXED_BACKEND}" --ignore-not-found
kply demo reset
kply demo teardown
RUN_FULL_CLEANUP_ON_EXIT=0

step "Walkthrough complete"
