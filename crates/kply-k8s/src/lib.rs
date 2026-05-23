use kply_core::SessionPlan;

/// Kubernetes execution mode for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    DryRun,
    Apply,
}

/// Planned Kubernetes action. This remains provider-neutral until adapters are
/// implemented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KubernetesAction {
    pub description: String,
}

#[must_use]
pub fn plan_session_actions(plan: &SessionPlan, mode: ExecutionMode) -> Vec<KubernetesAction> {
    let mode_label = match mode {
        ExecutionMode::DryRun => "dry-run",
        ExecutionMode::Apply => "apply",
    };

    vec![
        KubernetesAction {
            description: format!(
                "{mode_label}: create sandbox deployment for {}/{}",
                plan.workload.namespace, plan.workload.name
            ),
        },
        KubernetesAction {
            description: format!(
                "{mode_label}: create sandbox service for {}/{}",
                plan.workload.namespace, plan.workload.name
            ),
        },
    ]
}
