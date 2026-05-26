//! Offline integration tests for Kubernetes mutation adapters.

use std::collections::BTreeMap;
use std::time::Duration;

use http::{Method, Request, Response};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Service;
use kube::Client;
use kube::client::Body;
use serde_json::{json, to_vec};
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
async fn patches_deployment_annotations_with_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_deployment_annotation_patch_api(handle);
    let annotations = session_state_annotations("active");

    let summary = kply_k8s::patch_deployment_annotations(
        client,
        "shop",
        "checkout-plan-workload",
        &annotations,
    )
    .await
    .expect("mocked Deployment annotation patch should succeed");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(summary.namespace, "shop");
    assert_eq!(summary.name, "checkout-plan-workload");
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

#[tokio::test]
async fn patches_service_annotations_with_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_service_annotation_patch_api(handle);
    let annotations = session_state_annotations("active");

    let summary =
        kply_k8s::patch_service_annotations(client, "shop", "checkout-plan-service", &annotations)
            .await
            .expect("mocked Service annotation patch should succeed");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(summary.namespace, "shop");
    assert_eq!(summary.name, "checkout-plan-service");
}

#[tokio::test]
async fn runs_session_create_lifecycle_with_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_session_create_lifecycle_api(handle);
    let preparing_annotations = session_state_annotations("preparing");
    let active_annotations = session_state_annotations("active");

    let deployment = kply_k8s::create_deployment(client.clone(), "shop", &sandbox_deployment())
        .await
        .expect("mocked session Deployment create should succeed");
    let service = kply_k8s::create_service(client.clone(), "shop", &sandbox_service())
        .await
        .expect("mocked session Service create should succeed");
    let first_readiness =
        kply_k8s::get_deployment(client.clone(), "shop", "checkout-plan-workload")
            .await
            .expect("mocked first readiness check should succeed");
    let final_readiness =
        kply_k8s::get_deployment(client.clone(), "shop", "checkout-plan-workload")
            .await
            .expect("mocked final readiness check should succeed");
    let prepared_deployment = kply_k8s::patch_deployment_annotations(
        client.clone(),
        "shop",
        "checkout-plan-workload",
        &preparing_annotations,
    )
    .await
    .expect("mocked preparing Deployment state patch should succeed");
    let prepared_service = kply_k8s::patch_service_annotations(
        client.clone(),
        "shop",
        "checkout-plan-service",
        &preparing_annotations,
    )
    .await
    .expect("mocked preparing Service state patch should succeed");
    let active_deployment = kply_k8s::patch_deployment_annotations(
        client.clone(),
        "shop",
        "checkout-plan-workload",
        &active_annotations,
    )
    .await
    .expect("mocked active Deployment state patch should succeed");
    let active_service = kply_k8s::patch_service_annotations(
        client,
        "shop",
        "checkout-plan-service",
        &active_annotations,
    )
    .await
    .expect("mocked active Service state patch should succeed");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(deployment.name, "checkout-plan-workload");
    assert_eq!(service.name, "checkout-plan-service");
    assert_eq!(
        first_readiness.rollout.phase,
        kply_k8s::DeploymentRolloutPhase::Progressing
    );
    assert_eq!(
        final_readiness.rollout.phase,
        kply_k8s::DeploymentRolloutPhase::Complete
    );
    assert_eq!(prepared_deployment.name, "checkout-plan-workload");
    assert_eq!(prepared_service.name, "checkout-plan-service");
    assert_eq!(active_deployment.name, "checkout-plan-workload");
    assert_eq!(active_service.name, "checkout-plan-service");
}

#[tokio::test]
async fn deletes_session_resources_with_label_selectors_from_mocked_kubernetes_api() {
    let (client, handle) = mock_client();
    let server = spawn_mock_session_cleanup_api(handle);

    let deleted_resources = kply_k8s::delete_session_resources(client, "shop", "checkout-plan")
        .await
        .expect("mocked session cleanup should succeed");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(deleted_resources.len(), 2);
    assert_eq!(deleted_resources[0].kind, "Service");
    assert_eq!(deleted_resources[0].namespace, "shop");
    assert_eq!(deleted_resources[0].name, "checkout-plan-service");
    assert_eq!(deleted_resources[1].kind, "Deployment");
    assert_eq!(deleted_resources[1].namespace, "shop");
    assert_eq!(deleted_resources[1].name, "checkout-plan-workload");
}

#[tokio::test]
async fn lists_session_cleanup_resources_without_deleting_them() {
    let (client, handle) = mock_client();
    let server = spawn_mock_session_cleanup_dry_run_api(handle);

    let resources = kply_k8s::list_session_cleanup_resources(client, "shop", "checkout-plan")
        .await
        .expect("mocked session cleanup dry-run should succeed");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(resources.len(), 2);
    assert_eq!(resources[0].kind, "Service");
    assert_eq!(resources[0].name, "checkout-plan-service");
    assert_eq!(resources[1].kind, "Deployment");
    assert_eq!(resources[1].name, "checkout-plan-workload");
}

#[tokio::test]
async fn treats_session_cleanup_delete_not_found_as_already_deleted() {
    let (client, handle) = mock_client();
    let server = spawn_mock_session_cleanup_not_found_api(handle);

    let deleted_resources = kply_k8s::delete_session_resources(client, "shop", "checkout-plan")
        .await
        .expect("mocked session cleanup should treat NotFound as already deleted");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(deleted_resources.len(), 2);
    assert_eq!(deleted_resources[0].kind, "Service");
    assert_eq!(deleted_resources[0].name, "checkout-plan-service");
    assert_eq!(deleted_resources[1].kind, "Deployment");
    assert_eq!(deleted_resources[1].name, "checkout-plan-workload");
}

#[tokio::test]
async fn preserves_partial_session_cleanup_progress_on_later_delete_failure() {
    let (client, handle) = mock_client();
    let server = spawn_mock_partial_session_cleanup_failure_api(handle);

    let error = kply_k8s::delete_session_resources(client, "shop", "checkout-plan")
        .await
        .expect_err("mocked partial cleanup should fail");

    wait_for_mock_kubernetes_api(server).await;

    assert_eq!(error.deletion_accepted_resources.len(), 1);
    assert_eq!(error.deletion_accepted_resources[0].kind, "Service");
    assert_eq!(
        error.deletion_accepted_resources[0].name,
        "checkout-plan-service"
    );
    assert_eq!(error.pending_resources.len(), 1);
    assert_eq!(error.pending_resources[0].kind, "Deployment");
    assert_eq!(error.pending_resources[0].name, "checkout-plan-workload");
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

fn sandbox_progressing_deployment() -> Deployment {
    sandbox_deployment_with_rollout(2, 1, 1, 1, 1, 0, 1)
}

fn sandbox_complete_deployment() -> Deployment {
    sandbox_deployment_with_rollout(2, 2, 1, 1, 1, 1, 0)
}

fn sandbox_deployment_with_rollout(
    generation: i64,
    observed_generation: i64,
    replicas: i32,
    ready_replicas: i32,
    available_replicas: i32,
    updated_replicas: i32,
    unavailable_replicas: i32,
) -> Deployment {
    let mut deployment = serde_json::to_value(sandbox_deployment())
        .expect("sandbox Deployment fixture should serialize");
    deployment["metadata"]["generation"] = json!(generation);
    deployment["status"] = json!({
        "observedGeneration": observed_generation,
        "replicas": replicas,
        "readyReplicas": ready_replicas,
        "availableReplicas": available_replicas,
        "updatedReplicas": updated_replicas,
        "unavailableReplicas": unavailable_replicas
    });

    serde_json::from_value(deployment)
        .expect("sandbox Deployment rollout fixture should deserialize")
}

fn session_state_annotations(status: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("kply.dev/session-status".to_owned(), status.to_owned())])
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
                    to_vec(&sandbox_deployment())
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
                    to_vec(&sandbox_deployment())
                        .expect("sandbox Deployment response should serialize"),
                ))
                .expect("mock Deployment get response should build"),
        );
    })
}

fn spawn_mock_deployment_annotation_patch_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);
        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Deployment patch request");

        assert_eq!(request.method(), Method::PATCH);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments/checkout-plan-workload"
        );
        let body = request
            .into_body()
            .collect_bytes()
            .await
            .expect("mock Deployment patch request body should be collectable");
        assert_annotation_patch_body(&body, "active");

        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&sandbox_deployment())
                        .expect("sandbox Deployment response should serialize"),
                ))
                .expect("mock Deployment patch response should build"),
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
                    to_vec(&sandbox_service()).expect("sandbox Service response should serialize"),
                ))
                .expect("mock Service create response should build"),
        );
    })
}

fn spawn_mock_service_annotation_patch_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);
        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Service patch request");

        assert_eq!(request.method(), Method::PATCH);
        assert_eq!(
            request.uri().path(),
            "/api/v1/namespaces/shop/services/checkout-plan-service"
        );
        let body = request
            .into_body()
            .collect_bytes()
            .await
            .expect("mock Service patch request body should be collectable");
        assert_annotation_patch_body(&body, "active");

        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&sandbox_service()).expect("sandbox Service response should serialize"),
                ))
                .expect("mock Service patch response should build"),
        );
    })
}

fn spawn_mock_session_create_lifecycle_api(handle: MockKubeHandle) -> JoinHandle<()> {
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
                    to_vec(&sandbox_deployment())
                        .expect("sandbox Deployment response should serialize"),
                ))
                .expect("mock Deployment create response should build"),
        );

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
                    to_vec(&sandbox_service()).expect("sandbox Service response should serialize"),
                ))
                .expect("mock Service create response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive first Deployment readiness request");
        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments/checkout-plan-workload"
        );
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&sandbox_progressing_deployment())
                        .expect("progressing Deployment response should serialize"),
                ))
                .expect("mock first Deployment readiness response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive final Deployment readiness request");
        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments/checkout-plan-workload"
        );
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&sandbox_complete_deployment())
                        .expect("complete Deployment response should serialize"),
                ))
                .expect("mock final Deployment readiness response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive preparing Deployment patch request");
        assert_eq!(request.method(), Method::PATCH);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments/checkout-plan-workload"
        );
        let body = request
            .into_body()
            .collect_bytes()
            .await
            .expect("mock preparing Deployment patch body should be collectable");
        assert_annotation_patch_body(&body, "preparing");
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&sandbox_complete_deployment())
                        .expect("prepared Deployment response should serialize"),
                ))
                .expect("mock preparing Deployment patch response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive preparing Service patch request");
        assert_eq!(request.method(), Method::PATCH);
        assert_eq!(
            request.uri().path(),
            "/api/v1/namespaces/shop/services/checkout-plan-service"
        );
        let body = request
            .into_body()
            .collect_bytes()
            .await
            .expect("mock preparing Service patch body should be collectable");
        assert_annotation_patch_body(&body, "preparing");
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&sandbox_service()).expect("prepared Service response should serialize"),
                ))
                .expect("mock preparing Service patch response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive active Deployment patch request");
        assert_eq!(request.method(), Method::PATCH);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments/checkout-plan-workload"
        );
        let body = request
            .into_body()
            .collect_bytes()
            .await
            .expect("mock active Deployment patch body should be collectable");
        assert_annotation_patch_body(&body, "active");
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&sandbox_complete_deployment())
                        .expect("active Deployment response should serialize"),
                ))
                .expect("mock active Deployment patch response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive active Service patch request");
        assert_eq!(request.method(), Method::PATCH);
        assert_eq!(
            request.uri().path(),
            "/api/v1/namespaces/shop/services/checkout-plan-service"
        );
        let body = request
            .into_body()
            .collect_bytes()
            .await
            .expect("mock active Service patch body should be collectable");
        assert_annotation_patch_body(&body, "active");
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&sandbox_service()).expect("active Service response should serialize"),
                ))
                .expect("mock active Service patch response should build"),
        );
    })
}

fn spawn_mock_session_cleanup_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Service list request");
        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.uri().path(), "/api/v1/namespaces/shop/services");
        assert_session_cleanup_label_selector(&request);
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&session_cleanup_service_list_fixture())
                        .expect("session cleanup Service list response should serialize"),
                ))
                .expect("mock Service list response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Deployment list request");
        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments"
        );
        assert_session_cleanup_label_selector(&request);
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&session_cleanup_deployment_list_fixture())
                        .expect("session cleanup Deployment list response should serialize"),
                ))
                .expect("mock Deployment list response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Service delete request");
        assert_eq!(request.method(), Method::DELETE);
        assert_eq!(
            request.uri().path(),
            "/api/v1/namespaces/shop/services/checkout-plan-service"
        );
        assert_session_cleanup_background_delete(request).await;
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&session_cleanup_service())
                        .expect("session cleanup Service response should serialize"),
                ))
                .expect("mock Service delete response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Deployment delete request");
        assert_eq!(request.method(), Method::DELETE);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments/checkout-plan-workload"
        );
        assert_session_cleanup_background_delete(request).await;
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(to_vec(&session_cleanup_deployment()).expect(
                    "session cleanup Deployment response should serialize",
                )))
                .expect("mock Deployment delete response should build"),
        );
    })
}

fn spawn_mock_session_cleanup_dry_run_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Service list request");
        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.uri().path(), "/api/v1/namespaces/shop/services");
        assert_session_cleanup_label_selector(&request);
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&session_cleanup_service_list_fixture())
                        .expect("session cleanup Service list response should serialize"),
                ))
                .expect("mock Service list response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Deployment list request");
        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments"
        );
        assert_session_cleanup_label_selector(&request);
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&session_cleanup_deployment_list_fixture())
                        .expect("session cleanup Deployment list response should serialize"),
                ))
                .expect("mock Deployment list response should build"),
        );
    })
}

fn spawn_mock_session_cleanup_not_found_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Service list request");
        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.uri().path(), "/api/v1/namespaces/shop/services");
        assert_session_cleanup_label_selector(&request);
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&session_cleanup_service_list_fixture())
                        .expect("session cleanup Service list response should serialize"),
                ))
                .expect("mock Service list response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Deployment list request");
        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments"
        );
        assert_session_cleanup_label_selector(&request);
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&session_cleanup_deployment_list_fixture())
                        .expect("session cleanup Deployment list response should serialize"),
                ))
                .expect("mock Deployment list response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Service delete request");
        assert_eq!(request.method(), Method::DELETE);
        assert_eq!(
            request.uri().path(),
            "/api/v1/namespaces/shop/services/checkout-plan-service"
        );
        assert_session_cleanup_background_delete(request).await;
        send.send_response(
            Response::builder()
                .status(404)
                .body(Body::from(
                    to_vec(&json!({
                        "apiVersion": "v1",
                        "kind": "Status",
                        "status": "Failure",
                        "reason": "NotFound",
                        "message": "services \"checkout-plan-service\" not found",
                        "code": 404
                    }))
                    .expect("mock Status response should serialize"),
                ))
                .expect("mock Service delete NotFound response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Deployment delete request");
        assert_eq!(request.method(), Method::DELETE);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments/checkout-plan-workload"
        );
        assert_session_cleanup_background_delete(request).await;
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(to_vec(&session_cleanup_deployment()).expect(
                    "session cleanup Deployment response should serialize",
                )))
                .expect("mock Deployment delete response should build"),
        );
    })
}

fn spawn_mock_partial_session_cleanup_failure_api(handle: MockKubeHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut handle = std::pin::pin!(handle);

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Service list request");
        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.uri().path(), "/api/v1/namespaces/shop/services");
        assert_session_cleanup_label_selector(&request);
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&session_cleanup_service_list_fixture())
                        .expect("session cleanup Service list response should serialize"),
                ))
                .expect("mock Service list response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Deployment list request");
        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments"
        );
        assert_session_cleanup_label_selector(&request);
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&session_cleanup_deployment_list_fixture())
                        .expect("session cleanup Deployment list response should serialize"),
                ))
                .expect("mock Deployment list response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Service delete request");
        assert_eq!(request.method(), Method::DELETE);
        assert_eq!(
            request.uri().path(),
            "/api/v1/namespaces/shop/services/checkout-plan-service"
        );
        assert_session_cleanup_background_delete(request).await;
        send.send_response(
            Response::builder()
                .status(200)
                .body(Body::from(
                    to_vec(&session_cleanup_service())
                        .expect("session cleanup Service response should serialize"),
                ))
                .expect("mock Service delete response should build"),
        );

        let (request, send) = handle
            .next_request()
            .await
            .expect("mock Kubernetes API should receive Deployment delete request");
        assert_eq!(request.method(), Method::DELETE);
        assert_eq!(
            request.uri().path(),
            "/apis/apps/v1/namespaces/shop/deployments/checkout-plan-workload"
        );
        assert_session_cleanup_background_delete(request).await;
        send.send_response(
            Response::builder()
                .status(500)
                .body(Body::from(
                    to_vec(&json!({
                        "apiVersion": "v1",
                        "kind": "Status",
                        "status": "Failure",
                        "reason": "InternalError",
                        "message": "mock delete failed",
                        "code": 500
                    }))
                    .expect("mock Status response should serialize"),
                ))
                .expect("mock Deployment delete failure response should build"),
        );
    })
}

fn assert_session_cleanup_label_selector(request: &Request<Body>) {
    let query = request
        .uri()
        .query()
        .expect("session cleanup list should include a label selector");
    assert!(
        query.contains("kply.dev%2Fmanaged-by%3Dkply"),
        "session cleanup should filter by Kply ownership"
    );
    assert!(
        query.contains("kply.dev%2Fsession-id%3Dcheckout-plan"),
        "session cleanup should filter by session id"
    );
}

async fn assert_session_cleanup_background_delete(request: Request<Body>) {
    let query = request.uri().query().unwrap_or_default().to_owned();
    let body = request
        .into_body()
        .collect_bytes()
        .await
        .expect("session cleanup delete request body should be collectable");
    let body = String::from_utf8(body.to_vec())
        .expect("session cleanup delete request body should be UTF-8");
    assert!(
        query.contains("propagationPolicy=Background")
            || body.contains(r#""propagationPolicy":"Background""#),
        "session cleanup deletes should use background propagation"
    );
}

fn session_cleanup_service_list_fixture() -> serde_json::Value {
    json!({
        "apiVersion": "v1",
        "kind": "ServiceList",
        "items": [session_cleanup_service()]
    })
}

fn session_cleanup_deployment_list_fixture() -> serde_json::Value {
    json!({
        "apiVersion": "apps/v1",
        "kind": "DeploymentList",
        "items": [session_cleanup_deployment()]
    })
}

fn session_cleanup_service() -> Service {
    let mut service = sandbox_service();
    service.metadata.labels = Some(BTreeMap::from([
        ("kply.dev/managed-by".to_owned(), "kply".to_owned()),
        ("kply.dev/session-id".to_owned(), "checkout-plan".to_owned()),
    ]));
    service
}

fn session_cleanup_deployment() -> Deployment {
    let mut deployment = sandbox_deployment();
    deployment.metadata.labels = Some(BTreeMap::from([
        ("kply.dev/managed-by".to_owned(), "kply".to_owned()),
        ("kply.dev/session-id".to_owned(), "checkout-plan".to_owned()),
    ]));
    deployment
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

fn assert_annotation_patch_body(body: &[u8], status: &str) {
    let actual: serde_json::Value =
        serde_json::from_slice(body).expect("mock annotation patch body should deserialize");

    assert_eq!(
        actual["metadata"]["annotations"]["kply.dev/session-status"],
        status
    );
}

async fn wait_for_mock_kubernetes_api(server: JoinHandle<()>) {
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .expect("mock Kubernetes API should receive all expected requests")
        .expect("mock Kubernetes API task should complete");
}
