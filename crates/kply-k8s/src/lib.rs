//! Kubernetes adapters for future safe session execution.

use std::path::Path;

use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{Container, Pod, Probe, Service};
use k8s_openapi::api::networking::v1::{Ingress, IngressBackend};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kply_core::WorkloadRef;
use kube::{
    Api, Client, Config, ResourceExt,
    api::ListParams,
    config::{KubeConfigOptions, Kubeconfig, KubeconfigError},
    core::{ApiResource, DynamicObject, GroupVersionKind},
};
use serde::Serialize;
use serde_json::Value;

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
    /// Readiness and liveness probes in pod template container order.
    pub probes: Vec<ContainerProbeSummary>,
    /// Resource requests and limits in pod template container order.
    pub resources: Vec<ContainerResourceSummary>,
    /// Basic rollout status derived from Deployment status.
    pub rollout: DeploymentRolloutSummary,
}

/// Basic rollout status for a Kubernetes Deployment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DeploymentRolloutSummary {
    /// Rollout phase derived from observed Deployment status.
    pub phase: DeploymentRolloutPhase,
    /// Desired Deployment generation from object metadata.
    pub generation: Option<i64>,
    /// Observed Deployment generation from status.
    pub observed_generation: Option<i64>,
    /// Desired replica count from the Deployment spec.
    pub desired_replicas: Option<i32>,
    /// Observed ready replicas from Deployment status.
    pub ready_replicas: Option<i32>,
    /// Observed available replicas from Deployment status.
    pub available_replicas: Option<i32>,
    /// Observed updated replicas from Deployment status.
    pub updated_replicas: Option<i32>,
    /// Observed unavailable replicas from Deployment status.
    pub unavailable_replicas: Option<i32>,
    /// Deployment conditions in manifest order.
    pub conditions: Vec<DeploymentConditionSummary>,
}

/// Basic Deployment rollout phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentRolloutPhase {
    /// Status does not contain enough information to classify the rollout.
    Unknown,
    /// Rollout is still converging.
    Progressing,
    /// Rollout has no available replicas.
    Unavailable,
    /// Rollout is fully updated and available.
    Complete,
}

/// Read-only summary of one Deployment condition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DeploymentConditionSummary {
    /// Condition type, such as `Available` or `Progressing`.
    pub type_: String,
    /// Condition status, such as `True`, `False`, or `Unknown`.
    pub status: String,
    /// Optional condition reason.
    pub reason: Option<String>,
    /// Optional condition message.
    pub message: Option<String>,
}

/// Read-only readiness and liveness probe metadata for one container.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContainerProbeSummary {
    /// Container name.
    pub container_name: String,
    /// Readiness probe metadata when configured.
    pub readiness: Option<ProbeSummary>,
    /// Liveness probe metadata when configured.
    pub liveness: Option<ProbeSummary>,
}

/// Read-only probe metadata without sensitive payload values.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProbeSummary {
    /// Probe handler type and safe route metadata.
    pub handler: ProbeHandlerSummary,
    /// Initial delay before probing in seconds.
    pub initial_delay_seconds: Option<i32>,
    /// Probe period in seconds.
    pub period_seconds: Option<i32>,
    /// Probe timeout in seconds.
    pub timeout_seconds: Option<i32>,
    /// Consecutive success threshold.
    pub success_threshold: Option<i32>,
    /// Consecutive failure threshold.
    pub failure_threshold: Option<i32>,
    /// Probe-specific termination grace period in seconds.
    pub termination_grace_period_seconds: Option<i64>,
}

/// Read-only probe handler metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProbeHandlerSummary {
    /// Exec probe without command payload.
    Exec,
    /// HTTP GET probe metadata without header values.
    HttpGet {
        /// Optional HTTP host override.
        host: Option<String>,
        /// Optional HTTP request path.
        path: Option<String>,
        /// HTTP target port as a string, preserving named ports.
        port: String,
        /// Optional HTTP scheme.
        scheme: Option<String>,
        /// Number of configured HTTP headers.
        header_count: usize,
    },
    /// TCP socket probe metadata.
    TcpSocket {
        /// Optional TCP host override.
        host: Option<String>,
        /// TCP target port as a string, preserving named ports.
        port: String,
    },
    /// gRPC probe metadata.
    Grpc {
        /// gRPC health check port.
        port: i32,
        /// Optional gRPC service name.
        service: Option<String>,
    },
    /// Probe has no supported handler metadata.
    Unknown,
}

/// Read-only resource request and limit metadata for one container.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContainerResourceSummary {
    /// Container name.
    pub container_name: String,
    /// Resource requests in deterministic resource-name order.
    pub requests: Vec<ResourceQuantitySummary>,
    /// Resource limits in deterministic resource-name order.
    pub limits: Vec<ResourceQuantitySummary>,
}

/// One Kubernetes resource quantity value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResourceQuantitySummary {
    /// Resource name, such as `cpu` or `memory`.
    pub name: String,
    /// Kubernetes quantity string.
    pub quantity: String,
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

/// Read-only summary of a Kubernetes Pod.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PodSummary {
    /// Pod namespace.
    pub namespace: String,
    /// Pod name.
    pub name: String,
    /// Current Pod phase, such as `Running`, `Pending`, or `Failed`.
    pub phase: Option<String>,
    /// Node name where the Pod is scheduled.
    pub node_name: Option<String>,
    /// Pod IP assigned by Kubernetes.
    pub pod_ip: Option<String>,
    /// Declared container images in pod spec order.
    pub images: Vec<String>,
    /// Readiness and liveness probes in pod spec container order.
    pub probes: Vec<ContainerProbeSummary>,
    /// Resource requests and limits in pod spec container order.
    pub resources: Vec<ContainerResourceSummary>,
    /// Owner references in manifest order.
    pub owner_references: Vec<OwnerReferenceSummary>,
}

/// Read-only summary of a Kubernetes owner reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OwnerReferenceSummary {
    /// Referenced owner kind.
    pub kind: String,
    /// Referenced owner name.
    pub name: String,
    /// Referenced owner uid.
    pub uid: String,
    /// Whether this owner reference is marked as the controlling owner.
    pub controller: Option<bool>,
}

/// Read-only summary of a Kubernetes Ingress.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IngressSummary {
    /// Ingress namespace.
    pub namespace: String,
    /// Ingress name.
    pub name: String,
    /// Optional IngressClass name.
    pub ingress_class_name: Option<String>,
    /// Default backend service when configured.
    pub default_backend: Option<IngressBackendSummary>,
    /// Ingress rules in manifest order.
    pub rules: Vec<IngressRuleSummary>,
    /// TLS host groups in manifest order. Secret names are metadata only.
    pub tls: Vec<IngressTlsSummary>,
}

/// Read-only summary of an Ingress backend service.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IngressBackendSummary {
    /// Backend Service name.
    pub service_name: String,
    /// Backend Service port as a string, preserving named ports.
    pub service_port: String,
}

/// Read-only summary of an Ingress host rule.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IngressRuleSummary {
    /// Optional host matched by the rule.
    pub host: Option<String>,
    /// HTTP paths for the rule.
    pub paths: Vec<IngressPathSummary>,
}

/// Read-only summary of an Ingress HTTP path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IngressPathSummary {
    /// Optional matched path.
    pub path: Option<String>,
    /// Optional path type, such as `Prefix` or `Exact`.
    pub path_type: Option<String>,
    /// Backend service for the path when configured.
    pub backend: Option<IngressBackendSummary>,
}

/// Read-only summary of an Ingress TLS entry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IngressTlsSummary {
    /// TLS hosts in manifest order.
    pub hosts: Vec<String>,
    /// Referenced Secret name, kept as metadata only.
    pub secret_name: Option<String>,
}

/// Read-only summary of a Gateway API GatewayClass.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GatewayClassSummary {
    /// GatewayClass name.
    pub name: String,
    /// Controller name responsible for this GatewayClass.
    pub controller_name: Option<String>,
    /// Optional human-readable description.
    pub description: Option<String>,
}

/// Read-only summary of a Gateway API Gateway.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GatewaySummary {
    /// Gateway namespace.
    pub namespace: String,
    /// Gateway name.
    pub name: String,
    /// Referenced GatewayClass name.
    pub gateway_class_name: Option<String>,
    /// Gateway listeners in manifest order.
    pub listeners: Vec<GatewayListenerSummary>,
}

/// Read-only summary of a Gateway API Gateway listener.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GatewayListenerSummary {
    /// Listener name.
    pub name: Option<String>,
    /// Optional listener hostname.
    pub hostname: Option<String>,
    /// Listener port.
    pub port: Option<i64>,
    /// Listener protocol.
    pub protocol: Option<String>,
}

/// Read-only summary of a Gateway API HTTPRoute.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HttpRouteSummary {
    /// HTTPRoute namespace.
    pub namespace: String,
    /// HTTPRoute name.
    pub name: String,
    /// Hostnames matched by the HTTPRoute.
    pub hostnames: Vec<String>,
    /// Parent references in manifest order.
    pub parent_refs: Vec<RouteParentRefSummary>,
    /// Rules in manifest order.
    pub rules: Vec<HttpRouteRuleSummary>,
}

/// Read-only summary of a Gateway API parent reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RouteParentRefSummary {
    /// Parent resource kind.
    pub kind: Option<String>,
    /// Parent resource namespace.
    pub namespace: Option<String>,
    /// Parent resource name.
    pub name: Option<String>,
    /// Optional section name on the parent resource.
    pub section_name: Option<String>,
}

/// Read-only summary of a Gateway API HTTPRoute rule.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HttpRouteRuleSummary {
    /// Backend references in manifest order.
    pub backend_refs: Vec<HttpRouteBackendRefSummary>,
}

/// Read-only summary of a Gateway API HTTPRoute backend reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HttpRouteBackendRefSummary {
    /// Backend resource kind.
    pub kind: Option<String>,
    /// Backend namespace.
    pub namespace: Option<String>,
    /// Backend name.
    pub name: Option<String>,
    /// Backend port.
    pub port: Option<i64>,
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

/// List [`Pod`] objects directly owned by a [`WorkloadRef`] in one namespace.
///
/// This matches [`Pod`] owner references against the [`WorkloadRef`] kind and
/// name. It does not follow intermediate controller chains such as Deployment
/// to ReplicaSet to [`Pod`].
///
/// # Errors
///
/// Returns [`kube::Error`] when the Kubernetes API request fails.
pub async fn list_pods_owned_by_workload(
    client: Client,
    workload: &WorkloadRef,
) -> Result<Vec<PodSummary>, kube::Error> {
    let pods: Api<Pod> = Api::namespaced(client, workload.namespace());
    let mut summaries = pods
        .list(&ListParams::default())
        .await?
        .iter()
        .filter(|pod| pod_is_owned_by_workload(pod, workload))
        .map(pod_summary)
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(summaries)
}

/// List Ingress objects in one namespace without mutating cluster state.
///
/// # Errors
///
/// Returns [`kube::Error`] when the Kubernetes API request fails.
pub async fn list_ingresses(
    client: Client,
    namespace: &str,
) -> Result<Vec<IngressSummary>, kube::Error> {
    let ingresses: Api<Ingress> = Api::namespaced(client, namespace);
    let mut summaries = ingresses
        .list(&ListParams::default())
        .await?
        .iter()
        .map(ingress_summary)
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(summaries)
}

/// List Gateway API GatewayClass objects without mutating cluster state.
///
/// # Errors
///
/// Returns [`kube::Error`] when the Kubernetes API request fails.
pub async fn list_gateway_classes(client: Client) -> Result<Vec<GatewayClassSummary>, kube::Error> {
    let gateway_classes: Api<DynamicObject> = Api::all_with(
        client,
        &gateway_api_resource("GatewayClass", "gatewayclasses"),
    );
    let mut summaries = gateway_classes
        .list(&ListParams::default())
        .await?
        .iter()
        .map(gateway_class_summary)
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(summaries)
}

/// List Gateway API Gateway objects in one namespace without mutating cluster state.
///
/// # Errors
///
/// Returns [`kube::Error`] when the Kubernetes API request fails.
pub async fn list_gateways(
    client: Client,
    namespace: &str,
) -> Result<Vec<GatewaySummary>, kube::Error> {
    let gateways: Api<DynamicObject> = Api::namespaced_with(
        client,
        namespace,
        &gateway_api_resource("Gateway", "gateways"),
    );
    let mut summaries = gateways
        .list(&ListParams::default())
        .await?
        .iter()
        .map(gateway_summary)
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(summaries)
}

/// List Gateway API HTTPRoute objects in one namespace without mutating cluster state.
///
/// # Errors
///
/// Returns [`kube::Error`] when the Kubernetes API request fails.
pub async fn list_http_routes(
    client: Client,
    namespace: &str,
) -> Result<Vec<HttpRouteSummary>, kube::Error> {
    let routes: Api<DynamicObject> = Api::namespaced_with(
        client,
        namespace,
        &gateway_api_resource("HTTPRoute", "httproutes"),
    );
    let mut summaries = routes
        .list(&ListParams::default())
        .await?
        .iter()
        .map(http_route_summary)
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
    let probes = spec
        .and_then(|spec| spec.template.spec.as_ref())
        .map(|pod_spec| container_probe_summaries(&pod_spec.containers))
        .unwrap_or_default();
    let resources = spec
        .and_then(|spec| spec.template.spec.as_ref())
        .map(|pod_spec| container_resource_summaries(&pod_spec.containers))
        .unwrap_or_default();

    DeploymentSummary {
        namespace: deployment.namespace().unwrap_or_default(),
        name: deployment.name_any(),
        replicas: spec.and_then(|spec| spec.replicas),
        available_replicas: status.and_then(|status| status.available_replicas),
        ready_replicas: status.and_then(|status| status.ready_replicas),
        updated_replicas: status.and_then(|status| status.updated_replicas),
        images,
        probes,
        resources,
        rollout: deployment_rollout_summary(deployment),
    }
}

/// Convert a Kubernetes [`Deployment`] into basic rollout status.
pub fn deployment_rollout_summary(deployment: &Deployment) -> DeploymentRolloutSummary {
    let spec = deployment.spec.as_ref();
    let status = deployment.status.as_ref();
    let desired_replicas = spec.and_then(|spec| spec.replicas);
    let ready_replicas = status.and_then(|status| status.ready_replicas);
    let available_replicas = status.and_then(|status| status.available_replicas);
    let updated_replicas = status.and_then(|status| status.updated_replicas);
    let unavailable_replicas = status.and_then(|status| status.unavailable_replicas);
    let generation = deployment.metadata.generation;
    let observed_generation = status.and_then(|status| status.observed_generation);
    let conditions = status
        .and_then(|status| status.conditions.as_deref())
        .unwrap_or_default()
        .iter()
        .map(|condition| DeploymentConditionSummary {
            type_: condition.type_.clone(),
            status: condition.status.clone(),
            reason: condition.reason.clone(),
            message: condition.message.clone(),
        })
        .collect::<Vec<_>>();

    DeploymentRolloutSummary {
        phase: deployment_rollout_phase(
            generation,
            observed_generation,
            desired_replicas,
            ready_replicas,
            available_replicas,
            updated_replicas,
            unavailable_replicas,
        ),
        generation,
        observed_generation,
        desired_replicas,
        ready_replicas,
        available_replicas,
        updated_replicas,
        unavailable_replicas,
        conditions,
    }
}

fn deployment_rollout_phase(
    generation: Option<i64>,
    observed_generation: Option<i64>,
    desired_replicas: Option<i32>,
    ready_replicas: Option<i32>,
    available_replicas: Option<i32>,
    updated_replicas: Option<i32>,
    unavailable_replicas: Option<i32>,
) -> DeploymentRolloutPhase {
    let Some(desired_replicas) = desired_replicas else {
        return DeploymentRolloutPhase::Unknown;
    };
    let Some(updated_replicas) = updated_replicas else {
        return DeploymentRolloutPhase::Progressing;
    };
    let Some(available_replicas) = available_replicas else {
        return DeploymentRolloutPhase::Progressing;
    };

    if generation
        .zip(observed_generation)
        .is_some_and(|(generation, observed)| observed < generation)
    {
        return DeploymentRolloutPhase::Progressing;
    }

    if available_replicas == 0 && desired_replicas > 0 {
        return DeploymentRolloutPhase::Unavailable;
    }

    if updated_replicas == desired_replicas
        && available_replicas == desired_replicas
        && ready_replicas.unwrap_or_default() == desired_replicas
        && unavailable_replicas.unwrap_or_default() == 0
    {
        return DeploymentRolloutPhase::Complete;
    }

    DeploymentRolloutPhase::Progressing
}

fn container_probe_summaries(containers: &[Container]) -> Vec<ContainerProbeSummary> {
    containers
        .iter()
        .filter_map(|container| {
            let readiness = container.readiness_probe.as_ref().map(probe_summary);
            let liveness = container.liveness_probe.as_ref().map(probe_summary);

            (readiness.is_some() || liveness.is_some()).then(|| ContainerProbeSummary {
                container_name: container.name.clone(),
                readiness,
                liveness,
            })
        })
        .collect()
}

fn probe_summary(probe: &Probe) -> ProbeSummary {
    ProbeSummary {
        handler: probe_handler_summary(probe),
        initial_delay_seconds: probe.initial_delay_seconds,
        period_seconds: probe.period_seconds,
        timeout_seconds: probe.timeout_seconds,
        success_threshold: probe.success_threshold,
        failure_threshold: probe.failure_threshold,
        termination_grace_period_seconds: probe.termination_grace_period_seconds,
    }
}

fn probe_handler_summary(probe: &Probe) -> ProbeHandlerSummary {
    if let Some(http_get) = probe.http_get.as_ref() {
        return ProbeHandlerSummary::HttpGet {
            host: http_get.host.clone(),
            path: http_get.path.clone(),
            port: format_int_or_string(&http_get.port),
            scheme: http_get.scheme.clone(),
            header_count: http_get.http_headers.as_ref().map_or(0, Vec::len),
        };
    }

    if let Some(tcp_socket) = probe.tcp_socket.as_ref() {
        return ProbeHandlerSummary::TcpSocket {
            host: tcp_socket.host.clone(),
            port: format_int_or_string(&tcp_socket.port),
        };
    }

    if let Some(grpc) = probe.grpc.as_ref() {
        return ProbeHandlerSummary::Grpc {
            port: grpc.port,
            service: grpc.service.clone(),
        };
    }

    if probe.exec.is_some() {
        return ProbeHandlerSummary::Exec;
    }

    ProbeHandlerSummary::Unknown
}

fn container_resource_summaries(containers: &[Container]) -> Vec<ContainerResourceSummary> {
    containers
        .iter()
        .filter_map(|container| {
            let resources = container.resources.as_ref()?;
            let requests = resource_quantity_summaries(resources.requests.as_ref());
            let limits = resource_quantity_summaries(resources.limits.as_ref());

            (!requests.is_empty() || !limits.is_empty()).then(|| ContainerResourceSummary {
                container_name: container.name.clone(),
                requests,
                limits,
            })
        })
        .collect()
}

fn resource_quantity_summaries(
    quantities: Option<&std::collections::BTreeMap<String, Quantity>>,
) -> Vec<ResourceQuantitySummary> {
    quantities
        .into_iter()
        .flat_map(|quantities| quantities.iter())
        .map(|(name, quantity)| ResourceQuantitySummary {
            name: name.clone(),
            quantity: quantity.0.clone(),
        })
        .collect()
}

/// Convert a Kubernetes [`Ingress`] into a deterministic summary.
pub fn ingress_summary(ingress: &Ingress) -> IngressSummary {
    let spec = ingress.spec.as_ref();
    let rules = spec
        .map(|spec| {
            spec.rules
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|rule| IngressRuleSummary {
                    host: rule.host.clone(),
                    paths: rule
                        .http
                        .as_ref()
                        .map(|http| {
                            http.paths
                                .iter()
                                .map(|path| IngressPathSummary {
                                    path: path.path.clone(),
                                    path_type: Some(path.path_type.clone()),
                                    backend: ingress_backend_summary(&path.backend),
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let tls = spec
        .map(|spec| {
            spec.tls
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|tls| IngressTlsSummary {
                    hosts: tls.hosts.clone().unwrap_or_default(),
                    secret_name: tls.secret_name.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    IngressSummary {
        namespace: ingress.namespace().unwrap_or_default(),
        name: ingress.name_any(),
        ingress_class_name: spec.and_then(|spec| spec.ingress_class_name.clone()),
        default_backend: spec.and_then(|spec| {
            spec.default_backend
                .as_ref()
                .and_then(ingress_backend_summary)
        }),
        rules,
        tls,
    }
}

/// Convert a Gateway API GatewayClass [`DynamicObject`] into a deterministic summary.
pub fn gateway_class_summary(gateway_class: &DynamicObject) -> GatewayClassSummary {
    let spec = gateway_class.data.get("spec");

    GatewayClassSummary {
        name: gateway_class.name_any(),
        controller_name: spec.and_then(|spec| string_field(spec, "controllerName")),
        description: spec.and_then(|spec| string_field(spec, "description")),
    }
}

/// Convert a Gateway API Gateway [`DynamicObject`] into a deterministic summary.
pub fn gateway_summary(gateway: &DynamicObject) -> GatewaySummary {
    let spec = gateway.data.get("spec");
    let listeners = spec
        .and_then(|spec| spec.get("listeners"))
        .and_then(Value::as_array)
        .map(|listeners| {
            listeners
                .iter()
                .map(|listener| GatewayListenerSummary {
                    name: string_field(listener, "name"),
                    hostname: string_field(listener, "hostname"),
                    port: int_field(listener, "port"),
                    protocol: string_field(listener, "protocol"),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    GatewaySummary {
        namespace: gateway.namespace().unwrap_or_default(),
        name: gateway.name_any(),
        gateway_class_name: spec.and_then(|spec| string_field(spec, "gatewayClassName")),
        listeners,
    }
}

/// Convert a Gateway API HTTPRoute [`DynamicObject`] into a deterministic summary.
pub fn http_route_summary(route: &DynamicObject) -> HttpRouteSummary {
    let spec = route.data.get("spec");
    let hostnames = spec
        .and_then(|spec| spec.get("hostnames"))
        .and_then(Value::as_array)
        .map(|hostnames| string_array(hostnames))
        .unwrap_or_default();
    let parent_refs = spec
        .and_then(|spec| spec.get("parentRefs"))
        .and_then(Value::as_array)
        .map(|parent_refs| {
            parent_refs
                .iter()
                .map(|parent_ref| RouteParentRefSummary {
                    kind: string_field(parent_ref, "kind"),
                    namespace: string_field(parent_ref, "namespace"),
                    name: string_field(parent_ref, "name"),
                    section_name: string_field(parent_ref, "sectionName"),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let rules = spec
        .and_then(|spec| spec.get("rules"))
        .and_then(Value::as_array)
        .map(|rules| {
            rules
                .iter()
                .map(|rule| HttpRouteRuleSummary {
                    backend_refs: rule
                        .get("backendRefs")
                        .and_then(Value::as_array)
                        .map(|backend_refs| {
                            backend_refs
                                .iter()
                                .map(|backend_ref| HttpRouteBackendRefSummary {
                                    kind: string_field(backend_ref, "kind"),
                                    namespace: string_field(backend_ref, "namespace"),
                                    name: string_field(backend_ref, "name"),
                                    port: int_field(backend_ref, "port"),
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    HttpRouteSummary {
        namespace: route.namespace().unwrap_or_default(),
        name: route.name_any(),
        hostnames,
        parent_refs,
        rules,
    }
}

fn ingress_backend_summary(backend: &IngressBackend) -> Option<IngressBackendSummary> {
    let service = backend.service.as_ref()?;
    let port = service.port.as_ref()?;
    let service_port = port
        .name
        .clone()
        .or_else(|| port.number.map(|number| number.to_string()))?;

    Some(IngressBackendSummary {
        service_name: service.name.clone(),
        service_port,
    })
}

fn gateway_api_resource(kind: &str, plural: &str) -> ApiResource {
    let group_version_kind = GroupVersionKind::gvk("gateway.networking.k8s.io", "v1", kind);
    ApiResource::from_gvk_with_plural(&group_version_kind, plural)
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn int_field(value: &Value, field: &str) -> Option<i64> {
    value.get(field).and_then(Value::as_i64)
}

fn string_array(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

/// Convert a Kubernetes [`Pod`] into a deterministic summary.
pub fn pod_summary(pod: &Pod) -> PodSummary {
    let spec = pod.spec.as_ref();
    let status = pod.status.as_ref();
    let images = spec
        .map(|spec| {
            spec.containers
                .iter()
                .map(|container| container.image.clone().unwrap_or_default())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let probes = spec
        .map(|spec| container_probe_summaries(&spec.containers))
        .unwrap_or_default();
    let resources = spec
        .map(|spec| container_resource_summaries(&spec.containers))
        .unwrap_or_default();
    let owner_references = pod
        .metadata
        .owner_references
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|owner| OwnerReferenceSummary {
            kind: owner.kind.clone(),
            name: owner.name.clone(),
            uid: owner.uid.clone(),
            controller: owner.controller,
        })
        .collect::<Vec<_>>();

    PodSummary {
        namespace: pod.namespace().unwrap_or_default(),
        name: pod.name_any(),
        phase: status.and_then(|status| status.phase.clone()),
        node_name: spec.and_then(|spec| spec.node_name.clone()),
        pod_ip: status.and_then(|status| status.pod_ip.clone()),
        images,
        probes,
        resources,
        owner_references,
    }
}

/// Return true when a [`Pod`] has a direct owner reference to `workload`.
pub fn pod_is_owned_by_workload(pod: &Pod, workload: &WorkloadRef) -> bool {
    pod.namespace().as_deref() == Some(workload.namespace())
        && pod
            .metadata
            .owner_references
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|owner| owner.kind == workload.kind() && owner.name == workload.name())
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
        ClusterInfo, ContainerProbeSummary, ContainerResourceSummary, DeploymentConditionSummary,
        DeploymentRolloutPhase, DeploymentRolloutSummary, DeploymentSummary, GatewayClassSummary,
        GatewayListenerSummary, GatewaySummary, HttpRouteBackendRefSummary, HttpRouteRuleSummary,
        HttpRouteSummary, IngressBackendSummary, IngressPathSummary, IngressRuleSummary,
        IngressSummary, IngressTlsSummary, LabelSelectorEntry, OwnerReferenceSummary, PodSummary,
        ProbeHandlerSummary, ProbeSummary, ResourceQuantitySummary, RouteParentRefSummary,
        ServicePortSummary, ServiceSummary, deployment_rollout_summary, deployment_summary,
        gateway_api_resource, gateway_class_summary, gateway_summary, http_route_summary,
        ingress_summary, load_kube_config_path, load_kube_config_with_options,
        pod_is_owned_by_workload, pod_summary, service_summary,
    };
    use k8s_openapi::api::apps::v1::{
        Deployment, DeploymentCondition, DeploymentSpec, DeploymentStatus,
    };
    use k8s_openapi::api::core::v1::{
        Container, HTTPGetAction, HTTPHeader, Pod, PodSpec, PodStatus, PodTemplateSpec, Probe,
        ResourceRequirements, Service, ServicePort, ServiceSpec, TCPSocketAction,
    };
    use k8s_openapi::api::networking::v1::{
        HTTPIngressPath, HTTPIngressRuleValue, Ingress, IngressBackend, IngressRule,
        IngressServiceBackend, IngressSpec, IngressTLS, ServiceBackendPort,
    };
    use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::{
        LabelSelector, ObjectMeta, OwnerReference,
    };
    use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
    use kply_core::WorkloadRef;
    use kube::config::KubeConfigOptions;
    use kube::core::{DynamicObject, ObjectList};
    use serde_json::json;
    use std::{collections::BTreeMap, env, fs, path::Path};
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
                probes: vec![ContainerProbeSummary {
                    container_name: "checkout-v2".to_owned(),
                    readiness: Some(http_probe_summary("/ready", "http")),
                    liveness: Some(tcp_probe_summary(8080)),
                }],
                resources: vec![checkout_resource_summary("checkout-v2")],
                rollout: DeploymentRolloutSummary {
                    phase: DeploymentRolloutPhase::Progressing,
                    generation: Some(2),
                    observed_generation: Some(2),
                    desired_replicas: Some(3),
                    ready_replicas: Some(2),
                    available_replicas: Some(2),
                    updated_replicas: Some(3),
                    unavailable_replicas: Some(1),
                    conditions: vec![
                        DeploymentConditionSummary {
                            type_: "Progressing".to_owned(),
                            status: "True".to_owned(),
                            reason: Some("NewReplicaSetAvailable".to_owned()),
                            message: Some("ReplicaSet is progressing".to_owned()),
                        },
                        DeploymentConditionSummary {
                            type_: "Available".to_owned(),
                            status: "False".to_owned(),
                            reason: Some("MinimumReplicasUnavailable".to_owned()),
                            message: None,
                        },
                    ],
                },
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
                probes: Vec::new(),
                resources: Vec::new(),
                rollout: DeploymentRolloutSummary {
                    phase: DeploymentRolloutPhase::Unknown,
                    generation: None,
                    observed_generation: None,
                    desired_replicas: None,
                    ready_replicas: None,
                    available_replicas: None,
                    updated_replicas: None,
                    unavailable_replicas: None,
                    conditions: Vec::new(),
                },
            }
        );
    }

    #[test]
    fn summarizes_complete_deployment_rollout() {
        let deployment = fake_ready_deployment("shop", "checkout-api");

        let rollout = deployment_rollout_summary(&deployment);

        assert_eq!(
            rollout,
            DeploymentRolloutSummary {
                phase: DeploymentRolloutPhase::Complete,
                generation: Some(7),
                observed_generation: Some(7),
                desired_replicas: Some(3),
                ready_replicas: Some(3),
                available_replicas: Some(3),
                updated_replicas: Some(3),
                unavailable_replicas: Some(0),
                conditions: vec![DeploymentConditionSummary {
                    type_: "Available".to_owned(),
                    status: "True".to_owned(),
                    reason: Some("MinimumReplicasAvailable".to_owned()),
                    message: Some("Deployment has minimum availability".to_owned()),
                }],
            }
        );
    }

    #[test]
    fn summarizes_unavailable_deployment_rollout() {
        let deployment = fake_unavailable_deployment("shop", "checkout-api");

        let rollout = deployment_rollout_summary(&deployment);

        assert_eq!(rollout.phase, DeploymentRolloutPhase::Unavailable);
        assert_eq!(rollout.desired_replicas, Some(3));
        assert_eq!(rollout.available_replicas, Some(0));
        assert_eq!(rollout.unavailable_replicas, Some(3));
    }

    #[test]
    fn loads_read_only_app_kubernetes_response_fixtures() {
        let deployments =
            read_k8s_response_fixture::<ObjectList<Deployment>>("read-only-app/deployments.json");
        let services =
            read_k8s_response_fixture::<ObjectList<Service>>("read-only-app/services.json");
        let pods = read_k8s_response_fixture::<ObjectList<Pod>>("read-only-app/pods.json");
        let ingresses =
            read_k8s_response_fixture::<ObjectList<Ingress>>("read-only-app/ingresses.json");
        let gateway_classes = read_k8s_response_fixture::<ObjectList<DynamicObject>>(
            "read-only-app/gatewayclasses.json",
        );
        let gateways =
            read_k8s_response_fixture::<ObjectList<DynamicObject>>("read-only-app/gateways.json");
        let http_routes =
            read_k8s_response_fixture::<ObjectList<DynamicObject>>("read-only-app/httproutes.json");

        let deployment = deployment_summary(
            deployments
                .items
                .first()
                .expect("deployments fixture should contain at least one item"),
        );
        let service = service_summary(
            services
                .items
                .first()
                .expect("services fixture should contain at least one item"),
        );
        let pod = pod_summary(
            pods.items
                .first()
                .expect("pods fixture should contain at least one item"),
        );
        let ingress = ingress_summary(
            ingresses
                .items
                .first()
                .expect("ingresses fixture should contain at least one item"),
        );
        let gateway_class = gateway_class_summary(
            gateway_classes
                .items
                .first()
                .expect("gateway_classes fixture should contain at least one item"),
        );
        let gateway = gateway_summary(
            gateways
                .items
                .first()
                .expect("gateways fixture should contain at least one item"),
        );
        let http_route = http_route_summary(
            http_routes
                .items
                .first()
                .expect("http_routes fixture should contain at least one item"),
        );

        kply_test::insta::assert_json_snapshot!(
            "read_only_app_kubernetes_response_summaries",
            json!({
                "deployment": deployment,
                "service": service,
                "pod": pod,
                "ingress": ingress,
                "gateway_class": gateway_class,
                "gateway": gateway,
                "http_route": http_route,
            })
        );
    }

    #[test]
    fn summarizes_pod_metadata_status_and_owners() {
        let pod = fake_pod(
            "shop",
            "checkout-api-7f7c8d9b9d-x1",
            &[("checkout", "checkout:v2"), ("sidecar", "sidecar:v1")],
            vec![OwnerReference {
                api_version: "apps/v1".to_owned(),
                kind: "ReplicaSet".to_owned(),
                name: "checkout-api-7f7c8d9b9d".to_owned(),
                uid: "replicaset-uid".to_owned(),
                controller: Some(true),
                ..OwnerReference::default()
            }],
        );

        let summary = pod_summary(&pod);

        assert_eq!(
            summary,
            PodSummary {
                namespace: "shop".to_owned(),
                name: "checkout-api-7f7c8d9b9d-x1".to_owned(),
                phase: Some("Running".to_owned()),
                node_name: Some("worker-a".to_owned()),
                pod_ip: Some("10.244.0.12".to_owned()),
                images: vec!["checkout:v2".to_owned(), "sidecar:v1".to_owned()],
                probes: vec![ContainerProbeSummary {
                    container_name: "checkout".to_owned(),
                    readiness: Some(http_probe_summary("/ready", "http")),
                    liveness: Some(tcp_probe_summary(8080)),
                }],
                resources: vec![checkout_resource_summary("checkout")],
                owner_references: vec![OwnerReferenceSummary {
                    kind: "ReplicaSet".to_owned(),
                    name: "checkout-api-7f7c8d9b9d".to_owned(),
                    uid: "replicaset-uid".to_owned(),
                    controller: Some(true),
                }],
            }
        );
    }

    #[test]
    fn summarizes_minimal_pod_without_optional_fields() {
        let pod = Pod {
            metadata: ObjectMeta {
                name: Some("minimal".to_owned()),
                ..ObjectMeta::default()
            },
            ..Pod::default()
        };

        let summary = pod_summary(&pod);

        assert_eq!(
            summary,
            PodSummary {
                namespace: String::new(),
                name: "minimal".to_owned(),
                phase: None,
                node_name: None,
                pod_ip: None,
                images: Vec::new(),
                probes: Vec::new(),
                resources: Vec::new(),
                owner_references: Vec::new(),
            }
        );
    }

    #[test]
    fn detects_pods_directly_owned_by_workload() {
        let workload =
            WorkloadRef::new("shop", "ReplicaSet", "checkout-api-7f7c8d9b9d").expect("workload");
        let owned_pod = fake_pod(
            "shop",
            "checkout-api-7f7c8d9b9d-x1",
            &[("checkout", "checkout:v2")],
            vec![OwnerReference {
                api_version: "apps/v1".to_owned(),
                kind: "ReplicaSet".to_owned(),
                name: "checkout-api-7f7c8d9b9d".to_owned(),
                uid: "replicaset-uid".to_owned(),
                ..OwnerReference::default()
            }],
        );
        let other_namespace_pod = fake_pod(
            "qa",
            "checkout-api-7f7c8d9b9d-x2",
            &[("checkout", "checkout:v2")],
            vec![OwnerReference {
                api_version: "apps/v1".to_owned(),
                kind: "ReplicaSet".to_owned(),
                name: "checkout-api-7f7c8d9b9d".to_owned(),
                uid: "replicaset-uid".to_owned(),
                ..OwnerReference::default()
            }],
        );
        let unrelated_pod = fake_pod(
            "shop",
            "catalog-api-5c7d9f5c6f-z1",
            &[("catalog", "catalog:v1")],
            vec![OwnerReference {
                api_version: "apps/v1".to_owned(),
                kind: "ReplicaSet".to_owned(),
                name: "catalog-api-5c7d9f5c6f".to_owned(),
                uid: "catalog-replicaset-uid".to_owned(),
                ..OwnerReference::default()
            }],
        );

        assert!(pod_is_owned_by_workload(&owned_pod, &workload));
        assert!(!pod_is_owned_by_workload(&other_namespace_pod, &workload));
        assert!(!pod_is_owned_by_workload(&unrelated_pod, &workload));
    }

    #[test]
    fn summarizes_ingress_rules_backends_and_tls_metadata() {
        let ingress = fake_ingress("shop", "checkout-ingress");

        let summary = ingress_summary(&ingress);

        assert_eq!(
            summary,
            IngressSummary {
                namespace: "shop".to_owned(),
                name: "checkout-ingress".to_owned(),
                ingress_class_name: Some("nginx".to_owned()),
                default_backend: Some(IngressBackendSummary {
                    service_name: "checkout-http".to_owned(),
                    service_port: "http".to_owned(),
                }),
                rules: vec![IngressRuleSummary {
                    host: Some("checkout.example.com".to_owned()),
                    paths: vec![
                        IngressPathSummary {
                            path: Some("/".to_owned()),
                            path_type: Some("Prefix".to_owned()),
                            backend: Some(IngressBackendSummary {
                                service_name: "checkout-http".to_owned(),
                                service_port: "80".to_owned(),
                            }),
                        },
                        IngressPathSummary {
                            path: Some("/metrics".to_owned()),
                            path_type: Some("Exact".to_owned()),
                            backend: Some(IngressBackendSummary {
                                service_name: "checkout-metrics".to_owned(),
                                service_port: "metrics".to_owned(),
                            }),
                        },
                    ],
                }],
                tls: vec![IngressTlsSummary {
                    hosts: vec!["checkout.example.com".to_owned()],
                    secret_name: Some("checkout-tls".to_owned()),
                }],
            }
        );
    }

    #[test]
    fn summarizes_minimal_ingress_without_optional_fields() {
        let ingress = Ingress {
            metadata: ObjectMeta {
                name: Some("minimal".to_owned()),
                ..ObjectMeta::default()
            },
            ..Ingress::default()
        };

        let summary = ingress_summary(&ingress);

        assert_eq!(
            summary,
            IngressSummary {
                namespace: String::new(),
                name: "minimal".to_owned(),
                ingress_class_name: None,
                default_backend: None,
                rules: Vec::new(),
                tls: Vec::new(),
            }
        );
    }

    #[test]
    fn summarizes_gateway_class_metadata() {
        let gateway_class = DynamicObject::new(
            "public",
            &gateway_api_resource("GatewayClass", "gatewayclasses"),
        )
        .data(json!({
            "spec": {
                "controllerName": "example.com/gateway-controller",
                "description": "Public edge gateways"
            }
        }));

        let summary = gateway_class_summary(&gateway_class);

        assert_eq!(
            summary,
            GatewayClassSummary {
                name: "public".to_owned(),
                controller_name: Some("example.com/gateway-controller".to_owned()),
                description: Some("Public edge gateways".to_owned()),
            }
        );
    }

    #[test]
    fn summarizes_gateway_listeners() {
        let gateway = DynamicObject::new(
            "public-gateway",
            &gateway_api_resource("Gateway", "gateways"),
        )
        .within("shop")
        .data(json!({
            "spec": {
                "gatewayClassName": "public",
                "listeners": [
                    {
                        "name": "http",
                        "hostname": "checkout.example.com",
                        "port": 80,
                        "protocol": "HTTP"
                    },
                    {
                        "name": "https",
                        "port": 443,
                        "protocol": "HTTPS"
                    }
                ]
            }
        }));

        let summary = gateway_summary(&gateway);

        assert_eq!(
            summary,
            GatewaySummary {
                namespace: "shop".to_owned(),
                name: "public-gateway".to_owned(),
                gateway_class_name: Some("public".to_owned()),
                listeners: vec![
                    GatewayListenerSummary {
                        name: Some("http".to_owned()),
                        hostname: Some("checkout.example.com".to_owned()),
                        port: Some(80),
                        protocol: Some("HTTP".to_owned()),
                    },
                    GatewayListenerSummary {
                        name: Some("https".to_owned()),
                        hostname: None,
                        port: Some(443),
                        protocol: Some("HTTPS".to_owned()),
                    },
                ],
            }
        );
    }

    #[test]
    fn summarizes_http_route_parents_hosts_and_backends() {
        let route = DynamicObject::new(
            "checkout-route",
            &gateway_api_resource("HTTPRoute", "httproutes"),
        )
        .within("shop")
        .data(json!({
            "spec": {
                "hostnames": ["checkout.example.com"],
                "parentRefs": [
                    {
                        "kind": "Gateway",
                        "namespace": "platform",
                        "name": "public-gateway",
                        "sectionName": "https"
                    }
                ],
                "rules": [
                    {
                        "backendRefs": [
                            {
                                "kind": "Service",
                                "name": "checkout-http",
                                "port": 80
                            },
                            {
                                "kind": "Service",
                                "namespace": "shared",
                                "name": "checkout-canary",
                                "port": 8080
                            }
                        ]
                    }
                ]
            }
        }));

        let summary = http_route_summary(&route);

        assert_eq!(
            summary,
            HttpRouteSummary {
                namespace: "shop".to_owned(),
                name: "checkout-route".to_owned(),
                hostnames: vec!["checkout.example.com".to_owned()],
                parent_refs: vec![RouteParentRefSummary {
                    kind: Some("Gateway".to_owned()),
                    namespace: Some("platform".to_owned()),
                    name: Some("public-gateway".to_owned()),
                    section_name: Some("https".to_owned()),
                }],
                rules: vec![HttpRouteRuleSummary {
                    backend_refs: vec![
                        HttpRouteBackendRefSummary {
                            kind: Some("Service".to_owned()),
                            namespace: None,
                            name: Some("checkout-http".to_owned()),
                            port: Some(80),
                        },
                        HttpRouteBackendRefSummary {
                            kind: Some("Service".to_owned()),
                            namespace: Some("shared".to_owned()),
                            name: Some("checkout-canary".to_owned()),
                            port: Some(8080),
                        },
                    ],
                }],
            }
        );
    }

    #[test]
    fn summarizes_minimal_gateway_api_resources() {
        let gateway_class = DynamicObject::new(
            "minimal-class",
            &gateway_api_resource("GatewayClass", "gatewayclasses"),
        );
        let gateway = DynamicObject::new(
            "minimal-gateway",
            &gateway_api_resource("Gateway", "gateways"),
        );
        let route = DynamicObject::new(
            "minimal-route",
            &gateway_api_resource("HTTPRoute", "httproutes"),
        );

        assert_eq!(
            gateway_class_summary(&gateway_class),
            GatewayClassSummary {
                name: "minimal-class".to_owned(),
                controller_name: None,
                description: None,
            }
        );
        assert_eq!(
            gateway_summary(&gateway),
            GatewaySummary {
                namespace: String::new(),
                name: "minimal-gateway".to_owned(),
                gateway_class_name: None,
                listeners: Vec::new(),
            }
        );
        assert_eq!(
            http_route_summary(&route),
            HttpRouteSummary {
                namespace: String::new(),
                name: "minimal-route".to_owned(),
                hostnames: Vec::new(),
                parent_refs: Vec::new(),
                rules: Vec::new(),
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
                generation: Some(2),
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
                                liveness_probe: image
                                    .starts_with("checkout:")
                                    .then(|| tcp_probe(8080)),
                                readiness_probe: image
                                    .starts_with("checkout:")
                                    .then(|| http_probe("/ready", "http")),
                                resources: image.starts_with("checkout:").then(checkout_resources),
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
                conditions: Some(vec![
                    DeploymentCondition {
                        type_: "Progressing".to_owned(),
                        status: "True".to_owned(),
                        reason: Some("NewReplicaSetAvailable".to_owned()),
                        message: Some("ReplicaSet is progressing".to_owned()),
                        ..DeploymentCondition::default()
                    },
                    DeploymentCondition {
                        type_: "Available".to_owned(),
                        status: "False".to_owned(),
                        reason: Some("MinimumReplicasUnavailable".to_owned()),
                        ..DeploymentCondition::default()
                    },
                ]),
                observed_generation: Some(2),
                ready_replicas: Some(2),
                replicas: Some(3),
                updated_replicas: Some(3),
                unavailable_replicas: Some(1),
                ..DeploymentStatus::default()
            }),
        }
    }

    fn read_k8s_response_fixture<T>(relative_path: &str) -> T
    where
        T: serde::de::DeserializeOwned,
    {
        let fixture_path = kply_test::fixture_path(Path::new("k8s-responses").join(relative_path));
        let source = fs::read_to_string(&fixture_path).unwrap_or_else(|error| {
            panic!(
                "Kubernetes response fixture {} should be readable: {error}",
                fixture_path.display()
            )
        });

        serde_json::from_str(&source).unwrap_or_else(|error| {
            panic!(
                "Kubernetes response fixture {} should deserialize: {error}",
                fixture_path.display()
            )
        })
    }

    fn fake_ready_deployment(namespace: &str, name: &str) -> Deployment {
        let mut deployment = fake_deployment(namespace, name, &["checkout:v2"]);
        deployment.metadata.generation = Some(7);
        deployment.status = Some(DeploymentStatus {
            available_replicas: Some(3),
            conditions: Some(vec![DeploymentCondition {
                type_: "Available".to_owned(),
                status: "True".to_owned(),
                reason: Some("MinimumReplicasAvailable".to_owned()),
                message: Some("Deployment has minimum availability".to_owned()),
                ..DeploymentCondition::default()
            }]),
            observed_generation: Some(7),
            ready_replicas: Some(3),
            replicas: Some(3),
            updated_replicas: Some(3),
            unavailable_replicas: Some(0),
            ..DeploymentStatus::default()
        });
        deployment
    }

    fn fake_unavailable_deployment(namespace: &str, name: &str) -> Deployment {
        let mut deployment = fake_deployment(namespace, name, &["checkout:v2"]);
        deployment.status = Some(DeploymentStatus {
            available_replicas: Some(0),
            observed_generation: Some(2),
            ready_replicas: Some(0),
            replicas: Some(3),
            updated_replicas: Some(1),
            unavailable_replicas: Some(3),
            ..DeploymentStatus::default()
        });
        deployment
    }

    fn fake_pod(
        namespace: &str,
        name: &str,
        containers: &[(&str, &str)],
        owner_references: Vec<OwnerReference>,
    ) -> Pod {
        Pod {
            metadata: ObjectMeta {
                name: Some(name.to_owned()),
                namespace: Some(namespace.to_owned()),
                owner_references: Some(owner_references),
                ..ObjectMeta::default()
            },
            spec: Some(PodSpec {
                containers: containers
                    .iter()
                    .map(|(name, image)| Container {
                        name: (*name).to_owned(),
                        image: Some((*image).to_owned()),
                        liveness_probe: (*name == "checkout").then(|| tcp_probe(8080)),
                        readiness_probe: (*name == "checkout")
                            .then(|| http_probe("/ready", "http")),
                        resources: (*name == "checkout").then(checkout_resources),
                        ..Container::default()
                    })
                    .collect(),
                node_name: Some("worker-a".to_owned()),
                ..PodSpec::default()
            }),
            status: Some(PodStatus {
                phase: Some("Running".to_owned()),
                pod_ip: Some("10.244.0.12".to_owned()),
                ..PodStatus::default()
            }),
        }
    }

    fn http_probe(path: &str, port: &str) -> Probe {
        Probe {
            http_get: Some(HTTPGetAction {
                http_headers: Some(vec![HTTPHeader {
                    name: "Authorization".to_owned(),
                    value: "Bearer redacted".to_owned(),
                }]),
                path: Some(path.to_owned()),
                port: IntOrString::String(port.to_owned()),
                scheme: Some("HTTP".to_owned()),
                ..HTTPGetAction::default()
            }),
            failure_threshold: Some(3),
            initial_delay_seconds: Some(5),
            period_seconds: Some(10),
            success_threshold: Some(1),
            timeout_seconds: Some(2),
            ..Probe::default()
        }
    }

    fn tcp_probe(port: i32) -> Probe {
        Probe {
            tcp_socket: Some(TCPSocketAction {
                port: IntOrString::Int(port),
                ..TCPSocketAction::default()
            }),
            failure_threshold: Some(5),
            initial_delay_seconds: Some(15),
            period_seconds: Some(20),
            success_threshold: Some(1),
            termination_grace_period_seconds: Some(30),
            timeout_seconds: Some(3),
            ..Probe::default()
        }
    }

    fn checkout_resources() -> ResourceRequirements {
        ResourceRequirements {
            limits: Some(BTreeMap::from([
                ("cpu".to_owned(), Quantity("500m".to_owned())),
                ("memory".to_owned(), Quantity("512Mi".to_owned())),
            ])),
            requests: Some(BTreeMap::from([
                ("cpu".to_owned(), Quantity("250m".to_owned())),
                ("memory".to_owned(), Quantity("256Mi".to_owned())),
            ])),
            ..ResourceRequirements::default()
        }
    }

    fn http_probe_summary(path: &str, port: &str) -> ProbeSummary {
        ProbeSummary {
            handler: ProbeHandlerSummary::HttpGet {
                host: None,
                path: Some(path.to_owned()),
                port: port.to_owned(),
                scheme: Some("HTTP".to_owned()),
                header_count: 1,
            },
            failure_threshold: Some(3),
            initial_delay_seconds: Some(5),
            period_seconds: Some(10),
            success_threshold: Some(1),
            termination_grace_period_seconds: None,
            timeout_seconds: Some(2),
        }
    }

    fn tcp_probe_summary(port: i32) -> ProbeSummary {
        ProbeSummary {
            handler: ProbeHandlerSummary::TcpSocket {
                host: None,
                port: port.to_string(),
            },
            failure_threshold: Some(5),
            initial_delay_seconds: Some(15),
            period_seconds: Some(20),
            success_threshold: Some(1),
            termination_grace_period_seconds: Some(30),
            timeout_seconds: Some(3),
        }
    }

    fn checkout_resource_summary(container_name: &str) -> ContainerResourceSummary {
        ContainerResourceSummary {
            container_name: container_name.to_owned(),
            requests: vec![
                ResourceQuantitySummary {
                    name: "cpu".to_owned(),
                    quantity: "250m".to_owned(),
                },
                ResourceQuantitySummary {
                    name: "memory".to_owned(),
                    quantity: "256Mi".to_owned(),
                },
            ],
            limits: vec![
                ResourceQuantitySummary {
                    name: "cpu".to_owned(),
                    quantity: "500m".to_owned(),
                },
                ResourceQuantitySummary {
                    name: "memory".to_owned(),
                    quantity: "512Mi".to_owned(),
                },
            ],
        }
    }

    fn fake_ingress(namespace: &str, name: &str) -> Ingress {
        Ingress {
            metadata: ObjectMeta {
                name: Some(name.to_owned()),
                namespace: Some(namespace.to_owned()),
                ..ObjectMeta::default()
            },
            spec: Some(IngressSpec {
                ingress_class_name: Some("nginx".to_owned()),
                default_backend: Some(ingress_backend_name_port("checkout-http", "http")),
                rules: Some(vec![IngressRule {
                    host: Some("checkout.example.com".to_owned()),
                    http: Some(HTTPIngressRuleValue {
                        paths: vec![
                            HTTPIngressPath {
                                path: Some("/".to_owned()),
                                path_type: "Prefix".to_owned(),
                                backend: ingress_backend_number_port("checkout-http", 80),
                            },
                            HTTPIngressPath {
                                path: Some("/metrics".to_owned()),
                                path_type: "Exact".to_owned(),
                                backend: ingress_backend_name_port("checkout-metrics", "metrics"),
                            },
                        ],
                    }),
                }]),
                tls: Some(vec![IngressTLS {
                    hosts: Some(vec!["checkout.example.com".to_owned()]),
                    secret_name: Some("checkout-tls".to_owned()),
                }]),
            }),
            ..Ingress::default()
        }
    }

    fn ingress_backend_name_port(service_name: &str, port_name: &str) -> IngressBackend {
        IngressBackend {
            service: Some(IngressServiceBackend {
                name: service_name.to_owned(),
                port: Some(ServiceBackendPort {
                    name: Some(port_name.to_owned()),
                    ..ServiceBackendPort::default()
                }),
            }),
            ..IngressBackend::default()
        }
    }

    fn ingress_backend_number_port(service_name: &str, port_number: i32) -> IngressBackend {
        IngressBackend {
            service: Some(IngressServiceBackend {
                name: service_name.to_owned(),
                port: Some(ServiceBackendPort {
                    number: Some(port_number),
                    ..ServiceBackendPort::default()
                }),
            }),
            ..IngressBackend::default()
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
