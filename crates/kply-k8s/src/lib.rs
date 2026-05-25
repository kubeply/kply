//! Kubernetes adapters for future safe session execution.

use std::path::Path;

use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    Api, Client, Config, ResourceExt,
    api::ListParams,
    config::{KubeConfigOptions, Kubeconfig, KubeconfigError},
};
use serde::Serialize;

/// Read-only Kubernetes cluster facts resolved from kubeconfig.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClusterInfo {
    /// Kubernetes API server URL selected by kubeconfig resolution.
    pub cluster_url: String,
    /// Default namespace selected by the active kubeconfig context.
    pub default_namespace: String,
}

/// Read-only summary of a Kubernetes Deployment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DeploymentSummary {
    /// Deployment namespace.
    pub namespace: String,
    /// Deployment name.
    pub name: String,
    /// Desired replica count from the Deployment spec.
    pub replicas: Option<i32>,
    /// Observed available replicas from Deployment status.
    pub available_replicas: Option<i32>,
    /// Observed ready replicas from Deployment status.
    pub ready_replicas: Option<i32>,
    /// Observed updated replicas from Deployment status.
    pub updated_replicas: Option<i32>,
    /// Declared container images in pod template order.
    pub images: Vec<String>,
}

/// List Deployments in one namespace without mutating cluster state.
///
/// # Errors
///
/// Returns [`kube::Error`] when the Kubernetes API request fails.
pub async fn list_deployments(
    client: Client,
    namespace: &str,
) -> Result<Vec<DeploymentSummary>, kube::Error> {
    let deployments: Api<Deployment> = Api::namespaced(client, namespace);
    let mut summaries = deployments
        .list(&ListParams::default())
        .await?
        .iter()
        .map(deployment_summary)
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(summaries)
}

/// Convert a Kubernetes [`Deployment`] into a deterministic summary.
pub fn deployment_summary(deployment: &Deployment) -> DeploymentSummary {
    let spec = deployment.spec.as_ref();
    let status = deployment.status.as_ref();
    let images = spec
        .and_then(|spec| spec.template.spec.as_ref())
        .map(|pod_spec| {
            pod_spec
                .containers
                .iter()
                .map(|container| container.image.clone().unwrap_or_default())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    DeploymentSummary {
        namespace: deployment.namespace().unwrap_or_default(),
        name: deployment.name_any(),
        replicas: spec.and_then(|spec| spec.replicas),
        available_replicas: status.and_then(|status| status.available_replicas),
        ready_replicas: status.and_then(|status| status.ready_replicas),
        updated_replicas: status.and_then(|status| status.updated_replicas),
        images,
    }
}

impl From<Config> for ClusterInfo {
    fn from(config: Config) -> Self {
        Self {
            cluster_url: config.cluster_url.to_string(),
            default_namespace: config.default_namespace,
        }
    }
}

/// Load read-only cluster facts using standard kubeconfig conventions.
///
/// This resolves kubeconfig locally and does not contact the cluster.
///
/// # Errors
///
/// Returns [`KubeconfigError`] when kube-rs cannot find, read, parse, or
/// resolve the selected kubeconfig.
pub async fn cluster_info() -> Result<ClusterInfo, KubeconfigError> {
    load_kube_config().await.map(ClusterInfo::from)
}

/// Load Kubernetes client config using standard kubeconfig conventions.
///
/// This reads the kubeconfig selected by `KUBECONFIG`, or `~/.kube/config`
/// when `KUBECONFIG` is not set. It does not contact the cluster.
///
/// # Errors
///
/// Returns [`KubeconfigError`] when kube-rs cannot find, read, parse, or
/// resolve the selected kubeconfig.
pub async fn load_kube_config() -> Result<Config, KubeconfigError> {
    load_kube_config_with_options(&KubeConfigOptions::default()).await
}

/// Load Kubernetes client config using explicit kubeconfig selection options.
///
/// This keeps context, cluster, and user selection aligned with kube-rs and
/// Kubernetes client conventions. It does not contact the cluster.
///
/// # Errors
///
/// Returns [`KubeconfigError`] when kube-rs cannot find, read, parse, or
/// resolve the selected kubeconfig.
pub async fn load_kube_config_with_options(
    options: &KubeConfigOptions,
) -> Result<Config, KubeconfigError> {
    Config::from_kubeconfig(options).await
}

/// Load Kubernetes client config from an explicit kubeconfig path.
///
/// This helper is primarily useful for deterministic tests and future CLI
/// paths that need to resolve a known kubeconfig file. It does not contact the
/// cluster.
///
/// # Errors
///
/// Returns [`KubeconfigError`] when kube-rs cannot read, parse, or resolve the
/// kubeconfig at `path`.
pub async fn load_kube_config_path(path: impl AsRef<Path>) -> Result<Config, KubeconfigError> {
    let kubeconfig = Kubeconfig::read_from(path)?;
    Config::from_custom_kubeconfig(kubeconfig, &KubeConfigOptions::default()).await
}

#[cfg(test)]
mod tests {
    use super::{
        ClusterInfo, DeploymentSummary, deployment_summary, load_kube_config_path,
        load_kube_config_with_options,
    };
    use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec, DeploymentStatus};
    use k8s_openapi::api::core::v1::{Container, PodSpec, PodTemplateSpec};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
    use kube::config::KubeConfigOptions;
    use std::collections::BTreeMap;
    use std::env;
    use tokio::sync::Mutex;

    static KUBECONFIG_ENV_LOCK: Mutex<()> = Mutex::const_new(());

    #[tokio::test]
    async fn loads_kube_config_from_explicit_path() {
        let workspace = kply_test::temp_workspace();
        let kubeconfig_path = kply_test::write_fake_kubeconfig(&workspace);

        let config = load_kube_config_path(kubeconfig_path)
            .await
            .expect("fake kubeconfig should load");

        assert_eq!(config.cluster_url.to_string(), "https://127.0.0.1:6443/");
        assert_eq!(config.default_namespace, "default");
    }

    #[test]
    fn creates_cluster_info_from_kube_config() {
        let config = kube::Config::new("https://127.0.0.1:6443".parse().expect("valid URL"));

        let info = ClusterInfo::from(config);

        assert_eq!(info.cluster_url, "https://127.0.0.1:6443/");
        assert_eq!(info.default_namespace, "default");
    }

    #[test]
    fn summarizes_deployment_metadata_and_status() {
        let deployment = fake_deployment("shop", "checkout-api", &["checkout:v2", "sidecar:v1"]);

        let summary = deployment_summary(&deployment);

        assert_eq!(
            summary,
            DeploymentSummary {
                namespace: "shop".to_owned(),
                name: "checkout-api".to_owned(),
                replicas: Some(3),
                available_replicas: Some(2),
                ready_replicas: Some(2),
                updated_replicas: Some(3),
                images: vec!["checkout:v2".to_owned(), "sidecar:v1".to_owned()],
            }
        );
    }

    #[test]
    fn summarizes_minimal_deployment_without_optional_fields() {
        let deployment = Deployment {
            metadata: ObjectMeta {
                name: Some("minimal".to_owned()),
                ..ObjectMeta::default()
            },
            ..Deployment::default()
        };

        let summary = deployment_summary(&deployment);

        assert_eq!(
            summary,
            DeploymentSummary {
                namespace: String::new(),
                name: "minimal".to_owned(),
                replicas: None,
                available_replicas: None,
                ready_replicas: None,
                updated_replicas: None,
                images: Vec::new(),
            }
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn loads_kube_config_with_explicit_context_option() {
        let workspace = kply_test::temp_workspace();
        let kubeconfig_path = kply_test::write_temp_file(
            &workspace,
            "kubeconfig.yaml",
            r#"
apiVersion: v1
kind: Config
clusters:
  - name: cluster-a
    cluster:
      server: https://127.0.0.1:6443
users:
  - name: user-a
    user:
      token: fake-token
contexts:
  - name: context-a
    context:
      cluster: cluster-a
      user: user-a
      namespace: qa
current-context: context-a
"#,
        );
        let options = KubeConfigOptions {
            context: Some("context-a".to_owned()),
            ..KubeConfigOptions::default()
        };
        let _env_lock = KUBECONFIG_ENV_LOCK.lock().await;
        let previous_kubeconfig = env::var_os("KUBECONFIG");

        // SAFETY: environment mutation is serialized by KUBECONFIG_ENV_LOCK and
        // restored before releasing the lock.
        unsafe {
            env::set_var("KUBECONFIG", &kubeconfig_path);
        }
        let result = load_kube_config_with_options(&options).await;
        // SAFETY: restore the process environment to the value captured before
        // this test changed KUBECONFIG while still holding KUBECONFIG_ENV_LOCK.
        unsafe {
            if let Some(previous_kubeconfig) = previous_kubeconfig {
                env::set_var("KUBECONFIG", previous_kubeconfig);
            } else {
                env::remove_var("KUBECONFIG");
            }
        }

        let config = result.expect("fake kubeconfig should resolve");

        assert_eq!(config.default_namespace, "qa");
    }

    #[tokio::test]
    async fn reports_missing_explicit_kube_config_path() {
        let workspace = kply_test::temp_workspace();
        let missing_path = workspace.path().join("missing").join("kubeconfig.yaml");

        let error = load_kube_config_path(missing_path)
            .await
            .expect_err("missing kubeconfig should fail");

        assert!(
            matches!(error, kube::config::KubeconfigError::ReadConfig(_, _)),
            "unexpected kubeconfig error: {error}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn loads_kube_config_from_kubeconfig_environment_variable() {
        let workspace = kply_test::temp_workspace();
        let kubeconfig_path = kply_test::write_fake_kubeconfig(&workspace);
        let _env_lock = KUBECONFIG_ENV_LOCK.lock().await;
        let previous_kubeconfig = env::var_os("KUBECONFIG");

        // SAFETY: environment mutation is serialized by KUBECONFIG_ENV_LOCK and
        // restored before releasing the lock.
        unsafe {
            env::set_var("KUBECONFIG", &kubeconfig_path);
        }
        let result = load_kube_config_with_options(&KubeConfigOptions::default()).await;
        // SAFETY: restore the process environment to the value captured before
        // this test changed KUBECONFIG while still holding KUBECONFIG_ENV_LOCK.
        unsafe {
            if let Some(previous_kubeconfig) = previous_kubeconfig {
                env::set_var("KUBECONFIG", previous_kubeconfig);
            } else {
                env::remove_var("KUBECONFIG");
            }
        }

        let config = result.expect("KUBECONFIG-selected fake kubeconfig should load");

        assert_eq!(config.cluster_url.to_string(), "https://127.0.0.1:6443/");
    }

    fn fake_deployment(namespace: &str, name: &str, images: &[&str]) -> Deployment {
        let labels = BTreeMap::from([("app".to_owned(), name.to_owned())]);

        Deployment {
            metadata: ObjectMeta {
                name: Some(name.to_owned()),
                namespace: Some(namespace.to_owned()),
                ..ObjectMeta::default()
            },
            spec: Some(DeploymentSpec {
                replicas: Some(3),
                selector: LabelSelector {
                    match_labels: Some(labels.clone()),
                    ..LabelSelector::default()
                },
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(labels),
                        ..ObjectMeta::default()
                    }),
                    spec: Some(PodSpec {
                        containers: images
                            .iter()
                            .map(|image| Container {
                                name: image.replace([':', '/'], "-"),
                                image: Some((*image).to_owned()),
                                ..Container::default()
                            })
                            .collect(),
                        ..PodSpec::default()
                    }),
                },
                ..DeploymentSpec::default()
            }),
            status: Some(DeploymentStatus {
                available_replicas: Some(2),
                ready_replicas: Some(2),
                updated_replicas: Some(3),
                ..DeploymentStatus::default()
            }),
        }
    }
}
