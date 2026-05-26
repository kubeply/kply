//! Optional live-cluster integration tests for Kubernetes adapters.

use std::collections::BTreeMap;
use std::error::Error;

use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{Namespace, Service};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{
    Api, Client,
    api::{DeleteParams, PostParams},
};

const LIVE_TESTS_ENV: &str = "KPLY_LIVE_K8S_TESTS";
const LIVE_NAMESPACE_ENV: &str = "KPLY_LIVE_K8S_NAMESPACE";
const LIVE_KIND_TESTS_ENV: &str = "KPLY_LIVE_KIND_TESTS";
const LIVE_KIND_NAMESPACE_ENV: &str = "KPLY_LIVE_KIND_NAMESPACE";
const LIVE_KIND_NAMESPACE_DEFAULT: &str = "kply-live-kind";
const LIVE_KIND_SESSION_ID: &str = "kply-live-kind-session";
const LIVE_KIND_DEPLOYMENT_NAME: &str = "kply-live-kind-workload";
const LIVE_KIND_SERVICE_NAME: &str = "kply-live-kind-service";

#[tokio::test]
async fn live_cluster_read_only_discovery_lists_core_resources_when_enabled() {
    let Some(namespace) = live_test_namespace() else {
        return;
    };

    let client = Client::try_default()
        .await
        .expect("live Kubernetes test should resolve kubeconfig when enabled");

    let deployments = kply_k8s::list_deployments(client.clone(), &namespace)
        .await
        .expect("live Kubernetes test should list Deployments with read-only access");
    let services = kply_k8s::list_services(client, &namespace)
        .await
        .expect("live Kubernetes test should list Services with read-only access");

    eprintln!(
        "discovered {} deployment(s) and {} service(s) in namespace {namespace}",
        deployments.len(),
        services.len()
    );

    assert!(
        deployments
            .iter()
            .all(|deployment| deployment.namespace == namespace),
        "live Deployment summaries should stay scoped to {namespace}"
    );
    assert!(
        services
            .iter()
            .all(|service| service.namespace == namespace),
        "live Service summaries should stay scoped to {namespace}"
    );
}

fn live_test_namespace() -> Option<String> {
    if std::env::var_os(LIVE_TESTS_ENV).as_deref() != Some(std::ffi::OsStr::new("1")) {
        return None;
    }

    Some(std::env::var(LIVE_NAMESPACE_ENV).unwrap_or_else(|_| "default".to_owned()))
}

#[tokio::test]
async fn live_kind_session_resources_create_patch_list_and_cleanup_when_enabled() {
    let Some(namespace) = live_kind_test_namespace() else {
        return;
    };

    let client = Client::try_default()
        .await
        .expect("live Kind test should resolve kubeconfig when enabled");

    ensure_namespace(client.clone(), &namespace)
        .await
        .expect("live Kind test should create or reuse its namespace");
    let result = run_live_kind_session_resource_flow(client.clone(), &namespace).await;
    delete_namespace(client, &namespace).await;

    result.expect("live Kind session resource flow should complete");
}

async fn run_live_kind_session_resource_flow(
    client: Client,
    namespace: &str,
) -> Result<(), Box<dyn Error>> {
    let _ =
        kply_k8s::delete_session_resources(client.clone(), namespace, LIVE_KIND_SESSION_ID).await;

    let deployment = kply_k8s::create_deployment(
        client.clone(),
        namespace,
        &kind_session_deployment(namespace),
    )
    .await?;
    let service =
        kply_k8s::create_service(client.clone(), namespace, &kind_session_service(namespace))
            .await?;
    let annotations = session_state_annotations("active");

    let patched_deployment = kply_k8s::patch_deployment_annotations(
        client.clone(),
        namespace,
        LIVE_KIND_DEPLOYMENT_NAME,
        &annotations,
    )
    .await?;
    let patched_service = kply_k8s::patch_service_annotations(
        client.clone(),
        namespace,
        LIVE_KIND_SERVICE_NAME,
        &annotations,
    )
    .await?;
    let cleanup_candidates =
        kply_k8s::list_session_cleanup_resources(client.clone(), namespace, LIVE_KIND_SESSION_ID)
            .await?;
    let deleted_resources =
        kply_k8s::delete_session_resources(client.clone(), namespace, LIVE_KIND_SESSION_ID).await?;

    assert_eq!(deployment.name, LIVE_KIND_DEPLOYMENT_NAME);
    assert_eq!(service.name, LIVE_KIND_SERVICE_NAME);
    assert_eq!(patched_deployment.name, LIVE_KIND_DEPLOYMENT_NAME);
    assert_eq!(patched_service.name, LIVE_KIND_SERVICE_NAME);
    assert_eq!(
        cleanup_candidates
            .iter()
            .map(|resource| (resource.kind.as_str(), resource.name.as_str()))
            .collect::<Vec<_>>(),
        [
            ("Service", LIVE_KIND_SERVICE_NAME),
            ("Deployment", LIVE_KIND_DEPLOYMENT_NAME)
        ]
    );
    assert_eq!(deleted_resources, cleanup_candidates);

    Ok(())
}

fn live_kind_test_namespace() -> Option<String> {
    if std::env::var_os(LIVE_KIND_TESTS_ENV).as_deref() != Some(std::ffi::OsStr::new("1")) {
        return None;
    }

    Some(
        std::env::var(LIVE_KIND_NAMESPACE_ENV)
            .unwrap_or_else(|_| LIVE_KIND_NAMESPACE_DEFAULT.to_owned()),
    )
}

async fn ensure_namespace(client: Client, name: &str) -> Result<(), kube::Error> {
    let namespaces: Api<Namespace> = Api::all(client);
    let namespace = Namespace {
        metadata: ObjectMeta {
            name: Some(name.to_owned()),
            labels: Some(BTreeMap::from([(
                "kply.dev/live-test".to_owned(),
                "kind".to_owned(),
            )])),
            ..ObjectMeta::default()
        },
        ..Namespace::default()
    };

    match namespaces.create(&PostParams::default(), &namespace).await {
        Ok(_) => Ok(()),
        Err(kube::Error::Api(error)) if error.code == 409 => Ok(()),
        Err(error) => Err(error),
    }
}

async fn delete_namespace(client: Client, name: &str) {
    let namespaces: Api<Namespace> = Api::all(client);
    if let Err(error) = namespaces.delete(name, &DeleteParams::default()).await
        && !matches!(&error, kube::Error::Api(status) if status.code == 404)
    {
        eprintln!("failed to delete live Kind test namespace {name}: {error}");
    }
}

fn kind_session_deployment(namespace: &str) -> Deployment {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": LIVE_KIND_DEPLOYMENT_NAME,
            "namespace": namespace,
            "labels": session_labels()
        },
        "spec": {
            "replicas": 0,
            "selector": {
                "matchLabels": {
                    "app.kubernetes.io/name": LIVE_KIND_DEPLOYMENT_NAME
                }
            },
            "template": {
                "metadata": {
                    "labels": {
                        "app.kubernetes.io/name": LIVE_KIND_DEPLOYMENT_NAME
                    }
                },
                "spec": {
                    "containers": [
                        {
                            "name": "app",
                            "image": "registry.k8s.io/pause:3.10"
                        }
                    ]
                }
            }
        }
    }))
    .expect("live Kind Deployment fixture should deserialize")
}

fn kind_session_service(namespace: &str) -> Service {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": {
            "name": LIVE_KIND_SERVICE_NAME,
            "namespace": namespace,
            "labels": session_labels()
        },
        "spec": {
            "type": "ClusterIP",
            "selector": {
                "app.kubernetes.io/name": LIVE_KIND_DEPLOYMENT_NAME
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
    .expect("live Kind Service fixture should deserialize")
}

fn session_labels() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("kply.dev/managed-by".to_owned(), "kply".to_owned()),
        (
            "kply.dev/session-id".to_owned(),
            LIVE_KIND_SESSION_ID.to_owned(),
        ),
    ])
}

fn session_state_annotations(status: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("kply.dev/session-status".to_owned(), status.to_owned())])
}
