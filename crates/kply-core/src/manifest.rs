//! Sandbox Kubernetes manifest generation.

use crate::{KubernetesResourceRef, MetadataEntry, SessionPlan};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

const REQUIRED_OWNERSHIP_LABELS: [&str; 4] = [
    "kply.dev/app",
    "kply.dev/managed-by",
    "kply.dev/session-id",
    "kply.dev/session-name",
];
const REQUIRED_AUDIT_ANNOTATIONS: [&str; 3] = [
    "kply.dev/image",
    "kply.dev/route-strategy",
    "kply.dev/workload",
];

/// Stable application identity labels safe to preserve in sandbox manifests.
///
/// Keep this allowlist limited to generic, non-sensitive app identity metadata.
/// Exclude controller, rollout, and version labels because sandbox sessions can
/// run a different image than production and should not inherit stale identity.
const SAFE_APP_LABELS: [&str; 4] = [
    "app.kubernetes.io/component",
    "app.kubernetes.io/instance",
    "app.kubernetes.io/name",
    "app.kubernetes.io/part-of",
];

/// Generate a sandbox Kubernetes Deployment manifest from a dry-run session plan.
pub fn sandbox_deployment_manifest(
    plan: &SessionPlan,
) -> Result<SandboxDeploymentManifest, SandboxManifestError> {
    let deployment = unique_planned_resource(plan, "Deployment")?;
    let planned_labels = metadata_entries_to_map(plan.planned_labels());
    ensure_ownership_labels(&planned_labels)?;
    let labels = sandbox_labels(&planned_labels);
    let planned_annotations = metadata_entries_to_map(plan.planned_annotations());
    ensure_audit_annotations(&planned_annotations)?;
    let annotations = sandbox_annotations(&planned_annotations);

    Ok(SandboxDeploymentManifest {
        api_version: "apps/v1",
        kind: "Deployment",
        metadata: SandboxObjectMetadata {
            name: deployment.name().to_owned(),
            namespace: deployment.namespace().to_owned(),
            labels: labels.clone(),
            annotations: annotations.clone(),
        },
        spec: SandboxDeploymentSpec {
            replicas: 1,
            selector: SandboxLabelSelector {
                match_labels: labels.clone(),
            },
            template: SandboxPodTemplate {
                metadata: SandboxPodTemplateMetadata {
                    labels,
                    annotations,
                },
                spec: SandboxPodSpec {
                    containers: vec![SandboxContainer {
                        name: plan.name().as_str().to_owned(),
                        image: plan.image().as_str().to_owned(),
                    }],
                },
            },
        },
    })
}

/// Generate a sandbox Kubernetes Service manifest from a dry-run session plan.
pub fn sandbox_service_manifest(
    plan: &SessionPlan,
) -> Result<SandboxServiceManifest, SandboxManifestError> {
    sandbox_service_manifest_with_port(plan, SandboxServicePortConfig::http_default())
}

/// Generate a sandbox Kubernetes Service manifest with explicit port settings.
pub fn sandbox_service_manifest_with_port(
    plan: &SessionPlan,
    port_config: SandboxServicePortConfig,
) -> Result<SandboxServiceManifest, SandboxManifestError> {
    let service = unique_planned_resource(plan, "Service")?;
    let planned_labels = metadata_entries_to_map(plan.planned_labels());
    ensure_ownership_labels(&planned_labels)?;
    let labels = sandbox_labels(&planned_labels);
    let planned_annotations = metadata_entries_to_map(plan.planned_annotations());
    ensure_audit_annotations(&planned_annotations)?;
    let annotations = sandbox_annotations(&planned_annotations);

    Ok(SandboxServiceManifest {
        api_version: "v1",
        kind: "Service",
        metadata: SandboxObjectMetadata {
            name: service.name().to_owned(),
            namespace: service.namespace().to_owned(),
            labels: labels.clone(),
            annotations,
        },
        spec: SandboxServiceSpec {
            selector: labels,
            ports: vec![SandboxServicePort {
                name: port_config.name,
                port: port_config.port,
                target_port: port_config.target_port,
            }],
        },
    })
}

/// Kubernetes Deployment manifest generated for a sandbox session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxDeploymentManifest {
    #[serde(rename = "apiVersion")]
    api_version: &'static str,
    kind: &'static str,
    metadata: SandboxObjectMetadata,
    spec: SandboxDeploymentSpec,
}

/// Kubernetes Service manifest generated for a sandbox session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxServiceManifest {
    #[serde(rename = "apiVersion")]
    api_version: &'static str,
    kind: &'static str,
    metadata: SandboxObjectMetadata,
    spec: SandboxServiceSpec,
}

/// Kubernetes object metadata for generated sandbox manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxObjectMetadata {
    name: String,
    namespace: String,
    labels: BTreeMap<String, String>,
    annotations: BTreeMap<String, String>,
}

/// Kubernetes Deployment spec for generated sandbox manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxDeploymentSpec {
    replicas: i32,
    selector: SandboxLabelSelector,
    template: SandboxPodTemplate,
}

/// Kubernetes Service spec for generated sandbox manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxServiceSpec {
    selector: BTreeMap<String, String>,
    ports: Vec<SandboxServicePort>,
}

/// Kubernetes Service port for generated sandbox manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxServicePort {
    name: String,
    port: i32,
    #[serde(rename = "targetPort")]
    target_port: i32,
}

/// Port settings used when generating a sandbox Service manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxServicePortConfig {
    name: String,
    port: i32,
    target_port: i32,
}

impl SandboxServicePortConfig {
    /// Create the default HTTP service port config.
    pub fn http_default() -> Self {
        Self {
            name: "http".to_owned(),
            port: 80,
            target_port: 80,
        }
    }

    /// Create a service port config from explicit values.
    pub fn new(
        name: impl Into<String>,
        port: i32,
        target_port: i32,
    ) -> Result<Self, SandboxManifestError> {
        let name = name.into();
        if !is_valid_service_port_name(&name) {
            return Err(SandboxManifestError::InvalidServicePortName);
        }
        validate_service_port("port", port)?;
        validate_service_port("targetPort", target_port)?;

        Ok(Self {
            name,
            port,
            target_port,
        })
    }
}

/// Kubernetes label selector for generated sandbox manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxLabelSelector {
    #[serde(rename = "matchLabels")]
    match_labels: BTreeMap<String, String>,
}

/// Kubernetes pod template for generated sandbox workloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxPodTemplate {
    metadata: SandboxPodTemplateMetadata,
    spec: SandboxPodSpec,
}

/// Kubernetes pod template metadata for generated sandbox workloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxPodTemplateMetadata {
    labels: BTreeMap<String, String>,
    annotations: BTreeMap<String, String>,
}

/// Kubernetes pod spec for generated sandbox workloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxPodSpec {
    containers: Vec<SandboxContainer>,
}

/// Kubernetes container spec for generated sandbox workloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxContainer {
    name: String,
    image: String,
}

/// Error returned when sandbox manifests cannot be generated from a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxManifestError {
    /// The session plan did not include a Deployment resource.
    MissingDeploymentResource,
    /// The session plan included more than one Deployment resource.
    MultipleDeploymentResources,
    /// The session plan did not include a Service resource.
    MissingServiceResource,
    /// The session plan included more than one Service resource.
    MultipleServiceResources,
    /// The session plan did not include labels for generated selectors.
    MissingSelectorLabels,
    /// The session plan did not include a required ownership label.
    MissingOwnershipLabel { key: &'static str },
    /// The session plan did not include a required audit annotation.
    MissingAuditAnnotation { key: &'static str },
    /// The requested resource kind is not supported for sandbox manifests.
    UnsupportedResourceKind { kind: String },
    /// The service port name was not a valid Kubernetes DNS label.
    InvalidServicePortName,
    /// The service port value was outside the Kubernetes port range.
    InvalidServicePort { field: &'static str, value: i32 },
}

impl fmt::Display for SandboxManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDeploymentResource => {
                formatter.write_str("session plan does not include a Deployment resource")
            }
            Self::MultipleDeploymentResources => {
                formatter.write_str("session plan includes multiple Deployment resources")
            }
            Self::MissingServiceResource => {
                formatter.write_str("session plan does not include a Service resource")
            }
            Self::MultipleServiceResources => {
                formatter.write_str("session plan includes multiple Service resources")
            }
            Self::MissingSelectorLabels => {
                formatter.write_str("session plan does not include selector labels")
            }
            Self::MissingOwnershipLabel { key } => {
                write!(
                    formatter,
                    "session plan does not include ownership label `{key}`"
                )
            }
            Self::MissingAuditAnnotation { key } => {
                write!(
                    formatter,
                    "session plan does not include audit annotation `{key}`"
                )
            }
            Self::UnsupportedResourceKind { kind } => {
                write!(
                    formatter,
                    "unsupported sandbox manifest resource kind `{kind}`"
                )
            }
            Self::InvalidServicePortName => {
                formatter.write_str("service port name must be a valid Kubernetes DNS label")
            }
            Self::InvalidServicePort { field, value } => {
                write!(
                    formatter,
                    "service {field} must be between 1 and 65535, got {value}"
                )
            }
        }
    }
}

impl std::error::Error for SandboxManifestError {}

fn unique_planned_resource<'a>(
    plan: &'a SessionPlan,
    kind: &str,
) -> Result<&'a KubernetesResourceRef, SandboxManifestError> {
    let resources = plan
        .planned_resources()
        .iter()
        .filter(|resource| resource.kind() == kind)
        .collect::<Vec<_>>();
    let [resource] = resources.as_slice() else {
        return Err(match (kind, resources.len()) {
            ("Deployment", 0) => SandboxManifestError::MissingDeploymentResource,
            ("Deployment", _) => SandboxManifestError::MultipleDeploymentResources,
            ("Service", 0) => SandboxManifestError::MissingServiceResource,
            ("Service", _) => SandboxManifestError::MultipleServiceResources,
            (kind, _) => SandboxManifestError::UnsupportedResourceKind {
                kind: kind.to_owned(),
            },
        });
    };

    Ok(resource)
}

fn ensure_ownership_labels(labels: &BTreeMap<String, String>) -> Result<(), SandboxManifestError> {
    if labels.is_empty() {
        return Err(SandboxManifestError::MissingSelectorLabels);
    }

    for key in REQUIRED_OWNERSHIP_LABELS {
        if !labels.contains_key(key) {
            return Err(SandboxManifestError::MissingOwnershipLabel { key });
        }
    }

    Ok(())
}

fn ensure_audit_annotations(
    annotations: &BTreeMap<String, String>,
) -> Result<(), SandboxManifestError> {
    for key in REQUIRED_AUDIT_ANNOTATIONS {
        if !annotations.contains_key(key) {
            return Err(SandboxManifestError::MissingAuditAnnotation { key });
        }
    }

    Ok(())
}

/// Return only required ownership labels and safe app identity labels.
///
/// This avoids copying controller-managed labels such as `pod-template-hash`
/// into generated manifests where they could create misleading selectors.
fn sandbox_labels(planned_labels: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    planned_labels
        .iter()
        .filter(|(key, _)| should_preserve_label(key))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// Check whether a planned label belongs in generated sandbox manifests.
fn should_preserve_label(key: &str) -> bool {
    REQUIRED_OWNERSHIP_LABELS.contains(&key) || SAFE_APP_LABELS.contains(&key)
}

/// Return only Kply audit annotations for generated sandbox manifests.
///
/// Production annotations often control ingress, sidecars, policy, or external
/// integrations. They are intentionally not copied unless Kply owns the key.
fn sandbox_annotations(planned_annotations: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    planned_annotations
        .iter()
        .filter(|(key, _)| should_preserve_annotation(key))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// Check whether a planned annotation belongs in generated sandbox manifests.
fn should_preserve_annotation(key: &str) -> bool {
    REQUIRED_AUDIT_ANNOTATIONS.contains(&key)
}

fn metadata_entries_to_map(metadata: &[MetadataEntry]) -> BTreeMap<String, String> {
    metadata
        .iter()
        .map(|entry| (entry.key().to_owned(), entry.value().to_owned()))
        .collect()
}

fn validate_service_port(field: &'static str, value: i32) -> Result<(), SandboxManifestError> {
    if (1..=65535).contains(&value) {
        return Ok(());
    }

    Err(SandboxManifestError::InvalidServicePort { field, value })
}

fn is_valid_service_port_name(value: &str) -> bool {
    if value.is_empty() || value.len() > 63 {
        return false;
    }

    let mut characters = value.chars();
    let first_character = characters.next().unwrap_or_default();
    let last_character = value.chars().next_back().unwrap_or_default();

    is_lowercase_ascii_alphanumeric(first_character)
        && is_lowercase_ascii_alphanumeric(last_character)
        && value
            .chars()
            .all(|character| is_lowercase_ascii_alphanumeric(character) || character == '-')
}

fn is_lowercase_ascii_alphanumeric(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::{
        REQUIRED_AUDIT_ANNOTATIONS, SandboxManifestError, SandboxServicePortConfig,
        sandbox_deployment_manifest, sandbox_service_manifest, sandbox_service_manifest_with_port,
    };
    use crate::{
        ImageRef, KubernetesResourceRef, MetadataEntry, SessionId, SessionName, SessionPlan,
        SessionPolicy, WorkloadRef,
    };

    fn test_session_plan() -> SessionPlan {
        SessionPlan::new(
            SessionId::new("session-123").expect("session id"),
            SessionName::new("checkout-test").expect("session name"),
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
            ImageRef::new("registry.example.com/checkout/api:v2").expect("image ref"),
            SessionPolicy::sandbox(),
        )
    }

    fn test_labeled_session_plan() -> SessionPlan {
        test_session_plan()
            .with_planned_resources([
                KubernetesResourceRef::new("checkout", "Service", "session-123-service")
                    .expect("planned service"),
                KubernetesResourceRef::new("checkout", "Deployment", "session-123-workload")
                    .expect("planned workload"),
            ])
            .with_planned_labels([
                MetadataEntry::new_label("app.kubernetes.io/component", "api").expect("label"),
                MetadataEntry::new_label("kply.dev/app", "checkout").expect("label"),
                MetadataEntry::new_label("kply.dev/managed-by", "kply").expect("label"),
                MetadataEntry::new_label("kply.dev/session-id", "session-123").expect("label"),
                MetadataEntry::new_label("kply.dev/session-name", "checkout-test").expect("label"),
                MetadataEntry::new_label("pod-template-hash", "abc123").expect("label"),
            ])
            .expect("planned labels")
            .with_planned_annotations([
                MetadataEntry::new("kply.dev/image", "registry.example.com/checkout/api:v2")
                    .expect("annotation"),
                MetadataEntry::new("kply.dev/route-strategy", "header").expect("annotation"),
                MetadataEntry::new("kply.dev/workload", "checkout/Deployment/checkout-api")
                    .expect("annotation"),
                MetadataEntry::new("nginx.ingress.kubernetes.io/rewrite-target", "/")
                    .expect("annotation"),
            ])
    }

    fn test_ownership_labels() -> Vec<MetadataEntry> {
        vec![
            MetadataEntry::new_label("kply.dev/app", "checkout").expect("label"),
            MetadataEntry::new_label("kply.dev/managed-by", "kply").expect("label"),
            MetadataEntry::new_label("kply.dev/session-id", "session-123").expect("label"),
            MetadataEntry::new_label("kply.dev/session-name", "checkout-test").expect("label"),
        ]
    }

    fn test_audit_annotations_except(missing_key: &str) -> Vec<MetadataEntry> {
        let mut annotations = vec![
            MetadataEntry::new("kply.dev/image", "registry.example.com/checkout/api:v2")
                .expect("annotation"),
            MetadataEntry::new("kply.dev/route-strategy", "header").expect("annotation"),
            MetadataEntry::new("kply.dev/workload", "checkout/Deployment/checkout-api")
                .expect("annotation"),
        ];
        annotations.retain(|entry| entry.key() != missing_key);
        annotations
    }

    #[test]
    fn generates_sandbox_deployment_manifest() {
        let plan = test_labeled_session_plan();
        let manifest = sandbox_deployment_manifest(&plan).expect("deployment manifest");
        let value = serde_json::to_value(manifest).expect("manifest should serialize");

        insta::assert_json_snapshot!("sandbox_deployment_manifest", value);
    }

    #[test]
    fn generates_sandbox_service_manifest() {
        let plan = test_labeled_session_plan();
        let manifest = sandbox_service_manifest(&plan).expect("service manifest");
        let value = serde_json::to_value(manifest).expect("manifest should serialize");

        insta::assert_json_snapshot!("sandbox_service_manifest", value);
    }

    #[test]
    fn generates_sandbox_service_manifest_with_explicit_port() {
        let plan = test_labeled_session_plan();
        let manifest = sandbox_service_manifest_with_port(
            &plan,
            SandboxServicePortConfig::new("https", 443, 8443).expect("port config"),
        )
        .expect("service manifest");
        let value = serde_json::to_value(manifest).expect("manifest should serialize");

        insta::assert_json_snapshot!("sandbox_service_manifest_with_explicit_port", value);
    }

    #[test]
    fn preserves_safe_app_labels_in_sandbox_deployment_manifest() {
        let plan = test_labeled_session_plan();
        let manifest = sandbox_deployment_manifest(&plan).expect("deployment manifest");
        let value = serde_json::to_value(manifest).expect("manifest should serialize");

        assert_eq!(
            value["metadata"]["labels"]["app.kubernetes.io/component"],
            "api"
        );
        assert_eq!(
            value["spec"]["template"]["metadata"]["labels"]["app.kubernetes.io/component"],
            "api"
        );
        assert!(
            value["metadata"]["labels"]
                .get("pod-template-hash")
                .is_none()
        );
        assert!(
            value["spec"]["template"]["metadata"]["labels"]
                .get("pod-template-hash")
                .is_none()
        );
    }

    #[test]
    fn preserves_safe_app_labels_in_sandbox_service_manifest() {
        let plan = test_labeled_session_plan();
        let manifest = sandbox_service_manifest(&plan).expect("service manifest");
        let value = serde_json::to_value(manifest).expect("manifest should serialize");

        assert_eq!(
            value["metadata"]["labels"]["app.kubernetes.io/component"],
            "api"
        );
        assert_eq!(
            value["spec"]["selector"]["app.kubernetes.io/component"],
            "api"
        );
        assert!(
            value["metadata"]["labels"]
                .get("pod-template-hash")
                .is_none()
        );
        assert!(value["spec"]["selector"].get("pod-template-hash").is_none());
    }

    #[test]
    fn filters_unsafe_annotations_from_sandbox_deployment_manifest() {
        let plan = test_labeled_session_plan();
        let manifest = sandbox_deployment_manifest(&plan).expect("deployment manifest");
        let value = serde_json::to_value(manifest).expect("manifest should serialize");

        assert!(
            value["metadata"]["annotations"]
                .get("nginx.ingress.kubernetes.io/rewrite-target")
                .is_none()
        );
        assert!(
            value["spec"]["template"]["metadata"]["annotations"]
                .get("nginx.ingress.kubernetes.io/rewrite-target")
                .is_none()
        );
    }

    #[test]
    fn filters_unsafe_annotations_from_sandbox_service_manifest() {
        let plan = test_labeled_session_plan();
        let manifest = sandbox_service_manifest(&plan).expect("service manifest");
        let value = serde_json::to_value(manifest).expect("manifest should serialize");

        assert!(
            value["metadata"]["annotations"]
                .get("nginx.ingress.kubernetes.io/rewrite-target")
                .is_none()
        );
    }

    #[test]
    fn rejects_sandbox_deployment_manifest_without_deployment_resource() {
        let plan = test_session_plan().with_planned_resources([KubernetesResourceRef::new(
            "checkout",
            "Service",
            "session-123-service",
        )
        .expect("planned service")]);

        let error = sandbox_deployment_manifest(&plan).expect_err("deployment should be required");

        assert_eq!(error, SandboxManifestError::MissingDeploymentResource);
        assert_eq!(
            error.to_string(),
            "session plan does not include a Deployment resource"
        );
    }

    #[test]
    fn rejects_sandbox_deployment_manifest_with_multiple_deployment_resources() {
        let plan = test_session_plan().with_planned_resources([
            KubernetesResourceRef::new("checkout", "Deployment", "session-123-alpha")
                .expect("first deployment"),
            KubernetesResourceRef::new("checkout", "Deployment", "session-123-beta")
                .expect("second deployment"),
        ]);

        let error = sandbox_deployment_manifest(&plan).expect_err("deployment should be unique");

        assert_eq!(error, SandboxManifestError::MultipleDeploymentResources);
        assert_eq!(
            error.to_string(),
            "session plan includes multiple Deployment resources"
        );
    }

    #[test]
    fn rejects_sandbox_deployment_manifest_without_selector_labels() {
        let plan = test_session_plan().with_planned_resources([KubernetesResourceRef::new(
            "checkout",
            "Deployment",
            "session-123-workload",
        )
        .expect("planned deployment")]);

        let error = sandbox_deployment_manifest(&plan).expect_err("labels should be required");

        assert_eq!(error, SandboxManifestError::MissingSelectorLabels);
        assert_eq!(
            error.to_string(),
            "session plan does not include selector labels"
        );
    }

    #[test]
    fn rejects_sandbox_deployment_manifest_without_ownership_labels() {
        let plan = test_session_plan()
            .with_planned_resources([KubernetesResourceRef::new(
                "checkout",
                "Deployment",
                "session-123-workload",
            )
            .expect("planned deployment")])
            .with_planned_labels([
                MetadataEntry::new_label("kply.dev/app", "checkout").expect("label"),
                MetadataEntry::new_label("kply.dev/managed-by", "kply").expect("label"),
                MetadataEntry::new_label("kply.dev/session-id", "session-123").expect("label"),
            ])
            .expect("planned labels");

        let error = sandbox_deployment_manifest(&plan).expect_err("ownership should be required");

        assert_eq!(
            error,
            SandboxManifestError::MissingOwnershipLabel {
                key: "kply.dev/session-name"
            }
        );
        assert_eq!(
            error.to_string(),
            "session plan does not include ownership label `kply.dev/session-name`"
        );
    }

    #[test]
    fn rejects_sandbox_service_manifest_without_ownership_labels() {
        let plan = test_session_plan()
            .with_planned_resources([KubernetesResourceRef::new(
                "checkout",
                "Service",
                "session-123-service",
            )
            .expect("planned service")])
            .with_planned_labels([
                MetadataEntry::new_label("kply.dev/app", "checkout").expect("label"),
                MetadataEntry::new_label("kply.dev/managed-by", "kply").expect("label"),
                MetadataEntry::new_label("kply.dev/session-id", "session-123").expect("label"),
            ])
            .expect("planned labels");

        let error = sandbox_service_manifest(&plan).expect_err("ownership should be required");

        assert_eq!(
            error,
            SandboxManifestError::MissingOwnershipLabel {
                key: "kply.dev/session-name"
            }
        );
        assert_eq!(
            error.to_string(),
            "session plan does not include ownership label `kply.dev/session-name`"
        );
    }

    #[test]
    fn rejects_sandbox_deployment_manifest_without_audit_annotations() {
        for missing_key in REQUIRED_AUDIT_ANNOTATIONS {
            let plan = test_session_plan()
                .with_planned_resources([KubernetesResourceRef::new(
                    "checkout",
                    "Deployment",
                    "session-123-workload",
                )
                .expect("planned deployment")])
                .with_planned_labels(test_ownership_labels())
                .expect("planned labels")
                .with_planned_annotations(test_audit_annotations_except(missing_key));

            let error = sandbox_deployment_manifest(&plan)
                .expect_err("audit annotations should be required");

            assert_eq!(
                error,
                SandboxManifestError::MissingAuditAnnotation { key: missing_key }
            );
            assert_eq!(
                error.to_string(),
                format!("session plan does not include audit annotation `{missing_key}`")
            );
        }
    }

    #[test]
    fn rejects_sandbox_service_manifest_without_audit_annotations() {
        for missing_key in REQUIRED_AUDIT_ANNOTATIONS {
            let plan = test_session_plan()
                .with_planned_resources([KubernetesResourceRef::new(
                    "checkout",
                    "Service",
                    "session-123-service",
                )
                .expect("planned service")])
                .with_planned_labels(test_ownership_labels())
                .expect("planned labels")
                .with_planned_annotations(test_audit_annotations_except(missing_key));

            let error =
                sandbox_service_manifest(&plan).expect_err("audit annotations should be required");

            assert_eq!(
                error,
                SandboxManifestError::MissingAuditAnnotation { key: missing_key }
            );
            assert_eq!(
                error.to_string(),
                format!("session plan does not include audit annotation `{missing_key}`")
            );
        }
    }

    #[test]
    fn rejects_sandbox_service_manifest_without_service_resource() {
        let plan = test_session_plan().with_planned_resources([KubernetesResourceRef::new(
            "checkout",
            "Deployment",
            "session-123-workload",
        )
        .expect("planned deployment")]);

        let error = sandbox_service_manifest(&plan).expect_err("service should be required");

        assert_eq!(error, SandboxManifestError::MissingServiceResource);
        assert_eq!(
            error.to_string(),
            "session plan does not include a Service resource"
        );
    }

    #[test]
    fn rejects_sandbox_service_manifest_with_multiple_service_resources() {
        let plan = test_session_plan().with_planned_resources([
            KubernetesResourceRef::new("checkout", "Service", "session-123-alpha")
                .expect("first service"),
            KubernetesResourceRef::new("checkout", "Service", "session-123-beta")
                .expect("second service"),
        ]);

        let error = sandbox_service_manifest(&plan).expect_err("service should be unique");

        assert_eq!(error, SandboxManifestError::MultipleServiceResources);
        assert_eq!(
            error.to_string(),
            "session plan includes multiple Service resources"
        );
    }

    #[test]
    fn rejects_sandbox_service_manifest_without_selector_labels() {
        let plan = test_session_plan().with_planned_resources([KubernetesResourceRef::new(
            "checkout",
            "Service",
            "session-123-service",
        )
        .expect("planned service")]);

        let error = sandbox_service_manifest(&plan).expect_err("labels should be required");

        assert_eq!(error, SandboxManifestError::MissingSelectorLabels);
        assert_eq!(
            error.to_string(),
            "session plan does not include selector labels"
        );
    }

    #[test]
    fn rejects_empty_sandbox_service_port_name() {
        let error =
            SandboxServicePortConfig::new("", 80, 80).expect_err("empty name should be rejected");

        assert_eq!(error, SandboxManifestError::InvalidServicePortName);
        assert_eq!(
            error.to_string(),
            "service port name must be a valid Kubernetes DNS label"
        );
    }

    #[test]
    fn rejects_invalid_sandbox_service_port_names() {
        for name in ["HTTP_API", "foo bar", "-http", "http-"] {
            let error = SandboxServicePortConfig::new(name, 80, 80)
                .expect_err("invalid name should be rejected");

            assert_eq!(error, SandboxManifestError::InvalidServicePortName);
        }

        let long_name = "a".repeat(64);
        let error = SandboxServicePortConfig::new(long_name, 80, 80)
            .expect_err("long name should be rejected");

        assert_eq!(error, SandboxManifestError::InvalidServicePortName);
    }

    #[test]
    fn rejects_invalid_sandbox_service_port_values() {
        let error =
            SandboxServicePortConfig::new("http", 0, 80).expect_err("port should be rejected");
        assert_eq!(
            error,
            SandboxManifestError::InvalidServicePort {
                field: "port",
                value: 0
            }
        );
        assert_eq!(
            error.to_string(),
            "service port must be between 1 and 65535, got 0"
        );

        let error = SandboxServicePortConfig::new("http", 80, 65536)
            .expect_err("target port should be rejected");
        assert_eq!(
            error,
            SandboxManifestError::InvalidServicePort {
                field: "targetPort",
                value: 65536
            }
        );
        assert_eq!(
            error.to_string(),
            "service targetPort must be between 1 and 65535, got 65536"
        );
    }
}
