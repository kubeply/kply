//! Sandbox Kubernetes manifest generation.

use crate::{MetadataEntry, SessionPlan};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

/// Generate a sandbox Kubernetes Deployment manifest from a dry-run session plan.
pub fn sandbox_deployment_manifest(
    plan: &SessionPlan,
) -> Result<SandboxDeploymentManifest, SandboxManifestError> {
    let deployments = plan
        .planned_resources()
        .iter()
        .filter(|resource| resource.kind() == "Deployment")
        .collect::<Vec<_>>();
    let [deployment] = deployments.as_slice() else {
        return Err(match deployments.len() {
            0 => SandboxManifestError::MissingDeploymentResource,
            _ => SandboxManifestError::MultipleDeploymentResources,
        });
    };

    let labels = metadata_entries_to_map(plan.planned_labels());
    if labels.is_empty() {
        return Err(SandboxManifestError::MissingSelectorLabels);
    }
    let annotations = metadata_entries_to_map(plan.planned_annotations());

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

/// Kubernetes Deployment manifest generated for a sandbox session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SandboxDeploymentManifest {
    #[serde(rename = "apiVersion")]
    api_version: &'static str,
    kind: &'static str,
    metadata: SandboxObjectMetadata,
    spec: SandboxDeploymentSpec,
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
    /// The session plan did not include labels for the Deployment selector.
    MissingSelectorLabels,
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
            Self::MissingSelectorLabels => {
                formatter.write_str("session plan does not include selector labels")
            }
        }
    }
}

impl std::error::Error for SandboxManifestError {}

fn metadata_entries_to_map(metadata: &[MetadataEntry]) -> BTreeMap<String, String> {
    metadata
        .iter()
        .map(|entry| (entry.key().to_owned(), entry.value().to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{SandboxManifestError, sandbox_deployment_manifest};
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

    #[test]
    fn generates_sandbox_deployment_manifest() {
        let plan = test_session_plan()
            .with_planned_resources([
                KubernetesResourceRef::new("checkout", "Service", "session-123-service")
                    .expect("planned service"),
                KubernetesResourceRef::new("checkout", "Deployment", "session-123-workload")
                    .expect("planned workload"),
            ])
            .with_planned_labels([
                MetadataEntry::new_label("kply.dev/app", "checkout").expect("label"),
                MetadataEntry::new_label("kply.dev/managed-by", "kply").expect("label"),
                MetadataEntry::new_label("kply.dev/session-id", "session-123").expect("label"),
            ])
            .expect("planned labels")
            .with_planned_annotations([
                MetadataEntry::new("kply.dev/image", "registry.example.com/checkout/api:v2")
                    .expect("annotation"),
                MetadataEntry::new("kply.dev/workload", "checkout/Deployment/checkout-api")
                    .expect("annotation"),
            ]);

        let manifest = sandbox_deployment_manifest(&plan).expect("deployment manifest");
        let value = serde_json::to_value(manifest).expect("manifest should serialize");

        insta::assert_json_snapshot!("sandbox_deployment_manifest", value);
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
}
