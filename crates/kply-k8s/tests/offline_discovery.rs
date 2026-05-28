//! Offline integration tests for read-only Kubernetes discovery.

use std::path::Path;
use std::time::Duration;

use http::{Method, Request, Response};
use kply_core::WorkloadRef;
use kube::Client;
use kube::client::Body;
use serde_json::{json, to_vec};
use tokio::task::JoinHandle;
use tower_test::mock::{self, Handle};

type MockKubeHandle = Handle<Request<Body>, Response<Body>>;

#[tokio::test]
async fn lists_namespaces_from_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_namespace_list_api(handle);

    let namespaces = kply_k8s::list_namespaces(client)
        .await
        .expect("mocked Namespace list should succeed");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(
        namespaces
            .iter()
            .map(|namespace| namespace.name.as_str())
            .collect::<Vec<_>>(),
        ["kube-system", "shop"]
    );
}

#[tokio::test]
async fn discovers_read_only_app_from_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_kubernetes_api(
        handle,
        &[
            ExpectedListResponse {
                path: "/apis/apps/v1/namespaces/shop/deployments",
                fixture_path: "read-only-app/deployments.json",
            },
            ExpectedListResponse {
                path: "/api/v1/namespaces/shop/services",
                fixture_path: "read-only-app/services.json",
            },
            ExpectedListResponse {
                path: "/api/v1/namespaces/shop/pods",
                fixture_path: "read-only-app/pods.json",
            },
            ExpectedListResponse {
                path: "/apis/networking.k8s.io/v1/namespaces/shop/ingresses",
                fixture_path: "read-only-app/ingresses.json",
            },
            ExpectedListResponse {
                path: "/apis/gateway.networking.k8s.io/v1/gatewayclasses",
                fixture_path: "read-only-app/gatewayclasses.json",
            },
            ExpectedListResponse {
                path: "/apis/gateway.networking.k8s.io/v1/namespaces/shop/gateways",
                fixture_path: "read-only-app/gateways.json",
            },
            ExpectedListResponse {
                path: "/apis/gateway.networking.k8s.io/v1/namespaces/shop/httproutes",
                fixture_path: "read-only-app/httproutes.json",
            },
        ],
    );

    let deployment_summaries = kply_k8s::list_deployments(client.clone(), "shop")
        .await
        .expect("mocked Deployment list should succeed");
    let service_summaries = kply_k8s::list_services(client.clone(), "shop")
        .await
        .expect("mocked Service list should succeed");
    let workload = WorkloadRef::new("shop", "ReplicaSet", "checkout-api-7d9f4d9d")
        .expect("workload reference should be valid");
    let pod_summaries = kply_k8s::list_pods_owned_by_workload(client.clone(), &workload)
        .await
        .expect("mocked Pod list should succeed");
    let ingress_summaries = kply_k8s::list_ingresses(client.clone(), "shop")
        .await
        .expect("mocked Ingress list should succeed");
    let gateway_class_summaries = kply_k8s::list_gateway_classes(client.clone())
        .await
        .expect("mocked GatewayClass list should succeed");
    let gateway_summaries = kply_k8s::list_gateways(client.clone(), "shop")
        .await
        .expect("mocked Gateway list should succeed");
    let http_route_summaries = kply_k8s::list_http_routes(client, "shop")
        .await
        .expect("mocked HTTPRoute list should succeed");

    wait_for_mock_kubernetes_api(server).await;

    kply_test::insta::assert_json_snapshot!(
        "read_only_app_offline_discovery",
        json!({
            "deployments": deployment_summaries,
            "services": service_summaries,
            "pods": pod_summaries,
            "ingresses": ingress_summaries,
            "gateway_classes": gateway_class_summaries,
            "gateways": gateway_summaries,
            "http_routes": http_route_summaries,
        })
    );
}

#[tokio::test]
async fn lists_sessions_from_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_session_list_api(handle);

    let sessions = kply_k8s::list_sessions(client, "shop")
        .await
        .expect("mocked session list should succeed");

    wait_for_mock_kubernetes_api(server).await;

    kply_test::insta::assert_json_snapshot!("session_list_offline_discovery", sessions);
}

#[tokio::test]
async fn gets_session_from_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_session_get_api(handle);

    let session = kply_k8s::get_session(client, "shop", "checkout-plan")
        .await
        .expect("mocked session get should succeed")
        .expect("mocked session should be found");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(session.id, "checkout-plan");
    assert_eq!(session.status.as_deref(), Some("active"));
    assert_eq!(session.workload_name, "checkout-plan-workload");
}

#[tokio::test]
async fn returns_none_for_missing_session_from_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_missing_session_get_api(handle);

    let session = kply_k8s::get_session(client, "shop", "missing-plan")
        .await
        .expect("mocked missing session get should succeed");

    wait_for_mock_kubernetes_api(server).await;

    assert!(session.is_none());
}

struct ExpectedListResponse {
    path: &'static str,
    fixture_path: &'static str,
}

fn mock_client() -> (Client, MockKubeHandle) {
    let (mock_service, handle) = mock::pair::<Request<Body>, Response<Body>>();

    (Client::new(mock_service, "default"), handle)
}

fn spawn_mock_namespace_list_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);
        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive expected request");

        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.uri().path(), "/api/v1/namespaces");

        send.send_response(Response::new(Body::from(
            to_vec(&json!({
                "apiVersion": "v1",
                "kind": "NamespaceList",
                "items": [
                    { "metadata": { "name": "shop" } },
                    { "metadata": { "name": "kube-system" } }
                ]
            }))
            .expect("namespace fixture should serialize"),
        )));
    })
}

fn spawn_mock_session_list_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);
        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive expected request");

        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments"
        );
        assert!(
            request
                .uri()
                .query()
                .is_some_and(|query| query.contains("labelSelector=kply.dev%2Fmanaged-by%3Dkply")),
            "session list should include the Kply ownership label selector"
        );

        send.send_response(Response::new(Body::from(
            to_vec(&session_deployment_list_fixture())
                .expect("session list fixture should serialize"),
        )));
    })
}

fn spawn_mock_session_get_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);
        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive expected request");

        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments"
        );
        let query = request
            .uri()
            .query()
            .expect("session get should include a label selector");
        assert!(
            query.contains("kply.dev%2Fmanaged-by%3Dkply"),
            "session get should filter by Kply ownership"
        );
        assert!(
            query.contains("kply.dev%2Fsession-id%3Dcheckout-plan"),
            "session get should filter by session id"
        );

        send.send_response(Response::new(Body::from(
            to_vec(&session_deployment_list_fixture())
                .expect("session get fixture should serialize"),
        )));
    })
}

fn spawn_mock_missing_session_get_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);
        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive expected request");

        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments"
        );
        let query = request
            .uri()
            .query()
            .expect("missing session get should include a label selector");
        assert!(
            query.contains("kply.dev%2Fmanaged-by%3Dkply"),
            "missing session get should filter by Kply ownership"
        );
        assert!(
            query.contains("kply.dev%2Fsession-id%3Dmissing-plan"),
            "missing session get should filter by session id"
        );

        send.send_response(Response::new(Body::from(
            to_vec(&empty_deployment_list_fixture())
                .expect("empty deployment list fixture should serialize"),
        )));
    })
}

fn session_deployment_list_fixture() -> serde_json::Value {
    json!({
        "apiVersion": "apps/v1",
        "kind": "DeploymentList",
        "items": [
            {
                "apiVersion": "apps/v1",
                "kind": "Deployment",
                "metadata": {
                    "name": "checkout-plan-workload",
                    "namespace": "shop",
                    "labels": {
                        "kply.dev/app": "checkout",
                        "kply.dev/managed-by": "kply",
                        "kply.dev/session-id": "checkout-plan",
                        "kply.dev/session-name": "checkout-plan"
                    },
                    "annotations": {
                        "kply.dev/session-status": "active"
                    }
                }
            },
            {
                "apiVersion": "apps/v1",
                "kind": "Deployment",
                "metadata": {
                    "name": "ignored-workload",
                    "namespace": "shop",
                    "labels": {
                        "app.kubernetes.io/managed-by": "kply"
                    }
                }
            }
        ]
    })
}

fn empty_deployment_list_fixture() -> serde_json::Value {
    json!({
        "apiVersion": "apps/v1",
        "kind": "DeploymentList",
        "items": []
    })
}

fn spawn_mock_kubernetes_api(
    handle: MockKubeHandle,
    expected_responses: &'static [ExpectedListResponse],
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);

        for expected_response in expected_responses {
            let (request, send) = handle
                .next_request()
                .await
                .expect("mock Kubernetes API should receive expected request");

            assert_eq!(request.method(), Method::GET);
            assert_eq!(request.uri().path(), expected_response.path);

            let fixture_path = Path::new("k8s-responses").join(expected_response.fixture_path);
            let fixture_body = std::fs::read(kply_test::fixture_path(fixture_path))
                .expect("Kubernetes response fixture should be readable");

            send.send_response(Response::new(Body::from(fixture_body)));
        }
    })
}

async fn wait_for_mock_kubernetes_api(server: JoinHandle<()>) {
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .expect("mock Kubernetes API should receive all expected requests")
        .expect("mock Kubernetes API task should complete");
}
