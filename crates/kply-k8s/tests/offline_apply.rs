//! Offline integration tests for Kubernetes mutation adapters.

use std::time::Duration;

use http::{Method, Request, Response};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Service;
use kube::Client;
use kube::client::Body;
use serde_json::json;
use tokio::task::JoinHandle;
use tower_test::mock::{self, Handle};

type MockKubeHandle = Handle<Request<Body>, Response<Body>>;

#[tokio::test]
async fn creates_deployment_with_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_deployment_create_api(handle);
    let deployment = sandbox_deployment();

    let summary = kply_k8s::create_deployment(client, "shop", &deployment)
        .await
        .expect("mocked Deployment create should succeed");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(summary.namespace, "shop");
    assert_eq!(summary.name, "checkout-plan-workload");
    assert_eq!(summary.replicas, Some(1));
    assert_eq!(summary.images, ["ghcr.io/acme/checkout:next"]);
}

#[tokio::test]
async fn gets_deployment_with_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_deployment_get_api(handle);

    let summary = kply_k8s::get_deployment(client, "shop", "checkout-plan-workload")
        .await
        .expect("mocked Deployment get should succeed");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(summary.namespace, "shop");
    assert_eq!(summary.name, "checkout-plan-workload");
    assert_eq!(summary.replicas, Some(1));
    assert_eq!(summary.images, ["ghcr.io/acme/checkout:next"]);
}

#[tokio::test]
async fn creates_service_with_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_service_create_api(handle);
    let service = sandbox_service();

    let summary = kply_k8s::create_service(client, "shop", &service)
        .await
        .expect("mocked Service create should succeed");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(summary.namespace, "shop");
    assert_eq!(summary.name, "checkout-plan-service");
    assert_eq!(summary.service_type, Some("ClusterIP".to_owned()));
    assert_eq!(summary.ports.len(), 1);
    assert_eq!(summary.ports[0].port, 8080);
}

fn mock_client() -> (Client, MockKubeHandle) {
    let (mock_service, handle) = mock::pair::<Request<Body>, Response<Body>>();

    (Client::new(mock_service, "default"), handle)
}

fn sandbox_deployment() -> Deployment {
    serde_json::from_value(json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": "checkout-plan-workload",
            "namespace": "shop",
            "labels": {
                "app.kubernetes.io/managed-by": "kply"
            }
        },
        "spec": {
            "replicas": 1,
            "selector": {
                "matchLabels": {
                    "app.kubernetes.io/name": "checkout-plan-workload"
                }
            },
            "template": {
                "metadata": {
                    "labels": {
                        "app.kubernetes.io/name": "checkout-plan-workload"
                    }
                },
                "spec": {
                    "containers": [
                        {
                            "name": "checkout",
                            "image": "ghcr.io/acme/checkout:next"
                        }
                    ]
                }
            }
        }
    }))
    .expect("sandbox Deployment fixture should deserialize")
}

fn sandbox_service() -> Service {
    serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": {
            "name": "checkout-plan-service",
            "namespace": "shop",
            "labels": {
                "app.kubernetes.io/managed-by": "kply"
            }
        },
        "spec": {
            "type": "ClusterIP",
            "selector": {
                "app.kubernetes.io/name": "checkout-plan-workload"
            },
            "ports": [
                {
                    "name": "http",
                    "port": 8080,
                    "targetPort": 8080,
                    "protocol": "TCP"
                }
            ]
        }
    }))
    .expect("sandbox Service fixture should deserialize")
}

fn spawn_mock_deployment_create_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);
        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Deployment create request");

        assert_eq!(request.method(), Method::POST);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments"
        );
        let body = request
            .into_body()
            .collect_bytes()
            .await
            .expect("mock Deployment request body should be collectable");
        assert_deployment_request_body(&body);

        send.send_response(
            Response::builder()
                .status(201)
                .body(Body::from(
                    serde_json::to_vec(&sandbox_deployment())
                        .expect("sandbox Deployment response should serialize"),
                ))
                .expect("mock Deployment create response should build"),
        );
    })
}

fn spawn_mock_deployment_get_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);
        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Deployment get request");

        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments/checkout-plan-workload"
        );

        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    serde_json::to_vec(&sandbox_deployment())
                        .expect("sandbox Deployment response should serialize"),
                ))
                .expect("mock Deployment get response should build"),
        );
    })
}

fn spawn_mock_service_create_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);
        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Service create request");

        assert_eq!(request.method(), Method::POST);
        assert_eq!(request.uri().path(), "/api/v1/namespaces/shop/services");
        let body = request
            .into_body()
            .collect_bytes()
            .await
            .expect("mock Service request body should be collectable");
        assert_service_request_body(&body);

        send.send_response(
            Response::builder()
                .status(201)
                .body(Body::from(
                    serde_json::to_vec(&sandbox_service())
                        .expect("sandbox Service response should serialize"),
                ))
                .expect("mock Service create response should build"),
        );
    })
}

fn assert_deployment_request_body(body: &[u8]) {
    let actual: Deployment =
        serde_json::from_slice(body).expect("mock Deployment request body should deserialize");
    let expected = sandbox_deployment();

    assert_eq!(actual.metadata.name, expected.metadata.name);
    assert_eq!(actual.metadata.namespace, expected.metadata.namespace);
    assert_eq!(actual.metadata.labels, expected.metadata.labels);
    assert_eq!(
        actual.spec.as_ref().and_then(|spec| spec.replicas),
        expected.spec.as_ref().and_then(|spec| spec.replicas)
    );

    let actual_template = actual
        .spec
        .as_ref()
        .and_then(|spec| spec.template.spec.as_ref())
        .expect("actual Deployment should include a pod template spec");
    let expected_template = expected
        .spec
        .as_ref()
        .and_then(|spec| spec.template.spec.as_ref())
        .expect("expected Deployment should include a pod template spec");

    assert_eq!(
        actual_template
            .containers
            .iter()
            .map(|container| (&container.name, &container.image))
            .collect::<Vec<_>>(),
        expected_template
            .containers
            .iter()
            .map(|container| (&container.name, &container.image))
            .collect::<Vec<_>>()
    );
}

fn assert_service_request_body(body: &[u8]) {
    let actual: Service =
        serde_json::from_slice(body).expect("mock Service request body should deserialize");
    let expected = sandbox_service();

    assert_eq!(actual.metadata.name, expected.metadata.name);
    assert_eq!(actual.metadata.namespace, expected.metadata.namespace);
    assert_eq!(actual.metadata.labels, expected.metadata.labels);
    assert_eq!(
        actual.spec.as_ref().and_then(|spec| spec.type_.clone()),
        expected.spec.as_ref().and_then(|spec| spec.type_.clone())
    );
    assert_eq!(
        actual.spec.as_ref().and_then(|spec| spec.selector.clone()),
        expected
            .spec
            .as_ref()
            .and_then(|spec| spec.selector.clone())
    );
    assert_eq!(
        actual
            .spec
            .as_ref()
            .map(|spec| {
                spec.ports
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|port| (&port.name, port.port, &port.target_port))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        expected
            .spec
            .as_ref()
            .map(|spec| {
                spec.ports
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .map(|port| (&port.name, port.port, &port.target_port))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    );
}

async fn wait_for_mock_kubernetes_api(server: JoinHandle<()>) {
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .expect("mock Kubernetes API should receive all expected requests")
        .expect("mock Kubernetes API task should complete");
}
