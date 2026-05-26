//! Offline integration tests for Kubernetes mutation adapters.

use std::time::Duration;

use http::{Method, Request, Response};
use k8s_openapi::api::apps::v1::Deployment;
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

async fn wait_for_mock_kubernetes_api(server: JoinHandle<()>) {
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .expect("mock Kubernetes API should receive all expected requests")
        .expect("mock Kubernetes API task should complete");
}
