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

/// Input facts for evaluating one Kubernetes Service endpoint check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceEndpointInput {
    service: String,
    declared_ports: usize,
    ready_endpoints: Option<usize>,
    not_ready_endpoints: Option<usize>,
    unknown_endpoints: Option<usize>,
}

impl ServiceEndpointInput {
    /// Create service endpoint input from Service and EndpointSlice facts.
    pub fn new(
        service: impl Into<String>,
        declared_ports: usize,
        ready_endpoints: Option<usize>,
        not_ready_endpoints: Option<usize>,
        unknown_endpoints: Option<usize>,
    ) -> Self {
        Self {
            service: service.into(),
            declared_ports,
            ready_endpoints,
            not_ready_endpoints,
            unknown_endpoints,
        }
    }

    /// Borrow the service name.
    pub fn service(&self) -> &str {
        &self.service
    }

    /// Return the declared Service port count.
    pub const fn declared_ports(&self) -> usize {
        self.declared_ports
    }

    /// Return the ready endpoint count.
    pub const fn ready_endpoints(&self) -> Option<usize> {
        self.ready_endpoints
    }

    /// Return the not-ready endpoint count.
    pub const fn not_ready_endpoints(&self) -> Option<usize> {
        self.not_ready_endpoints
    }

    /// Return the endpoint count without known readiness.
    pub const fn unknown_endpoints(&self) -> Option<usize> {
        self.unknown_endpoints
    }
}

/// Summary produced by the service endpoint check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceEndpointCheckResult {
    status: CheckResultStatus,
    service: String,
    declared_ports: usize,
    ready_endpoints: Option<usize>,
    not_ready_endpoints: Option<usize>,
    unknown_endpoints: Option<usize>,
}

impl ServiceEndpointCheckResult {
    /// Return the stable check result status.
    pub const fn status(&self) -> CheckResultStatus {
        self.status
    }

    /// Borrow the evaluated service name.
    pub fn service(&self) -> &str {
        &self.service
    }

    /// Return the declared Service port count.
    pub const fn declared_ports(&self) -> usize {
        self.declared_ports
    }

    /// Return the ready endpoint count.
    pub const fn ready_endpoints(&self) -> Option<usize> {
        self.ready_endpoints
    }

    /// Return the not-ready endpoint count.
    pub const fn not_ready_endpoints(&self) -> Option<usize> {
        self.not_ready_endpoints
    }

    /// Return the endpoint count without known readiness.
    pub const fn unknown_endpoints(&self) -> Option<usize> {
        self.unknown_endpoints
    }
}

/// Evaluate whether a Service has usable ready endpoints.
pub fn check_service_endpoints(service: &ServiceEndpointInput) -> ServiceEndpointCheckResult {
    let status = if service.declared_ports == 0 {
        CheckResultStatus::Skipped
    } else if service_has_missing_endpoint_evidence(service) {
        CheckResultStatus::Warning
    } else if service.ready_endpoints == Some(0) {
        CheckResultStatus::Failed
    } else if service_has_mixed_endpoint_evidence(service) {
        CheckResultStatus::Warning
    } else {
        CheckResultStatus::Passed
    };

    ServiceEndpointCheckResult {
        status,
        service: service.service.clone(),
        declared_ports: service.declared_ports,
        ready_endpoints: service.ready_endpoints,
        not_ready_endpoints: service.not_ready_endpoints,
        unknown_endpoints: service.unknown_endpoints,
    }
}

/// Return whether endpoint readiness evidence is incomplete.
fn service_has_missing_endpoint_evidence(service: &ServiceEndpointInput) -> bool {
    service.ready_endpoints.is_none()
        || service.not_ready_endpoints.is_none()
        || service.unknown_endpoints.is_none()
}

/// Return whether a Service has both usable and non-usable endpoints.
fn service_has_mixed_endpoint_evidence(service: &ServiceEndpointInput) -> bool {
    service.ready_endpoints.unwrap_or_default() > 0
        && (service.not_ready_endpoints.unwrap_or_default() > 0
            || service.unknown_endpoints.unwrap_or_default() > 0)
}

/// Input facts for evaluating one HTTP smoke check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HttpSmokeInput {
    target: Option<String>,
    expected_status_code: u16,
    observed_status_code: Option<u16>,
    transport_error: Option<String>,
}

impl HttpSmokeInput {
    /// Create HTTP smoke input from one completed probe attempt.
    pub fn new(
        target: Option<String>,
        expected_status_code: u16,
        observed_status_code: Option<u16>,
        transport_error: Option<String>,
    ) -> Self {
        Self {
            target,
            expected_status_code,
            observed_status_code,
            transport_error,
        }
    }

    /// Borrow the configured target.
    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    /// Return the expected HTTP status code.
    pub const fn expected_status_code(&self) -> u16 {
        self.expected_status_code
    }

    /// Return the observed HTTP status code.
    pub const fn observed_status_code(&self) -> Option<u16> {
        self.observed_status_code
    }

    /// Borrow the transport error label.
    pub fn transport_error(&self) -> Option<&str> {
        self.transport_error.as_deref()
    }
}

/// Summary produced by the HTTP smoke check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HttpSmokeCheckResult {
    status: CheckResultStatus,
    target: Option<String>,
    expected_status_code: u16,
    observed_status_code: Option<u16>,
    transport_error: Option<String>,
}

impl HttpSmokeCheckResult {
    /// Return the stable check result status.
    pub const fn status(&self) -> CheckResultStatus {
        self.status
    }

    /// Borrow the evaluated target.
    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    /// Return the expected HTTP status code.
    pub const fn expected_status_code(&self) -> u16 {
        self.expected_status_code
    }

    /// Return the observed HTTP status code.
    pub const fn observed_status_code(&self) -> Option<u16> {
        self.observed_status_code
    }

    /// Borrow the transport error label.
    pub fn transport_error(&self) -> Option<&str> {
        self.transport_error.as_deref()
    }
}

/// Evaluate whether an HTTP probe returned the expected status code.
pub fn check_http_smoke(smoke: &HttpSmokeInput) -> HttpSmokeCheckResult {
    let status = if smoke_target_is_empty(smoke) {
        CheckResultStatus::Skipped
    } else if smoke.transport_error.is_some() {
        CheckResultStatus::Failed
    } else if smoke.observed_status_code.is_none() {
        CheckResultStatus::Warning
    } else if smoke.observed_status_code == Some(smoke.expected_status_code) {
        CheckResultStatus::Passed
    } else {
        CheckResultStatus::Failed
    };

    HttpSmokeCheckResult {
        status,
        target: smoke.target.clone(),
        expected_status_code: smoke.expected_status_code,
        observed_status_code: smoke.observed_status_code,
        transport_error: smoke.transport_error.clone(),
    }
}

/// Return whether the smoke check target is absent.
fn smoke_target_is_empty(smoke: &HttpSmokeInput) -> bool {
    smoke
        .target
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty)
}

/// Input facts for evaluating one log fatal-pattern check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogFatalPatternInput {
    source: String,
    scanned_lines: usize,
    fatal_patterns: Vec<String>,
    matched_patterns: Vec<String>,
    log_collection_error: Option<String>,
}

impl LogFatalPatternInput {
    /// Create log fatal-pattern input from collected log scan facts.
    pub fn new(
        source: impl Into<String>,
        scanned_lines: usize,
        fatal_patterns: Vec<String>,
        matched_patterns: Vec<String>,
        log_collection_error: Option<String>,
    ) -> Self {
        Self {
            source: source.into(),
            scanned_lines,
            fatal_patterns,
            matched_patterns,
            log_collection_error,
        }
    }

    /// Borrow the log source name.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Return the number of scanned log lines.
    pub const fn scanned_lines(&self) -> usize {
        self.scanned_lines
    }

    /// Borrow the configured fatal patterns.
    pub fn fatal_patterns(&self) -> &[String] {
        &self.fatal_patterns
    }

    /// Borrow the matched fatal patterns.
    pub fn matched_patterns(&self) -> &[String] {
        &self.matched_patterns
    }

    /// Borrow the log collection error label.
    pub fn log_collection_error(&self) -> Option<&str> {
        self.log_collection_error.as_deref()
    }
}

/// Summary produced by the log fatal-pattern check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogFatalPatternCheckResult {
    status: CheckResultStatus,
    source: String,
    scanned_lines: usize,
    fatal_patterns: Vec<String>,
    matched_patterns: Vec<String>,
    log_collection_error: Option<String>,
}

impl LogFatalPatternCheckResult {
    /// Return the stable check result status.
    pub const fn status(&self) -> CheckResultStatus {
        self.status
    }

    /// Borrow the evaluated log source name.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Return the number of scanned log lines.
    pub const fn scanned_lines(&self) -> usize {
        self.scanned_lines
    }

    /// Borrow the configured fatal patterns.
    pub fn fatal_patterns(&self) -> &[String] {
        &self.fatal_patterns
    }

    /// Borrow the matched fatal patterns.
    pub fn matched_patterns(&self) -> &[String] {
        &self.matched_patterns
    }

    /// Borrow the log collection error label.
    pub fn log_collection_error(&self) -> Option<&str> {
        self.log_collection_error.as_deref()
    }
}

/// Evaluate whether collected logs contain fatal patterns.
pub fn check_log_fatal_patterns(logs: &LogFatalPatternInput) -> LogFatalPatternCheckResult {
    let status = if logs.fatal_patterns.is_empty() {
        CheckResultStatus::Skipped
    } else if logs.log_collection_error.is_some() {
        CheckResultStatus::Warning
    } else if !logs.matched_patterns.is_empty() {
        CheckResultStatus::Failed
    } else {
        CheckResultStatus::Passed
    };

    LogFatalPatternCheckResult {
        status,
        source: logs.source.clone(),
        scanned_lines: logs.scanned_lines,
        fatal_patterns: logs.fatal_patterns.clone(),
        matched_patterns: logs.matched_patterns.clone(),
        log_collection_error: logs.log_collection_error.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HttpSmokeInput, LogFatalPatternInput, PodReadinessInput, RolloutAvailabilityInput,
        ServiceEndpointInput, check_http_smoke, check_log_fatal_patterns, check_pod_readiness,
        check_rollout_availability, check_service_endpoints,
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

    /// Builds a service endpoint input fixture.
    fn service(
        declared_ports: usize,
        ready_endpoints: Option<usize>,
        not_ready_endpoints: Option<usize>,
        unknown_endpoints: Option<usize>,
    ) -> ServiceEndpointInput {
        ServiceEndpointInput::new(
            "checkout-api",
            declared_ports,
            ready_endpoints,
            not_ready_endpoints,
            unknown_endpoints,
        )
    }

    /// Builds an HTTP smoke input fixture.
    fn http_smoke(
        target: Option<&str>,
        expected_status_code: u16,
        observed_status_code: Option<u16>,
        transport_error: Option<&str>,
    ) -> HttpSmokeInput {
        HttpSmokeInput::new(
            target.map(ToOwned::to_owned),
            expected_status_code,
            observed_status_code,
            transport_error.map(ToOwned::to_owned),
        )
    }

    /// Builds a log fatal-pattern input fixture.
    fn log_fatal_patterns(
        scanned_lines: usize,
        fatal_patterns: &[&str],
        matched_patterns: &[&str],
        log_collection_error: Option<&str>,
    ) -> LogFatalPatternInput {
        LogFatalPatternInput::new(
            "checkout-api",
            scanned_lines,
            fatal_patterns.iter().map(ToString::to_string).collect(),
            matched_patterns.iter().map(ToString::to_string).collect(),
            log_collection_error.map(ToOwned::to_owned),
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

    #[test]
    /// Passes when every observed service endpoint is ready.
    fn passes_when_service_has_ready_endpoints() {
        let result = check_service_endpoints(&service(1, Some(2), Some(0), Some(0)));

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(result.service(), "checkout-api");
        assert_eq!(result.declared_ports(), 1);
        assert_eq!(result.ready_endpoints(), Some(2));
        assert_eq!(result.not_ready_endpoints(), Some(0));
        assert_eq!(result.unknown_endpoints(), Some(0));
    }

    #[test]
    /// Fails when a service has no ready endpoints.
    fn fails_when_service_has_no_ready_endpoints() {
        let result = check_service_endpoints(&service(1, Some(0), Some(2), Some(0)));

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.ready_endpoints(), Some(0));
        assert_eq!(result.not_ready_endpoints(), Some(2));
    }

    #[test]
    /// Warns when service endpoint readiness evidence is incomplete.
    fn warns_when_service_endpoint_evidence_is_missing() {
        let result = check_service_endpoints(&service(1, Some(2), None, Some(0)));

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.not_ready_endpoints(), None);
    }

    #[test]
    /// Warns when service endpoints mix ready and non-ready evidence.
    fn warns_when_service_has_mixed_endpoint_evidence() {
        let result = check_service_endpoints(&service(1, Some(2), Some(1), Some(0)));

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.ready_endpoints(), Some(2));
        assert_eq!(result.not_ready_endpoints(), Some(1));
    }

    #[test]
    /// Skips when a service has no declared ports.
    fn skips_when_service_has_no_declared_ports() {
        let result = check_service_endpoints(&service(0, Some(0), Some(0), Some(0)));

        assert_eq!(result.status(), CheckResultStatus::Skipped);
        assert_eq!(result.declared_ports(), 0);
    }

    #[test]
    /// Passes when the HTTP probe returns the expected status code.
    fn passes_when_http_smoke_status_matches() {
        let result = check_http_smoke(&http_smoke(Some("/healthz"), 200, Some(200), None));

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(result.target(), Some("/healthz"));
        assert_eq!(result.expected_status_code(), 200);
        assert_eq!(result.observed_status_code(), Some(200));
        assert_eq!(result.transport_error(), None);
    }

    #[test]
    /// Fails when the HTTP probe returns an unexpected status code.
    fn fails_when_http_smoke_status_differs() {
        let result = check_http_smoke(&http_smoke(Some("/healthz"), 200, Some(503), None));

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.expected_status_code(), 200);
        assert_eq!(result.observed_status_code(), Some(503));
    }

    #[test]
    /// Fails when the HTTP probe records a transport error.
    fn fails_when_http_smoke_has_transport_error() {
        let result = check_http_smoke(&http_smoke(
            Some("/healthz"),
            200,
            None,
            Some("connection_refused"),
        ));

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.transport_error(), Some("connection_refused"));
    }

    #[test]
    /// Warns when the HTTP probe has no observed status or error.
    fn warns_when_http_smoke_evidence_is_missing() {
        let result = check_http_smoke(&http_smoke(Some("/healthz"), 200, None, None));

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.observed_status_code(), None);
    }

    #[test]
    /// Skips when no HTTP smoke target is configured.
    fn skips_when_http_smoke_target_is_missing() {
        let result = check_http_smoke(&http_smoke(Some("  "), 200, None, None));

        assert_eq!(result.status(), CheckResultStatus::Skipped);
        assert_eq!(result.target(), Some("  "));
    }

    #[test]
    /// Passes when scanned logs contain no fatal patterns.
    fn passes_when_logs_have_no_fatal_patterns() {
        let result =
            check_log_fatal_patterns(&log_fatal_patterns(120, &["panic", "fatal"], &[], None));

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(result.source(), "checkout-api");
        assert_eq!(result.scanned_lines(), 120);
        assert_eq!(
            result.fatal_patterns(),
            &["panic".to_owned(), "fatal".to_owned()]
        );
        assert!(result.matched_patterns().is_empty());
        assert_eq!(result.log_collection_error(), None);
    }

    #[test]
    /// Fails when scanned logs contain at least one fatal pattern.
    fn fails_when_logs_have_fatal_patterns() {
        let result = check_log_fatal_patterns(&log_fatal_patterns(
            42,
            &["panic", "fatal"],
            &["panic"],
            None,
        ));

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.scanned_lines(), 42);
        assert_eq!(result.matched_patterns(), &["panic".to_owned()]);
    }

    #[test]
    /// Warns when log collection fails before patterns can be trusted.
    fn warns_when_log_collection_fails() {
        let result = check_log_fatal_patterns(&log_fatal_patterns(
            0,
            &["panic", "fatal"],
            &[],
            Some("permission_denied"),
        ));

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.log_collection_error(), Some("permission_denied"));
    }

    #[test]
    /// Skips when no fatal patterns are configured.
    fn skips_when_no_log_fatal_patterns_are_configured() {
        let result = check_log_fatal_patterns(&log_fatal_patterns(12, &[], &[], None));

        assert_eq!(result.status(), CheckResultStatus::Skipped);
        assert!(result.fatal_patterns().is_empty());
    }
}
