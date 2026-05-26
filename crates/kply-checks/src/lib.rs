//! Runtime checks for Kply verification workflows.

use kply_core::{CheckResultStatus, ProbeKind};
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

/// Input facts for evaluating one container restart count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RestartCountInput {
    pod: String,
    container: String,
    restart_count: Option<u32>,
}

impl RestartCountInput {
    /// Create restart count input from one container status.
    pub fn new(
        pod: impl Into<String>,
        container: impl Into<String>,
        restart_count: Option<u32>,
    ) -> Self {
        Self {
            pod: pod.into(),
            container: container.into(),
            restart_count,
        }
    }

    /// Borrow the pod name.
    pub fn pod(&self) -> &str {
        &self.pod
    }

    /// Borrow the container name.
    pub fn container(&self) -> &str {
        &self.container
    }

    /// Return the observed restart count.
    pub const fn restart_count(&self) -> Option<u32> {
        self.restart_count
    }
}

/// Summary produced by the restart count check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RestartCountCheckResult {
    status: CheckResultStatus,
    restart_threshold: u32,
    total_containers: usize,
    restarted_containers: usize,
    threshold_exceeded_containers: usize,
    unknown_containers: usize,
    max_restart_count: Option<u32>,
}

impl RestartCountCheckResult {
    /// Return the stable check result status.
    pub const fn status(&self) -> CheckResultStatus {
        self.status
    }

    /// Return the configured restart threshold.
    pub const fn restart_threshold(&self) -> u32 {
        self.restart_threshold
    }

    /// Return the number of containers considered by the check.
    pub const fn total_containers(&self) -> usize {
        self.total_containers
    }

    /// Return the number of containers with at least one restart.
    pub const fn restarted_containers(&self) -> usize {
        self.restarted_containers
    }

    /// Return the number of containers above the restart threshold.
    pub const fn threshold_exceeded_containers(&self) -> usize {
        self.threshold_exceeded_containers
    }

    /// Return the number of containers without restart evidence.
    pub const fn unknown_containers(&self) -> usize {
        self.unknown_containers
    }

    /// Return the maximum observed restart count.
    pub const fn max_restart_count(&self) -> Option<u32> {
        self.max_restart_count
    }
}

/// Evaluate whether container restart counts exceed a threshold.
pub fn check_restart_counts(
    containers: &[RestartCountInput],
    restart_threshold: u32,
) -> RestartCountCheckResult {
    let total_containers = containers.len();
    let restarted_containers = containers
        .iter()
        .filter(|container| container.restart_count.unwrap_or_default() > 0)
        .count();
    let threshold_exceeded_containers = containers
        .iter()
        .filter(|container| {
            container
                .restart_count
                .is_some_and(|restart_count| restart_count > restart_threshold)
        })
        .count();
    let unknown_containers = containers
        .iter()
        .filter(|container| container.restart_count.is_none())
        .count();
    let max_restart_count = containers
        .iter()
        .filter_map(|container| container.restart_count)
        .max();
    let status = if total_containers == 0 {
        CheckResultStatus::Skipped
    } else if threshold_exceeded_containers > 0 {
        CheckResultStatus::Failed
    } else if unknown_containers > 0 || restarted_containers > 0 {
        CheckResultStatus::Warning
    } else {
        CheckResultStatus::Passed
    };

    RestartCountCheckResult {
        status,
        restart_threshold,
        total_containers,
        restarted_containers,
        threshold_exceeded_containers,
        unknown_containers,
        max_restart_count,
    }
}

/// Input facts for evaluating one container resource request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceRequestInput {
    pod: String,
    container: String,
    cpu_request: Option<String>,
    memory_request: Option<String>,
}

impl ResourceRequestInput {
    /// Create resource request input from one container spec.
    pub fn new(
        pod: impl Into<String>,
        container: impl Into<String>,
        cpu_request: Option<String>,
        memory_request: Option<String>,
    ) -> Self {
        Self {
            pod: pod.into(),
            container: container.into(),
            cpu_request,
            memory_request,
        }
    }

    /// Borrow the pod name.
    pub fn pod(&self) -> &str {
        &self.pod
    }

    /// Borrow the container name.
    pub fn container(&self) -> &str {
        &self.container
    }

    /// Borrow the observed CPU request.
    pub fn cpu_request(&self) -> Option<&str> {
        self.cpu_request.as_deref()
    }

    /// Borrow the observed memory request.
    pub fn memory_request(&self) -> Option<&str> {
        self.memory_request.as_deref()
    }
}

/// Summary produced by the resource request sanity check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceRequestSanityCheckResult {
    status: CheckResultStatus,
    total_containers: usize,
    complete_request_containers: usize,
    missing_cpu_request_containers: usize,
    missing_memory_request_containers: usize,
    empty_request_containers: usize,
}

impl ResourceRequestSanityCheckResult {
    /// Return the stable check result status.
    pub const fn status(&self) -> CheckResultStatus {
        self.status
    }

    /// Return the number of containers considered by the check.
    pub const fn total_containers(&self) -> usize {
        self.total_containers
    }

    /// Return the number of containers with CPU and memory requests.
    pub const fn complete_request_containers(&self) -> usize {
        self.complete_request_containers
    }

    /// Return the number of containers without a CPU request.
    pub const fn missing_cpu_request_containers(&self) -> usize {
        self.missing_cpu_request_containers
    }

    /// Return the number of containers without a memory request.
    pub const fn missing_memory_request_containers(&self) -> usize {
        self.missing_memory_request_containers
    }

    /// Return the number of containers with blank request values.
    pub const fn empty_request_containers(&self) -> usize {
        self.empty_request_containers
    }
}

/// Evaluate whether containers declare sane CPU and memory requests.
pub fn check_resource_request_sanity(
    containers: &[ResourceRequestInput],
) -> ResourceRequestSanityCheckResult {
    let total_containers = containers.len();
    let complete_request_containers = containers
        .iter()
        .filter(|container| container_has_complete_requests(container))
        .count();
    let missing_cpu_request_containers = containers
        .iter()
        .filter(|container| container.cpu_request.is_none())
        .count();
    let missing_memory_request_containers = containers
        .iter()
        .filter(|container| container.memory_request.is_none())
        .count();
    let empty_request_containers = containers
        .iter()
        .filter(|container| container_has_empty_request(container))
        .count();
    let status = if total_containers == 0 {
        CheckResultStatus::Skipped
    } else if empty_request_containers > 0 {
        CheckResultStatus::Failed
    } else if missing_cpu_request_containers > 0 || missing_memory_request_containers > 0 {
        CheckResultStatus::Warning
    } else {
        CheckResultStatus::Passed
    };

    ResourceRequestSanityCheckResult {
        status,
        total_containers,
        complete_request_containers,
        missing_cpu_request_containers,
        missing_memory_request_containers,
        empty_request_containers,
    }
}

/// Return whether a container declares non-empty CPU and memory requests.
fn container_has_complete_requests(container: &ResourceRequestInput) -> bool {
    request_value_is_present(container.cpu_request())
        && request_value_is_present(container.memory_request())
}

/// Return whether a container declares a blank request value.
fn container_has_empty_request(container: &ResourceRequestInput) -> bool {
    request_value_is_empty(container.cpu_request())
        || request_value_is_empty(container.memory_request())
}

/// Return whether a request value is present and non-blank.
fn request_value_is_present(request: Option<&str>) -> bool {
    request.is_some_and(|value| !value.trim().is_empty())
}

/// Return whether a request value is present but blank.
fn request_value_is_empty(request: Option<&str>) -> bool {
    request.is_some_and(|value| value.trim().is_empty())
}

/// Input facts for evaluating one container probe configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProbeExistenceInput {
    workload: String,
    container: String,
    readiness_probe: bool,
    liveness_probe: bool,
    startup_probe: bool,
}

impl ProbeExistenceInput {
    /// Create probe existence input from one container spec.
    pub fn new(
        workload: impl Into<String>,
        container: impl Into<String>,
        readiness_probe: bool,
        liveness_probe: bool,
        startup_probe: bool,
    ) -> Self {
        Self {
            workload: workload.into(),
            container: container.into(),
            readiness_probe,
            liveness_probe,
            startup_probe,
        }
    }

    /// Borrow the workload name.
    pub fn workload(&self) -> &str {
        &self.workload
    }

    /// Borrow the container name.
    pub fn container(&self) -> &str {
        &self.container
    }

    /// Return whether a readiness probe is configured.
    pub const fn readiness_probe(&self) -> bool {
        self.readiness_probe
    }

    /// Return whether a liveness probe is configured.
    pub const fn liveness_probe(&self) -> bool {
        self.liveness_probe
    }

    /// Return whether a startup probe is configured.
    pub const fn startup_probe(&self) -> bool {
        self.startup_probe
    }
}

/// Summary produced by the probe existence check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProbeExistenceCheckResult {
    status: CheckResultStatus,
    required_probe_kinds: Vec<ProbeKind>,
    total_containers: usize,
    complete_probe_containers: usize,
    missing_probe_containers: usize,
    missing_readiness_probe_containers: usize,
    missing_liveness_probe_containers: usize,
    missing_startup_probe_containers: usize,
}

impl ProbeExistenceCheckResult {
    /// Return the stable check result status.
    pub const fn status(&self) -> CheckResultStatus {
        self.status
    }

    /// Borrow the required probe kinds considered by the check.
    pub fn required_probe_kinds(&self) -> &[ProbeKind] {
        &self.required_probe_kinds
    }

    /// Return the number of containers considered by the check.
    pub const fn total_containers(&self) -> usize {
        self.total_containers
    }

    /// Return the number of containers with every required probe.
    pub const fn complete_probe_containers(&self) -> usize {
        self.complete_probe_containers
    }

    /// Return the number of containers missing at least one required probe.
    pub const fn missing_probe_containers(&self) -> usize {
        self.missing_probe_containers
    }

    /// Return the number of containers missing a required readiness probe.
    pub const fn missing_readiness_probe_containers(&self) -> usize {
        self.missing_readiness_probe_containers
    }

    /// Return the number of containers missing a required liveness probe.
    pub const fn missing_liveness_probe_containers(&self) -> usize {
        self.missing_liveness_probe_containers
    }

    /// Return the number of containers missing a required startup probe.
    pub const fn missing_startup_probe_containers(&self) -> usize {
        self.missing_startup_probe_containers
    }
}

/// Evaluate whether containers declare required probes.
pub fn check_probe_existence(
    containers: &[ProbeExistenceInput],
    required_probe_kinds: &[ProbeKind],
) -> ProbeExistenceCheckResult {
    let required_probe_kinds = normalized_probe_kinds(required_probe_kinds);
    let total_containers = containers.len();
    let complete_probe_containers = containers
        .iter()
        .filter(|container| container_has_required_probes(container, &required_probe_kinds))
        .count();
    let missing_probe_containers = total_containers.saturating_sub(complete_probe_containers);
    let missing_readiness_probe_containers =
        count_missing_probe_kind(containers, &required_probe_kinds, ProbeKind::Readiness);
    let missing_liveness_probe_containers =
        count_missing_probe_kind(containers, &required_probe_kinds, ProbeKind::Liveness);
    let missing_startup_probe_containers =
        count_missing_probe_kind(containers, &required_probe_kinds, ProbeKind::Startup);
    let status = if total_containers == 0 || required_probe_kinds.is_empty() {
        CheckResultStatus::Skipped
    } else if missing_probe_containers > 0 {
        CheckResultStatus::Warning
    } else {
        CheckResultStatus::Passed
    };

    ProbeExistenceCheckResult {
        status,
        required_probe_kinds,
        total_containers,
        complete_probe_containers,
        missing_probe_containers,
        missing_readiness_probe_containers,
        missing_liveness_probe_containers,
        missing_startup_probe_containers,
    }
}

/// Return required probe kinds in deterministic order without duplicates.
fn normalized_probe_kinds(required_probe_kinds: &[ProbeKind]) -> Vec<ProbeKind> {
    let mut normalized_probe_kinds = required_probe_kinds.to_vec();
    normalized_probe_kinds.sort_unstable();
    normalized_probe_kinds.dedup();
    normalized_probe_kinds
}

/// Return whether a container has every required probe.
fn container_has_required_probes(
    container: &ProbeExistenceInput,
    required_probe_kinds: &[ProbeKind],
) -> bool {
    required_probe_kinds
        .iter()
        .all(|probe_kind| container_has_probe_kind(container, *probe_kind))
}

/// Count containers missing one required probe kind.
fn count_missing_probe_kind(
    containers: &[ProbeExistenceInput],
    required_probe_kinds: &[ProbeKind],
    probe_kind: ProbeKind,
) -> usize {
    if !required_probe_kinds.contains(&probe_kind) {
        return 0;
    }

    containers
        .iter()
        .filter(|container| !container_has_probe_kind(container, probe_kind))
        .count()
}

/// Return whether a container has one probe kind.
fn container_has_probe_kind(container: &ProbeExistenceInput, probe_kind: ProbeKind) -> bool {
    match probe_kind {
        ProbeKind::Readiness => container.readiness_probe,
        ProbeKind::Liveness => container.liveness_probe,
        ProbeKind::Startup => container.startup_probe,
    }
}

/// Input facts for evaluating one check timeout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckTimeoutInput {
    check_name: String,
    timeout_millis: u64,
    elapsed_millis: Option<u64>,
    completed: bool,
}

impl CheckTimeoutInput {
    /// Create timeout input from one check execution attempt.
    pub fn new(
        check_name: impl Into<String>,
        timeout_millis: u64,
        elapsed_millis: Option<u64>,
        completed: bool,
    ) -> Self {
        Self {
            check_name: check_name.into(),
            timeout_millis,
            elapsed_millis,
            completed,
        }
    }

    /// Borrow the check name.
    pub fn check_name(&self) -> &str {
        &self.check_name
    }

    /// Return the configured timeout budget in milliseconds.
    pub const fn timeout_millis(&self) -> u64 {
        self.timeout_millis
    }

    /// Return the observed elapsed time in milliseconds.
    pub const fn elapsed_millis(&self) -> Option<u64> {
        self.elapsed_millis
    }

    /// Return whether the check completed before evaluation.
    pub const fn completed(&self) -> bool {
        self.completed
    }
}

/// Summary produced by the check timeout evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckTimeoutResult {
    status: CheckResultStatus,
    check_name: String,
    timeout_millis: u64,
    elapsed_millis: Option<u64>,
    completed: bool,
    timed_out: bool,
}

impl CheckTimeoutResult {
    /// Return the stable check result status.
    pub const fn status(&self) -> CheckResultStatus {
        self.status
    }

    /// Borrow the evaluated check name.
    pub fn check_name(&self) -> &str {
        &self.check_name
    }

    /// Return the configured timeout budget in milliseconds.
    pub const fn timeout_millis(&self) -> u64 {
        self.timeout_millis
    }

    /// Return the observed elapsed time in milliseconds.
    pub const fn elapsed_millis(&self) -> Option<u64> {
        self.elapsed_millis
    }

    /// Return whether the check completed before evaluation.
    pub const fn completed(&self) -> bool {
        self.completed
    }

    /// Return whether timeout evidence breached the budget.
    ///
    /// Completed checks time out when `elapsed_millis > timeout_millis`; incomplete checks time out
    /// when `elapsed_millis >= timeout_millis`.
    pub const fn timed_out(&self) -> bool {
        self.timed_out
    }
}

/// Evaluate whether one check execution exceeded its timeout budget.
pub fn check_timeout(timeout: &CheckTimeoutInput) -> CheckTimeoutResult {
    let timed_out = check_timed_out(timeout);
    let status = if timeout.timeout_millis == 0 {
        CheckResultStatus::Skipped
    } else if timed_out {
        CheckResultStatus::Failed
    } else if timeout.elapsed_millis.is_none() || !timeout.completed {
        CheckResultStatus::Warning
    } else {
        CheckResultStatus::Passed
    };

    CheckTimeoutResult {
        status,
        check_name: timeout.check_name.clone(),
        timeout_millis: timeout.timeout_millis,
        elapsed_millis: timeout.elapsed_millis,
        completed: timeout.completed,
        timed_out,
    }
}

/// Return whether timeout evidence shows a deadline breach.
fn check_timed_out(timeout: &CheckTimeoutInput) -> bool {
    if timeout.timeout_millis == 0 {
        return false;
    }

    let Some(elapsed_millis) = timeout.elapsed_millis else {
        return false;
    };

    if timeout.completed {
        elapsed_millis > timeout.timeout_millis
    } else {
        elapsed_millis >= timeout.timeout_millis
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CheckTimeoutInput, HttpSmokeInput, LogFatalPatternInput, PodReadinessInput,
        ProbeExistenceInput, ResourceRequestInput, RestartCountInput, RolloutAvailabilityInput,
        ServiceEndpointInput, check_http_smoke, check_log_fatal_patterns, check_pod_readiness,
        check_probe_existence, check_resource_request_sanity, check_restart_counts,
        check_rollout_availability, check_service_endpoints, check_timeout,
    };
    use kply_core::{CheckResultStatus, ProbeKind};
    use serde::Serialize;
    use std::path::{Path, PathBuf};

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

    /// Builds a restart count input fixture.
    fn restart_count(pod: &str, container: &str, restart_count: Option<u32>) -> RestartCountInput {
        RestartCountInput::new(pod, container, restart_count)
    }

    /// Builds a resource request input fixture.
    fn resource_request(
        pod: &str,
        container: &str,
        cpu_request: Option<&str>,
        memory_request: Option<&str>,
    ) -> ResourceRequestInput {
        ResourceRequestInput::new(
            pod,
            container,
            cpu_request.map(ToOwned::to_owned),
            memory_request.map(ToOwned::to_owned),
        )
    }

    /// Builds a probe existence input fixture.
    fn probe_existence(
        workload: &str,
        container: &str,
        readiness_probe: bool,
        liveness_probe: bool,
        startup_probe: bool,
    ) -> ProbeExistenceInput {
        ProbeExistenceInput::new(
            workload,
            container,
            readiness_probe,
            liveness_probe,
            startup_probe,
        )
    }

    /// Builds a check timeout input fixture.
    fn timeout(
        check_name: &str,
        timeout_millis: u64,
        elapsed_millis: Option<u64>,
        completed: bool,
    ) -> CheckTimeoutInput {
        CheckTimeoutInput::new(check_name, timeout_millis, elapsed_millis, completed)
    }

    /// Return the repository fixture root.
    fn fixture_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("kply-checks should live under crates/")
            .join("fixtures")
    }

    /// Assert a check result matches the named JSON fixture.
    fn assert_check_result_fixture(name: &str, result: &impl Serialize) {
        let fixture_path = fixture_root()
            .join("reports")
            .join("check-results")
            .join(format!("{name}.json"));
        let fixture = std::fs::read_to_string(&fixture_path).unwrap_or_else(|error| {
            panic!(
                "fixture {} should be readable: {error}",
                fixture_path.display()
            )
        });
        let expected: serde_json::Value =
            serde_json::from_str(&fixture).expect("fixture should be valid JSON");
        let actual = serde_json::to_value(result).expect("check result should serialize");

        assert_eq!(actual, expected, "check result fixture {name} drifted");
    }

    #[test]
    /// Verifies the reusable check result fixture suite.
    fn matches_check_result_fixture_suite() {
        assert_check_result_fixture(
            "pod-readiness",
            &check_pod_readiness(&[
                pod("checkout-a", Some("Running"), Some(true)),
                pod("checkout-b", Some("Running"), Some(false)),
                pod("checkout-c", Some("Running"), None),
            ]),
        );
        assert_check_result_fixture(
            "rollout-availability",
            &check_rollout_availability(&rollout(Some(3), Some(2), Some(2), Some(3), Some(1))),
        );
        assert_check_result_fixture(
            "service-endpoint",
            &check_service_endpoints(&service(1, Some(2), Some(1), Some(0))),
        );
        assert_check_result_fixture(
            "http-smoke",
            &check_http_smoke(&http_smoke(
                Some("http://checkout-api.shop.svc.cluster.local/healthz"),
                200,
                None,
                Some("connection_refused"),
            )),
        );
        assert_check_result_fixture(
            "log-fatal-pattern",
            &check_log_fatal_patterns(&log_fatal_patterns(
                42,
                &["panic", "fatal"],
                &["panic"],
                None,
            )),
        );
        assert_check_result_fixture(
            "restart-count",
            &check_restart_counts(
                &[
                    restart_count("checkout-a", "api", Some(1)),
                    restart_count("checkout-b", "api", Some(4)),
                    restart_count("checkout-c", "worker", None),
                ],
                3,
            ),
        );
        assert_check_result_fixture(
            "resource-request-sanity",
            &check_resource_request_sanity(&[
                resource_request("checkout-a", "api", Some(" "), Some("256Mi")),
                resource_request("checkout-b", "worker", Some("100m"), Some("")),
            ]),
        );
        assert_check_result_fixture(
            "probe-existence",
            &check_probe_existence(
                &[
                    probe_existence("checkout-api", "api", true, false, false),
                    probe_existence("checkout-api", "worker", false, true, false),
                ],
                &[ProbeKind::Readiness, ProbeKind::Liveness],
            ),
        );
        assert_check_result_fixture(
            "check-timeout",
            &check_timeout(&timeout("rollout_availability", 5_000, Some(5_001), true)),
        );
    }

    #[test]
    /// Snapshots the pod readiness check result JSON contract.
    fn snapshots_pod_readiness_check_result() {
        let result = check_pod_readiness(&[
            pod("checkout-a", Some("Running"), Some(true)),
            pod("checkout-b", Some("Running"), Some(false)),
            pod("checkout-c", Some("Running"), None),
        ]);

        insta::assert_json_snapshot!("pod_readiness_check_result", result);
    }

    #[test]
    /// Snapshots the rollout availability check result JSON contract.
    fn snapshots_rollout_availability_check_result() {
        let result =
            check_rollout_availability(&rollout(Some(3), Some(2), Some(2), Some(3), Some(1)));

        insta::assert_json_snapshot!("rollout_availability_check_result", result);
    }

    #[test]
    /// Snapshots the service endpoint check result JSON contract.
    fn snapshots_service_endpoint_check_result() {
        let result = check_service_endpoints(&service(1, Some(2), Some(1), Some(0)));

        insta::assert_json_snapshot!("service_endpoint_check_result", result);
    }

    #[test]
    /// Snapshots the HTTP smoke check result JSON contract.
    fn snapshots_http_smoke_check_result() {
        let result = check_http_smoke(&http_smoke(
            Some("http://checkout-api.shop.svc.cluster.local/healthz"),
            200,
            None,
            Some("connection_refused"),
        ));

        insta::assert_json_snapshot!("http_smoke_check_result", result);
    }

    #[test]
    /// Snapshots the log fatal-pattern check result JSON contract.
    fn snapshots_log_fatal_pattern_check_result() {
        let result = check_log_fatal_patterns(&log_fatal_patterns(
            42,
            &["panic", "fatal"],
            &["panic"],
            None,
        ));

        insta::assert_json_snapshot!("log_fatal_pattern_check_result", result);
    }

    #[test]
    /// Snapshots the restart count check result JSON contract.
    fn snapshots_restart_count_check_result() {
        let result = check_restart_counts(
            &[
                restart_count("checkout-a", "api", Some(1)),
                restart_count("checkout-b", "api", Some(4)),
                restart_count("checkout-c", "worker", None),
            ],
            3,
        );

        insta::assert_json_snapshot!("restart_count_check_result", result);
    }

    #[test]
    /// Snapshots the resource request sanity check result JSON contract.
    fn snapshots_resource_request_sanity_check_result() {
        let result = check_resource_request_sanity(&[
            resource_request("checkout-a", "api", Some(" "), Some("256Mi")),
            resource_request("checkout-b", "worker", Some("100m"), Some("")),
        ]);

        insta::assert_json_snapshot!("resource_request_sanity_check_result", result);
    }

    #[test]
    /// Snapshots the probe existence check result JSON contract.
    fn snapshots_probe_existence_check_result() {
        let result = check_probe_existence(
            &[
                probe_existence("checkout-api", "api", true, false, false),
                probe_existence("checkout-api", "worker", false, true, false),
            ],
            &[ProbeKind::Readiness, ProbeKind::Liveness],
        );

        insta::assert_json_snapshot!("probe_existence_check_result", result);
    }

    #[test]
    /// Snapshots the check timeout result JSON contract.
    fn snapshots_check_timeout_result() {
        let result = check_timeout(&timeout("rollout_availability", 5_000, Some(5_001), true));

        insta::assert_json_snapshot!("check_timeout_result", result);
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

    #[test]
    /// Passes when all container restart counts are zero.
    fn passes_when_restart_counts_are_zero() {
        let result = check_restart_counts(
            &[
                restart_count("checkout-a", "api", Some(0)),
                restart_count("checkout-b", "api", Some(0)),
            ],
            3,
        );

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(result.restart_threshold(), 3);
        assert_eq!(result.total_containers(), 2);
        assert_eq!(result.restarted_containers(), 0);
        assert_eq!(result.threshold_exceeded_containers(), 0);
        assert_eq!(result.unknown_containers(), 0);
        assert_eq!(result.max_restart_count(), Some(0));
    }

    #[test]
    /// Fails when any container restart count exceeds the threshold.
    fn fails_when_restart_count_exceeds_threshold() {
        let result = check_restart_counts(
            &[
                restart_count("checkout-a", "api", Some(1)),
                restart_count("checkout-b", "api", Some(4)),
            ],
            3,
        );

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.restarted_containers(), 2);
        assert_eq!(result.threshold_exceeded_containers(), 1);
        assert_eq!(result.max_restart_count(), Some(4));
    }

    #[test]
    /// Warns when restarts are present but below the threshold.
    fn warns_when_restart_count_is_below_threshold() {
        let result = check_restart_counts(&[restart_count("checkout-a", "api", Some(1))], 3);

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.restarted_containers(), 1);
        assert_eq!(result.threshold_exceeded_containers(), 0);
    }

    #[test]
    /// Warns when restart count evidence is incomplete.
    fn warns_when_restart_count_evidence_is_missing() {
        let result = check_restart_counts(&[restart_count("checkout-a", "api", None)], 3);

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.unknown_containers(), 1);
        assert_eq!(result.max_restart_count(), None);
    }

    #[test]
    /// Skips when no containers are available for restart evaluation.
    fn skips_when_no_restart_count_inputs_are_available() {
        let result = check_restart_counts(&[], 3);

        assert_eq!(result.status(), CheckResultStatus::Skipped);
        assert_eq!(result.total_containers(), 0);
        assert_eq!(result.max_restart_count(), None);
    }

    #[test]
    /// Passes when every container declares CPU and memory requests.
    fn passes_when_resource_requests_are_complete() {
        let result = check_resource_request_sanity(&[
            resource_request("checkout-a", "api", Some("250m"), Some("256Mi")),
            resource_request("checkout-b", "worker", Some("100m"), Some("128Mi")),
        ]);

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(result.total_containers(), 2);
        assert_eq!(result.complete_request_containers(), 2);
        assert_eq!(result.missing_cpu_request_containers(), 0);
        assert_eq!(result.missing_memory_request_containers(), 0);
        assert_eq!(result.empty_request_containers(), 0);
    }

    #[test]
    /// Warns when any container is missing CPU or memory requests.
    fn warns_when_resource_requests_are_missing() {
        let result = check_resource_request_sanity(&[
            resource_request("checkout-a", "api", None, Some("256Mi")),
            resource_request("checkout-b", "worker", Some("100m"), None),
        ]);

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.total_containers(), 2);
        assert_eq!(result.complete_request_containers(), 0);
        assert_eq!(result.missing_cpu_request_containers(), 1);
        assert_eq!(result.missing_memory_request_containers(), 1);
        assert_eq!(result.empty_request_containers(), 0);
    }

    #[test]
    /// Fails when any container has a blank request value.
    fn fails_when_resource_request_values_are_blank() {
        let result = check_resource_request_sanity(&[
            resource_request("checkout-a", "api", Some(" "), Some("256Mi")),
            resource_request("checkout-b", "worker", Some("100m"), Some("")),
        ]);

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.total_containers(), 2);
        assert_eq!(result.complete_request_containers(), 0);
        assert_eq!(result.missing_cpu_request_containers(), 0);
        assert_eq!(result.missing_memory_request_containers(), 0);
        assert_eq!(result.empty_request_containers(), 2);
    }

    #[test]
    /// Skips when no containers are available for resource request evaluation.
    fn skips_when_no_resource_request_inputs_are_available() {
        let result = check_resource_request_sanity(&[]);

        assert_eq!(result.status(), CheckResultStatus::Skipped);
        assert_eq!(result.total_containers(), 0);
        assert_eq!(result.complete_request_containers(), 0);
        assert_eq!(result.empty_request_containers(), 0);
    }

    #[test]
    /// Passes when every container has every required probe.
    fn passes_when_required_probes_exist() {
        let result = check_probe_existence(
            &[
                probe_existence("checkout-api", "api", true, true, false),
                probe_existence("checkout-api", "worker", true, true, true),
            ],
            &[ProbeKind::Readiness, ProbeKind::Liveness],
        );

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(
            result.required_probe_kinds(),
            &[ProbeKind::Readiness, ProbeKind::Liveness]
        );
        assert_eq!(result.total_containers(), 2);
        assert_eq!(result.complete_probe_containers(), 2);
        assert_eq!(result.missing_probe_containers(), 0);
        assert_eq!(result.missing_readiness_probe_containers(), 0);
        assert_eq!(result.missing_liveness_probe_containers(), 0);
        assert_eq!(result.missing_startup_probe_containers(), 0);
    }

    #[test]
    /// Warns when any container is missing a required probe.
    fn warns_when_required_probes_are_missing() {
        let result = check_probe_existence(
            &[
                probe_existence("checkout-api", "api", true, false, false),
                probe_existence("checkout-api", "worker", false, true, false),
            ],
            &[ProbeKind::Readiness, ProbeKind::Liveness],
        );

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.total_containers(), 2);
        assert_eq!(result.complete_probe_containers(), 0);
        assert_eq!(result.missing_probe_containers(), 2);
        assert_eq!(result.missing_readiness_probe_containers(), 1);
        assert_eq!(result.missing_liveness_probe_containers(), 1);
        assert_eq!(result.missing_startup_probe_containers(), 0);
    }

    #[test]
    /// Deduplicates required probe kinds before evaluating containers.
    fn deduplicates_required_probe_kinds() {
        let result = check_probe_existence(
            &[probe_existence("checkout-api", "api", true, true, false)],
            &[
                ProbeKind::Liveness,
                ProbeKind::Readiness,
                ProbeKind::Liveness,
            ],
        );

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(
            result.required_probe_kinds(),
            &[ProbeKind::Readiness, ProbeKind::Liveness]
        );
    }

    #[test]
    /// Skips when no required probe kinds are configured.
    fn skips_when_no_probe_kinds_are_required() {
        let result = check_probe_existence(
            &[probe_existence("checkout-api", "api", false, false, false)],
            &[],
        );

        assert_eq!(result.status(), CheckResultStatus::Skipped);
        assert!(result.required_probe_kinds().is_empty());
        assert_eq!(result.total_containers(), 1);
        assert_eq!(result.missing_probe_containers(), 0);
    }

    #[test]
    /// Skips when no containers are available for probe evaluation.
    fn skips_when_no_probe_existence_inputs_are_available() {
        let result = check_probe_existence(&[], &[ProbeKind::Readiness, ProbeKind::Startup]);

        assert_eq!(result.status(), CheckResultStatus::Skipped);
        assert_eq!(result.total_containers(), 0);
        assert_eq!(result.complete_probe_containers(), 0);
        assert_eq!(result.missing_probe_containers(), 0);
        assert_eq!(result.missing_readiness_probe_containers(), 0);
        assert_eq!(result.missing_startup_probe_containers(), 0);
    }

    #[test]
    /// Passes when a check completes within its timeout budget.
    fn passes_when_check_finishes_before_timeout() {
        let result = check_timeout(&timeout("http_smoke", 5_000, Some(4_999), true));

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(result.check_name(), "http_smoke");
        assert_eq!(result.timeout_millis(), 5_000);
        assert_eq!(result.elapsed_millis(), Some(4_999));
        assert!(result.completed());
        assert!(!result.timed_out());
    }

    #[test]
    /// Passes when a completed check lands exactly on its timeout budget.
    fn passes_when_completed_check_at_timeout_boundary() {
        let result = check_timeout(&timeout("http_smoke", 5_000, Some(5_000), true));

        assert_eq!(result.status(), CheckResultStatus::Passed);
        assert_eq!(result.elapsed_millis(), Some(5_000));
        assert!(result.completed());
        assert!(!result.timed_out());
    }

    #[test]
    /// Fails when a completed check exceeds its timeout budget.
    fn fails_when_completed_check_exceeds_timeout() {
        let result = check_timeout(&timeout("rollout_availability", 5_000, Some(5_001), true));

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.elapsed_millis(), Some(5_001));
        assert!(result.completed());
        assert!(result.timed_out());
    }

    #[test]
    /// Fails when an incomplete check reaches its timeout budget.
    fn fails_when_incomplete_check_reaches_timeout() {
        let result = check_timeout(&timeout("pod_readiness", 5_000, Some(5_000), false));

        assert_eq!(result.status(), CheckResultStatus::Failed);
        assert_eq!(result.elapsed_millis(), Some(5_000));
        assert!(!result.completed());
        assert!(result.timed_out());
    }

    #[test]
    /// Warns when timeout evidence is missing.
    fn warns_when_timeout_elapsed_evidence_is_missing() {
        let result = check_timeout(&timeout("log_fatal_patterns", 5_000, None, true));

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.elapsed_millis(), None);
        assert!(result.completed());
        assert!(!result.timed_out());
    }

    #[test]
    /// Warns when a check is incomplete but has not reached its timeout.
    fn warns_when_check_is_incomplete_before_timeout() {
        let result = check_timeout(&timeout("restart_counts", 5_000, Some(2_500), false));

        assert_eq!(result.status(), CheckResultStatus::Warning);
        assert_eq!(result.elapsed_millis(), Some(2_500));
        assert!(!result.completed());
        assert!(!result.timed_out());
    }

    #[test]
    /// Skips when no timeout budget is configured.
    fn skips_when_timeout_budget_is_disabled() {
        let result = check_timeout(&timeout("probe_existence", 0, Some(25_000), false));

        assert_eq!(result.status(), CheckResultStatus::Skipped);
        assert_eq!(result.timeout_millis(), 0);
        assert!(!result.timed_out());
    }
}
