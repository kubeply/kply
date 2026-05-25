//! Optional live-cluster integration tests for read-only Kubernetes discovery.

use kube::Client;

const LIVE_TESTS_ENV: &str = "KPLY_LIVE_K8S_TESTS";
const LIVE_NAMESPACE_ENV: &str = "KPLY_LIVE_K8S_NAMESPACE";

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
