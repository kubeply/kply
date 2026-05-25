//! Kubernetes adapters for future safe session execution.

use std::path::Path;

use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Service;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
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

/// Read-only summary of a Kubernetes Service.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ServiceSummary {
    /// Service namespace.
    pub namespace: String,
    /// Service name.
    pub name: String,
    /// Service type, such as `ClusterIP`, `NodePort`, or `LoadBalancer`.
    pub service_type: Option<String>,
    /// Service selector labels in deterministic key order.
    pub selector: Vec<LabelSelectorEntry>,
    /// Declared Service ports in manifest order.
    pub ports: Vec<ServicePortSummary>,
}

/// One Service selector label.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LabelSelectorEntry {
    /// Selector label key.
    pub key: String,
    /// Selector label value.
    pub value: String,
}

/// Read-only summary of one Kubernetes Service port.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ServicePortSummary {
    /// Optional Service port name.
    pub name: Option<String>,
    /// Exposed Service port.
    pub port: i32,
    /// Optional app protocol for the Service port.
    pub app_protocol: Option<String>,
    /// Transport protocol, usually `TCP`, `UDP`, or `SCTP`.
    pub protocol: Option<String>,
    /// Target port as a string, preserving named target ports.
    pub target_port: Option<String>,
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

/// List Services in one namespace without mutating cluster state.
///
/// # Errors
///
/// Returns [`kube::Error`] when the Kubernetes API request fails.
pub async fn list_services(
    client: Client,
    namespace: &str,
) -> Result<Vec<ServiceSummary>, kube::Error> {
    let services: Api<Service> = Api::namespaced(client, namespace);
    let mut summaries = services
        .list(&ListParams::default())
        .await?
        .iter()
        .map(service_summary)
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

/// Convert a Kubernetes [`Service`] into a deterministic summary.
pub fn service_summary(service: &Service) -> ServiceSummary {
    let spec = service.spec.as_ref();
    let selector = spec
        .and_then(|spec| spec.selector.as_ref())
        .map(|selector| {
            selector
                .iter()
                .map(|(key, value)| LabelSelectorEntry {
                    key: key.clone(),
                    value: value.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let ports = spec
        .map(|spec| {
            spec.ports
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|port| ServicePortSummary {
                    name: port.name.clone(),
                    port: port.port,
                    app_protocol: port.app_protocol.clone(),
                    protocol: port.protocol.clone(),
                    target_port: port.target_port.as_ref().map(format_int_or_string),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    ServiceSummary {
        namespace: service.namespace().unwrap_or_default(),
        name: service.name_any(),
        service_type: spec.and_then(|spec| spec.type_.clone()),
        selector,
        ports,
    }
}

fn format_int_or_string(value: &IntOrString) -> String {
    match value {
        IntOrString::Int(value) => value.to_string(),
        IntOrString::String(value) => value.clone(),
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
        ClusterInfo, DeploymentSummary, LabelSelectorEntry, ServicePortSummary, ServiceSummary,
        deployment_summary, load_kube_config_path, load_kube_config_with_options, service_summary,
    };
    use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec, DeploymentStatus};
    use k8s_openapi::api::core::v1::{
        Container, PodSpec, PodTemplateSpec, Service, ServicePort, ServiceSpec,
    };
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
    use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
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

    #[test]
    fn summarizes_service_selector_and_ports() {
        let service = fake_service(
            "shop",
            "checkout-http",
            [("app", "checkout"), ("tier", "backend")],
            vec![
                ServicePort {
                    name: Some("http".to_owned()),
                    port: 80,
                    app_protocol: Some("http".to_owned()),
                    protocol: Some("TCP".to_owned()),
                    target_port: Some(IntOrString::String("web".to_owned())),
                    ..ServicePort::default()
                },
                ServicePort {
                    name: Some("metrics".to_owned()),
                    port: 9090,
                    protocol: Some("TCP".to_owned()),
                    target_port: Some(IntOrString::Int(9091)),
                    ..ServicePort::default()
                },
            ],
        );

        let summary = service_summary(&service);

        assert_eq!(
            summary,
            ServiceSummary {
                namespace: "shop".to_owned(),
                name: "checkout-http".to_owned(),
                service_type: Some("ClusterIP".to_owned()),
                selector: vec![
                    LabelSelectorEntry {
                        key: "app".to_owned(),
                        value: "checkout".to_owned(),
                    },
                    LabelSelectorEntry {
                        key: "tier".to_owned(),
                        value: "backend".to_owned(),
                    },
                ],
                ports: vec![
                    ServicePortSummary {
                        name: Some("http".to_owned()),
                        port: 80,
                        app_protocol: Some("http".to_owned()),
                        protocol: Some("TCP".to_owned()),
                        target_port: Some("web".to_owned()),
                    },
                    ServicePortSummary {
                        name: Some("metrics".to_owned()),
                        port: 9090,
                        app_protocol: None,
                        protocol: Some("TCP".to_owned()),
                        target_port: Some("9091".to_owned()),
                    },
                ],
            }
        );
    }

    #[test]
    fn summarizes_minimal_service_without_optional_fields() {
        let service = Service {
            metadata: ObjectMeta {
                name: Some("minimal".to_owned()),
                ..ObjectMeta::default()
            },
            ..Service::default()
        };

        let summary = service_summary(&service);

        assert_eq!(
            summary,
            ServiceSummary {
                namespace: String::new(),
                name: "minimal".to_owned(),
                service_type: None,
                selector: Vec::new(),
                ports: Vec::new(),
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

    fn fake_service<const N: usize>(
        namespace: &str,
        name: &str,
        selector: [(&str, &str); N],
        ports: Vec<ServicePort>,
    ) -> Service {
        Service {
            metadata: ObjectMeta {
                name: Some(name.to_owned()),
                namespace: Some(namespace.to_owned()),
                ..ObjectMeta::default()
            },
            spec: Some(ServiceSpec {
                selector: Some(
                    selector
                        .into_iter()
                        .map(|(key, value)| (key.to_owned(), value.to_owned()))
                        .collect(),
                ),
                ports: Some(ports),
                type_: Some("ClusterIP".to_owned()),
                ..ServiceSpec::default()
            }),
            ..Service::default()
        }
    }
}
