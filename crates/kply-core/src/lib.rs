//! Core domain model for future Kply session primitives.

use serde::de::Error as DeserializeError;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

const SESSION_TOKEN_MAX_LEN: usize = 63;
const WORKLOAD_KIND_MAX_LEN: usize = 63;
const RESOURCE_QUANTITY_MAX_LEN: usize = 63;

/// Maximum allowed length for an [`ImageRef`] value.
pub const IMAGE_REF_MAX_LEN: usize = 255;
/// Maximum allowed length for a route header name.
pub const ROUTE_HEADER_NAME_MAX_LEN: usize = 63;
/// Maximum allowed length for a route header value.
pub const ROUTE_HEADER_VALUE_MAX_LEN: usize = 255;
/// Maximum allowed length for a route host.
pub const ROUTE_HOST_MAX_LEN: usize = 253;
/// Maximum allowed length for a route host label.
pub const ROUTE_HOST_LABEL_MAX_LEN: usize = 63;
/// Maximum allowed length for a session time-to-live value.
pub const TIME_TO_LIVE_MAX_LEN: usize = 32;

/// Stable identifier for a future Kply session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct SessionId(String);

impl SessionId {
    /// Create a [`SessionId`] from a validated string value.
    pub fn new(value: impl Into<String>) -> Result<Self, SessionIdError> {
        let value = value.into();
        validate_session_token(&value)?;
        Ok(Self(value))
    }

    /// Borrow the session identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<String> for SessionId {
    type Error = SessionIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<SessionId> for String {
    fn from(value: SessionId) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_validated_string(deserializer, Self::new)
    }
}

/// Stable user-facing name for a future Kply session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct SessionName(String);

impl SessionName {
    /// Create a [`SessionName`] from a validated string value.
    pub fn new(value: impl Into<String>) -> Result<Self, SessionNameError> {
        let value = value.into();
        validate_session_token(&value)?;
        Ok(Self(value))
    }

    /// Borrow the session name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<String> for SessionName {
    type Error = SessionNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<SessionName> for String {
    fn from(value: SessionName) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for SessionName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_validated_string(deserializer, Self::new)
    }
}

/// Lifecycle status for a future Kply session.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Session inputs have been accepted but no cluster preparation has started.
    Planned,
    /// Kply is preparing sandbox resources or route isolation.
    Preparing,
    /// The sandbox session is available for agent or test traffic.
    Active,
    /// Kply is running checks against the active session.
    Verifying,
    /// The session cannot proceed until an explicit issue is resolved.
    Blocked,
    /// The session passed checks and is ready for promotion or human approval.
    Ready,
    /// Kply has removed the temporary session resources.
    CleanedUp,
    /// The session failed and requires inspection.
    Failed,
}

impl SessionStatus {
    /// Return every session lifecycle status in declaration order, including terminal states.
    pub const fn all() -> &'static [Self] {
        &[
            Self::Planned,
            Self::Preparing,
            Self::Active,
            Self::Verifying,
            Self::Blocked,
            Self::Ready,
            Self::CleanedUp,
            Self::Failed,
        ]
    }

    /// Return the stable snake_case status name used in agent-readable output.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Preparing => "preparing",
            Self::Active => "active",
            Self::Verifying => "verifying",
            Self::Blocked => "blocked",
            Self::Ready => "ready",
            Self::CleanedUp => "cleaned_up",
            Self::Failed => "failed",
        }
    }

    /// Return whether this status may transition to the next [`SessionStatus`].
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Planned, Self::Preparing)
                | (Self::Planned, Self::Failed)
                | (Self::Preparing, Self::Active)
                | (Self::Preparing, Self::Blocked)
                | (Self::Preparing, Self::Failed)
                | (Self::Active, Self::Verifying)
                | (Self::Active, Self::Blocked)
                | (Self::Active, Self::CleanedUp)
                | (Self::Active, Self::Failed)
                | (Self::Verifying, Self::Ready)
                | (Self::Verifying, Self::Blocked)
                | (Self::Verifying, Self::Failed)
                | (Self::Blocked, Self::Preparing)
                | (Self::Blocked, Self::Active)
                | (Self::Blocked, Self::CleanedUp)
                | (Self::Blocked, Self::Failed)
                | (Self::Ready, Self::CleanedUp)
                | (Self::Ready, Self::Failed)
                | (Self::Failed, Self::CleanedUp)
        )
    }

    /// Validate that this status may transition to the next [`SessionStatus`].
    pub const fn validate_transition_to(self, next: Self) -> Result<(), SessionTransitionError> {
        if self.can_transition_to(next) {
            Ok(())
        } else {
            Err(SessionTransitionError::Invalid {
                from: self,
                to: next,
            })
        }
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Kubernetes workload target for a future Kply session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct WorkloadRef {
    namespace: String,
    kind: String,
    name: String,
}

impl WorkloadRef {
    /// Create a [`WorkloadRef`] from validated namespace, kind, and name parts.
    pub fn new(
        namespace: impl Into<String>,
        kind: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, WorkloadRefError> {
        let namespace = namespace.into();
        let kind = kind.into();
        let name = name.into();

        validate_session_token(&namespace)
            .map_err(WorkloadTokenError::from)
            .map_err(WorkloadRefError::Namespace)?;
        validate_workload_kind(&kind).map_err(WorkloadRefError::Kind)?;
        validate_session_token(&name)
            .map_err(WorkloadTokenError::from)
            .map_err(WorkloadRefError::Name)?;

        Ok(Self {
            namespace,
            kind,
            name,
        })
    }

    /// Borrow the workload namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Borrow the workload kind.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Borrow the workload name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for WorkloadRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}/{}", self.namespace, self.kind, self.name)
    }
}

impl<'de> Deserialize<'de> for WorkloadRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = WorkloadRefFields::deserialize(deserializer)?;
        Self::new(fields.namespace, fields.kind, fields.name).map_err(D::Error::custom)
    }
}

/// Namespaced Kubernetes resource planned for a future Kply session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct KubernetesResourceRef {
    namespace: String,
    kind: String,
    name: String,
}

impl KubernetesResourceRef {
    /// Create a [`KubernetesResourceRef`] from validated namespace, kind, and name parts.
    pub fn new(
        namespace: impl Into<String>,
        kind: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, KubernetesResourceRefError> {
        let namespace = namespace.into();
        let kind = kind.into();
        let name = name.into();

        validate_session_token(&namespace)
            .map_err(WorkloadTokenError::from)
            .map_err(KubernetesResourceRefError::Namespace)?;
        validate_workload_kind(&kind).map_err(KubernetesResourceRefError::Kind)?;
        validate_session_token(&name)
            .map_err(WorkloadTokenError::from)
            .map_err(KubernetesResourceRefError::Name)?;

        Ok(Self {
            namespace,
            kind,
            name,
        })
    }

    /// Borrow the resource namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Borrow the resource kind.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Borrow the resource name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for KubernetesResourceRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}/{}", self.namespace, self.kind, self.name)
    }
}

impl<'de> Deserialize<'de> for KubernetesResourceRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = KubernetesResourceRefFields::deserialize(deserializer)?;
        Self::new(fields.namespace, fields.kind, fields.name).map_err(D::Error::custom)
    }
}

/// Kubernetes Pod reference used by the app graph model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct PodRef {
    namespace: String,
    name: String,
}

impl PodRef {
    /// Create a [`PodRef`] from validated namespace and name parts.
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Result<Self, PodRefError> {
        let namespace = namespace.into();
        let name = name.into();

        validate_session_token(&namespace)
            .map_err(WorkloadTokenError::from)
            .map_err(PodRefError::Namespace)?;
        validate_session_token(&name)
            .map_err(WorkloadTokenError::from)
            .map_err(PodRefError::Name)?;

        Ok(Self { namespace, name })
    }

    /// Borrow the Pod namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Borrow the Pod name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for PodRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.namespace, self.name)
    }
}

impl<'de> Deserialize<'de> for PodRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = PodRefFields::deserialize(deserializer)?;
        Self::new(fields.namespace, fields.name).map_err(D::Error::custom)
    }
}

/// Stable Kubernetes Service reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct ServiceRef {
    namespace: String,
    name: String,
}

impl ServiceRef {
    /// Create a [`ServiceRef`] from validated namespace and Service name parts.
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, ServiceRefError> {
        let namespace = namespace.into();
        let name = name.into();
        validate_session_token(&namespace)
            .map_err(WorkloadTokenError::from)
            .map_err(ServiceRefError::Namespace)?;
        validate_session_token(&name)
            .map_err(WorkloadTokenError::from)
            .map_err(ServiceRefError::Name)?;
        Ok(Self { namespace, name })
    }

    /// Borrow the Service namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Borrow the Service name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ServiceRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.namespace, self.name)
    }
}

impl<'de> Deserialize<'de> for ServiceRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = ServiceRefFields::deserialize(deserializer)?;
        Self::new(fields.namespace, fields.name).map_err(D::Error::custom)
    }
}

/// Stable Kubernetes ConfigMap reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct ConfigMapRef {
    namespace: String,
    name: String,
}

impl ConfigMapRef {
    /// Create a [`ConfigMapRef`] from validated namespace and ConfigMap name parts.
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, ConfigMapRefError> {
        let namespace = namespace.into();
        let name = name.into();
        validate_session_token(&namespace)
            .map_err(WorkloadTokenError::from)
            .map_err(ConfigMapRefError::Namespace)?;
        validate_session_token(&name)
            .map_err(WorkloadTokenError::from)
            .map_err(ConfigMapRefError::Name)?;
        Ok(Self { namespace, name })
    }

    /// Borrow the ConfigMap namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Borrow the ConfigMap name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ConfigMapRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.namespace, self.name)
    }
}

impl<'de> Deserialize<'de> for ConfigMapRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = ConfigMapRefFields::deserialize(deserializer)?;
        Self::new(fields.namespace, fields.name).map_err(D::Error::custom)
    }
}

/// Stable Kubernetes Secret metadata reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct SecretMetadataRef {
    namespace: String,
    name: String,
}

impl SecretMetadataRef {
    /// Create a [`SecretMetadataRef`] from validated namespace and Secret name parts.
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, SecretMetadataRefError> {
        let namespace = namespace.into();
        let name = name.into();
        validate_session_token(&namespace)
            .map_err(WorkloadTokenError::from)
            .map_err(SecretMetadataRefError::Namespace)?;
        validate_session_token(&name)
            .map_err(WorkloadTokenError::from)
            .map_err(SecretMetadataRefError::Name)?;
        Ok(Self { namespace, name })
    }

    /// Borrow the Secret namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Borrow the Secret name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for SecretMetadataRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.namespace, self.name)
    }
}

impl<'de> Deserialize<'de> for SecretMetadataRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = SecretMetadataRefFields::deserialize(deserializer)?;
        Self::new(fields.namespace, fields.name).map_err(D::Error::custom)
    }
}

/// Stable Kubernetes route reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct RouteRef {
    namespace: String,
    kind: String,
    name: String,
}

impl RouteRef {
    /// Create a [`RouteRef`] from validated namespace, kind, and name parts.
    pub fn new(
        namespace: impl Into<String>,
        kind: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, RouteRefError> {
        let namespace = namespace.into();
        let kind = kind.into();
        let name = name.into();
        validate_session_token(&namespace)
            .map_err(WorkloadTokenError::from)
            .map_err(RouteRefError::Namespace)?;
        validate_workload_kind(&kind).map_err(RouteRefError::Kind)?;
        validate_session_token(&name)
            .map_err(WorkloadTokenError::from)
            .map_err(RouteRefError::Name)?;
        Ok(Self {
            namespace,
            kind,
            name,
        })
    }

    /// Borrow the route namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Borrow the route kind.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Borrow the route name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for RouteRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}/{}", self.namespace, self.kind, self.name)
    }
}

impl<'de> Deserialize<'de> for RouteRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = RouteRefFields::deserialize(deserializer)?;
        Self::new(fields.namespace, fields.kind, fields.name).map_err(D::Error::custom)
    }
}

/// Directed graph edge from a Service to a route object.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ServiceRouteRef {
    service: ServiceRef,
    route: RouteRef,
}

impl ServiceRouteRef {
    /// Create a [`ServiceRouteRef`] from validated Service and route references.
    pub fn new(service: ServiceRef, route: RouteRef) -> Self {
        Self { service, route }
    }

    /// Borrow the Service side of the route reference.
    pub fn service(&self) -> &ServiceRef {
        &self.service
    }

    /// Borrow the route side of the route reference.
    pub fn route(&self) -> &RouteRef {
        &self.route
    }
}

/// Stable container reference within a workload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct ContainerRef {
    workload: WorkloadRef,
    name: String,
}

impl ContainerRef {
    /// Create a [`ContainerRef`] from a workload and validated container name.
    pub fn new(workload: WorkloadRef, name: impl Into<String>) -> Result<Self, ContainerRefError> {
        let name = name.into();
        validate_session_token(&name)
            .map_err(WorkloadTokenError::from)
            .map_err(ContainerRefError::Name)?;
        Ok(Self { workload, name })
    }

    /// Borrow the workload that owns this container.
    pub fn workload(&self) -> &WorkloadRef {
        &self.workload
    }

    /// Borrow the container name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ContainerRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.workload, self.name)
    }
}

impl<'de> Deserialize<'de> for ContainerRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = ContainerRefFields::deserialize(deserializer)?;
        Self::new(fields.workload, fields.name).map_err(D::Error::custom)
    }
}

/// Probe facts discovered for a workload container.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProbeFacts {
    container: ContainerRef,
    readiness_probe: bool,
    liveness_probe: bool,
    startup_probe: bool,
}

impl ProbeFacts {
    /// Create [`ProbeFacts`] for a validated container reference.
    pub fn new(
        container: ContainerRef,
        readiness_probe: bool,
        liveness_probe: bool,
        startup_probe: bool,
    ) -> Self {
        Self {
            container,
            readiness_probe,
            liveness_probe,
            startup_probe,
        }
    }

    /// Borrow the container these probe facts describe.
    pub fn container(&self) -> &ContainerRef {
        &self.container
    }

    /// Return whether a readiness probe is configured.
    pub fn readiness_probe(&self) -> bool {
        self.readiness_probe
    }

    /// Return whether a liveness probe is configured.
    pub fn liveness_probe(&self) -> bool {
        self.liveness_probe
    }

    /// Return whether a startup probe is configured.
    pub fn startup_probe(&self) -> bool {
        self.startup_probe
    }
}

/// Kubernetes container probe kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeKind {
    /// Readiness probe kind.
    Readiness,
    /// Liveness probe kind.
    Liveness,
    /// Startup probe kind.
    Startup,
}

impl ProbeKind {
    /// Return the stable serialized probe kind name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Readiness => "readiness",
            Self::Liveness => "liveness",
            Self::Startup => "startup",
        }
    }
}

impl fmt::Display for ProbeKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Image facts discovered for a workload container.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ImageFacts {
    container: ContainerRef,
    image: ImageRef,
}

impl ImageFacts {
    /// Create [`ImageFacts`] for a validated container and image reference.
    pub fn new(container: ContainerRef, image: ImageRef) -> Self {
        Self { container, image }
    }

    /// Borrow the container these image facts describe.
    pub fn container(&self) -> &ContainerRef {
        &self.container
    }

    /// Borrow the image reference configured for this container.
    pub fn image(&self) -> &ImageRef {
        &self.image
    }
}

/// Kubernetes resource quantity string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct ResourceQuantity(String);

impl ResourceQuantity {
    /// Create a [`ResourceQuantity`] from a validated Kubernetes quantity string.
    pub fn new(value: impl Into<String>) -> Result<Self, ResourceQuantityError> {
        let value = value.into();
        validate_resource_quantity(&value)?;
        Ok(Self(value))
    }

    /// Borrow the resource quantity as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceQuantity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ResourceQuantity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_validated_string(deserializer, Self::new)
    }
}

/// Resource facts discovered for a workload container.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceFacts {
    container: ContainerRef,
    cpu_request: Option<ResourceQuantity>,
    cpu_limit: Option<ResourceQuantity>,
    memory_request: Option<ResourceQuantity>,
    memory_limit: Option<ResourceQuantity>,
}

impl ResourceFacts {
    /// Create [`ResourceFacts`] for a validated container reference.
    pub fn new(
        container: ContainerRef,
        cpu_request: Option<ResourceQuantity>,
        cpu_limit: Option<ResourceQuantity>,
        memory_request: Option<ResourceQuantity>,
        memory_limit: Option<ResourceQuantity>,
    ) -> Self {
        Self {
            container,
            cpu_request,
            cpu_limit,
            memory_request,
            memory_limit,
        }
    }

    /// Borrow the container these resource facts describe.
    pub fn container(&self) -> &ContainerRef {
        &self.container
    }

    /// Borrow the CPU request quantity when configured.
    pub fn cpu_request(&self) -> Option<&ResourceQuantity> {
        self.cpu_request.as_ref()
    }

    /// Borrow the CPU limit quantity when configured.
    pub fn cpu_limit(&self) -> Option<&ResourceQuantity> {
        self.cpu_limit.as_ref()
    }

    /// Borrow the memory request quantity when configured.
    pub fn memory_request(&self) -> Option<&ResourceQuantity> {
        self.memory_request.as_ref()
    }

    /// Borrow the memory limit quantity when configured.
    pub fn memory_limit(&self) -> Option<&ResourceQuantity> {
        self.memory_limit.as_ref()
    }
}

/// Metadata reference from a workload container to a ConfigMap.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConfigReference {
    container: ContainerRef,
    config_map: ConfigMapRef,
}

impl ConfigReference {
    /// Create a [`ConfigReference`] from validated container and ConfigMap references.
    pub fn new(container: ContainerRef, config_map: ConfigMapRef) -> Self {
        Self {
            container,
            config_map,
        }
    }

    /// Borrow the container side of the ConfigMap reference.
    pub fn container(&self) -> &ContainerRef {
        &self.container
    }

    /// Borrow the ConfigMap metadata reference.
    pub fn config_map(&self) -> &ConfigMapRef {
        &self.config_map
    }
}

/// Metadata reference from a workload container to a Secret.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SecretReference {
    container: ContainerRef,
    secret: SecretMetadataRef,
}

impl SecretReference {
    /// Create a [`SecretReference`] from validated container and Secret metadata references.
    pub fn new(container: ContainerRef, secret: SecretMetadataRef) -> Self {
        Self { container, secret }
    }

    /// Borrow the container side of the Secret reference.
    pub fn container(&self) -> &ContainerRef {
        &self.container
    }

    /// Borrow the Secret metadata reference.
    pub fn secret(&self) -> &SecretMetadataRef {
        &self.secret
    }
}

/// App graph relationship that can carry confidence metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphRelationship {
    /// Relationship between the root workload and an owned Pod.
    WorkloadPodOwnership { pod: PodRef },
    /// Relationship between the root workload and a selecting Service.
    WorkloadServiceSelection { service: ServiceRef },
    /// Relationship between a Service and a route object.
    ServiceRouteReference {
        service: ServiceRef,
        route: RouteRef,
    },
    /// Metadata relationship between a container and a ConfigMap.
    ContainerConfigReference {
        container: ContainerRef,
        config_map: ConfigMapRef,
    },
    /// Metadata relationship between a container and a Secret.
    ContainerSecretReference {
        container: ContainerRef,
        secret: SecretMetadataRef,
    },
}

/// Confidence level assigned to an inferred graph relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    /// Low confidence relationship.
    Low,
    /// Medium confidence relationship.
    Medium,
    /// High confidence relationship.
    High,
}

impl ConfidenceLevel {
    /// Return the stable serialized confidence level name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl fmt::Display for ConfidenceLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Confidence metadata for an inferred graph relationship.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationshipConfidence {
    relationship: GraphRelationship,
    confidence: ConfidenceLevel,
}

impl RelationshipConfidence {
    /// Create confidence metadata for a graph relationship.
    pub fn new(relationship: GraphRelationship, confidence: ConfidenceLevel) -> Self {
        Self {
            relationship,
            confidence,
        }
    }

    /// Borrow the graph relationship this confidence describes.
    pub fn relationship(&self) -> &GraphRelationship {
        &self.relationship
    }

    /// Return the confidence level for this relationship.
    pub const fn confidence(&self) -> ConfidenceLevel {
        self.confidence
    }
}

/// Warning emitted while building the app graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppGraphWarning {
    /// A Service selector can plausibly refer to more than one workload.
    AmbiguousServiceSelector {
        service: ServiceRef,
        candidate_workloads: Vec<WorkloadRef>,
    },
    /// A selected Service has no discovered route reference.
    MissingRoute { service: ServiceRef },
    /// A [`ContainerRef`] is missing one or more [`ProbeKind`] entries.
    MissingProbes {
        container: ContainerRef,
        missing_probes: Vec<ProbeKind>,
    },
}

impl AppGraphWarning {
    /// Create an ambiguous Service selector warning with deterministic candidates.
    pub fn ambiguous_service_selector(
        service: ServiceRef,
        candidate_workloads: impl IntoIterator<Item = WorkloadRef>,
    ) -> Self {
        let mut candidate_workloads = candidate_workloads.into_iter().collect::<Vec<_>>();
        candidate_workloads.sort_unstable();
        candidate_workloads.dedup();
        Self::AmbiguousServiceSelector {
            service,
            candidate_workloads,
        }
    }

    /// Create a missing route warning for a selected [`ServiceRef`].
    pub fn missing_route(service: ServiceRef) -> Self {
        Self::MissingRoute { service }
    }

    /// Create a missing probes warning with deterministic [`ProbeKind`] entries.
    pub fn missing_probes(
        container: ContainerRef,
        missing_probes: impl IntoIterator<Item = ProbeKind>,
    ) -> Self {
        let mut missing_probes = missing_probes.into_iter().collect::<Vec<_>>();
        missing_probes.sort_unstable();
        missing_probes.dedup();
        Self::MissingProbes {
            container,
            missing_probes,
        }
    }
}

impl<'de> Deserialize<'de> for AppGraphWarning {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match AppGraphWarningFields::deserialize(deserializer)? {
            AppGraphWarningFields::AmbiguousServiceSelector {
                service,
                candidate_workloads,
            } => Ok(Self::ambiguous_service_selector(
                service,
                candidate_workloads,
            )),
            AppGraphWarningFields::MissingRoute { service } => Ok(Self::missing_route(service)),
            AppGraphWarningFields::MissingProbes {
                container,
                missing_probes,
            } => Ok(Self::missing_probes(container, missing_probes)),
        }
    }
}

/// App-level graph rooted at a Kubernetes workload.
///
/// The graph stores relationships as Kply domain references instead of raw
/// Kubernetes client types. Future roadmap tasks add service, route, fact,
/// confidence, and warning relationships.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct AppGraph {
    workload: WorkloadRef,
    #[serde(default)]
    owned_pods: Vec<PodRef>,
    #[serde(default)]
    selecting_services: Vec<ServiceRef>,
    #[serde(default)]
    service_routes: Vec<ServiceRouteRef>,
    #[serde(default)]
    probe_facts: Vec<ProbeFacts>,
    #[serde(default)]
    image_facts: Vec<ImageFacts>,
    #[serde(default)]
    resource_facts: Vec<ResourceFacts>,
    #[serde(default)]
    config_references: Vec<ConfigReference>,
    #[serde(default)]
    secret_references: Vec<SecretReference>,
    #[serde(default)]
    relationship_confidences: Vec<RelationshipConfidence>,
    #[serde(default)]
    warnings: Vec<AppGraphWarning>,
}

impl AppGraph {
    /// Create an [`AppGraph`] rooted at a validated workload reference.
    pub fn new(workload: WorkloadRef) -> Self {
        Self {
            workload,
            owned_pods: Vec::new(),
            selecting_services: Vec::new(),
            service_routes: Vec::new(),
            probe_facts: Vec::new(),
            image_facts: Vec::new(),
            resource_facts: Vec::new(),
            config_references: Vec::new(),
            secret_references: Vec::new(),
            relationship_confidences: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Return a copy of this graph with Pods owned by the root workload.
    pub fn with_owned_pods(mut self, owned_pods: impl IntoIterator<Item = PodRef>) -> Self {
        self.owned_pods = owned_pods.into_iter().collect();
        self.owned_pods.sort_unstable();
        self.owned_pods.dedup();
        self
    }

    /// Return a copy of this graph with graph-building warnings.
    pub fn with_warnings(mut self, warnings: impl IntoIterator<Item = AppGraphWarning>) -> Self {
        self.warnings = warnings.into_iter().collect();
        self.warnings.sort_unstable();
        self.warnings.dedup();
        self
    }

    /// Return a copy of this graph with relationship confidence metadata.
    pub fn with_relationship_confidences(
        mut self,
        relationship_confidences: impl IntoIterator<Item = RelationshipConfidence>,
    ) -> Self {
        self.relationship_confidences = relationship_confidences.into_iter().collect();
        self.relationship_confidences.sort_unstable();
        self.relationship_confidences.dedup();
        self
    }

    /// Return a copy of this graph with ConfigMap metadata references.
    pub fn with_config_references(
        mut self,
        config_references: impl IntoIterator<Item = ConfigReference>,
    ) -> Self {
        self.config_references = config_references.into_iter().collect();
        self.config_references.sort_unstable();
        self.config_references.dedup();
        self
    }

    /// Return a copy of this graph with Secret metadata references.
    pub fn with_secret_references(
        mut self,
        secret_references: impl IntoIterator<Item = SecretReference>,
    ) -> Self {
        self.secret_references = secret_references.into_iter().collect();
        self.secret_references.sort_unstable();
        self.secret_references.dedup();
        self
    }

    /// Return a copy of this graph with resource facts for workload containers.
    pub fn with_resource_facts(
        mut self,
        resource_facts: impl IntoIterator<Item = ResourceFacts>,
    ) -> Self {
        self.resource_facts = resource_facts.into_iter().collect();
        self.resource_facts.sort_unstable();
        self.resource_facts.dedup();
        self
    }

    /// Return a copy of this graph with image facts for workload containers.
    pub fn with_image_facts(mut self, image_facts: impl IntoIterator<Item = ImageFacts>) -> Self {
        self.image_facts = image_facts.into_iter().collect();
        self.image_facts.sort_unstable();
        self.image_facts.dedup();
        self
    }

    /// Return a copy of this graph with probe facts for workload containers.
    pub fn with_probe_facts(mut self, probe_facts: impl IntoIterator<Item = ProbeFacts>) -> Self {
        self.probe_facts = probe_facts.into_iter().collect();
        self.probe_facts.sort_unstable();
        self.probe_facts.dedup();
        self
    }

    /// Return a copy of this graph with route references for selected Services.
    pub fn with_service_routes(
        mut self,
        service_routes: impl IntoIterator<Item = ServiceRouteRef>,
    ) -> Self {
        self.service_routes = service_routes.into_iter().collect();
        self.service_routes.sort_unstable();
        self.service_routes.dedup();
        self
    }

    /// Return a copy of this graph with Services selecting the root workload.
    pub fn with_selecting_services(
        mut self,
        selecting_services: impl IntoIterator<Item = ServiceRef>,
    ) -> Self {
        self.selecting_services = selecting_services.into_iter().collect();
        self.selecting_services.sort_unstable();
        self.selecting_services.dedup();
        self
    }

    /// Borrow the root workload for this app graph.
    pub fn workload(&self) -> &WorkloadRef {
        &self.workload
    }

    /// Borrow Pods owned by the root workload in deterministic order.
    pub fn owned_pods(&self) -> &[PodRef] {
        &self.owned_pods
    }

    /// Borrow Services selecting the root workload in deterministic order.
    pub fn selecting_services(&self) -> &[ServiceRef] {
        &self.selecting_services
    }

    /// Borrow route references for selected Services in deterministic order.
    pub fn service_routes(&self) -> &[ServiceRouteRef] {
        &self.service_routes
    }

    /// Borrow probe facts for workload containers in deterministic order.
    pub fn probe_facts(&self) -> &[ProbeFacts] {
        &self.probe_facts
    }

    /// Borrow image facts for workload containers in deterministic order.
    pub fn image_facts(&self) -> &[ImageFacts] {
        &self.image_facts
    }

    /// Borrow resource facts for workload containers in deterministic order.
    pub fn resource_facts(&self) -> &[ResourceFacts] {
        &self.resource_facts
    }

    /// Borrow ConfigMap metadata references in deterministic order.
    pub fn config_references(&self) -> &[ConfigReference] {
        &self.config_references
    }

    /// Borrow Secret metadata references in deterministic order.
    pub fn secret_references(&self) -> &[SecretReference] {
        &self.secret_references
    }

    /// Borrow relationship confidence metadata in deterministic order.
    pub fn relationship_confidences(&self) -> &[RelationshipConfidence] {
        &self.relationship_confidences
    }

    /// Borrow graph-building warnings in deterministic order.
    pub fn warnings(&self) -> &[AppGraphWarning] {
        &self.warnings
    }
}

impl<'de> Deserialize<'de> for AppGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = AppGraphFields::deserialize(deserializer)?;
        Ok(Self::new(fields.workload)
            .with_owned_pods(fields.owned_pods)
            .with_selecting_services(fields.selecting_services)
            .with_service_routes(fields.service_routes)
            .with_probe_facts(fields.probe_facts)
            .with_image_facts(fields.image_facts)
            .with_resource_facts(fields.resource_facts)
            .with_config_references(fields.config_references)
            .with_secret_references(fields.secret_references)
            .with_relationship_confidences(fields.relationship_confidences)
            .with_warnings(fields.warnings))
    }
}

/// Container image reference proposed for a future sandbox workload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct ImageRef(String);

impl ImageRef {
    /// Create an [`ImageRef`] from a validated image reference string.
    pub fn new(value: impl Into<String>) -> Result<Self, ImageRefError> {
        let value = value.into();
        validate_image_ref(&value)?;
        Ok(Self(value))
    }

    /// Borrow the image reference as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ImageRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<String> for ImageRef {
    type Error = ImageRefError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ImageRef> for String {
    fn from(value: ImageRef) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for ImageRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_validated_string(deserializer, Self::new)
    }
}

/// Positive session lifetime using compact duration spelling.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct TimeToLive(String);

impl TimeToLive {
    /// Create a [`TimeToLive`] from a validated duration string.
    pub fn new(value: impl Into<String>) -> Result<Self, TimeToLiveError> {
        let value = value.into();
        validate_time_to_live(&value)?;
        Ok(Self(value))
    }

    /// Borrow the duration value as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TimeToLive {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<String> for TimeToLive {
    type Error = TimeToLiveError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<TimeToLive> for String {
    fn from(value: TimeToLive) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for TimeToLive {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_validated_string(deserializer, Self::new)
    }
}

/// Traffic selector for routing future test requests to a sandbox workload.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RouteSelector {
    /// Match requests by HTTP header name and value.
    Header { name: String, value: String },
    /// Match requests by host name.
    Host { hostname: String },
}

impl RouteSelector {
    /// Create a header-based [`RouteSelector`].
    pub fn header(
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, RouteSelectorError> {
        let name = name.into();
        let value = value.into();

        validate_route_header_name(&name).map_err(RouteSelectorError::HeaderName)?;
        validate_route_header_value(&value).map_err(RouteSelectorError::HeaderValue)?;

        Ok(Self::Header { name, value })
    }

    /// Create a host-based [`RouteSelector`].
    pub fn host(hostname: impl Into<String>) -> Result<Self, RouteSelectorError> {
        let hostname = hostname.into();
        validate_route_host(&hostname).map_err(RouteSelectorError::Host)?;

        Ok(Self::Host { hostname })
    }

    /// Return the stable selector kind used in agent-readable output.
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Header { .. } => "header",
            Self::Host { .. } => "host",
        }
    }

    /// Borrow the header selector parts when this selector matches by header.
    pub fn header_parts(&self) -> Option<(&str, &str)> {
        match self {
            Self::Header { name, value } => Some((name, value)),
            Self::Host { .. } => None,
        }
    }

    /// Borrow the host name when this selector matches by host.
    pub fn hostname(&self) -> Option<&str> {
        match self {
            Self::Header { .. } => None,
            Self::Host { hostname } => Some(hostname),
        }
    }
}

impl fmt::Display for RouteSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header { name, value } => write!(formatter, "header:{name}={value}"),
            Self::Host { hostname } => write!(formatter, "host:{hostname}"),
        }
    }
}

impl<'de> Deserialize<'de> for RouteSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match RouteSelectorFields::deserialize(deserializer)? {
            RouteSelectorFields::Header { name, value } => {
                Self::header(name, value).map_err(D::Error::custom)
            }
            RouteSelectorFields::Host { hostname } => {
                Self::host(hostname).map_err(D::Error::custom)
            }
        }
    }
}

/// Operation that a future Kply session policy may allow.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionOperation {
    /// Read workload and cluster state without mutating resources.
    Inspect,
    /// Produce a dry-run session plan.
    Plan,
    /// Create or update temporary sandbox resources.
    Prepare,
    /// Configure temporary test traffic routing.
    Route,
    /// Run checks against the active sandbox session.
    Verify,
    /// Remove temporary session resources.
    Cleanup,
    /// Promote a verified change outside the sandbox boundary.
    Promote,
}

impl SessionOperation {
    /// Return every known session operation in declaration order.
    pub const fn all() -> &'static [Self] {
        &[
            Self::Inspect,
            Self::Plan,
            Self::Prepare,
            Self::Route,
            Self::Verify,
            Self::Cleanup,
            Self::Promote,
        ]
    }

    /// Return the stable snake_case operation name used in agent-readable output.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Plan => "plan",
            Self::Prepare => "prepare",
            Self::Route => "route",
            Self::Verify => "verify",
            Self::Cleanup => "cleanup",
            Self::Promote => "promote",
        }
    }
}

impl fmt::Display for SessionOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Allowed operation set for a future Kply session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SessionPolicy {
    allowed_operations: Vec<SessionOperation>,
}

impl SessionPolicy {
    /// Create a [`SessionPolicy`] from a non-empty list of unique operations.
    pub fn new(
        allowed_operations: impl IntoIterator<Item = SessionOperation>,
    ) -> Result<Self, SessionPolicyError> {
        let mut allowed_operations = allowed_operations.into_iter().collect::<Vec<_>>();

        if allowed_operations.is_empty() {
            return Err(SessionPolicyError::Empty);
        }

        allowed_operations.sort_unstable();
        if let Some(operation) = duplicate_session_operation(&allowed_operations) {
            return Err(SessionPolicyError::Duplicate { operation });
        }

        Ok(Self { allowed_operations })
    }

    /// Create the default sandbox-only [`SessionPolicy`].
    pub fn sandbox() -> Self {
        let mut allowed_operations = vec![
            SessionOperation::Inspect,
            SessionOperation::Plan,
            SessionOperation::Prepare,
            SessionOperation::Route,
            SessionOperation::Verify,
            SessionOperation::Cleanup,
        ];
        allowed_operations.sort_unstable();

        Self { allowed_operations }
    }

    /// Borrow the policy's allowed operations in stable order.
    pub fn allowed_operations(&self) -> &[SessionOperation] {
        &self.allowed_operations
    }

    /// Return whether the policy allows the given operation.
    pub fn allows(&self, operation: SessionOperation) -> bool {
        self.allowed_operations.binary_search(&operation).is_ok()
    }
}

impl Default for SessionPolicy {
    fn default() -> Self {
        Self::sandbox()
    }
}

impl<'de> Deserialize<'de> for SessionPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = SessionPolicyFields::deserialize(deserializer)?;
        Self::new(fields.allowed_operations).map_err(D::Error::custom)
    }
}

/// Error returned when a [`SessionPolicy`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionPolicyError {
    /// Session policies must allow at least one operation.
    Empty,
    /// Session policies cannot contain the same operation more than once.
    Duplicate { operation: SessionOperation },
}

impl fmt::Display for SessionPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("session policy cannot be empty"),
            Self::Duplicate { operation } => {
                write!(
                    formatter,
                    "session policy contains duplicate operation `{operation}`"
                )
            }
        }
    }
}

impl std::error::Error for SessionPolicyError {}

/// Dry-run description of a future Kply session.
///
/// A plan captures the [`SessionId`], [`SessionName`], target [`WorkloadRef`],
/// proposed [`ImageRef`], optional time-to-live, planned
/// [`KubernetesResourceRef`] resources, optional [`RouteSelector`],
/// [`SessionPolicy`], and initial [`SessionStatus`] for a session that has not
/// yet been executed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SessionPlan {
    id: SessionId,
    name: SessionName,
    workload: WorkloadRef,
    image: ImageRef,
    #[serde(rename = "ttl")]
    time_to_live: Option<TimeToLive>,
    planned_resources: Vec<KubernetesResourceRef>,
    route_selector: Option<RouteSelector>,
    policy: SessionPolicy,
    status: SessionStatus,
}

impl SessionPlan {
    /// Create a [`SessionPlan`] for a future sandbox session.
    pub fn new(
        id: SessionId,
        name: SessionName,
        workload: WorkloadRef,
        image: ImageRef,
        policy: SessionPolicy,
    ) -> Self {
        Self {
            id,
            name,
            workload,
            image,
            time_to_live: None,
            planned_resources: Vec::new(),
            route_selector: None,
            policy,
            status: SessionStatus::Planned,
        }
    }

    /// Return a copy of this plan with a session lifetime.
    pub fn with_time_to_live(mut self, time_to_live: TimeToLive) -> Self {
        self.time_to_live = Some(time_to_live);
        self
    }

    /// Return a copy of this plan with planned [`KubernetesResourceRef`] resources.
    pub fn with_planned_resources(
        mut self,
        planned_resources: impl IntoIterator<Item = KubernetesResourceRef>,
    ) -> Self {
        self.planned_resources = planned_resources.into_iter().collect();
        self.planned_resources.sort_unstable();
        self.planned_resources.dedup();
        self
    }

    /// Return a copy of this plan with a test traffic [`RouteSelector`].
    pub fn with_route_selector(mut self, route_selector: RouteSelector) -> Self {
        self.route_selector = Some(route_selector);
        self
    }

    /// Borrow the [`SessionId`].
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// Borrow the [`SessionName`].
    pub fn name(&self) -> &SessionName {
        &self.name
    }

    /// Borrow the target [`WorkloadRef`].
    pub fn workload(&self) -> &WorkloadRef {
        &self.workload
    }

    /// Borrow the proposed sandbox [`ImageRef`].
    pub fn image(&self) -> &ImageRef {
        &self.image
    }

    /// Borrow the optional session lifetime.
    pub fn time_to_live(&self) -> Option<&TimeToLive> {
        self.time_to_live.as_ref()
    }

    /// Borrow planned [`KubernetesResourceRef`] resources in deterministic order.
    pub fn planned_resources(&self) -> &[KubernetesResourceRef] {
        &self.planned_resources
    }

    /// Borrow the optional [`RouteSelector`].
    pub fn route_selector(&self) -> Option<&RouteSelector> {
        self.route_selector.as_ref()
    }

    /// Borrow the [`SessionPolicy`].
    pub fn policy(&self) -> &SessionPolicy {
        &self.policy
    }

    /// Return the planned [`SessionStatus`].
    pub const fn status(&self) -> SessionStatus {
        self.status
    }
}

impl<'de> Deserialize<'de> for SessionPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = SessionPlanFields::deserialize(deserializer)?;
        if fields.status != SessionStatus::Planned {
            return Err(D::Error::custom(format!(
                "session plan status `{}` is not planned",
                fields.status
            )));
        }

        let mut plan = Self::new(
            fields.id,
            fields.name,
            fields.workload,
            fields.image,
            fields.policy,
        );
        plan = plan.with_planned_resources(fields.planned_resources);
        if let Some(route_selector) = fields.route_selector {
            plan = plan.with_route_selector(route_selector);
        }
        if let Some(time_to_live) = fields.time_to_live {
            plan = plan.with_time_to_live(time_to_live);
        }

        Ok(plan)
    }
}

/// Final report for an executed [`SessionPlan`].
///
/// A report preserves the original [`SessionPlan`] and records a reportable
/// [`SessionStatus`] such as [`SessionStatus::Ready`], [`SessionStatus::Blocked`],
/// [`SessionStatus::CleanedUp`], or [`SessionStatus::Failed`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SessionReport {
    plan: SessionPlan,
    status: SessionStatus,
}

impl SessionReport {
    /// Create a [`SessionReport`] from a plan and reportable status.
    pub fn new(plan: SessionPlan, status: SessionStatus) -> Result<Self, SessionReportError> {
        if !is_report_status(status) {
            return Err(SessionReportError::NonReportableStatus { status });
        }

        Ok(Self { plan, status })
    }

    /// Borrow the original [`SessionPlan`].
    pub fn plan(&self) -> &SessionPlan {
        &self.plan
    }

    /// Return the final report [`SessionStatus`].
    pub const fn status(&self) -> SessionStatus {
        self.status
    }
}

/// Error returned when a [`SessionReport`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionReportError {
    /// Session reports must use a reportable terminal or blocked status.
    NonReportableStatus { status: SessionStatus },
}

impl fmt::Display for SessionReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonReportableStatus { status } => {
                write!(
                    formatter,
                    "session report status `{status}` is not reportable"
                )
            }
        }
    }
}

impl std::error::Error for SessionReportError {}

/// Error returned when a session status transition is not valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTransitionError {
    /// The requested transition is not allowed by the session lifecycle.
    Invalid {
        /// Current session status.
        from: SessionStatus,
        /// Requested next session status.
        to: SessionStatus,
    },
}

impl fmt::Display for SessionTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid { from, to } => {
                write!(
                    formatter,
                    "cannot transition session from `{from}` to `{to}`"
                )
            }
        }
    }
}

impl std::error::Error for SessionTransitionError {}

impl<'de> Deserialize<'de> for SessionReport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = SessionReportFields::deserialize(deserializer)?;
        Self::new(fields.plan, fields.status).map_err(D::Error::custom)
    }
}

/// Audit event kind for future Kply session history.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventKind {
    /// A session plan was created.
    Planned,
    /// Temporary sandbox resources are being prepared.
    Preparing,
    /// The session became active for test traffic.
    Active,
    /// Session verification started.
    Verifying,
    /// The session became blocked.
    Blocked,
    /// The session became ready for approval or promotion.
    Ready,
    /// Temporary session resources were cleaned up.
    CleanedUp,
    /// The session failed.
    Failed,
}

impl SessionEventKind {
    /// Return every known session event kind in declaration order.
    pub const fn all() -> &'static [Self] {
        &[
            Self::Planned,
            Self::Preparing,
            Self::Active,
            Self::Verifying,
            Self::Blocked,
            Self::Ready,
            Self::CleanedUp,
            Self::Failed,
        ]
    }

    /// Return the stable snake_case event kind name used in agent-readable output.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Preparing => "preparing",
            Self::Active => "active",
            Self::Verifying => "verifying",
            Self::Blocked => "blocked",
            Self::Ready => "ready",
            Self::CleanedUp => "cleaned_up",
            Self::Failed => "failed",
        }
    }

    /// Return the [`SessionStatus`] represented by this event kind.
    pub const fn status(&self) -> SessionStatus {
        match self {
            Self::Planned => SessionStatus::Planned,
            Self::Preparing => SessionStatus::Preparing,
            Self::Active => SessionStatus::Active,
            Self::Verifying => SessionStatus::Verifying,
            Self::Blocked => SessionStatus::Blocked,
            Self::Ready => SessionStatus::Ready,
            Self::CleanedUp => SessionStatus::CleanedUp,
            Self::Failed => SessionStatus::Failed,
        }
    }
}

impl fmt::Display for SessionEventKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Deterministic audit event for future Kply session history.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct SessionEvent {
    session_id: SessionId,
    sequence: u64,
    kind: SessionEventKind,
    status: SessionStatus,
}

impl SessionEvent {
    /// Create a [`SessionEvent`] from a session id, sequence, and event kind.
    pub fn new(session_id: SessionId, sequence: u64, kind: SessionEventKind) -> Self {
        Self {
            session_id,
            sequence,
            kind,
            status: kind.status(),
        }
    }

    /// Borrow the event [`SessionId`].
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Return the event sequence number.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Return the [`SessionEventKind`].
    pub const fn kind(&self) -> SessionEventKind {
        self.kind
    }

    /// Return the event [`SessionStatus`].
    pub const fn status(&self) -> SessionStatus {
        self.status
    }
}

impl<'de> Deserialize<'de> for SessionEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = SessionEventFields::deserialize(deserializer)?;
        let event = Self::new(fields.session_id, fields.sequence, fields.kind);
        if event.status != fields.status {
            return Err(D::Error::custom(format!(
                "session event status `{}` does not match kind `{}`",
                fields.status, fields.kind
            )));
        }

        Ok(event)
    }
}

/// Error returned when a [`SessionId`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionIdError {
    /// Session identifiers cannot be empty.
    Empty,
    /// Session identifiers must fit common Kubernetes label value limits.
    TooLong { max_len: usize },
    /// Session identifiers must start and end with a lowercase ASCII letter or digit.
    InvalidBoundary,
    /// Session identifiers only allow lowercase ASCII letters, digits, and hyphens.
    InvalidCharacter { character: char },
}

impl fmt::Display for SessionIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("session id cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "session id cannot exceed {max_len} characters")
            }
            Self::InvalidBoundary => formatter
                .write_str("session id must start and end with a lowercase ASCII letter or digit"),
            Self::InvalidCharacter { character } => write!(
                formatter,
                "session id contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for SessionIdError {}

/// Error returned when a [`SessionName`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionNameError {
    /// Session names cannot be empty.
    Empty,
    /// Session names must fit common Kubernetes label value limits.
    TooLong { max_len: usize },
    /// Session names must start and end with a lowercase ASCII letter or digit.
    InvalidBoundary,
    /// Session names only allow lowercase ASCII letters, digits, and hyphens.
    InvalidCharacter { character: char },
}

impl fmt::Display for SessionNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("session name cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "session name cannot exceed {max_len} characters")
            }
            Self::InvalidBoundary => formatter.write_str(
                "session name must start and end with a lowercase ASCII letter or digit",
            ),
            Self::InvalidCharacter { character } => write!(
                formatter,
                "session name contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for SessionNameError {}

/// Error returned when an [`ImageRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageRefError {
    /// Image references cannot be empty.
    Empty,
    /// Image references must stay bounded for stable reports and labels.
    TooLong { max_len: usize },
    /// Image references must include a non-empty image name component.
    MissingName,
    /// Image references must start and end with an ASCII letter, digit, or digest value.
    InvalidBoundary,
    /// Image references only allow ASCII image reference characters.
    InvalidCharacter { character: char },
}

impl fmt::Display for ImageRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("image ref cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "image ref cannot exceed {max_len} characters")
            }
            Self::MissingName => formatter.write_str("image ref must include an image name"),
            Self::InvalidBoundary => {
                formatter.write_str("image ref has an invalid boundary character")
            }
            Self::InvalidCharacter { character } => {
                write!(
                    formatter,
                    "image ref contains invalid character `{character}`"
                )
            }
        }
    }
}

impl std::error::Error for ImageRefError {}

/// Error returned when a [`TimeToLive`] value is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeToLiveError {
    /// Time-to-live values cannot be empty.
    Empty,
    /// Time-to-live values cannot exceed the maximum length.
    TooLong { max_len: usize },
    /// Time-to-live values must end with a supported unit.
    InvalidUnit { unit: char },
    /// Time-to-live values must start with ASCII digits.
    InvalidNumber,
    /// Time-to-live values must be greater than zero.
    Zero,
}

impl fmt::Display for TimeToLiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("ttl cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "ttl cannot exceed {max_len} characters")
            }
            Self::InvalidUnit { unit } => {
                write!(
                    formatter,
                    "invalid ttl unit `{unit}`; expected s, m, h, or d"
                )
            }
            Self::InvalidNumber => {
                formatter.write_str("invalid ttl; expected a positive integer duration")
            }
            Self::Zero => formatter.write_str("invalid ttl; duration must be greater than zero"),
        }
    }
}

impl std::error::Error for TimeToLiveError {}

/// Error returned when a [`ResourceQuantity`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceQuantityError {
    /// Resource quantities cannot be empty.
    Empty,
    /// Resource quantities must stay bounded for stable reports and labels.
    TooLong { max_len: usize },
    /// Resource quantities must start and end with an ASCII letter or digit.
    InvalidBoundary,
    /// Resource quantities only allow ASCII letters, digits, dots, plus, minus, and underscores.
    InvalidCharacter { character: char },
}

impl fmt::Display for ResourceQuantityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("resource quantity cannot be empty"),
            Self::TooLong { max_len } => {
                write!(
                    formatter,
                    "resource quantity cannot exceed {max_len} characters"
                )
            }
            Self::InvalidBoundary => formatter
                .write_str("resource quantity must start and end with an ASCII letter or digit"),
            Self::InvalidCharacter { character } => write!(
                formatter,
                "resource quantity contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for ResourceQuantityError {}

/// Error returned when a [`RouteSelector`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteSelectorError {
    /// Header selector names must be valid HTTP field names.
    HeaderName(RouteHeaderNameError),
    /// Header selector values must be printable ASCII values.
    HeaderValue(RouteHeaderValueError),
    /// Host selectors must be lowercase DNS host names.
    Host(RouteHostError),
}

impl fmt::Display for RouteSelectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderName(error) => write!(formatter, "invalid route header name: {error}"),
            Self::HeaderValue(error) => write!(formatter, "invalid route header value: {error}"),
            Self::Host(error) => write!(formatter, "invalid route host: {error}"),
        }
    }
}

impl std::error::Error for RouteSelectorError {}

/// Error returned when a route header name is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteHeaderNameError {
    /// Header names cannot be empty.
    Empty,
    /// Header names must stay bounded for stable reports.
    TooLong { max_len: usize },
    /// Header names only allow ASCII HTTP token characters.
    InvalidCharacter { character: char },
}

impl fmt::Display for RouteHeaderNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("route header name cannot be empty"),
            Self::TooLong { max_len } => {
                write!(
                    formatter,
                    "route header name cannot exceed {max_len} characters"
                )
            }
            Self::InvalidCharacter { character } => write!(
                formatter,
                "route header name contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for RouteHeaderNameError {}

/// Error returned when a route header value is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteHeaderValueError {
    /// Header values cannot be empty.
    Empty,
    /// Header values must stay bounded for stable reports.
    TooLong { max_len: usize },
    /// Header values only allow visible ASCII characters.
    InvalidCharacter { character: char },
}

impl fmt::Display for RouteHeaderValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("route header value cannot be empty"),
            Self::TooLong { max_len } => {
                write!(
                    formatter,
                    "route header value cannot exceed {max_len} characters"
                )
            }
            Self::InvalidCharacter { character } => write!(
                formatter,
                "route header value contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for RouteHeaderValueError {}

/// Error returned when a route host is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteHostError {
    /// Hosts cannot be empty.
    Empty,
    /// Hosts must stay within DNS host length limits.
    TooLong { max_len: usize },
    /// Host labels must stay within DNS label length limits.
    LabelTooLong { max_len: usize },
    /// Hosts cannot contain empty labels.
    EmptyLabel,
    /// Hosts must start and end with a lowercase ASCII letter or digit.
    InvalidBoundary,
    /// Hosts only allow lowercase ASCII letters, digits, dots, and hyphens.
    InvalidCharacter { character: char },
}

impl fmt::Display for RouteHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("route host cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "route host cannot exceed {max_len} characters")
            }
            Self::LabelTooLong { max_len } => {
                write!(
                    formatter,
                    "route host label cannot exceed {max_len} characters"
                )
            }
            Self::EmptyLabel => formatter.write_str("route host cannot contain empty labels"),
            Self::InvalidBoundary => formatter
                .write_str("route host must start and end with a lowercase ASCII letter or digit"),
            Self::InvalidCharacter { character } => {
                write!(
                    formatter,
                    "route host contains invalid character `{character}`"
                )
            }
        }
    }
}

impl std::error::Error for RouteHostError {}

/// Error returned when a [`WorkloadRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadRefError {
    /// Workload namespaces use the same token rules as session names and identifiers.
    Namespace(WorkloadTokenError),
    /// Workload kinds must be non-empty Kubernetes-style kind identifiers.
    Kind(WorkloadKindError),
    /// Workload names use the same token rules as session names and identifiers.
    Name(WorkloadTokenError),
}

impl fmt::Display for WorkloadRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Namespace(error) => {
                write!(formatter, "invalid workload namespace: {error}")
            }
            Self::Kind(error) => write!(formatter, "invalid workload kind: {error}"),
            Self::Name(error) => write!(formatter, "invalid workload name: {error}"),
        }
    }
}

impl std::error::Error for WorkloadRefError {}

/// Error returned when a [`KubernetesResourceRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KubernetesResourceRefError {
    /// Resource namespaces use the same token rules as workload namespaces.
    Namespace(WorkloadTokenError),
    /// Resource kinds must be non-empty Kubernetes-style kind identifiers.
    Kind(WorkloadKindError),
    /// Resource names use the same token rules as workload names.
    Name(WorkloadTokenError),
}

impl fmt::Display for KubernetesResourceRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Namespace(error) => {
                write!(formatter, "invalid kubernetes resource namespace: {error}")
            }
            Self::Kind(error) => write!(formatter, "invalid kubernetes resource kind: {error}"),
            Self::Name(error) => write!(formatter, "invalid kubernetes resource name: {error}"),
        }
    }
}

impl std::error::Error for KubernetesResourceRefError {}

/// Error returned when a [`PodRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PodRefError {
    /// Pod namespaces use the same token rules as workload namespaces.
    Namespace(WorkloadTokenError),
    /// Pod names use the same token rules as workload names.
    Name(WorkloadTokenError),
}

impl fmt::Display for PodRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Namespace(error) => {
                write!(formatter, "invalid pod namespace: {error}")
            }
            Self::Name(error) => write!(formatter, "invalid pod name: {error}"),
        }
    }
}

impl std::error::Error for PodRefError {}

/// Error returned when a [`ServiceRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceRefError {
    /// Service namespaces use the same token rules as workload namespaces.
    Namespace(WorkloadTokenError),
    /// Service names use the same token rules as workload names.
    Name(WorkloadTokenError),
}

impl fmt::Display for ServiceRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Namespace(error) => {
                write!(formatter, "invalid service namespace: {error}")
            }
            Self::Name(error) => write!(formatter, "invalid service name: {error}"),
        }
    }
}

impl std::error::Error for ServiceRefError {}

/// Error returned when a [`ConfigMapRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigMapRefError {
    /// ConfigMap namespaces use the same token rules as workload namespaces.
    Namespace(WorkloadTokenError),
    /// ConfigMap names use the same token rules as workload names.
    Name(WorkloadTokenError),
}

impl fmt::Display for ConfigMapRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Namespace(error) => {
                write!(formatter, "invalid configmap namespace: {error}")
            }
            Self::Name(error) => write!(formatter, "invalid configmap name: {error}"),
        }
    }
}

impl std::error::Error for ConfigMapRefError {}

/// Error returned when a [`SecretMetadataRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretMetadataRefError {
    /// Secret namespaces use the same token rules as workload namespaces.
    Namespace(WorkloadTokenError),
    /// Secret names use the same token rules as workload names.
    Name(WorkloadTokenError),
}

impl fmt::Display for SecretMetadataRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Namespace(error) => {
                write!(formatter, "invalid secret namespace: {error}")
            }
            Self::Name(error) => write!(formatter, "invalid secret name: {error}"),
        }
    }
}

impl std::error::Error for SecretMetadataRefError {}

/// Error returned when a [`RouteRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteRefError {
    /// Route namespaces use the same token rules as workload namespaces.
    Namespace(WorkloadTokenError),
    /// Route kinds must be non-empty Kubernetes-style kind identifiers.
    Kind(WorkloadKindError),
    /// Route names use the same token rules as workload names.
    Name(WorkloadTokenError),
}

impl fmt::Display for RouteRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Namespace(error) => {
                write!(formatter, "invalid route namespace: {error}")
            }
            Self::Kind(error) => write!(formatter, "invalid route kind: {error}"),
            Self::Name(error) => write!(formatter, "invalid route name: {error}"),
        }
    }
}

impl std::error::Error for RouteRefError {}

/// Error returned when a [`ContainerRef`] is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerRefError {
    /// Container names use the same token rules as workload names.
    Name(WorkloadTokenError),
}

impl fmt::Display for ContainerRefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(error) => write!(formatter, "invalid container name: {error}"),
        }
    }
}

impl std::error::Error for ContainerRefError {}

/// Error returned when a workload namespace or name is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadTokenError {
    /// Workload namespace and name values cannot be empty.
    Empty,
    /// Workload namespace and name values must fit common Kubernetes name limits.
    TooLong { max_len: usize },
    /// Workload namespace and name values must start and end with a lowercase ASCII letter or digit.
    InvalidBoundary,
    /// Workload namespace and name values only allow lowercase ASCII letters, digits, and hyphens.
    InvalidCharacter { character: char },
}

impl fmt::Display for WorkloadTokenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("value cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "value cannot exceed {max_len} characters")
            }
            Self::InvalidBoundary => formatter
                .write_str("value must start and end with a lowercase ASCII letter or digit"),
            Self::InvalidCharacter { character } => {
                write!(formatter, "value contains invalid character `{character}`")
            }
        }
    }
}

impl std::error::Error for WorkloadTokenError {}

/// Error returned when a workload kind is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadKindError {
    /// Workload kinds cannot be empty.
    Empty,
    /// Workload kinds must stay bounded for stable reports and labels.
    TooLong { max_len: usize },
    /// Workload kinds must start and end with an ASCII letter or digit.
    InvalidBoundary,
    /// Workload kinds only allow ASCII letters, digits, dots, and hyphens.
    InvalidCharacter { character: char },
}

impl fmt::Display for WorkloadKindError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("workload kind cannot be empty"),
            Self::TooLong { max_len } => {
                write!(
                    formatter,
                    "workload kind cannot exceed {max_len} characters"
                )
            }
            Self::InvalidBoundary => formatter
                .write_str("workload kind must start and end with an ASCII letter or digit"),
            Self::InvalidCharacter { character } => write!(
                formatter,
                "workload kind contains invalid character `{character}`"
            ),
        }
    }
}

impl std::error::Error for WorkloadKindError {}

impl From<SessionTokenError> for SessionIdError {
    fn from(error: SessionTokenError) -> Self {
        match error {
            SessionTokenError::Empty => Self::Empty,
            SessionTokenError::TooLong { max_len } => Self::TooLong { max_len },
            SessionTokenError::InvalidBoundary => Self::InvalidBoundary,
            SessionTokenError::InvalidCharacter { character } => {
                Self::InvalidCharacter { character }
            }
        }
    }
}

impl From<SessionTokenError> for SessionNameError {
    fn from(error: SessionTokenError) -> Self {
        match error {
            SessionTokenError::Empty => Self::Empty,
            SessionTokenError::TooLong { max_len } => Self::TooLong { max_len },
            SessionTokenError::InvalidBoundary => Self::InvalidBoundary,
            SessionTokenError::InvalidCharacter { character } => {
                Self::InvalidCharacter { character }
            }
        }
    }
}

impl From<SessionTokenError> for WorkloadTokenError {
    fn from(error: SessionTokenError) -> Self {
        match error {
            SessionTokenError::Empty => Self::Empty,
            SessionTokenError::TooLong { max_len } => Self::TooLong { max_len },
            SessionTokenError::InvalidBoundary => Self::InvalidBoundary,
            SessionTokenError::InvalidCharacter { character } => {
                Self::InvalidCharacter { character }
            }
        }
    }
}

#[derive(Deserialize)]
struct WorkloadRefFields {
    namespace: String,
    kind: String,
    name: String,
}

#[derive(Deserialize)]
struct KubernetesResourceRefFields {
    namespace: String,
    kind: String,
    name: String,
}

#[derive(Deserialize)]
struct PodRefFields {
    namespace: String,
    name: String,
}

#[derive(Deserialize)]
struct ServiceRefFields {
    namespace: String,
    name: String,
}

#[derive(Deserialize)]
struct ConfigMapRefFields {
    namespace: String,
    name: String,
}

#[derive(Deserialize)]
struct SecretMetadataRefFields {
    namespace: String,
    name: String,
}

#[derive(Deserialize)]
struct RouteRefFields {
    namespace: String,
    kind: String,
    name: String,
}

#[derive(Deserialize)]
struct ContainerRefFields {
    workload: WorkloadRef,
    name: String,
}

#[derive(Deserialize)]
struct AppGraphFields {
    workload: WorkloadRef,
    #[serde(default)]
    owned_pods: Vec<PodRef>,
    #[serde(default)]
    selecting_services: Vec<ServiceRef>,
    #[serde(default)]
    service_routes: Vec<ServiceRouteRef>,
    #[serde(default)]
    probe_facts: Vec<ProbeFacts>,
    #[serde(default)]
    image_facts: Vec<ImageFacts>,
    #[serde(default)]
    resource_facts: Vec<ResourceFacts>,
    #[serde(default)]
    config_references: Vec<ConfigReference>,
    #[serde(default)]
    secret_references: Vec<SecretReference>,
    #[serde(default)]
    relationship_confidences: Vec<RelationshipConfidence>,
    #[serde(default)]
    warnings: Vec<AppGraphWarning>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AppGraphWarningFields {
    AmbiguousServiceSelector {
        service: ServiceRef,
        candidate_workloads: Vec<WorkloadRef>,
    },
    MissingRoute {
        service: ServiceRef,
    },
    MissingProbes {
        container: ContainerRef,
        missing_probes: Vec<ProbeKind>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RouteSelectorFields {
    Header { name: String, value: String },
    Host { hostname: String },
}

#[derive(Deserialize)]
struct SessionPolicyFields {
    allowed_operations: Vec<SessionOperation>,
}

#[derive(Deserialize)]
struct SessionPlanFields {
    id: SessionId,
    name: SessionName,
    workload: WorkloadRef,
    image: ImageRef,
    #[serde(default, rename = "ttl")]
    time_to_live: Option<TimeToLive>,
    #[serde(default)]
    planned_resources: Vec<KubernetesResourceRef>,
    route_selector: Option<RouteSelector>,
    policy: SessionPolicy,
    status: SessionStatus,
}

#[derive(Deserialize)]
struct SessionReportFields {
    plan: SessionPlan,
    status: SessionStatus,
}

#[derive(Deserialize)]
struct SessionEventFields {
    session_id: SessionId,
    sequence: u64,
    kind: SessionEventKind,
    status: SessionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionTokenError {
    Empty,
    TooLong { max_len: usize },
    InvalidBoundary,
    InvalidCharacter { character: char },
}

fn deserialize_validated_string<'de, D, T, E, F>(
    deserializer: D,
    constructor: F,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    E: fmt::Display,
    F: FnOnce(String) -> Result<T, E>,
{
    let value = String::deserialize(deserializer)?;
    constructor(value).map_err(D::Error::custom)
}

/// Find the first duplicate operation in a sorted slice.
///
/// This only checks adjacent elements, so callers must sort operations first.
/// Returns [`None`] when all adjacent operations are unique.
fn duplicate_session_operation(operations: &[SessionOperation]) -> Option<SessionOperation> {
    operations
        .windows(2)
        .find(|window| window[0] == window[1])
        .map(|window| window[0])
}

const fn is_report_status(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Blocked
            | SessionStatus::Ready
            | SessionStatus::CleanedUp
            | SessionStatus::Failed
    )
}

fn validate_session_token(value: &str) -> Result<(), SessionTokenError> {
    if value.is_empty() {
        return Err(SessionTokenError::Empty);
    }

    if value.len() > SESSION_TOKEN_MAX_LEN {
        return Err(SessionTokenError::TooLong {
            max_len: SESSION_TOKEN_MAX_LEN,
        });
    }

    let mut characters = value.chars();
    let first_character = characters.next().ok_or(SessionTokenError::Empty)?;
    let last_character = characters.next_back().unwrap_or(first_character);

    if !is_session_token_boundary(first_character) || !is_session_token_boundary(last_character) {
        return Err(SessionTokenError::InvalidBoundary);
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_session_token_character(*character))
    {
        return Err(SessionTokenError::InvalidCharacter { character });
    }

    Ok(())
}

fn is_session_token_character(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
}

fn is_session_token_boundary(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit()
}

fn validate_route_header_name(value: &str) -> Result<(), RouteHeaderNameError> {
    if value.is_empty() {
        return Err(RouteHeaderNameError::Empty);
    }

    if value.len() > ROUTE_HEADER_NAME_MAX_LEN {
        return Err(RouteHeaderNameError::TooLong {
            max_len: ROUTE_HEADER_NAME_MAX_LEN,
        });
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_route_header_name_character(*character))
    {
        return Err(RouteHeaderNameError::InvalidCharacter { character });
    }

    Ok(())
}

fn is_route_header_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '!' | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '*'
                | '+'
                | '-'
                | '.'
                | '^'
                | '_'
                | '`'
                | '|'
                | '~'
        )
}

fn validate_route_header_value(value: &str) -> Result<(), RouteHeaderValueError> {
    if value.is_empty() {
        return Err(RouteHeaderValueError::Empty);
    }

    if value.len() > ROUTE_HEADER_VALUE_MAX_LEN {
        return Err(RouteHeaderValueError::TooLong {
            max_len: ROUTE_HEADER_VALUE_MAX_LEN,
        });
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_route_header_value_character(*character))
    {
        return Err(RouteHeaderValueError::InvalidCharacter { character });
    }

    Ok(())
}

// Route selectors use deterministic token-like header values, so spaces are
// rejected even though HTTP permits broader field values.
fn is_route_header_value_character(character: char) -> bool {
    character.is_ascii_graphic()
}

fn validate_route_host(value: &str) -> Result<(), RouteHostError> {
    if value.is_empty() {
        return Err(RouteHostError::Empty);
    }

    if value.len() > ROUTE_HOST_MAX_LEN {
        return Err(RouteHostError::TooLong {
            max_len: ROUTE_HOST_MAX_LEN,
        });
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_route_host_character(*character))
    {
        return Err(RouteHostError::InvalidCharacter { character });
    }

    let mut characters = value.chars();
    let first_character = characters.next().ok_or(RouteHostError::Empty)?;
    let last_character = characters.next_back().unwrap_or(first_character);

    if !is_route_host_boundary(first_character) || !is_route_host_boundary(last_character) {
        return Err(RouteHostError::InvalidBoundary);
    }

    for label in value.split('.') {
        if label.is_empty() {
            return Err(RouteHostError::EmptyLabel);
        }

        if label.len() > ROUTE_HOST_LABEL_MAX_LEN {
            return Err(RouteHostError::LabelTooLong {
                max_len: ROUTE_HOST_LABEL_MAX_LEN,
            });
        }

        let (label_first_character, label_last_character) = route_host_label_boundaries(label)?;

        if !is_route_host_boundary(label_first_character)
            || !is_route_host_boundary(label_last_character)
        {
            return Err(RouteHostError::InvalidBoundary);
        }
    }

    Ok(())
}

fn route_host_label_boundaries(label: &str) -> Result<(char, char), RouteHostError> {
    let mut label_characters = label.chars();
    let label_first_character = label_characters.next().ok_or(RouteHostError::EmptyLabel)?;
    let label_last_character = label_characters
        .next_back()
        .unwrap_or(label_first_character);

    Ok((label_first_character, label_last_character))
}

fn is_route_host_character(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || character == '-'
        || character == '.'
}

fn is_route_host_boundary(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit()
}

fn validate_image_ref(value: &str) -> Result<(), ImageRefError> {
    if value.is_empty() {
        return Err(ImageRefError::Empty);
    }

    if value.len() > IMAGE_REF_MAX_LEN {
        return Err(ImageRefError::TooLong {
            max_len: IMAGE_REF_MAX_LEN,
        });
    }

    let mut characters = value.chars();
    let first_character = characters.next().ok_or(ImageRefError::Empty)?;
    let last_character = characters.next_back().unwrap_or(first_character);

    if !is_image_ref_boundary(first_character) || !is_image_ref_boundary(last_character) {
        return Err(ImageRefError::InvalidBoundary);
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_image_ref_character(*character))
    {
        return Err(ImageRefError::InvalidCharacter { character });
    }

    if value
        .split(['/', ':', '@'])
        .any(|component| component.is_empty())
    {
        return Err(ImageRefError::MissingName);
    }

    validate_image_repository_components(value)?;

    Ok(())
}

fn validate_time_to_live(value: &str) -> Result<(), TimeToLiveError> {
    if value.is_empty() {
        return Err(TimeToLiveError::Empty);
    }

    if value.len() > TIME_TO_LIVE_MAX_LEN {
        return Err(TimeToLiveError::TooLong {
            max_len: TIME_TO_LIVE_MAX_LEN,
        });
    }

    let unit = value.chars().last().ok_or(TimeToLiveError::Empty)?;
    if !matches!(unit, 's' | 'm' | 'h' | 'd') {
        return Err(TimeToLiveError::InvalidUnit { unit });
    }

    let digits = &value[..value.len() - unit.len_utf8()];
    if digits.is_empty() || !digits.chars().all(|character| character.is_ascii_digit()) {
        return Err(TimeToLiveError::InvalidNumber);
    }

    if digits.trim_start_matches('0').is_empty() {
        return Err(TimeToLiveError::Zero);
    }

    Ok(())
}

fn validate_resource_quantity(value: &str) -> Result<(), ResourceQuantityError> {
    if value.is_empty() {
        return Err(ResourceQuantityError::Empty);
    }

    if value.len() > RESOURCE_QUANTITY_MAX_LEN {
        return Err(ResourceQuantityError::TooLong {
            max_len: RESOURCE_QUANTITY_MAX_LEN,
        });
    }

    let mut characters = value.chars();
    let first_character = characters.next().ok_or(ResourceQuantityError::Empty)?;
    let last_character = characters.next_back().unwrap_or(first_character);

    if !first_character.is_ascii_alphanumeric() || !last_character.is_ascii_alphanumeric() {
        return Err(ResourceQuantityError::InvalidBoundary);
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_resource_quantity_character(*character))
    {
        return Err(ResourceQuantityError::InvalidCharacter { character });
    }

    Ok(())
}

fn is_resource_quantity_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '+' | '-' | '_')
}

fn is_image_ref_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '/' | ':' | '@')
}

fn is_image_ref_repository_character(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || matches!(character, '.' | '_' | '-')
}

fn is_image_registry_character(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || matches!(character, '.' | '-' | ':')
}

fn is_image_ref_boundary(character: char) -> bool {
    character.is_ascii_alphanumeric()
}

fn validate_image_repository_components(value: &str) -> Result<(), ImageRefError> {
    let image_without_digest = value.split('@').next().unwrap_or(value);
    let components = image_without_digest.split('/').collect::<Vec<_>>();
    let last_component_index = components.len().saturating_sub(1);

    for (index, component) in components.iter().enumerate() {
        let component = if index == last_component_index {
            component.split(':').next().unwrap_or(component)
        } else {
            component
        };

        let valid_character = if index == 0 && is_registry_component(component) {
            is_image_registry_character
        } else {
            is_image_ref_repository_character
        };

        if let Some(character) = component
            .chars()
            .find(|character| !valid_character(*character))
        {
            return Err(ImageRefError::InvalidCharacter { character });
        }
    }

    Ok(())
}

fn is_registry_component(component: &str) -> bool {
    component == "localhost" || component.contains('.') || component.contains(':')
}

fn validate_workload_kind(value: &str) -> Result<(), WorkloadKindError> {
    if value.is_empty() {
        return Err(WorkloadKindError::Empty);
    }

    if value.len() > WORKLOAD_KIND_MAX_LEN {
        return Err(WorkloadKindError::TooLong {
            max_len: WORKLOAD_KIND_MAX_LEN,
        });
    }

    let mut characters = value.chars();
    let first_character = characters.next().ok_or(WorkloadKindError::Empty)?;
    let last_character = characters.next_back().unwrap_or(first_character);

    if !first_character.is_ascii_alphanumeric() || !last_character.is_ascii_alphanumeric() {
        return Err(WorkloadKindError::InvalidBoundary);
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_workload_kind_character(*character))
    {
        return Err(WorkloadKindError::InvalidCharacter { character });
    }

    Ok(())
}

fn is_workload_kind_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '.' || character == '-'
}

#[cfg(test)]
mod tests {
    use super::{
        AppGraph, AppGraphWarning, ConfidenceLevel, ConfigMapRef, ConfigMapRefError,
        ConfigReference, ContainerRef, ContainerRefError, GraphRelationship, IMAGE_REF_MAX_LEN,
        ImageFacts, ImageRef, ImageRefError, KubernetesResourceRef, KubernetesResourceRefError,
        PodRef, PodRefError, ProbeFacts, ProbeKind, RESOURCE_QUANTITY_MAX_LEN,
        ROUTE_HEADER_NAME_MAX_LEN, ROUTE_HEADER_VALUE_MAX_LEN, ROUTE_HOST_LABEL_MAX_LEN,
        ROUTE_HOST_MAX_LEN, RelationshipConfidence, ResourceFacts, ResourceQuantity,
        ResourceQuantityError, RouteHeaderNameError, RouteHeaderValueError, RouteHostError,
        RouteRef, RouteRefError, RouteSelector, RouteSelectorError, SESSION_TOKEN_MAX_LEN,
        SecretMetadataRef, SecretMetadataRefError, SecretReference, ServiceRef, ServiceRefError,
        ServiceRouteRef, SessionEvent, SessionEventKind, SessionId, SessionIdError, SessionName,
        SessionNameError, SessionOperation, SessionPlan, SessionPolicy, SessionPolicyError,
        SessionReport, SessionReportError, SessionStatus, SessionTransitionError,
        TIME_TO_LIVE_MAX_LEN, TimeToLive, TimeToLiveError, WORKLOAD_KIND_MAX_LEN,
        WorkloadKindError, WorkloadRef, WorkloadRefError, WorkloadTokenError,
    };
    use serde_json::json;

    fn test_session_plan() -> SessionPlan {
        SessionPlan::new(
            SessionId::new("session-123").expect("session id"),
            SessionName::new("checkout-test").expect("session name"),
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
            ImageRef::new("registry.example.com/checkout/api:v2").expect("image ref"),
            SessionPolicy::sandbox(),
        )
    }

    fn test_app_graph() -> AppGraph {
        AppGraph::new(
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
        )
        .with_owned_pods([
            PodRef::new("checkout", "checkout-api-7d9f4d9d-b").expect("pod ref"),
            PodRef::new("checkout", "checkout-api-7d9f4d9d-a").expect("pod ref"),
            PodRef::new("checkout", "checkout-api-7d9f4d9d-a").expect("pod ref"),
        ])
        .with_selecting_services([
            ServiceRef::new("checkout", "checkout-api-private").expect("service ref"),
            ServiceRef::new("checkout", "checkout-api").expect("service ref"),
            ServiceRef::new("checkout", "checkout-api").expect("service ref"),
        ])
        .with_service_routes([
            ServiceRouteRef::new(
                ServiceRef::new("checkout", "checkout-api-private").expect("service ref"),
                RouteRef::new("checkout", "HTTPRoute", "checkout-api-private").expect("route ref"),
            ),
            ServiceRouteRef::new(
                ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                RouteRef::new("checkout", "Ingress", "checkout").expect("route ref"),
            ),
            ServiceRouteRef::new(
                ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                RouteRef::new("checkout", "Ingress", "checkout").expect("route ref"),
            ),
        ])
        .with_config_references([
            ConfigReference::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "worker",
                )
                .expect("container ref"),
                ConfigMapRef::new("checkout", "checkout-worker-config").expect("configmap ref"),
            ),
            ConfigReference::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                ConfigMapRef::new("checkout", "checkout-api-config").expect("configmap ref"),
            ),
            ConfigReference::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                ConfigMapRef::new("checkout", "checkout-api-config").expect("configmap ref"),
            ),
        ])
        .with_secret_references([
            SecretReference::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "worker",
                )
                .expect("container ref"),
                SecretMetadataRef::new("checkout", "checkout-worker-credentials")
                    .expect("secret ref"),
            ),
            SecretReference::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                SecretMetadataRef::new("checkout", "checkout-api-credentials").expect("secret ref"),
            ),
            SecretReference::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                SecretMetadataRef::new("checkout", "checkout-api-credentials").expect("secret ref"),
            ),
        ])
        .with_probe_facts([
            ProbeFacts::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "worker",
                )
                .expect("container ref"),
                false,
                true,
                false,
            ),
            ProbeFacts::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                true,
                true,
                false,
            ),
            ProbeFacts::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                true,
                true,
                false,
            ),
        ])
        .with_image_facts([
            ImageFacts::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "worker",
                )
                .expect("container ref"),
                ImageRef::new("registry.example.com/checkout/worker:v1").expect("image ref"),
            ),
            ImageFacts::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                ImageRef::new("registry.example.com/checkout/api:v2").expect("image ref"),
            ),
            ImageFacts::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                ImageRef::new("registry.example.com/checkout/api:v2").expect("image ref"),
            ),
        ])
        .with_resource_facts([
            ResourceFacts::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "worker",
                )
                .expect("container ref"),
                None,
                None,
                Some(ResourceQuantity::new("128Mi").expect("resource quantity")),
                Some(ResourceQuantity::new("256Mi").expect("resource quantity")),
            ),
            ResourceFacts::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                Some(ResourceQuantity::new("250m").expect("resource quantity")),
                Some(ResourceQuantity::new("500m").expect("resource quantity")),
                Some(ResourceQuantity::new("512Mi").expect("resource quantity")),
                Some(ResourceQuantity::new("1Gi").expect("resource quantity")),
            ),
            ResourceFacts::new(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                Some(ResourceQuantity::new("250m").expect("resource quantity")),
                Some(ResourceQuantity::new("500m").expect("resource quantity")),
                Some(ResourceQuantity::new("512Mi").expect("resource quantity")),
                Some(ResourceQuantity::new("1Gi").expect("resource quantity")),
            ),
        ])
        .with_relationship_confidences([
            RelationshipConfidence::new(
                GraphRelationship::ContainerSecretReference {
                    container: ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    secret: SecretMetadataRef::new("checkout", "checkout-api-credentials")
                        .expect("secret ref"),
                },
                ConfidenceLevel::Low,
            ),
            RelationshipConfidence::new(
                GraphRelationship::WorkloadServiceSelection {
                    service: ServiceRef::new("checkout", "checkout-api-private")
                        .expect("service ref"),
                },
                ConfidenceLevel::Medium,
            ),
            RelationshipConfidence::new(
                GraphRelationship::ContainerConfigReference {
                    container: ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    config_map: ConfigMapRef::new("checkout", "checkout-api-config")
                        .expect("configmap ref"),
                },
                ConfidenceLevel::High,
            ),
            RelationshipConfidence::new(
                GraphRelationship::WorkloadPodOwnership {
                    pod: PodRef::new("checkout", "checkout-api-7d9f4d9d-a").expect("pod ref"),
                },
                ConfidenceLevel::High,
            ),
            RelationshipConfidence::new(
                GraphRelationship::ServiceRouteReference {
                    service: ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                    route: RouteRef::new("checkout", "Ingress", "checkout").expect("route ref"),
                },
                ConfidenceLevel::High,
            ),
            RelationshipConfidence::new(
                GraphRelationship::ServiceRouteReference {
                    service: ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                    route: RouteRef::new("checkout", "Ingress", "checkout").expect("route ref"),
                },
                ConfidenceLevel::High,
            ),
        ])
        .with_warnings([
            AppGraphWarning::ambiguous_service_selector(
                ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                [
                    WorkloadRef::new("checkout", "Deployment", "checkout-worker")
                        .expect("workload ref"),
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                ],
            ),
            AppGraphWarning::ambiguous_service_selector(
                ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                [
                    WorkloadRef::new("checkout", "Deployment", "checkout-worker")
                        .expect("workload ref"),
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                ],
            ),
            AppGraphWarning::missing_route(
                ServiceRef::new("checkout", "checkout-api-private").expect("service ref"),
            ),
            AppGraphWarning::missing_route(
                ServiceRef::new("checkout", "checkout-api-private").expect("service ref"),
            ),
            AppGraphWarning::missing_probes(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "worker",
                )
                .expect("container ref"),
                [
                    ProbeKind::Startup,
                    ProbeKind::Readiness,
                    ProbeKind::Readiness,
                ],
            ),
            AppGraphWarning::missing_probes(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "worker",
                )
                .expect("container ref"),
                [ProbeKind::Readiness, ProbeKind::Startup],
            ),
        ])
    }

    fn minimal_app_graph() -> AppGraph {
        AppGraph::new(
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
        )
    }

    fn routed_app_graph() -> AppGraph {
        AppGraph::new(
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
        )
        .with_selecting_services(
            [ServiceRef::new("checkout", "checkout-api").expect("service ref")],
        )
        .with_service_routes([ServiceRouteRef::new(
            ServiceRef::new("checkout", "checkout-api").expect("service ref"),
            RouteRef::new("checkout", "HTTPRoute", "checkout-api").expect("route ref"),
        )])
        .with_relationship_confidences([
            RelationshipConfidence::new(
                GraphRelationship::WorkloadServiceSelection {
                    service: ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                },
                ConfidenceLevel::High,
            ),
            RelationshipConfidence::new(
                GraphRelationship::ServiceRouteReference {
                    service: ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                    route: RouteRef::new("checkout", "HTTPRoute", "checkout-api")
                        .expect("route ref"),
                },
                ConfidenceLevel::High,
            ),
        ])
    }

    fn warning_app_graph() -> AppGraph {
        AppGraph::new(
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
        )
        .with_selecting_services([
            ServiceRef::new("checkout", "checkout-api").expect("service ref"),
            ServiceRef::new("checkout", "checkout-private").expect("service ref"),
        ])
        .with_probe_facts([ProbeFacts::new(
            ContainerRef::new(
                WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
                "api",
            )
            .expect("container ref"),
            true,
            false,
            false,
        )])
        .with_warnings([
            AppGraphWarning::ambiguous_service_selector(
                ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                [
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    WorkloadRef::new("checkout", "Deployment", "checkout-worker")
                        .expect("workload ref"),
                ],
            ),
            AppGraphWarning::missing_route(
                ServiceRef::new("checkout", "checkout-private").expect("service ref"),
            ),
            AppGraphWarning::missing_probes(
                ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "api",
                )
                .expect("container ref"),
                [ProbeKind::Liveness, ProbeKind::Startup],
            ),
        ])
    }

    #[test]
    fn creates_app_graph_from_workload_ref() {
        let workload =
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref");

        let graph = AppGraph::new(workload.clone());

        assert_eq!(graph.workload(), &workload);
        assert!(graph.owned_pods().is_empty());
        assert!(graph.selecting_services().is_empty());
        assert!(graph.service_routes().is_empty());
        assert!(graph.probe_facts().is_empty());
        assert!(graph.image_facts().is_empty());
        assert!(graph.resource_facts().is_empty());
        assert!(graph.config_references().is_empty());
        assert!(graph.secret_references().is_empty());
        assert!(graph.relationship_confidences().is_empty());
        assert!(graph.warnings().is_empty());
    }

    #[test]
    fn creates_pod_ref_from_valid_parts() {
        let pod =
            PodRef::new("checkout", "checkout-api-7d9f4d9d-a").expect("pod ref should be valid");

        assert_eq!(pod.namespace(), "checkout");
        assert_eq!(pod.name(), "checkout-api-7d9f4d9d-a");
        assert_eq!(pod.to_string(), "checkout/checkout-api-7d9f4d9d-a");
    }

    #[test]
    fn rejects_invalid_pod_ref_parts() {
        let namespace_error =
            PodRef::new("Checkout", "checkout-api").expect_err("namespace should be invalid");
        let name_error =
            PodRef::new("checkout", "checkout_api").expect_err("name should be invalid");

        assert_eq!(
            namespace_error,
            PodRefError::Namespace(WorkloadTokenError::InvalidBoundary)
        );
        assert_eq!(
            name_error,
            PodRefError::Name(WorkloadTokenError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn creates_service_ref_from_valid_parts() {
        let service =
            ServiceRef::new("checkout", "checkout-api").expect("service ref should be valid");

        assert_eq!(service.namespace(), "checkout");
        assert_eq!(service.name(), "checkout-api");
        assert_eq!(service.to_string(), "checkout/checkout-api");
    }

    #[test]
    fn rejects_invalid_service_ref_parts() {
        let namespace_error =
            ServiceRef::new("Checkout", "checkout-api").expect_err("namespace should be invalid");
        let name_error =
            ServiceRef::new("checkout", "checkout_api").expect_err("name should be invalid");

        assert_eq!(
            namespace_error,
            ServiceRefError::Namespace(WorkloadTokenError::InvalidBoundary)
        );
        assert_eq!(
            name_error,
            ServiceRefError::Name(WorkloadTokenError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn creates_config_map_ref_from_valid_parts() {
        let config_map = ConfigMapRef::new("checkout", "checkout-api-config")
            .expect("configmap ref should be valid");

        assert_eq!(config_map.namespace(), "checkout");
        assert_eq!(config_map.name(), "checkout-api-config");
        assert_eq!(config_map.to_string(), "checkout/checkout-api-config");
    }

    #[test]
    fn rejects_invalid_config_map_ref_parts() {
        let namespace_error = ConfigMapRef::new("Checkout", "checkout-api-config")
            .expect_err("namespace should be invalid");
        let name_error =
            ConfigMapRef::new("checkout", "checkout_api").expect_err("name should be invalid");

        assert_eq!(
            namespace_error,
            ConfigMapRefError::Namespace(WorkloadTokenError::InvalidBoundary)
        );
        assert_eq!(
            name_error,
            ConfigMapRefError::Name(WorkloadTokenError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn creates_secret_metadata_ref_from_valid_parts() {
        let secret = SecretMetadataRef::new("checkout", "checkout-api-credentials")
            .expect("secret ref should be valid");

        assert_eq!(secret.namespace(), "checkout");
        assert_eq!(secret.name(), "checkout-api-credentials");
        assert_eq!(secret.to_string(), "checkout/checkout-api-credentials");
    }

    #[test]
    fn rejects_invalid_secret_metadata_ref_parts() {
        let namespace_error = SecretMetadataRef::new("Checkout", "checkout-api-credentials")
            .expect_err("namespace should be invalid");
        let name_error =
            SecretMetadataRef::new("checkout", "checkout_api").expect_err("name should be invalid");

        assert_eq!(
            namespace_error,
            SecretMetadataRefError::Namespace(WorkloadTokenError::InvalidBoundary)
        );
        assert_eq!(
            name_error,
            SecretMetadataRefError::Name(WorkloadTokenError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn creates_route_ref_from_valid_parts() {
        let route = RouteRef::new("checkout", "HTTPRoute", "checkout-api")
            .expect("route ref should be valid");

        assert_eq!(route.namespace(), "checkout");
        assert_eq!(route.kind(), "HTTPRoute");
        assert_eq!(route.name(), "checkout-api");
        assert_eq!(route.to_string(), "checkout/HTTPRoute/checkout-api");
    }

    #[test]
    fn rejects_invalid_route_ref_parts() {
        let namespace_error = RouteRef::new("Checkout", "HTTPRoute", "checkout-api")
            .expect_err("namespace should be invalid");
        let kind_error = RouteRef::new("checkout", "_HTTPRoute", "checkout-api")
            .expect_err("kind should be invalid");
        let name_error = RouteRef::new("checkout", "HTTPRoute", "checkout_api")
            .expect_err("name should be invalid");

        assert_eq!(
            namespace_error,
            RouteRefError::Namespace(WorkloadTokenError::InvalidBoundary)
        );
        assert_eq!(
            kind_error,
            RouteRefError::Kind(WorkloadKindError::InvalidBoundary)
        );
        assert_eq!(
            name_error,
            RouteRefError::Name(WorkloadTokenError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn creates_service_route_ref_from_valid_refs() {
        let service = ServiceRef::new("checkout", "checkout-api").expect("service ref");
        let route = RouteRef::new("checkout", "Ingress", "checkout").expect("route ref");

        let edge = ServiceRouteRef::new(service.clone(), route.clone());

        assert_eq!(edge.service(), &service);
        assert_eq!(edge.route(), &route);
    }

    #[test]
    fn creates_container_ref_from_valid_parts() {
        let workload =
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref");

        let container =
            ContainerRef::new(workload.clone(), "api").expect("container ref should be valid");

        assert_eq!(container.workload(), &workload);
        assert_eq!(container.name(), "api");
        assert_eq!(
            container.to_string(),
            "checkout/Deployment/checkout-api/api"
        );
    }

    #[test]
    fn rejects_invalid_container_ref_name() {
        let workload =
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref");

        let error =
            ContainerRef::new(workload, "api_container").expect_err("name should be invalid");

        assert_eq!(
            error,
            ContainerRefError::Name(WorkloadTokenError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn rejects_invalid_container_ref_json() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "name": "api_container"
        });

        let error = serde_json::from_value::<ContainerRef>(value)
            .expect_err("invalid container name should be rejected");

        assert!(
            error.to_string().contains("invalid container name"),
            "unexpected container ref error: {error}"
        );
    }

    #[test]
    fn creates_probe_facts_from_valid_container() {
        let container = ContainerRef::new(
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
            "api",
        )
        .expect("container ref");

        let facts = ProbeFacts::new(container.clone(), true, false, true);

        assert_eq!(facts.container(), &container);
        assert!(facts.readiness_probe());
        assert!(!facts.liveness_probe());
        assert!(facts.startup_probe());
    }

    #[test]
    fn renders_probe_kind_names() {
        assert_eq!(ProbeKind::Readiness.as_str(), "readiness");
        assert_eq!(ProbeKind::Liveness.as_str(), "liveness");
        assert_eq!(ProbeKind::Startup.as_str(), "startup");
        assert_eq!(ProbeKind::Startup.to_string(), "startup");
    }

    #[test]
    fn creates_image_facts_from_valid_container_and_image() {
        let container = ContainerRef::new(
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
            "api",
        )
        .expect("container ref");
        let image = ImageRef::new("registry.example.com/checkout/api:v2").expect("image ref");

        let facts = ImageFacts::new(container.clone(), image.clone());

        assert_eq!(facts.container(), &container);
        assert_eq!(facts.image(), &image);
    }

    #[test]
    fn creates_resource_quantity_from_valid_value() {
        let quantity = ResourceQuantity::new("250m").expect("resource quantity should be valid");

        assert_eq!(quantity.as_str(), "250m");
        assert_eq!(quantity.to_string(), "250m");
    }

    #[test]
    fn rejects_invalid_resource_quantity_values() {
        let empty_error = ResourceQuantity::new("").expect_err("empty quantity should be invalid");
        let boundary_error =
            ResourceQuantity::new("-250m").expect_err("boundary should be invalid");
        let character_error = ResourceQuantity::new("250 m").expect_err("space should be invalid");
        let long_error = ResourceQuantity::new("1".repeat(RESOURCE_QUANTITY_MAX_LEN + 1))
            .expect_err("long quantity should be invalid");

        assert_eq!(empty_error, ResourceQuantityError::Empty);
        assert_eq!(boundary_error, ResourceQuantityError::InvalidBoundary);
        assert_eq!(
            character_error,
            ResourceQuantityError::InvalidCharacter { character: ' ' }
        );
        assert_eq!(
            long_error,
            ResourceQuantityError::TooLong {
                max_len: RESOURCE_QUANTITY_MAX_LEN
            }
        );
    }

    #[test]
    fn creates_resource_facts_from_valid_container_and_quantities() {
        let container = ContainerRef::new(
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
            "api",
        )
        .expect("container ref");
        let cpu_request = ResourceQuantity::new("250m").expect("resource quantity");
        let cpu_limit = ResourceQuantity::new("500m").expect("resource quantity");
        let memory_request = ResourceQuantity::new("512Mi").expect("resource quantity");
        let memory_limit = ResourceQuantity::new("1Gi").expect("resource quantity");

        let facts = ResourceFacts::new(
            container.clone(),
            Some(cpu_request.clone()),
            Some(cpu_limit.clone()),
            Some(memory_request.clone()),
            Some(memory_limit.clone()),
        );

        assert_eq!(facts.container(), &container);
        assert_eq!(facts.cpu_request(), Some(&cpu_request));
        assert_eq!(facts.cpu_limit(), Some(&cpu_limit));
        assert_eq!(facts.memory_request(), Some(&memory_request));
        assert_eq!(facts.memory_limit(), Some(&memory_limit));
    }

    #[test]
    fn records_owned_pods_in_stable_order() {
        let graph = test_app_graph();

        assert_eq!(
            graph.owned_pods(),
            &[
                PodRef::new("checkout", "checkout-api-7d9f4d9d-a").expect("pod ref"),
                PodRef::new("checkout", "checkout-api-7d9f4d9d-b").expect("pod ref"),
            ]
        );
    }

    #[test]
    fn records_selecting_services_in_stable_order() {
        let graph = test_app_graph();

        assert_eq!(
            graph.selecting_services(),
            &[
                ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                ServiceRef::new("checkout", "checkout-api-private").expect("service ref"),
            ]
        );
    }

    #[test]
    fn records_service_routes_in_stable_order() {
        let graph = test_app_graph();

        assert_eq!(
            graph.service_routes(),
            &[
                ServiceRouteRef::new(
                    ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                    RouteRef::new("checkout", "Ingress", "checkout").expect("route ref"),
                ),
                ServiceRouteRef::new(
                    ServiceRef::new("checkout", "checkout-api-private").expect("service ref"),
                    RouteRef::new("checkout", "HTTPRoute", "checkout-api-private")
                        .expect("route ref"),
                ),
            ]
        );
    }

    #[test]
    fn records_probe_facts_in_stable_order() {
        let graph = test_app_graph();

        assert_eq!(
            graph.probe_facts(),
            &[
                ProbeFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    true,
                    true,
                    false,
                ),
                ProbeFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    false,
                    true,
                    false,
                ),
            ]
        );
    }

    #[test]
    fn records_image_facts_in_stable_order() {
        let graph = test_app_graph();

        assert_eq!(
            graph.image_facts(),
            &[
                ImageFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    ImageRef::new("registry.example.com/checkout/api:v2").expect("image ref"),
                ),
                ImageFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    ImageRef::new("registry.example.com/checkout/worker:v1").expect("image ref"),
                ),
            ]
        );
    }

    #[test]
    fn records_resource_facts_in_stable_order() {
        let graph = test_app_graph();

        assert_eq!(
            graph.resource_facts(),
            &[
                ResourceFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    Some(ResourceQuantity::new("250m").expect("resource quantity")),
                    Some(ResourceQuantity::new("500m").expect("resource quantity")),
                    Some(ResourceQuantity::new("512Mi").expect("resource quantity")),
                    Some(ResourceQuantity::new("1Gi").expect("resource quantity")),
                ),
                ResourceFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    None,
                    None,
                    Some(ResourceQuantity::new("128Mi").expect("resource quantity")),
                    Some(ResourceQuantity::new("256Mi").expect("resource quantity")),
                ),
            ]
        );
    }

    #[test]
    fn records_config_references_in_stable_order() {
        let graph = test_app_graph();

        assert_eq!(
            graph.config_references(),
            &[
                ConfigReference::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    ConfigMapRef::new("checkout", "checkout-api-config").expect("configmap ref"),
                ),
                ConfigReference::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    ConfigMapRef::new("checkout", "checkout-worker-config").expect("configmap ref"),
                ),
            ]
        );
    }

    #[test]
    fn records_secret_references_in_stable_order() {
        let graph = test_app_graph();

        assert_eq!(
            graph.secret_references(),
            &[
                SecretReference::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    SecretMetadataRef::new("checkout", "checkout-api-credentials")
                        .expect("secret ref"),
                ),
                SecretReference::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    SecretMetadataRef::new("checkout", "checkout-worker-credentials")
                        .expect("secret ref"),
                ),
            ]
        );
    }

    #[test]
    fn creates_relationship_confidence_from_valid_parts() {
        let relationship = GraphRelationship::WorkloadServiceSelection {
            service: ServiceRef::new("checkout", "checkout-api").expect("service ref"),
        };

        let confidence = RelationshipConfidence::new(relationship.clone(), ConfidenceLevel::High);

        assert_eq!(confidence.relationship(), &relationship);
        assert_eq!(confidence.confidence(), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::High.as_str(), "high");
        assert_eq!(ConfidenceLevel::High.to_string(), "high");
    }

    #[test]
    fn records_relationship_confidences_in_stable_order() {
        let graph = test_app_graph();

        assert_eq!(
            graph.relationship_confidences(),
            &[
                RelationshipConfidence::new(
                    GraphRelationship::WorkloadPodOwnership {
                        pod: PodRef::new("checkout", "checkout-api-7d9f4d9d-a").expect("pod ref"),
                    },
                    ConfidenceLevel::High,
                ),
                RelationshipConfidence::new(
                    GraphRelationship::WorkloadServiceSelection {
                        service: ServiceRef::new("checkout", "checkout-api-private")
                            .expect("service ref"),
                    },
                    ConfidenceLevel::Medium,
                ),
                RelationshipConfidence::new(
                    GraphRelationship::ServiceRouteReference {
                        service: ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                        route: RouteRef::new("checkout", "Ingress", "checkout").expect("route ref"),
                    },
                    ConfidenceLevel::High,
                ),
                RelationshipConfidence::new(
                    GraphRelationship::ContainerConfigReference {
                        container: ContainerRef::new(
                            WorkloadRef::new("checkout", "Deployment", "checkout-api")
                                .expect("workload ref"),
                            "api",
                        )
                        .expect("container ref"),
                        config_map: ConfigMapRef::new("checkout", "checkout-api-config")
                            .expect("configmap ref"),
                    },
                    ConfidenceLevel::High,
                ),
                RelationshipConfidence::new(
                    GraphRelationship::ContainerSecretReference {
                        container: ContainerRef::new(
                            WorkloadRef::new("checkout", "Deployment", "checkout-api")
                                .expect("workload ref"),
                            "api",
                        )
                        .expect("container ref"),
                        secret: SecretMetadataRef::new("checkout", "checkout-api-credentials")
                            .expect("secret ref"),
                    },
                    ConfidenceLevel::Low,
                ),
            ]
        );
    }

    #[test]
    fn creates_ambiguous_service_selector_warning_in_stable_order() {
        let warning = AppGraphWarning::ambiguous_service_selector(
            ServiceRef::new("checkout", "checkout-api").expect("service ref"),
            [
                WorkloadRef::new("checkout", "Deployment", "checkout-worker")
                    .expect("workload ref"),
                WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
                WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
            ],
        );

        assert_eq!(
            warning,
            AppGraphWarning::AmbiguousServiceSelector {
                service: ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                candidate_workloads: vec![
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    WorkloadRef::new("checkout", "Deployment", "checkout-worker")
                        .expect("workload ref"),
                ],
            }
        );
    }

    #[test]
    fn creates_missing_route_warning() {
        let warning = AppGraphWarning::missing_route(
            ServiceRef::new("checkout", "checkout-api-private").expect("service ref"),
        );

        assert_eq!(
            warning,
            AppGraphWarning::MissingRoute {
                service: ServiceRef::new("checkout", "checkout-api-private").expect("service ref"),
            }
        );
    }

    #[test]
    fn creates_missing_probes_warning_in_stable_order() {
        let warning = AppGraphWarning::missing_probes(
            ContainerRef::new(
                WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref"),
                "worker",
            )
            .expect("container ref"),
            [ProbeKind::Startup, ProbeKind::Readiness, ProbeKind::Startup],
        );

        assert_eq!(
            warning,
            AppGraphWarning::MissingProbes {
                container: ContainerRef::new(
                    WorkloadRef::new("checkout", "Deployment", "checkout-api")
                        .expect("workload ref"),
                    "worker",
                )
                .expect("container ref"),
                missing_probes: vec![ProbeKind::Readiness, ProbeKind::Startup],
            }
        );
    }

    #[test]
    fn records_warnings_in_stable_order() {
        let graph = test_app_graph();

        assert_eq!(
            graph.warnings(),
            &[
                AppGraphWarning::AmbiguousServiceSelector {
                    service: ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                    candidate_workloads: vec![
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        WorkloadRef::new("checkout", "Deployment", "checkout-worker")
                            .expect("workload ref"),
                    ],
                },
                AppGraphWarning::MissingRoute {
                    service: ServiceRef::new("checkout", "checkout-api-private")
                        .expect("service ref"),
                },
                AppGraphWarning::MissingProbes {
                    container: ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    missing_probes: vec![ProbeKind::Readiness, ProbeKind::Startup],
                },
            ]
        );
    }

    #[test]
    fn deserializes_owned_pods_in_stable_order() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "owned_pods": [
                {
                    "namespace": "checkout",
                    "name": "checkout-api-7d9f4d9d-b"
                },
                {
                    "namespace": "checkout",
                    "name": "checkout-api-7d9f4d9d-a"
                },
                {
                    "namespace": "checkout",
                    "name": "checkout-api-7d9f4d9d-a"
                }
            ]
        });

        let graph: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(
            graph.owned_pods(),
            &[
                PodRef::new("checkout", "checkout-api-7d9f4d9d-a").expect("pod ref"),
                PodRef::new("checkout", "checkout-api-7d9f4d9d-b").expect("pod ref"),
            ]
        );
    }

    #[test]
    fn deserializes_selecting_services_in_stable_order() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "selecting_services": [
                {
                    "namespace": "checkout",
                    "name": "checkout-api-private"
                },
                {
                    "namespace": "checkout",
                    "name": "checkout-api"
                },
                {
                    "namespace": "checkout",
                    "name": "checkout-api"
                }
            ]
        });

        let graph: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(
            graph.selecting_services(),
            &[
                ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                ServiceRef::new("checkout", "checkout-api-private").expect("service ref"),
            ]
        );
    }

    #[test]
    fn deserializes_service_routes_in_stable_order() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "service_routes": [
                {
                    "service": {
                        "namespace": "checkout",
                        "name": "checkout-api-private"
                    },
                    "route": {
                        "namespace": "checkout",
                        "kind": "HTTPRoute",
                        "name": "checkout-api-private"
                    }
                },
                {
                    "service": {
                        "namespace": "checkout",
                        "name": "checkout-api"
                    },
                    "route": {
                        "namespace": "checkout",
                        "kind": "Ingress",
                        "name": "checkout"
                    }
                },
                {
                    "service": {
                        "namespace": "checkout",
                        "name": "checkout-api"
                    },
                    "route": {
                        "namespace": "checkout",
                        "kind": "Ingress",
                        "name": "checkout"
                    }
                }
            ]
        });

        let graph: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(
            graph.service_routes(),
            &[
                ServiceRouteRef::new(
                    ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                    RouteRef::new("checkout", "Ingress", "checkout").expect("route ref"),
                ),
                ServiceRouteRef::new(
                    ServiceRef::new("checkout", "checkout-api-private").expect("service ref"),
                    RouteRef::new("checkout", "HTTPRoute", "checkout-api-private")
                        .expect("route ref"),
                ),
            ]
        );
    }

    #[test]
    fn deserializes_probe_facts_in_stable_order() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "probe_facts": [
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "worker"
                    },
                    "readiness_probe": false,
                    "liveness_probe": true,
                    "startup_probe": false
                },
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "api"
                    },
                    "readiness_probe": true,
                    "liveness_probe": true,
                    "startup_probe": false
                },
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "api"
                    },
                    "readiness_probe": true,
                    "liveness_probe": true,
                    "startup_probe": false
                }
            ]
        });

        let graph: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(
            graph.probe_facts(),
            &[
                ProbeFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    true,
                    true,
                    false,
                ),
                ProbeFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    false,
                    true,
                    false,
                ),
            ]
        );
    }

    #[test]
    fn deserializes_image_facts_in_stable_order() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "image_facts": [
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "worker"
                    },
                    "image": "registry.example.com/checkout/worker:v1"
                },
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "api"
                    },
                    "image": "registry.example.com/checkout/api:v2"
                },
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "api"
                    },
                    "image": "registry.example.com/checkout/api:v2"
                }
            ]
        });

        let graph: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(
            graph.image_facts(),
            &[
                ImageFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    ImageRef::new("registry.example.com/checkout/api:v2").expect("image ref"),
                ),
                ImageFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    ImageRef::new("registry.example.com/checkout/worker:v1").expect("image ref"),
                ),
            ]
        );
    }

    #[test]
    fn deserializes_resource_facts_in_stable_order() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "resource_facts": [
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "worker"
                    },
                    "cpu_request": null,
                    "cpu_limit": null,
                    "memory_request": "128Mi",
                    "memory_limit": "256Mi"
                },
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "api"
                    },
                    "cpu_request": "250m",
                    "cpu_limit": "500m",
                    "memory_request": "512Mi",
                    "memory_limit": "1Gi"
                },
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "api"
                    },
                    "cpu_request": "250m",
                    "cpu_limit": "500m",
                    "memory_request": "512Mi",
                    "memory_limit": "1Gi"
                }
            ]
        });

        let graph: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(
            graph.resource_facts(),
            &[
                ResourceFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    Some(ResourceQuantity::new("250m").expect("resource quantity")),
                    Some(ResourceQuantity::new("500m").expect("resource quantity")),
                    Some(ResourceQuantity::new("512Mi").expect("resource quantity")),
                    Some(ResourceQuantity::new("1Gi").expect("resource quantity")),
                ),
                ResourceFacts::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    None,
                    None,
                    Some(ResourceQuantity::new("128Mi").expect("resource quantity")),
                    Some(ResourceQuantity::new("256Mi").expect("resource quantity")),
                ),
            ]
        );
    }

    #[test]
    fn deserializes_config_references_in_stable_order() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "config_references": [
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "worker"
                    },
                    "config_map": {
                        "namespace": "checkout",
                        "name": "checkout-worker-config"
                    }
                },
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "api"
                    },
                    "config_map": {
                        "namespace": "checkout",
                        "name": "checkout-api-config"
                    }
                },
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "api"
                    },
                    "config_map": {
                        "namespace": "checkout",
                        "name": "checkout-api-config"
                    }
                }
            ]
        });

        let graph: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(
            graph.config_references(),
            &[
                ConfigReference::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    ConfigMapRef::new("checkout", "checkout-api-config").expect("configmap ref"),
                ),
                ConfigReference::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    ConfigMapRef::new("checkout", "checkout-worker-config").expect("configmap ref"),
                ),
            ]
        );
    }

    #[test]
    fn deserializes_secret_references_in_stable_order() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "secret_references": [
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "worker"
                    },
                    "secret": {
                        "namespace": "checkout",
                        "name": "checkout-worker-credentials"
                    }
                },
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "api"
                    },
                    "secret": {
                        "namespace": "checkout",
                        "name": "checkout-api-credentials"
                    }
                },
                {
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "api"
                    },
                    "secret": {
                        "namespace": "checkout",
                        "name": "checkout-api-credentials"
                    }
                }
            ]
        });

        let graph: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(
            graph.secret_references(),
            &[
                SecretReference::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "api",
                    )
                    .expect("container ref"),
                    SecretMetadataRef::new("checkout", "checkout-api-credentials")
                        .expect("secret ref"),
                ),
                SecretReference::new(
                    ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    SecretMetadataRef::new("checkout", "checkout-worker-credentials")
                        .expect("secret ref"),
                ),
            ]
        );
    }

    #[test]
    fn deserializes_relationship_confidences_in_stable_order() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "relationship_confidences": [
                {
                    "relationship": {
                        "kind": "container_secret_reference",
                        "container": {
                            "workload": {
                                "namespace": "checkout",
                                "kind": "Deployment",
                                "name": "checkout-api"
                            },
                            "name": "api"
                        },
                        "secret": {
                            "namespace": "checkout",
                            "name": "checkout-api-credentials"
                        }
                    },
                    "confidence": "low"
                },
                {
                    "relationship": {
                        "kind": "service_route_reference",
                        "service": {
                            "namespace": "checkout",
                            "name": "checkout-api"
                        },
                        "route": {
                            "namespace": "checkout",
                            "kind": "Ingress",
                            "name": "checkout"
                        }
                    },
                    "confidence": "high"
                },
                {
                    "relationship": {
                        "kind": "workload_service_selection",
                        "service": {
                            "namespace": "checkout",
                            "name": "checkout-api-private"
                        }
                    },
                    "confidence": "medium"
                },
                {
                    "relationship": {
                        "kind": "container_config_reference",
                        "container": {
                            "workload": {
                                "namespace": "checkout",
                                "kind": "Deployment",
                                "name": "checkout-api"
                            },
                            "name": "api"
                        },
                        "config_map": {
                            "namespace": "checkout",
                            "name": "checkout-api-config"
                        }
                    },
                    "confidence": "high"
                },
                {
                    "relationship": {
                        "kind": "workload_pod_ownership",
                        "pod": {
                            "namespace": "checkout",
                            "name": "checkout-api-7d9f4d9d-a"
                        }
                    },
                    "confidence": "high"
                },
                {
                    "relationship": {
                        "kind": "service_route_reference",
                        "service": {
                            "namespace": "checkout",
                            "name": "checkout-api"
                        },
                        "route": {
                            "namespace": "checkout",
                            "kind": "Ingress",
                            "name": "checkout"
                        }
                    },
                    "confidence": "high"
                }
            ]
        });

        let graph: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(
            graph.relationship_confidences(),
            &[
                RelationshipConfidence::new(
                    GraphRelationship::WorkloadPodOwnership {
                        pod: PodRef::new("checkout", "checkout-api-7d9f4d9d-a").expect("pod ref"),
                    },
                    ConfidenceLevel::High,
                ),
                RelationshipConfidence::new(
                    GraphRelationship::WorkloadServiceSelection {
                        service: ServiceRef::new("checkout", "checkout-api-private")
                            .expect("service ref"),
                    },
                    ConfidenceLevel::Medium,
                ),
                RelationshipConfidence::new(
                    GraphRelationship::ServiceRouteReference {
                        service: ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                        route: RouteRef::new("checkout", "Ingress", "checkout").expect("route ref"),
                    },
                    ConfidenceLevel::High,
                ),
                RelationshipConfidence::new(
                    GraphRelationship::ContainerConfigReference {
                        container: ContainerRef::new(
                            WorkloadRef::new("checkout", "Deployment", "checkout-api")
                                .expect("workload ref"),
                            "api",
                        )
                        .expect("container ref"),
                        config_map: ConfigMapRef::new("checkout", "checkout-api-config")
                            .expect("configmap ref"),
                    },
                    ConfidenceLevel::High,
                ),
                RelationshipConfidence::new(
                    GraphRelationship::ContainerSecretReference {
                        container: ContainerRef::new(
                            WorkloadRef::new("checkout", "Deployment", "checkout-api")
                                .expect("workload ref"),
                            "api",
                        )
                        .expect("container ref"),
                        secret: SecretMetadataRef::new("checkout", "checkout-api-credentials")
                            .expect("secret ref"),
                    },
                    ConfidenceLevel::Low,
                ),
            ]
        );
    }

    #[test]
    fn deserializes_warnings_in_stable_order() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "warnings": [
                {
                    "kind": "missing_route",
                    "service": {
                        "namespace": "checkout",
                        "name": "checkout-api-private"
                    }
                },
                {
                    "kind": "ambiguous_service_selector",
                    "service": {
                        "namespace": "checkout",
                        "name": "checkout-api"
                    },
                    "candidate_workloads": [
                        {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-worker"
                        },
                        {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        }
                    ]
                },
                {
                    "kind": "missing_route",
                    "service": {
                        "namespace": "checkout",
                        "name": "checkout-api-private"
                    }
                },
                {
                    "kind": "missing_probes",
                    "container": {
                        "workload": {
                            "namespace": "checkout",
                            "kind": "Deployment",
                            "name": "checkout-api"
                        },
                        "name": "worker"
                    },
                    "missing_probes": ["startup", "readiness", "startup"]
                }
            ]
        });

        let graph: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(
            graph.warnings(),
            &[
                AppGraphWarning::AmbiguousServiceSelector {
                    service: ServiceRef::new("checkout", "checkout-api").expect("service ref"),
                    candidate_workloads: vec![
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        WorkloadRef::new("checkout", "Deployment", "checkout-worker")
                            .expect("workload ref"),
                    ],
                },
                AppGraphWarning::MissingRoute {
                    service: ServiceRef::new("checkout", "checkout-api-private")
                        .expect("service ref"),
                },
                AppGraphWarning::MissingProbes {
                    container: ContainerRef::new(
                        WorkloadRef::new("checkout", "Deployment", "checkout-api")
                            .expect("workload ref"),
                        "worker",
                    )
                    .expect("container ref"),
                    missing_probes: vec![ProbeKind::Readiness, ProbeKind::Startup],
                },
            ]
        );
    }

    #[test]
    fn round_trips_app_graph_json() {
        let graph = test_app_graph();
        let value = serde_json::to_value(&graph).expect("app graph should serialize");

        let parsed: AppGraph = serde_json::from_value(value).expect("app graph should deserialize");

        assert_eq!(parsed, graph);
    }

    #[test]
    fn rejects_invalid_app_graph_workload_json() {
        let value = json!({
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "CheckoutApi"
            }
        });

        let error = serde_json::from_value::<AppGraph>(value)
            .expect_err("invalid workload name should be rejected");

        assert!(
            error
                .to_string()
                .contains("invalid workload name: value must start and end"),
            "unexpected app graph error: {error}"
        );
    }

    #[test]
    fn snapshots_app_graph_json_contract() {
        insta::assert_json_snapshot!("app_graph_json_contract", test_app_graph());
    }

    #[test]
    fn snapshots_minimal_app_graph_shape() {
        insta::assert_json_snapshot!("minimal_app_graph_shape", minimal_app_graph());
    }

    #[test]
    fn snapshots_routed_app_graph_shape() {
        insta::assert_json_snapshot!("routed_app_graph_shape", routed_app_graph());
    }

    #[test]
    fn snapshots_warning_app_graph_shape() {
        insta::assert_json_snapshot!("warning_app_graph_shape", warning_app_graph());
    }

    #[test]
    fn creates_session_id_from_valid_value() {
        let session_id = SessionId::new("session-123").expect("session id should be valid");

        assert_eq!(session_id.as_str(), "session-123");
        assert_eq!(session_id.to_string(), "session-123");
    }

    #[test]
    fn rejects_empty_session_id() {
        let error = SessionId::new("").expect_err("empty session id should be rejected");

        assert_eq!(error, SessionIdError::Empty);
    }

    #[test]
    fn rejects_session_id_that_exceeds_max_length() {
        let value = "a".repeat(SESSION_TOKEN_MAX_LEN + 1);
        let error = SessionId::new(value).expect_err("long session id should be rejected");

        assert_eq!(
            error,
            SessionIdError::TooLong {
                max_len: SESSION_TOKEN_MAX_LEN
            }
        );
    }

    #[test]
    fn rejects_session_id_with_invalid_boundary() {
        for value in ["-session", "Session", "session-"] {
            let error = SessionId::new(value).expect_err("boundary should be rejected");

            assert_eq!(error, SessionIdError::InvalidBoundary);
        }
    }

    #[test]
    fn rejects_session_id_with_invalid_character() {
        let error = SessionId::new("sesSion").expect_err("uppercase should be rejected");

        assert_eq!(error, SessionIdError::InvalidCharacter { character: 'S' });
    }

    #[test]
    fn creates_session_name_from_valid_value() {
        let session_name = SessionName::new("checkout-test").expect("session name should be valid");

        assert_eq!(session_name.as_str(), "checkout-test");
        assert_eq!(session_name.to_string(), "checkout-test");
    }

    #[test]
    fn rejects_empty_session_name() {
        let error = SessionName::new("").expect_err("empty session name should be rejected");

        assert_eq!(error, SessionNameError::Empty);
    }

    #[test]
    fn rejects_session_name_that_exceeds_max_length() {
        let value = "a".repeat(SESSION_TOKEN_MAX_LEN + 1);
        let error = SessionName::new(value).expect_err("long session name should be rejected");

        assert_eq!(
            error,
            SessionNameError::TooLong {
                max_len: SESSION_TOKEN_MAX_LEN
            }
        );
    }

    #[test]
    fn rejects_session_name_with_invalid_boundary() {
        for value in ["-checkout", "Checkout", "checkout-"] {
            let error = SessionName::new(value).expect_err("boundary should be rejected");

            assert_eq!(error, SessionNameError::InvalidBoundary);
        }
    }

    #[test]
    fn rejects_session_name_with_invalid_character() {
        let error = SessionName::new("check_out").expect_err("underscore should be rejected");

        assert_eq!(error, SessionNameError::InvalidCharacter { character: '_' });
    }

    #[test]
    fn lists_session_statuses_in_lifecycle_order() {
        assert_eq!(
            SessionStatus::all(),
            &[
                SessionStatus::Planned,
                SessionStatus::Preparing,
                SessionStatus::Active,
                SessionStatus::Verifying,
                SessionStatus::Blocked,
                SessionStatus::Ready,
                SessionStatus::CleanedUp,
                SessionStatus::Failed,
            ]
        );
    }

    #[test]
    fn renders_session_status_names() {
        let status_names = SessionStatus::all()
            .iter()
            .map(SessionStatus::as_str)
            .collect::<Vec<_>>();

        assert_eq!(
            status_names,
            [
                "planned",
                "preparing",
                "active",
                "verifying",
                "blocked",
                "ready",
                "cleaned_up",
                "failed",
            ]
        );
        assert_eq!(SessionStatus::CleanedUp.to_string(), "cleaned_up");
    }

    #[test]
    fn validates_representative_session_status_transition() {
        assert!(SessionStatus::Planned.can_transition_to(SessionStatus::Preparing));
        assert_eq!(
            SessionStatus::Planned.validate_transition_to(SessionStatus::Preparing),
            Ok(())
        );
    }

    #[test]
    fn validates_additional_session_status_transitions() {
        for (from, to) in [
            (SessionStatus::Active, SessionStatus::CleanedUp),
            (SessionStatus::Blocked, SessionStatus::Preparing),
            (SessionStatus::Blocked, SessionStatus::Active),
            (SessionStatus::Ready, SessionStatus::CleanedUp),
            (SessionStatus::Failed, SessionStatus::CleanedUp),
        ] {
            assert!(
                from.can_transition_to(to),
                "{from} should transition to {to}"
            );
            assert_eq!(from.validate_transition_to(to), Ok(()));
        }
    }

    #[test]
    fn rejects_representative_session_status_transition() {
        let error = SessionStatus::CleanedUp
            .validate_transition_to(SessionStatus::Active)
            .expect_err("cleaned up sessions are terminal");

        assert_eq!(
            error,
            SessionTransitionError::Invalid {
                from: SessionStatus::CleanedUp,
                to: SessionStatus::Active
            }
        );
    }

    #[test]
    fn rejects_additional_session_status_transitions() {
        for (from, to) in [
            (SessionStatus::Ready, SessionStatus::Preparing),
            (SessionStatus::Active, SessionStatus::Planned),
            (SessionStatus::Ready, SessionStatus::Active),
            (SessionStatus::Failed, SessionStatus::Ready),
            (SessionStatus::CleanedUp, SessionStatus::Failed),
        ] {
            let error = from
                .validate_transition_to(to)
                .expect_err("transition should be rejected");

            assert!(!from.can_transition_to(to));
            assert_eq!(error, SessionTransitionError::Invalid { from, to });
        }
    }

    #[test]
    fn every_non_terminal_session_status_has_valid_outgoing_transition() {
        for (from, to) in [
            (SessionStatus::Planned, SessionStatus::Preparing),
            (SessionStatus::Preparing, SessionStatus::Active),
            (SessionStatus::Active, SessionStatus::Verifying),
            (SessionStatus::Verifying, SessionStatus::Ready),
            (SessionStatus::Blocked, SessionStatus::Preparing),
            (SessionStatus::Ready, SessionStatus::CleanedUp),
            (SessionStatus::Failed, SessionStatus::CleanedUp),
        ] {
            assert!(
                from.can_transition_to(to),
                "{from} should transition to {to}"
            );
            assert_eq!(from.validate_transition_to(to), Ok(()));
        }
    }

    #[test]
    fn cleaned_up_session_status_has_no_outgoing_transitions() {
        for status in SessionStatus::all() {
            let error = SessionStatus::CleanedUp
                .validate_transition_to(*status)
                .expect_err("cleaned up sessions should be terminal");

            assert!(!SessionStatus::CleanedUp.can_transition_to(*status));
            assert_eq!(
                error,
                SessionTransitionError::Invalid {
                    from: SessionStatus::CleanedUp,
                    to: *status
                }
            );
        }
    }

    #[test]
    fn session_status_self_transitions_are_rejected() {
        for status in SessionStatus::all() {
            let error = status
                .validate_transition_to(*status)
                .expect_err("self-transition should be rejected");

            assert!(!status.can_transition_to(*status));
            assert_eq!(
                error,
                SessionTransitionError::Invalid {
                    from: *status,
                    to: *status
                }
            );
        }
    }

    #[test]
    fn lists_session_event_kinds_in_lifecycle_order() {
        assert_eq!(
            SessionEventKind::all(),
            &[
                SessionEventKind::Planned,
                SessionEventKind::Preparing,
                SessionEventKind::Active,
                SessionEventKind::Verifying,
                SessionEventKind::Blocked,
                SessionEventKind::Ready,
                SessionEventKind::CleanedUp,
                SessionEventKind::Failed,
            ]
        );
    }

    #[test]
    fn renders_session_event_kind_names() {
        let kind_names = SessionEventKind::all()
            .iter()
            .map(SessionEventKind::as_str)
            .collect::<Vec<_>>();

        assert_eq!(
            kind_names,
            [
                "planned",
                "preparing",
                "active",
                "verifying",
                "blocked",
                "ready",
                "cleaned_up",
                "failed",
            ]
        );
        assert_eq!(SessionEventKind::CleanedUp.to_string(), "cleaned_up");
    }

    #[test]
    fn maps_session_event_kinds_to_statuses() {
        let statuses = SessionEventKind::all()
            .iter()
            .map(SessionEventKind::status)
            .collect::<Vec<_>>();

        assert_eq!(statuses, SessionStatus::all());
    }

    #[test]
    fn creates_session_event_for_audit_history() {
        let session_id = SessionId::new("session-123").expect("session id");
        let event = SessionEvent::new(session_id.clone(), 7, SessionEventKind::Verifying);

        assert_eq!(event.session_id(), &session_id);
        assert_eq!(event.sequence(), 7);
        assert_eq!(event.kind(), SessionEventKind::Verifying);
        assert_eq!(event.status(), SessionStatus::Verifying);
    }

    #[test]
    fn snapshots_session_plan_json_contract() {
        let route_selector =
            RouteSelector::header("x-kply-session", "session-123").expect("route selector");
        let plan = test_session_plan()
            .with_planned_resources([
                KubernetesResourceRef::new("checkout", "Deployment", "session-123-workload")
                    .expect("planned workload"),
                KubernetesResourceRef::new("checkout", "Service", "session-123-service")
                    .expect("planned service"),
            ])
            .with_route_selector(route_selector);
        let value = serde_json::to_value(plan).expect("session plan should serialize");

        insta::assert_json_snapshot!("session_plan_json_contract", value);
    }

    #[test]
    fn deserializes_session_plan_with_validated_fields() {
        let plan: SessionPlan = serde_json::from_value(json!({
            "id": "session-123",
            "name": "checkout-test",
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "image": "registry.example.com/checkout/api:v2",
            "ttl": "30m",
            "planned_resources": [
                {
                    "namespace": "checkout",
                    "kind": "Deployment",
                    "name": "session-123-workload"
                },
                {
                    "namespace": "checkout",
                    "kind": "Service",
                    "name": "session-123-service"
                }
            ],
            "route_selector": {
                "kind": "host",
                "hostname": "session-123.preview.example.com"
            },
            "policy": {
                "allowed_operations": [
                    "inspect",
                    "plan",
                    "prepare",
                    "route",
                    "verify",
                    "cleanup"
                ]
            },
            "status": "planned"
        }))
        .expect("session plan should deserialize");

        assert_eq!(plan.id().as_str(), "session-123");
        assert_eq!(plan.name().as_str(), "checkout-test");
        assert_eq!(
            plan.route_selector().and_then(RouteSelector::hostname),
            Some("session-123.preview.example.com")
        );
        assert_eq!(plan.time_to_live().map(TimeToLive::as_str), Some("30m"));
        assert_eq!(plan.planned_resources().len(), 2);
        assert_eq!(plan.planned_resources()[0].name(), "session-123-workload");
        assert_eq!(plan.status(), SessionStatus::Planned);
    }

    #[test]
    fn rejects_invalid_session_plan_time_to_live_json() {
        let error = serde_json::from_value::<SessionPlan>(json!({
            "id": "session-123",
            "name": "checkout-test",
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "image": "registry.example.com/checkout/api:v2",
            "ttl": "forever",
            "route_selector": null,
            "policy": {
                "allowed_operations": ["inspect"]
            },
            "status": "planned"
        }))
        .expect_err("invalid ttl should be rejected");

        assert!(error.to_string().contains("ttl"));
    }

    #[test]
    fn rejects_invalid_session_plan_json() {
        let error = serde_json::from_value::<SessionPlan>(json!({
            "id": "Session-123",
            "name": "checkout-test",
            "workload": {
                "namespace": "checkout",
                "kind": "Deployment",
                "name": "checkout-api"
            },
            "image": "registry.example.com/checkout/api:v2",
            "route_selector": null,
            "policy": {
                "allowed_operations": ["inspect"]
            },
            "status": "planned"
        }))
        .expect_err("invalid session id should be rejected");

        assert!(error.to_string().contains("session id"));
    }

    #[test]
    fn round_trips_session_report_json() {
        let report =
            SessionReport::new(test_session_plan(), SessionStatus::Ready).expect("session report");
        let value = serde_json::to_value(&report).expect("session report should serialize");
        let deserialized: SessionReport =
            serde_json::from_value(value).expect("session report should deserialize");

        assert_eq!(deserialized, report);
    }

    #[test]
    fn snapshots_session_report_json_contract() {
        let report =
            SessionReport::new(test_session_plan(), SessionStatus::Ready).expect("session report");
        let value = serde_json::to_value(report).expect("session report should serialize");

        insta::assert_json_snapshot!("session_report_json_contract", value);
    }

    #[test]
    fn rejects_session_report_json_with_non_reportable_status() {
        let error = serde_json::from_value::<SessionReport>(json!({
            "plan": {
                "id": "session-123",
                "name": "checkout-test",
                "workload": {
                    "namespace": "checkout",
                    "kind": "Deployment",
                    "name": "checkout-api"
                },
                "image": "registry.example.com/checkout/api:v2",
                "route_selector": null,
                "policy": {
                    "allowed_operations": ["inspect"]
                },
                "status": "planned"
            },
            "status": "active"
        }))
        .expect_err("non-reportable report status should be rejected");

        assert!(error.to_string().contains("not reportable"));
    }

    #[test]
    fn round_trips_session_event_json() {
        let event = SessionEvent::new(
            SessionId::new("session-123").expect("session id"),
            3,
            SessionEventKind::Ready,
        );
        let value = serde_json::to_value(&event).expect("session event should serialize");

        assert_eq!(
            value,
            json!({
                "session_id": "session-123",
                "sequence": 3,
                "kind": "ready",
                "status": "ready"
            })
        );

        let deserialized: SessionEvent =
            serde_json::from_value(value).expect("session event should deserialize");

        assert_eq!(deserialized, event);
    }

    #[test]
    fn rejects_session_event_json_with_mismatched_status() {
        let error = serde_json::from_value::<SessionEvent>(json!({
            "session_id": "session-123",
            "sequence": 3,
            "kind": "ready",
            "status": "failed"
        }))
        .expect_err("mismatched event status should be rejected");

        assert!(error.to_string().contains("does not match kind"));
    }

    #[test]
    fn lists_session_operations_in_declaration_order() {
        assert_eq!(
            SessionOperation::all(),
            &[
                SessionOperation::Inspect,
                SessionOperation::Plan,
                SessionOperation::Prepare,
                SessionOperation::Route,
                SessionOperation::Verify,
                SessionOperation::Cleanup,
                SessionOperation::Promote,
            ]
        );
    }

    #[test]
    fn renders_session_operation_names() {
        let operation_names = SessionOperation::all()
            .iter()
            .map(SessionOperation::as_str)
            .collect::<Vec<_>>();

        assert_eq!(
            operation_names,
            [
                "inspect", "plan", "prepare", "route", "verify", "cleanup", "promote",
            ]
        );
        assert_eq!(SessionOperation::Promote.to_string(), "promote");
    }

    #[test]
    fn creates_session_policy_with_stable_operation_order() {
        let policy = SessionPolicy::new([
            SessionOperation::Verify,
            SessionOperation::Inspect,
            SessionOperation::Cleanup,
        ])
        .expect("session policy");

        assert_eq!(
            policy.allowed_operations(),
            &[
                SessionOperation::Inspect,
                SessionOperation::Verify,
                SessionOperation::Cleanup,
            ]
        );
        assert!(policy.allows(SessionOperation::Verify));
        assert!(!policy.allows(SessionOperation::Promote));
    }

    #[test]
    fn creates_sandbox_session_policy_without_promotion() {
        let policy = SessionPolicy::sandbox();

        assert_eq!(
            policy.allowed_operations(),
            &[
                SessionOperation::Inspect,
                SessionOperation::Plan,
                SessionOperation::Prepare,
                SessionOperation::Route,
                SessionOperation::Verify,
                SessionOperation::Cleanup,
            ]
        );
        assert_eq!(SessionPolicy::default(), policy);
        assert!(!policy.allows(SessionOperation::Promote));
    }

    #[test]
    fn rejects_empty_session_policy() {
        let error = SessionPolicy::new([]).expect_err("empty policy");

        assert_eq!(error, SessionPolicyError::Empty);
    }

    #[test]
    fn rejects_duplicate_session_policy_operations() {
        let error = SessionPolicy::new([
            SessionOperation::Inspect,
            SessionOperation::Verify,
            SessionOperation::Inspect,
        ])
        .expect_err("duplicate policy operation");

        assert_eq!(
            error,
            SessionPolicyError::Duplicate {
                operation: SessionOperation::Inspect
            }
        );
    }

    #[test]
    fn creates_session_plan_for_dry_run_output() {
        let id = SessionId::new("session-123").expect("session id");
        let name = SessionName::new("checkout-test").expect("session name");
        let workload =
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref");
        let image = ImageRef::new("registry.example.com/checkout/api:v2").expect("image ref");
        let policy = SessionPolicy::sandbox();
        let plan = SessionPlan::new(
            id.clone(),
            name.clone(),
            workload.clone(),
            image.clone(),
            policy.clone(),
        );

        assert_eq!(plan.id(), &id);
        assert_eq!(plan.name(), &name);
        assert_eq!(plan.workload(), &workload);
        assert_eq!(plan.image(), &image);
        assert_eq!(plan.planned_resources(), []);
        assert_eq!(plan.route_selector(), None);
        assert_eq!(plan.policy(), &policy);
        assert_eq!(plan.status(), SessionStatus::Planned);
    }

    #[test]
    fn creates_session_plan_with_planned_resources() {
        let planned_service =
            KubernetesResourceRef::new("checkout", "Service", "session-123-service")
                .expect("planned service");
        let planned_workload =
            KubernetesResourceRef::new("checkout", "Deployment", "session-123-workload")
                .expect("planned workload");
        let plan = test_session_plan().with_planned_resources([
            planned_service.clone(),
            planned_workload.clone(),
            planned_service.clone(),
        ]);

        assert_eq!(
            plan.planned_resources(),
            [planned_workload, planned_service]
        );
    }

    #[test]
    fn creates_session_plan_with_route_selector() {
        let route_selector =
            RouteSelector::header("x-kply-session", "session-123").expect("route selector");
        let plan = test_session_plan().with_route_selector(route_selector.clone());

        assert_eq!(plan.route_selector(), Some(&route_selector));
    }

    #[test]
    fn creates_session_report_for_reportable_status() {
        for status in [
            SessionStatus::Blocked,
            SessionStatus::Ready,
            SessionStatus::CleanedUp,
            SessionStatus::Failed,
        ] {
            let plan = test_session_plan();
            let report =
                SessionReport::new(plan.clone(), status).expect("session report should be valid");

            assert_eq!(report.plan(), &plan);
            assert_eq!(report.status(), status);
        }
    }

    #[test]
    fn rejects_session_report_for_non_reportable_status() {
        for status in [
            SessionStatus::Planned,
            SessionStatus::Preparing,
            SessionStatus::Active,
            SessionStatus::Verifying,
        ] {
            let error =
                SessionReport::new(test_session_plan(), status).expect_err("session report");

            assert_eq!(error, SessionReportError::NonReportableStatus { status });
        }
    }

    #[test]
    fn creates_header_route_selector() {
        let selector =
            RouteSelector::header("x-kply-session", "session-123").expect("route selector");

        assert_eq!(selector.kind(), "header");
        assert_eq!(
            selector.header_parts(),
            Some(("x-kply-session", "session-123"))
        );
        assert_eq!(selector.hostname(), None);
        assert_eq!(selector.to_string(), "header:x-kply-session=session-123");
    }

    #[test]
    fn creates_header_route_selector_with_exact_max_value_length() {
        let value = "a".repeat(ROUTE_HEADER_VALUE_MAX_LEN);
        let selector =
            RouteSelector::header("x-kply-session", value.as_str()).expect("route selector");

        assert_eq!(
            selector.header_parts(),
            Some(("x-kply-session", value.as_str()))
        );
    }

    #[test]
    fn creates_header_route_selector_with_exact_max_name_length() {
        let name = "a".repeat(ROUTE_HEADER_NAME_MAX_LEN);
        let selector = RouteSelector::header(name.as_str(), "session-123").expect("route selector");

        assert_eq!(
            selector.header_parts(),
            Some((name.as_str(), "session-123"))
        );
    }

    #[test]
    fn creates_header_route_selector_with_special_token_characters() {
        for name in ["x_kply", "x.kply", "x+kply", "x~kply", "x!#$%&'*^`|kply"] {
            let selector = RouteSelector::header(name, "session-123").expect("route selector");

            assert_eq!(selector.header_parts(), Some((name, "session-123")));
        }
    }

    #[test]
    fn creates_host_route_selector() {
        let selector =
            RouteSelector::host("session-123.preview.example.com").expect("route selector");

        assert_eq!(selector.kind(), "host");
        assert_eq!(selector.header_parts(), None);
        assert_eq!(selector.hostname(), Some("session-123.preview.example.com"));
        assert_eq!(selector.to_string(), "host:session-123.preview.example.com");
    }

    #[test]
    fn rejects_route_selector_json_with_cross_variant_fields() {
        for value in [
            json!({
                "kind": "header",
                "name": "x-kply-session",
                "value": "session-123",
                "hostname": "session-123.preview.example.com"
            }),
            json!({
                "kind": "host",
                "hostname": "session-123.preview.example.com",
                "name": "x-kply-session"
            }),
        ] {
            serde_json::from_value::<RouteSelector>(value)
                .expect_err("cross-variant route selector fields should be rejected");
        }
    }

    #[test]
    fn rejects_route_selector_json_with_unknown_kind_or_field() {
        for value in [
            json!({
                "kind": "cookie",
                "name": "kply-session",
                "value": "session-123"
            }),
            json!({
                "kind": "header",
                "name": "x-kply-session",
                "value": "session-123",
                "extra": true
            }),
        ] {
            serde_json::from_value::<RouteSelector>(value)
                .expect_err("unknown route selector shape should be rejected");
        }
    }

    #[test]
    fn creates_host_route_selector_with_exact_max_length() {
        let label = "a".repeat(ROUTE_HOST_LABEL_MAX_LEN);
        let final_label = "a".repeat(ROUTE_HOST_LABEL_MAX_LEN - 2);
        let host = format!("{label}.{label}.{label}.{final_label}");
        assert_eq!(host.len(), ROUTE_HOST_MAX_LEN);

        let selector = RouteSelector::host(host.as_str()).expect("route selector");

        assert_eq!(selector.hostname(), Some(host.as_str()));
    }

    #[test]
    fn rejects_empty_route_header_name() {
        let error = RouteSelector::header("", "session-123").expect_err("header name");

        assert_eq!(
            error,
            RouteSelectorError::HeaderName(RouteHeaderNameError::Empty)
        );
    }

    #[test]
    fn rejects_long_route_header_name() {
        let name = "a".repeat(ROUTE_HEADER_NAME_MAX_LEN + 1);
        let error = RouteSelector::header(name, "session-123").expect_err("header name");

        assert_eq!(
            error,
            RouteSelectorError::HeaderName(RouteHeaderNameError::TooLong {
                max_len: ROUTE_HEADER_NAME_MAX_LEN
            })
        );
    }

    #[test]
    fn rejects_invalid_route_header_name_character() {
        let error =
            RouteSelector::header("x kply session", "session-123").expect_err("header name");

        assert_eq!(
            error,
            RouteSelectorError::HeaderName(RouteHeaderNameError::InvalidCharacter {
                character: ' '
            })
        );
    }

    #[test]
    fn rejects_empty_route_header_value() {
        let error = RouteSelector::header("x-kply-session", "").expect_err("header value");

        assert_eq!(
            error,
            RouteSelectorError::HeaderValue(RouteHeaderValueError::Empty)
        );
    }

    #[test]
    fn rejects_long_route_header_value() {
        let value = "a".repeat(ROUTE_HEADER_VALUE_MAX_LEN + 1);
        let error = RouteSelector::header("x-kply-session", value).expect_err("header value");

        assert_eq!(
            error,
            RouteSelectorError::HeaderValue(RouteHeaderValueError::TooLong {
                max_len: ROUTE_HEADER_VALUE_MAX_LEN
            })
        );
    }

    #[test]
    fn rejects_control_route_header_value_character() {
        let error =
            RouteSelector::header("x-kply-session", "session\n123").expect_err("header value");

        assert_eq!(
            error,
            RouteSelectorError::HeaderValue(RouteHeaderValueError::InvalidCharacter {
                character: '\n'
            })
        );
    }

    #[test]
    fn rejects_space_route_header_value_character() {
        let error =
            RouteSelector::header("x-kply-session", "session 123").expect_err("header value");

        assert_eq!(
            error,
            RouteSelectorError::HeaderValue(RouteHeaderValueError::InvalidCharacter {
                character: ' '
            })
        );
    }

    #[test]
    fn rejects_empty_route_host() {
        let error = RouteSelector::host("").expect_err("host");

        assert_eq!(error, RouteSelectorError::Host(RouteHostError::Empty));
    }

    #[test]
    fn rejects_long_route_host() {
        let host = format!("{}.example.com", "a".repeat(ROUTE_HOST_MAX_LEN));
        let error = RouteSelector::host(host).expect_err("host");

        assert_eq!(
            error,
            RouteSelectorError::Host(RouteHostError::TooLong {
                max_len: ROUTE_HOST_MAX_LEN
            })
        );
    }

    #[test]
    fn rejects_long_route_host_label() {
        let host = format!("{}.example.com", "a".repeat(ROUTE_HOST_LABEL_MAX_LEN + 1));
        let error = RouteSelector::host(host).expect_err("host");

        assert_eq!(
            error,
            RouteSelectorError::Host(RouteHostError::LabelTooLong {
                max_len: ROUTE_HOST_LABEL_MAX_LEN
            })
        );
    }

    #[test]
    fn rejects_route_host_empty_label() {
        let error = RouteSelector::host("session..example.com").expect_err("host");

        assert_eq!(error, RouteSelectorError::Host(RouteHostError::EmptyLabel));
    }

    #[test]
    fn rejects_route_host_invalid_boundary() {
        for host in ["-session.example.com", "session-.example.com"] {
            let error = RouteSelector::host(host).expect_err("host");

            assert_eq!(
                error,
                RouteSelectorError::Host(RouteHostError::InvalidBoundary)
            );
        }
    }

    #[test]
    fn rejects_route_host_invalid_internal_label_boundary() {
        for host in ["session.-example.com", "session.example-.com"] {
            let error = RouteSelector::host(host).expect_err("host");

            assert_eq!(
                error,
                RouteSelectorError::Host(RouteHostError::InvalidBoundary)
            );
        }
    }

    #[test]
    fn rejects_route_host_invalid_character() {
        let error = RouteSelector::host("session.exa_mple.com").expect_err("host");

        assert_eq!(
            error,
            RouteSelectorError::Host(RouteHostError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn rejects_route_host_uppercase() {
        let error = RouteSelector::host("Session.example.com").expect_err("host");

        assert_eq!(
            error,
            RouteSelectorError::Host(RouteHostError::InvalidCharacter { character: 'S' })
        );
    }

    #[test]
    fn creates_workload_ref_from_valid_parts() {
        let workload =
            WorkloadRef::new("checkout", "Deployment", "checkout-api").expect("workload ref");

        assert_eq!(workload.namespace(), "checkout");
        assert_eq!(workload.kind(), "Deployment");
        assert_eq!(workload.name(), "checkout-api");
        assert_eq!(workload.to_string(), "checkout/Deployment/checkout-api");
    }

    #[test]
    fn creates_workload_ref_for_custom_resource_kind() {
        let workload = WorkloadRef::new("platform", "Rollout.argoproj.io", "api-rollout")
            .expect("workload ref");

        assert_eq!(workload.kind(), "Rollout.argoproj.io");
    }

    #[test]
    fn rejects_workload_ref_with_invalid_namespace() {
        let error =
            WorkloadRef::new("Checkout", "Deployment", "checkout-api").expect_err("namespace");

        assert_eq!(
            error,
            WorkloadRefError::Namespace(WorkloadTokenError::InvalidBoundary)
        );
    }

    #[test]
    fn rejects_workload_ref_with_invalid_name() {
        let error = WorkloadRef::new("checkout", "Deployment", "checkout_api").expect_err("name");

        assert_eq!(
            error,
            WorkloadRefError::Name(WorkloadTokenError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn rejects_workload_ref_with_invalid_kind_boundary() {
        let error =
            WorkloadRef::new("checkout", "-Deployment", "checkout-api").expect_err("kind boundary");

        assert_eq!(
            error,
            WorkloadRefError::Kind(WorkloadKindError::InvalidBoundary)
        );
    }

    #[test]
    fn rejects_workload_ref_with_invalid_kind_character() {
        let error = WorkloadRef::new("checkout", "Deploy_ment", "checkout-api").expect_err("kind");

        assert_eq!(
            error,
            WorkloadRefError::Kind(WorkloadKindError::InvalidCharacter { character: '_' })
        );
    }

    #[test]
    fn rejects_workload_ref_with_long_kind() {
        let kind = "A".repeat(WORKLOAD_KIND_MAX_LEN + 1);
        let error = WorkloadRef::new("checkout", kind, "checkout-api").expect_err("long kind");

        assert_eq!(
            error,
            WorkloadRefError::Kind(WorkloadKindError::TooLong {
                max_len: WORKLOAD_KIND_MAX_LEN
            })
        );
    }

    #[test]
    fn creates_kubernetes_resource_ref_from_valid_parts() {
        let resource = KubernetesResourceRef::new("checkout", "HTTPRoute", "session-123-route")
            .expect("resource ref");

        assert_eq!(resource.namespace(), "checkout");
        assert_eq!(resource.kind(), "HTTPRoute");
        assert_eq!(resource.name(), "session-123-route");
        assert_eq!(resource.to_string(), "checkout/HTTPRoute/session-123-route");
    }

    #[test]
    fn rejects_invalid_kubernetes_resource_ref_parts() {
        let namespace_error =
            KubernetesResourceRef::new("Checkout", "HTTPRoute", "session-123-route")
                .expect_err("namespace should be invalid");
        let kind_error = KubernetesResourceRef::new("checkout", "", "session-123-route")
            .expect_err("kind should be invalid");
        let name_error = KubernetesResourceRef::new("checkout", "HTTPRoute", "Session-123")
            .expect_err("name should be invalid");

        assert_eq!(
            namespace_error,
            KubernetesResourceRefError::Namespace(WorkloadTokenError::InvalidBoundary)
        );
        assert_eq!(
            kind_error,
            KubernetesResourceRefError::Kind(WorkloadKindError::Empty)
        );
        assert_eq!(
            name_error,
            KubernetesResourceRefError::Name(WorkloadTokenError::InvalidBoundary)
        );
    }

    #[test]
    fn creates_image_ref_from_tagged_reference() {
        let image_ref =
            ImageRef::new("registry.example.com/platform/checkout-api:1.2.3").expect("image ref");

        assert_eq!(
            image_ref.as_str(),
            "registry.example.com/platform/checkout-api:1.2.3"
        );
        assert_eq!(
            image_ref.to_string(),
            "registry.example.com/platform/checkout-api:1.2.3"
        );
    }

    #[test]
    fn creates_image_ref_from_simple_name() {
        let image_ref = ImageRef::new("nginx").expect("image ref");

        assert_eq!(image_ref.as_str(), "nginx");
    }

    #[test]
    fn creates_image_ref_from_library_path() {
        let image_ref = ImageRef::new("library/nginx:latest").expect("image ref");

        assert_eq!(image_ref.as_str(), "library/nginx:latest");
    }

    #[test]
    fn creates_image_ref_with_repository_underscore() {
        let image_ref = ImageRef::new("my_image:v1").expect("image ref");

        assert_eq!(image_ref.as_str(), "my_image:v1");
    }

    #[test]
    fn creates_image_ref_from_deep_repository_path() {
        let image_ref = ImageRef::new("registry.io/a/b/c/image:tag").expect("image ref");

        assert_eq!(image_ref.as_str(), "registry.io/a/b/c/image:tag");
    }

    #[test]
    fn creates_image_ref_from_digest_reference() {
        let image_ref = ImageRef::new("registry.example.com/platform/checkout-api@sha256:abcdef")
            .expect("image ref");

        assert_eq!(
            image_ref.as_str(),
            "registry.example.com/platform/checkout-api@sha256:abcdef"
        );
    }

    #[test]
    fn creates_image_ref_with_tag_and_digest() {
        let image = "registry.example.com/platform/checkout-api:1.2.3@sha256:abcdef";
        let image_ref = ImageRef::new(image).expect("image ref");

        assert_eq!(image_ref.as_str(), image);
    }

    #[test]
    fn creates_image_ref_with_registry_port() {
        let image_ref = ImageRef::new("localhost:5000/platform/checkout-api:dev")
            .expect("image ref with registry port");

        assert_eq!(
            image_ref.as_str(),
            "localhost:5000/platform/checkout-api:dev"
        );
    }

    #[test]
    fn creates_image_ref_with_mixed_case_tag() {
        let image_ref =
            ImageRef::new("registry.example.com/platform/checkout-api:BuildA").expect("image ref");

        assert_eq!(
            image_ref.as_str(),
            "registry.example.com/platform/checkout-api:BuildA"
        );
    }

    #[test]
    fn rejects_empty_image_ref() {
        let error = ImageRef::new("").expect_err("empty image ref should be rejected");

        assert_eq!(error, ImageRefError::Empty);
    }

    #[test]
    fn rejects_long_image_ref() {
        let value = "a".repeat(IMAGE_REF_MAX_LEN + 1);
        let error = ImageRef::new(value).expect_err("long image ref should be rejected");

        assert_eq!(
            error,
            ImageRefError::TooLong {
                max_len: IMAGE_REF_MAX_LEN
            }
        );
    }

    #[test]
    fn creates_time_to_live_from_duration_spelling() {
        for value in ["1s", "30m", "12h", "7d"] {
            let time_to_live = TimeToLive::new(value).expect("ttl should be valid");

            assert_eq!(time_to_live.as_str(), value);
            assert_eq!(time_to_live.to_string(), value);
        }
    }

    #[test]
    fn rejects_invalid_time_to_live_values() {
        assert_eq!(TimeToLive::new("").unwrap_err(), TimeToLiveError::Empty);
        assert_eq!(TimeToLive::new("0m").unwrap_err(), TimeToLiveError::Zero);
        assert_eq!(
            TimeToLive::new("30").unwrap_err(),
            TimeToLiveError::InvalidUnit { unit: '0' }
        );
        assert_eq!(
            TimeToLive::new("tenm").unwrap_err(),
            TimeToLiveError::InvalidNumber
        );
        assert_eq!(
            TimeToLive::new("1w").unwrap_err(),
            TimeToLiveError::InvalidUnit { unit: 'w' }
        );

        let value = "1".repeat(TIME_TO_LIVE_MAX_LEN + 1);
        assert_eq!(
            TimeToLive::new(value).unwrap_err(),
            TimeToLiveError::TooLong {
                max_len: TIME_TO_LIVE_MAX_LEN
            }
        );
    }

    #[test]
    fn rejects_image_ref_with_invalid_boundary() {
        for value in ["/checkout-api:1.2.3", "checkout-api:"] {
            let error = ImageRef::new(value).expect_err("boundary should be rejected");

            assert_eq!(error, ImageRefError::InvalidBoundary);
        }
    }

    #[test]
    fn rejects_image_ref_with_invalid_character() {
        let error = ImageRef::new("checkout api:1.2.3").expect_err("space should be rejected");

        assert_eq!(error, ImageRefError::InvalidCharacter { character: ' ' });
    }

    #[test]
    fn rejects_image_ref_with_uppercase_repository() {
        let error =
            ImageRef::new("registry.example.com/platform/Checkout-api:1.2.3").expect_err("image");

        assert_eq!(error, ImageRefError::InvalidCharacter { character: 'C' });
    }

    #[test]
    fn rejects_uppercase_in_path_after_registry_port() {
        let error = ImageRef::new("localhost:5000/Platform/checkout-api:1.2.3").expect_err("image");

        assert_eq!(error, ImageRefError::InvalidCharacter { character: 'P' });
    }

    #[test]
    fn rejects_image_ref_with_empty_component() {
        let error = ImageRef::new("registry.example.com//checkout-api:1.2.3")
            .expect_err("empty path component should be rejected");

        assert_eq!(error, ImageRefError::MissingName);
    }
}
