//! Runtime checks for Kply verification workflows.

use kply_core::CheckResultStatus;
use serde::Serialize;

/// Input facts for evaluating one Kubernetes pod readiness check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PodReadinessInput {
    name: String,
    phase: Option<String>,
    ready: Option<bool>,
}

impl PodReadinessInput {
    /// Create pod readiness input from Kubernetes pod facts.
    pub fn new(name: impl Into<String>, phase: Option<String>, ready: Option<bool>) -> Self {
        Self {
            name: name.into(),
            phase,
            ready,
        }
    }

    /// Borrow the pod name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Borrow the observed Kubernetes pod phase.
    pub fn phase(&self) -> Option<&str> {
        self.phase.as_deref()
    }

    /// Return the observed pod readiness condition.
    pub const fn ready(&self) -> Option<bool> {
        self.ready
    }
}

/// Summary produced by the pod readiness check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PodReadinessCheckResult {
    status: CheckResultStatus,
    total_pods: usize,
    ready_pods: usize,
    not_ready_pods: usize,
    unknown_pods: usize,
}

impl PodReadinessCheckResult {
    /// Return the stable check result status.
    pub const fn status(&self) -> CheckResultStatus {
        self.status
    }

    /// Return the number of pods considered by the check.
    pub const fn total_pods(&self) -> usize {
        self.total_pods
    }

    /// Return the number of pods observed as ready.
    pub const fn ready_pods(&self) -> usize {
        self.ready_pods
    }

    /// Return the number of pods observed as not ready.
    pub const fn not_ready_pods(&self) -> usize {
        self.not_ready_pods
    }

    /// Return the number of pods without enough readiness evidence.
    pub const fn unknown_pods(&self) -> usize {
        self.unknown_pods
    }
}

/// Evaluate whether all observed pods are ready.
pub fn check_pod_readiness(pods: &[PodReadinessInput]) -> PodReadinessCheckResult {
    let total_pods = pods.len();
    let ready_pods = pods.iter().filter(|pod| pod_is_ready(pod)).count();
    let not_ready_pods = pods.iter().filter(|pod| pod_is_not_ready(pod)).count();
    let unknown_pods = total_pods
        .saturating_sub(ready_pods)
        .saturating_sub(not_ready_pods);
    let status = if total_pods == 0 {
        CheckResultStatus::Skipped
    } else if not_ready_pods > 0 {
        CheckResultStatus::Failed
    } else if unknown_pods > 0 {
        CheckResultStatus::Warning
    } else {
        CheckResultStatus::Passed
    };

    PodReadinessCheckResult {
        status,
        total_pods,
        ready_pods,
        not_ready_pods,
        unknown_pods,
    }
}

/// Return whether a pod has positive readiness evidence.
fn pod_is_ready(pod: &PodReadinessInput) -> bool {
    pod.ready == Some(true) && !pod_has_not_ready_phase(pod)
}

/// Return whether a pod has negative readiness evidence.
fn pod_is_not_ready(pod: &PodReadinessInput) -> bool {
    pod.ready == Some(false) || pod_has_not_ready_phase(pod)
}

/// Return whether a pod phase cannot serve workload traffic.
fn pod_has_not_ready_phase(pod: &PodReadinessInput) -> bool {
    matches!(
        pod.phase(),
        Some("Pending" | "Succeeded" | "Failed" | "Unknown")
    )
}

/// Input facts for evaluating one workload rollout availability check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RolloutAvailabilityInput {
    workload: String,
    desired_replicas: Option<u32>,
    ready_replicas: Option<u32>,
    available_replicas: Option<u32>,
    updated_replicas: Option<u32>,
    unavailable_replicas: Option<u32>,
}

impl RolloutAvailabilityInput {
    /// Create rollout availability input from workload replica facts.
    pub fn new(
        workload: impl Into<String>,
        desired_replicas: Option<u32>,
        ready_replicas: Option<u32>,
        available_replicas: Option<u32>,
        updated_replicas: Option<u32>,
        unavailable_replicas: Option<u32>,
    ) -> Self {
        Self {
            workload: workload.into(),
            desired_replicas,
            ready_replicas,
            available_replicas,
            updated_replicas,
            unavailable_replicas,
        }
    }

    /// Borrow the workload name.
    pub fn workload(&self) -> &str {
        &self.workload
    }

    /// Return the desired replica count.
    pub const fn desired_replicas(&self) -> Option<u32> {
        self.desired_replicas
    }

    /// Return the ready replica count.
    pub const fn ready_replicas(&self) -> Option<u32> {
        self.ready_replicas
    }

    /// Return the available replica count.
    pub const fn available_replicas(&self) -> Option<u32> {
        self.available_replicas
    }

    /// Return the updated replica count.
    pub const fn updated_replicas(&self) -> Option<u32> {
        self.updated_replicas
    }

    /// Return the unavailable replica count.
    pub const fn unavailable_replicas(&self) -> Option<u32> {
        self.unavailable_replicas
    }
}

/// Summary produced by the rollout availability check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RolloutAvailabilityCheckResult {
    status: CheckResultStatus,
    workload: String,
    desired_replicas: Option<u32>,
    ready_replicas: Option<u32>,
    available_replicas: Option<u32>,
    updated_replicas: Option<u32>,
    unavailable_replicas: Option<u32>,
}

impl RolloutAvailabilityCheckResult {
    /// Return the stable check result status.
    pub const fn status(&self) -> CheckResultStatus {
        self.status
    }

    /// Borrow the evaluated workload name.
    pub fn workload(&self) -> &str {
        &self.workload
    }

    /// Return the desired replica count.
    pub const fn desired_replicas(&self) -> Option<u32> {
        self.desired_replicas
    }

    /// Return the ready replica count.
    pub const fn ready_replicas(&self) -> Option<u32> {
        self.ready_replicas
    }

    /// Return the available replica count.
    pub const fn available_replicas(&self) -> Option<u32> {
        self.available_replicas
    }

    /// Return the updated replica count.
    pub const fn updated_replicas(&self) -> Option<u32> {
        self.updated_replicas
    }

    /// Return the unavailable replica count.
    pub const fn unavailable_replicas(&self) -> Option<u32> {
        self.unavailable_replicas
    }
}

/// Evaluate whether a workload rollout is fully available.
pub fn check_rollout_availability(
    rollout: &RolloutAvailabilityInput,
) -> RolloutAvailabilityCheckResult {
    let status = if rollout.desired_replicas == Some(0) {
        CheckResultStatus::Skipped
    } else if rollout_has_missing_evidence(rollout) {
        CheckResultStatus::Warning
    } else if rollout_has_complete_availability(rollout) {
        CheckResultStatus::Passed
    } else {
        CheckResultStatus::Failed
    };

    RolloutAvailabilityCheckResult {
        status,
        workload: rollout.workload.clone(),
        desired_replicas: rollout.desired_replicas,
        ready_replicas: rollout.ready_replicas,
        available_replicas: rollout.available_replicas,
        updated_replicas: rollout.updated_replicas,
        unavailable_replicas: rollout.unavailable_replicas,
    }
}

/// Return whether a rollout lacks required replica evidence.
fn rollout_has_missing_evidence(rollout: &RolloutAvailabilityInput) -> bool {
    rollout.desired_replicas.is_none()
        || rollout.ready_replicas.is_none()
        || rollout.available_replicas.is_none()
        || rollout.updated_replicas.is_none()
        || rollout.unavailable_replicas.is_none()
}

/// Return whether a rollout has reached full replica availability.
fn rollout_has_complete_availability(rollout: &RolloutAvailabilityInput) -> bool {
    let Some(desired_replicas) = rollout.desired_replicas else {
        return false;
    };
    let Some(ready_replicas) = rollout.ready_replicas else {
        return false;
    };
    let Some(available_replicas) = rollout.available_replicas else {
        return false;
    };
    let Some(updated_replicas) = rollout.updated_replicas else {
        return false;
    };
    let Some(unavailable_replicas) = rollout.unavailable_replicas else {
        return false;
    };

    desired_replicas > 0
        && ready_replicas >= desired_replicas
        && available_replicas >= desired_replicas
        && updated_replicas >= desired_replicas
        && unavailable_replicas == 0
}

#[cfg(test)]
mod tests {
    use super::{
        PodReadinessInput, RolloutAvailabilityInput, check_pod_readiness,
        check_rollout_availability,
    };
    use kply_core::CheckResultStatus;

    /// Builds a pod readiness input fixture.
    fn pod(name: &str, phase: Option<&str>, ready: Option<bool>) -> PodReadinessInput {
        PodReadinessInput::new(name, phase.map(ToOwned::to_owned), ready)
    }

    /// Builds a rollout availability input fixture.
    fn rollout(
        desired_replicas: Option<u32>,
        ready_replicas: Option<u32>,
        available_replicas: Option<u32>,
        updated_replicas: Option<u32>,
        unavailable_replicas: Option<u32>,
    ) -> RolloutAvailabilityInput {
        RolloutAvailabilityInput::new(
            "checkout-api",
            desired_replicas,
            ready_replicas,
            available_replicas,
            updated_replicas,
            unavailable_replicas,
        )
    }

    #[test]
    /// Passes when every observed pod is running and ready.
    fn passes_when_all_pods_are_ready() {
        let result = check_pod_readiness(&[
            pod("checkout-a", Some("Running"), Some(true)),
            pod("checkout-b", Some("Running"), Some(true)),
        ]);

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(result.total_pods(), 2);
        assert_eq!(result.ready_pods(), 2);
        assert_eq!(result.not_ready_pods(), 0);
        assert_eq!(result.unknown_pods(), 0);
    }

    #[test]
    /// Fails when any pod reports negative readiness evidence.
    fn fails_when_any_pod_is_not_ready() {
        let result = check_pod_readiness(&[
            pod("checkout-a", Some("Running"), Some(true)),
            pod("checkout-b", Some("Running"), Some(false)),
            pod("checkout-c", Some("Pending"), None),
        ]);

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.total_pods(), 3);
        assert_eq!(result.ready_pods(), 1);
        assert_eq!(result.not_ready_pods(), 2);
        assert_eq!(result.unknown_pods(), 0);
    }

    #[test]
    /// Fails when a terminal pod phase conflicts with stale ready state.
    fn fails_when_terminal_phase_has_stale_ready_state() {
        let result = check_pod_readiness(&[pod("checkout-a", Some("Succeeded"), Some(true))]);

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.total_pods(), 1);
        assert_eq!(result.ready_pods(), 0);
        assert_eq!(result.not_ready_pods(), 1);
        assert_eq!(result.unknown_pods(), 0);
    }

    #[test]
    /// Warns when a pod lacks readiness evidence.
    fn warns_when_readiness_evidence_is_unknown() {
        let result = check_pod_readiness(&[
            pod("checkout-a", Some("Running"), Some(true)),
            pod("checkout-b", Some("Running"), None),
        ]);

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.total_pods(), 2);
        assert_eq!(result.ready_pods(), 1);
        assert_eq!(result.not_ready_pods(), 0);
        assert_eq!(result.unknown_pods(), 1);
    }

    #[test]
    /// Skips when no pods are available for evaluation.
    fn skips_when_no_pods_are_available() {
        let result = check_pod_readiness(&[]);

        assert_eq!(result.status(), CheckResultStatus::Skipped);
        assert_eq!(result.total_pods(), 0);
        assert_eq!(result.ready_pods(), 0);
        assert_eq!(result.not_ready_pods(), 0);
        assert_eq!(result.unknown_pods(), 0);
    }

    #[test]
    /// Passes when the rollout has every desired replica ready and available.
    fn passes_when_rollout_is_fully_available() {
        let result =
            check_rollout_availability(&rollout(Some(3), Some(3), Some(3), Some(3), Some(0)));

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(result.workload(), "checkout-api");
        assert_eq!(result.desired_replicas(), Some(3));
        assert_eq!(result.ready_replicas(), Some(3));
        assert_eq!(result.available_replicas(), Some(3));
        assert_eq!(result.updated_replicas(), Some(3));
        assert_eq!(result.unavailable_replicas(), Some(0));
    }

    #[test]
    /// Fails when known rollout facts show incomplete availability.
    fn fails_when_rollout_is_not_fully_available() {
        let result =
            check_rollout_availability(&rollout(Some(3), Some(2), Some(2), Some(3), Some(1)));

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.desired_replicas(), Some(3));
        assert_eq!(result.ready_replicas(), Some(2));
        assert_eq!(result.available_replicas(), Some(2));
        assert_eq!(result.updated_replicas(), Some(3));
        assert_eq!(result.unavailable_replicas(), Some(1));
    }

    #[test]
    /// Warns when rollout replica evidence is incomplete.
    fn warns_when_rollout_evidence_is_missing() {
        let result = check_rollout_availability(&rollout(Some(3), Some(3), None, Some(3), Some(0)));

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.desired_replicas(), Some(3));
        assert_eq!(result.available_replicas(), None);
    }

    #[test]
    /// Skips when a rollout intentionally targets zero replicas.
    fn skips_when_rollout_has_no_desired_replicas() {
        let result =
            check_rollout_availability(&rollout(Some(0), Some(0), Some(0), Some(0), Some(0)));

        assert_eq!(result.status(), CheckResultStatus::Skipped);
        assert_eq!(result.desired_replicas(), Some(0));
    }
}
