//! Configuration primitives for future Kply project and cluster settings.

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::{error, fmt};

const DEFAULT_WORKLOAD_KIND: &str = "Deployment";
const POLICY_IMAGE_REGISTRY_MAX_LEN: usize = 253;
const POLICY_IMAGE_REGISTRY_LABEL_MAX_LEN: usize = 63;
const POLICY_DURATION_MAX_LEN: usize = 32;

/// Canonical Kply project configuration filename.
pub const CANONICAL_CONFIG_FILENAME: &str = "kply.yaml";

/// Top-level Kply project configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KplyConfig {
    #[serde(default)]
    version: ConfigVersion,
    #[serde(default)]
    apps: AppConfigs,
    #[serde(default)]
    routing: RoutingConfig,
    #[serde(default)]
    checks: CheckConfigs,
    #[serde(default)]
    policies: PolicyConfigs,
}

impl KplyConfig {
    /// Create a [`KplyConfig`] with explicit top-level sections.
    pub fn new(
        version: ConfigVersion,
        apps: AppConfigs,
        routing: RoutingConfig,
        checks: CheckConfigs,
        policies: PolicyConfigs,
    ) -> Self {
        Self {
            version,
            apps,
            routing,
            checks,
            policies,
        }
    }

    /// Return the config schema [`ConfigVersion`].
    pub const fn version(&self) -> ConfigVersion {
        self.version
    }

    /// Borrow the top-level [`AppConfigs`] section.
    pub const fn apps(&self) -> &AppConfigs {
        &self.apps
    }

    /// Borrow the top-level [`RoutingConfig`] section.
    pub const fn routing(&self) -> &RoutingConfig {
        &self.routing
    }

    /// Borrow the top-level [`CheckConfigs`] section.
    pub const fn checks(&self) -> &CheckConfigs {
        &self.checks
    }

    /// Borrow the top-level [`PolicyConfigs`] section.
    pub const fn policies(&self) -> &PolicyConfigs {
        &self.policies
    }

    /// Validate the config model before it is used by session planning.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigValidationErrors`] when one or more fields are invalid.
    pub fn validate(&self) -> Result<(), ConfigValidationErrors> {
        let mut errors = Vec::new();

        if let Err(error) = self.version.validate() {
            errors.push(ConfigValidationError::UnsupportedVersion(error));
        }

        for (index, app) in self.apps.entries().iter().enumerate() {
            errors.extend(app.validation_errors(index));
        }

        for (index, policy) in self.policies.entries().iter().enumerate() {
            errors.extend(policy.validation_errors(index));
        }

        ConfigValidationErrors::from_errors(errors)
    }
}

impl Default for KplyConfig {
    fn default() -> Self {
        Self::new(
            ConfigVersion::default(),
            AppConfigs::default(),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::default(),
        )
    }
}

/// Top-level config schema version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConfigVersion(u16);

impl ConfigVersion {
    /// Current provisional config schema version.
    pub const CURRENT: Self = Self(1);

    /// Minimum config schema version accepted by this binary.
    pub const MIN_SUPPORTED: Self = Self(1);

    /// Maximum config schema version accepted by this binary.
    pub const MAX_SUPPORTED: Self = Self::CURRENT;

    /// Create a config schema version.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Return the numeric config schema version.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Return true when this version is accepted by this binary.
    pub const fn is_supported(self) -> bool {
        self.0 >= Self::MIN_SUPPORTED.0 && self.0 <= Self::MAX_SUPPORTED.0
    }

    /// Validate that this version is accepted by this binary.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigVersionError::Unsupported`] when the version is outside
    /// the supported range.
    pub const fn validate(self) -> Result<Self, ConfigVersionError> {
        if self.is_supported() {
            Ok(self)
        } else {
            Err(ConfigVersionError::Unsupported {
                found: self,
                min_supported: Self::MIN_SUPPORTED,
                max_supported: Self::MAX_SUPPORTED,
            })
        }
    }
}

impl Default for ConfigVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

impl fmt::Display for ConfigVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.get())
    }
}

impl Serialize for ConfigVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(self.get())
    }
}

/// Error returned when a config schema version cannot be accepted.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigVersionError {
    /// Config schema version is outside this binary's supported range.
    Unsupported {
        /// Version found in the configuration.
        found: ConfigVersion,
        /// Minimum config schema version accepted by this binary.
        min_supported: ConfigVersion,
        /// Maximum config schema version accepted by this binary.
        max_supported: ConfigVersion,
    },
}

impl fmt::Display for ConfigVersionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported {
                found,
                min_supported,
                max_supported,
            } => write!(
                formatter,
                "unsupported config version {found}; supported range is {min_supported}..={max_supported}"
            ),
        }
    }
}

impl error::Error for ConfigVersionError {}

impl<'de> Deserialize<'de> for ConfigVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self::new(u16::deserialize(deserializer)?))
    }
}

/// Non-empty collection of config validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationErrors {
    errors: Vec<ConfigValidationError>,
}

impl ConfigValidationErrors {
    /// Create a validation error collection from one or more errors.
    ///
    /// # Errors
    ///
    /// Returns [`EmptyConfigValidationErrors`] when `errors` is empty.
    pub fn new(errors: Vec<ConfigValidationError>) -> Result<Self, EmptyConfigValidationErrors> {
        if errors.is_empty() {
            Err(EmptyConfigValidationErrors)
        } else {
            Ok(Self { errors })
        }
    }

    /// Create a validation result from a possibly empty error collection.
    fn from_errors(errors: Vec<ConfigValidationError>) -> Result<(), Self> {
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Self { errors })
        }
    }

    /// Borrow validation errors in deterministic discovery order.
    pub fn errors(&self) -> &[ConfigValidationError] {
        &self.errors
    }

    /// Return the number of validation errors.
    #[expect(
        clippy::len_without_is_empty,
        reason = "ConfigValidationErrors cannot be empty by construction"
    )]
    pub fn len(&self) -> usize {
        self.errors.len()
    }
}

impl fmt::Display for ConfigValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // INVARIANT: ConfigValidationErrors::new rejects empty vectors and
        // from_errors only constructs Self when errors is non-empty.
        let (first, remaining) = self
            .errors
            .split_first()
            .expect("invariant: ConfigValidationErrors cannot be empty");

        if remaining.is_empty() {
            write!(formatter, "{first}")
        } else {
            write!(
                formatter,
                "{} config validation errors; first error: {first}",
                self.errors.len()
            )
        }
    }
}

impl error::Error for ConfigValidationErrors {}

/// Error returned when creating an empty validation error collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyConfigValidationErrors;

impl fmt::Display for EmptyConfigValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("config validation error collection cannot be empty")
    }
}

impl error::Error for EmptyConfigValidationErrors {}

/// Single config validation error with field context.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValidationError {
    /// Config schema version is outside the supported range.
    UnsupportedVersion(ConfigVersionError),
    /// An app config field is required but blank.
    EmptyAppField {
        /// Zero-based app index in the top-level apps list.
        app_index: usize,
        /// App field that failed validation.
        field: AppConfigField,
    },
    /// A policy config field is required but blank.
    EmptyPolicyField {
        /// Zero-based policy index in the top-level policies list.
        policy_index: usize,
        /// Policy field that failed validation.
        field: PolicyConfigField,
    },
    /// A policy list entry is required but blank.
    EmptyPolicyListEntry {
        /// Zero-based policy index in the top-level policies list.
        policy_index: usize,
        /// Policy field that failed validation.
        field: PolicyConfigField,
        /// Zero-based entry index in the policy list field.
        entry_index: usize,
    },
    /// A policy list contains the same value more than once.
    DuplicatePolicyListEntry {
        /// Zero-based policy index in the top-level policies list.
        policy_index: usize,
        /// Policy field that failed validation.
        field: PolicyConfigField,
        /// Duplicate list value.
        value: String,
    },
    /// A policy duration field is present but invalid.
    InvalidPolicyDuration {
        /// Zero-based policy index in the top-level policies list.
        policy_index: usize,
        /// Policy field that failed validation.
        field: PolicyConfigField,
        /// Duration validation failure.
        reason: PolicyDurationError,
    },
    /// A policy image registry allowlist value is invalid.
    InvalidPolicyImageRegistry {
        /// Zero-based policy index in the top-level policies list.
        policy_index: usize,
        /// Policy field that failed validation.
        field: PolicyConfigField,
        /// Registry value that failed validation.
        value: String,
        /// Registry validation failure.
        reason: PolicyImageRegistryError,
    },
}

impl fmt::Display for ConfigValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(error) => write!(formatter, "version: {error}"),
            Self::EmptyAppField { app_index, field } => {
                write!(formatter, "apps[{app_index}].{field}: field is required")
            }
            Self::EmptyPolicyField {
                policy_index,
                field,
            } => {
                write!(
                    formatter,
                    "policies[{policy_index}].{field}: field is required"
                )
            }
            Self::EmptyPolicyListEntry {
                policy_index,
                field,
                entry_index,
            } => {
                write!(
                    formatter,
                    "policies[{policy_index}].{field}[{entry_index}]: field is required"
                )
            }
            Self::DuplicatePolicyListEntry {
                policy_index,
                field,
                value,
            } => {
                write!(
                    formatter,
                    "policies[{policy_index}].{field}: duplicate value `{value}`"
                )
            }
            Self::InvalidPolicyDuration {
                policy_index,
                field,
                reason,
            } => {
                write!(formatter, "policies[{policy_index}].{field}: {reason}")
            }
            Self::InvalidPolicyImageRegistry {
                policy_index,
                field,
                value,
                reason,
            } => {
                write!(
                    formatter,
                    "policies[{policy_index}].{field}: invalid value `{value}`: {reason}"
                )
            }
        }
    }
}

impl error::Error for ConfigValidationError {}

/// Error returned when a policy duration value is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDurationError {
    /// Duration values cannot be empty.
    Empty,
    /// Duration values cannot exceed the maximum length.
    TooLong { max_len: usize },
    /// Duration values must end with a supported unit.
    InvalidUnit { unit: char },
    /// Duration values must start with ASCII digits.
    InvalidNumber,
    /// Duration values must be greater than zero.
    Zero,
}

impl fmt::Display for PolicyDurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("duration cannot be empty"),
            Self::TooLong { max_len } => {
                write!(formatter, "duration cannot exceed {max_len} characters")
            }
            Self::InvalidUnit { unit } => {
                write!(
                    formatter,
                    "invalid duration unit `{unit}`; expected s, m, h, or d"
                )
            }
            Self::InvalidNumber => {
                formatter.write_str("invalid duration; expected a positive integer duration")
            }
            Self::Zero => formatter.write_str("invalid duration; value must be greater than zero"),
        }
    }
}

impl error::Error for PolicyDurationError {}

/// Error returned when a policy image registry value is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyImageRegistryError {
    /// Registry values cannot exceed the maximum length.
    TooLong { max_len: usize },
    /// Registry values must start and end with an ASCII lowercase letter or digit.
    InvalidBoundary,
    /// Registry host values cannot contain empty labels.
    EmptyLabel,
    /// Registry host labels cannot exceed the maximum length.
    LabelTooLong { max_len: usize },
    /// Registry values only allow lowercase host characters and an optional port.
    InvalidCharacter { character: char },
    /// Registry values must use a numeric non-zero port when a port is present.
    InvalidPort,
}

impl fmt::Display for PolicyImageRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong { max_len } => {
                write!(formatter, "registry cannot exceed {max_len} characters")
            }
            Self::InvalidBoundary => formatter
                .write_str("registry must start and end with a lowercase ASCII letter or digit"),
            Self::EmptyLabel => formatter.write_str("registry labels cannot be empty"),
            Self::LabelTooLong { max_len } => {
                write!(
                    formatter,
                    "registry labels cannot exceed {max_len} characters"
                )
            }
            Self::InvalidCharacter { character } => {
                write!(
                    formatter,
                    "registry contains invalid character `{character}`"
                )
            }
            Self::InvalidPort => formatter.write_str("registry port must be a non-zero number"),
        }
    }
}

impl error::Error for PolicyImageRegistryError {}

/// Field name for an app config validation error.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AppConfigField {
    /// App `name` field.
    Name,
    /// App `namespace` field.
    Namespace,
    /// App `workload` field.
    Workload,
    /// App `workload_kind` field.
    WorkloadKind,
    /// App `service` field.
    Service,
    /// App `default_image` field.
    DefaultImage,
}

impl AppConfigField {
    /// Return the stable config spelling for this [`AppConfigField`].
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Namespace => "namespace",
            Self::Workload => "workload",
            Self::WorkloadKind => "workload_kind",
            Self::Service => "service",
            Self::DefaultImage => "default_image",
        }
    }
}

impl fmt::Display for AppConfigField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Field name for a policy config validation error.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PolicyConfigField {
    /// Policy `name` field.
    Name,
    /// Policy `description` field.
    Description,
    /// Policy `allowed_namespaces` field.
    AllowedNamespaces,
    /// Policy `allowed_workload_kinds` field.
    AllowedWorkloadKinds,
    /// Policy `allowed_image_registries` field.
    AllowedImageRegistries,
    /// Policy `allowed_route_strategies` field.
    AllowedRouteStrategies,
    /// Policy `max_session_ttl` field.
    MaxSessionTtl,
    /// Policy `mutation_mode` field.
    MutationMode,
    /// Policy `secret_handling` field.
    SecretHandling,
}

impl PolicyConfigField {
    /// Return the stable config spelling for this [`PolicyConfigField`].
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Description => "description",
            Self::AllowedNamespaces => "allowed_namespaces",
            Self::AllowedWorkloadKinds => "allowed_workload_kinds",
            Self::AllowedImageRegistries => "allowed_image_registries",
            Self::AllowedRouteStrategies => "allowed_route_strategies",
            Self::MaxSessionTtl => "max_session_ttl",
            Self::MutationMode => "mutation_mode",
            Self::SecretHandling => "secret_handling",
        }
    }
}

impl fmt::Display for PolicyConfigField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Top-level application config collection.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct AppConfigs {
    entries: Vec<AppConfig>,
}

impl AppConfigs {
    /// Create an application config collection.
    pub fn new(entries: Vec<AppConfig>) -> Self {
        Self { entries }
    }

    /// Borrow configured application entries.
    pub fn entries(&self) -> &[AppConfig] {
        &self.entries
    }

    /// Return true when no apps are configured.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Serialize for AppConfigs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.entries.len()))?;
        for entry in &self.entries {
            sequence.serialize_element(entry)?;
        }
        sequence.end()
    }
}

/// Application target configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    name: String,
    namespace: String,
    workload: String,
    #[serde(default = "default_workload_kind")]
    workload_kind: String,
    service: String,
    #[serde(default)]
    default_image: Option<String>,
    route_strategy: RouteStrategy,
}

impl AppConfig {
    /// Create an [`AppConfig`] from explicit app fields.
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        workload: impl Into<String>,
        service: impl Into<String>,
        default_image: Option<String>,
        route_strategy: RouteStrategy,
    ) -> Self {
        Self {
            name: name.into(),
            namespace: namespace.into(),
            workload: workload.into(),
            workload_kind: default_workload_kind(),
            service: service.into(),
            default_image,
            route_strategy,
        }
    }

    /// Borrow the configured app name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Borrow the Kubernetes namespace for the app.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Borrow the Kubernetes workload name for the app.
    pub fn workload(&self) -> &str {
        &self.workload
    }

    /// Borrow the Kubernetes workload kind for the app.
    pub fn workload_kind(&self) -> &str {
        &self.workload_kind
    }

    /// Return a copy of this app config with an explicit workload kind.
    pub fn with_workload_kind(mut self, workload_kind: impl Into<String>) -> Self {
        self.workload_kind = workload_kind.into();
        self
    }

    /// Borrow the Kubernetes service name for the app.
    pub fn service(&self) -> &str {
        &self.service
    }

    /// Borrow the optional default sandbox image for the app.
    pub fn default_image(&self) -> Option<&str> {
        self.default_image.as_deref()
    }

    /// Return the configured [`RouteStrategy`] for the app.
    pub const fn route_strategy(&self) -> RouteStrategy {
        self.route_strategy
    }

    fn validation_errors(&self, app_index: usize) -> Vec<ConfigValidationError> {
        let mut errors = Vec::new();

        push_empty_app_field_error(&mut errors, app_index, AppConfigField::Name, self.name());
        push_empty_app_field_error(
            &mut errors,
            app_index,
            AppConfigField::Namespace,
            self.namespace(),
        );
        push_empty_app_field_error(
            &mut errors,
            app_index,
            AppConfigField::Workload,
            self.workload(),
        );
        push_empty_app_field_error(
            &mut errors,
            app_index,
            AppConfigField::WorkloadKind,
            self.workload_kind(),
        );
        push_empty_app_field_error(
            &mut errors,
            app_index,
            AppConfigField::Service,
            self.service(),
        );

        if let Some(default_image) = self.default_image() {
            push_empty_app_field_error(
                &mut errors,
                app_index,
                AppConfigField::DefaultImage,
                default_image,
            );
        }

        errors
    }
}

fn default_workload_kind() -> String {
    DEFAULT_WORKLOAD_KIND.to_owned()
}

fn push_empty_app_field_error(
    errors: &mut Vec<ConfigValidationError>,
    app_index: usize,
    field: AppConfigField,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(ConfigValidationError::EmptyAppField { app_index, field });
    }
}

/// Routing strategy requested for an application target.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStrategy {
    /// Route sandbox traffic by matching a request header.
    Header,
    /// Route sandbox traffic by matching a host name.
    Host,
    /// Expose sandbox traffic through a preview endpoint.
    Preview,
}

impl RouteStrategy {
    /// Return the stable config spelling for this [`RouteStrategy`].
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Host => "host",
            Self::Preview => "preview",
        }
    }
}

/// Mutation scope allowed by a policy entry.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MutationModePolicy {
    /// Allow read-only planning and inspection only.
    ReadOnly,
    /// Allow mutation only for sandbox-owned resources.
    SandboxOnly,
    /// Allow sandbox resources and route mutation for isolated test traffic.
    RouteMutation,
}

impl MutationModePolicy {
    /// Return the stable config spelling for this [`MutationModePolicy`].
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::SandboxOnly => "sandbox-only",
            Self::RouteMutation => "route-mutation",
        }
    }
}

/// Secret reference handling allowed by a policy entry.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecretHandlingPolicy {
    /// Allow Secret names and reference metadata only.
    MetadataOnly,
    /// Treat Secret references as denied for future planning and mutation.
    DenyReferences,
}

impl SecretHandlingPolicy {
    /// Return the stable config spelling for this [`SecretHandlingPolicy`].
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MetadataOnly => "metadata-only",
            Self::DenyReferences => "deny-references",
        }
    }
}

/// Top-level routing config section.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RoutingConfig;

impl<'de> Deserialize<'de> for RoutingConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(RoutingConfigVisitor)
    }
}

struct RoutingConfigVisitor;

impl<'de> serde::de::Visitor<'de> for RoutingConfigVisitor {
    type Value = RoutingConfig;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an empty routing config object")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        if let Some(key) = access.next_key::<String>()? {
            return Err(serde::de::Error::unknown_field(&key, &[]));
        }

        Ok(RoutingConfig)
    }
}

impl Serialize for RoutingConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_map(Some(0))?.end()
    }
}

/// Top-level check config collection.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct CheckConfigs {
    entries: Vec<CheckConfig>,
}

impl CheckConfigs {
    /// Create a check config collection.
    pub fn new(entries: Vec<CheckConfig>) -> Self {
        Self { entries }
    }

    /// Borrow configured check entries.
    pub fn entries(&self) -> &[CheckConfig] {
        &self.entries
    }

    /// Return true when no checks are configured.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Serialize for CheckConfigs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.entries.len()))?;
        for entry in &self.entries {
            sequence.serialize_element(entry)?;
        }
        sequence.end()
    }
}

/// Placeholder for a future check config entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckConfig;

/// Top-level policy config collection.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct PolicyConfigs {
    entries: Vec<PolicyConfig>,
}

impl PolicyConfigs {
    /// Create a policy config collection.
    pub fn new(entries: Vec<PolicyConfig>) -> Self {
        Self { entries }
    }

    /// Borrow configured policy entries.
    pub fn entries(&self) -> &[PolicyConfig] {
        &self.entries
    }

    /// Return true when no policies are configured.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Serialize for PolicyConfigs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.entries.len()))?;
        for entry in &self.entries {
            sequence.serialize_element(entry)?;
        }
        sequence.end()
    }
}

/// Named policy config entry for future safety enforcement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    name: String,
    #[serde(default = "default_policy_enabled")]
    enabled: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    allowed_namespaces: Vec<String>,
    #[serde(default)]
    allowed_workload_kinds: Vec<String>,
    #[serde(default)]
    allowed_image_registries: Vec<String>,
    #[serde(default)]
    allowed_route_strategies: Vec<RouteStrategy>,
    #[serde(default)]
    max_session_ttl: Option<String>,
    #[serde(default)]
    mutation_mode: Option<MutationModePolicy>,
    #[serde(default)]
    secret_handling: Option<SecretHandlingPolicy>,
}

impl PolicyConfig {
    /// Create an enabled named policy config entry.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: default_policy_enabled(),
            description: None,
            allowed_namespaces: Vec::new(),
            allowed_workload_kinds: Vec::new(),
            allowed_image_registries: Vec::new(),
            allowed_route_strategies: Vec::new(),
            max_session_ttl: None,
            mutation_mode: None,
            secret_handling: None,
        }
    }

    /// Borrow the stable policy name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return whether this policy entry is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Return a copy of this policy entry with an explicit enabled flag.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Borrow the optional human-readable policy description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Return a copy of this policy entry with a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Borrow the Kubernetes namespaces this policy will allow.
    pub fn allowed_namespaces(&self) -> &[String] {
        &self.allowed_namespaces
    }

    /// Return a copy of this policy entry with allowed namespaces.
    pub fn with_allowed_namespaces(
        mut self,
        allowed_namespaces: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.allowed_namespaces = allowed_namespaces
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        self
    }

    /// Borrow the Kubernetes workload kinds this policy will allow.
    pub fn allowed_workload_kinds(&self) -> &[String] {
        &self.allowed_workload_kinds
    }

    /// Return a copy of this policy entry with allowed workload kinds.
    pub fn with_allowed_workload_kinds(
        mut self,
        allowed_workload_kinds: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.allowed_workload_kinds = allowed_workload_kinds
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        self
    }

    /// Borrow the image registries this policy will allow.
    pub fn allowed_image_registries(&self) -> &[String] {
        &self.allowed_image_registries
    }

    /// Return a copy of this policy entry with allowed image registries.
    pub fn with_allowed_image_registries(
        mut self,
        allowed_image_registries: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.allowed_image_registries = allowed_image_registries
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        self
    }

    /// Borrow the route strategies this policy will allow.
    pub fn allowed_route_strategies(&self) -> &[RouteStrategy] {
        &self.allowed_route_strategies
    }

    /// Return a copy of this policy entry with allowed route strategies.
    pub fn with_allowed_route_strategies(
        mut self,
        allowed_route_strategies: impl IntoIterator<Item = RouteStrategy>,
    ) -> Self {
        self.allowed_route_strategies = allowed_route_strategies.into_iter().collect::<Vec<_>>();
        self
    }

    /// Borrow the maximum session TTL this policy will allow.
    pub fn max_session_ttl(&self) -> Option<&str> {
        self.max_session_ttl.as_deref()
    }

    /// Return a copy of this policy entry with a maximum session TTL.
    pub fn with_max_session_ttl(mut self, max_session_ttl: impl Into<String>) -> Self {
        self.max_session_ttl = Some(max_session_ttl.into());
        self
    }

    /// Return the mutation mode this policy will allow.
    pub const fn mutation_mode(&self) -> Option<MutationModePolicy> {
        self.mutation_mode
    }

    /// Return a copy of this policy entry with a mutation mode.
    pub fn with_mutation_mode(mut self, mutation_mode: MutationModePolicy) -> Self {
        self.mutation_mode = Some(mutation_mode);
        self
    }

    /// Return the secret handling behavior this policy will allow.
    pub const fn secret_handling(&self) -> Option<SecretHandlingPolicy> {
        self.secret_handling
    }

    /// Return a copy of this policy entry with secret handling behavior.
    pub fn with_secret_handling(mut self, secret_handling: SecretHandlingPolicy) -> Self {
        self.secret_handling = Some(secret_handling);
        self
    }

    /// Return validation errors for this policy entry with top-level index context.
    fn validation_errors(&self, policy_index: usize) -> Vec<ConfigValidationError> {
        let mut errors = Vec::new();

        push_empty_policy_field_error(
            &mut errors,
            policy_index,
            PolicyConfigField::Name,
            self.name(),
        );

        if let Some(description) = self.description() {
            push_empty_policy_field_error(
                &mut errors,
                policy_index,
                PolicyConfigField::Description,
                description,
            );
        }

        push_policy_list_entry_errors(
            &mut errors,
            policy_index,
            PolicyConfigField::AllowedNamespaces,
            self.allowed_namespaces(),
        );
        push_policy_list_entry_errors(
            &mut errors,
            policy_index,
            PolicyConfigField::AllowedWorkloadKinds,
            self.allowed_workload_kinds(),
        );
        push_policy_list_entry_errors(
            &mut errors,
            policy_index,
            PolicyConfigField::AllowedImageRegistries,
            self.allowed_image_registries(),
        );
        push_policy_image_registry_errors(
            &mut errors,
            policy_index,
            self.allowed_image_registries(),
        );
        push_policy_route_strategy_errors(
            &mut errors,
            policy_index,
            self.allowed_route_strategies(),
        );
        if let Some(max_session_ttl) = self.max_session_ttl() {
            push_policy_duration_error(
                &mut errors,
                policy_index,
                PolicyConfigField::MaxSessionTtl,
                max_session_ttl,
            );
        }

        errors
    }
}

/// Return the default enabled flag for policy entries.
fn default_policy_enabled() -> bool {
    true
}

/// Push a field-scoped error when a scalar policy field is blank.
fn push_empty_policy_field_error(
    errors: &mut Vec<ConfigValidationError>,
    policy_index: usize,
    field: PolicyConfigField,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(ConfigValidationError::EmptyPolicyField {
            policy_index,
            field,
        });
    }
}

/// Push field-scoped errors for blank or duplicate policy list values.
fn push_policy_list_entry_errors(
    errors: &mut Vec<ConfigValidationError>,
    policy_index: usize,
    field: PolicyConfigField,
    values: &[String],
) {
    let mut seen = std::collections::BTreeSet::<String>::new();

    for (entry_index, value) in values.iter().enumerate() {
        let normalized_value = value.trim();
        if normalized_value.is_empty() {
            errors.push(ConfigValidationError::EmptyPolicyListEntry {
                policy_index,
                field,
                entry_index,
            });
        } else if !seen.insert(normalized_value.to_owned()) {
            errors.push(ConfigValidationError::DuplicatePolicyListEntry {
                policy_index,
                field,
                value: normalized_value.to_owned(),
            });
        }
    }
}

/// Push field-scoped errors for duplicate policy route strategies.
fn push_policy_route_strategy_errors(
    errors: &mut Vec<ConfigValidationError>,
    policy_index: usize,
    values: &[RouteStrategy],
) {
    let mut seen = std::collections::BTreeSet::<RouteStrategy>::new();

    for value in values {
        if !seen.insert(*value) {
            errors.push(ConfigValidationError::DuplicatePolicyListEntry {
                policy_index,
                field: PolicyConfigField::AllowedRouteStrategies,
                value: value.as_str().to_owned(),
            });
        }
    }
}

/// Push field-scoped errors for invalid policy image registries.
fn push_policy_image_registry_errors(
    errors: &mut Vec<ConfigValidationError>,
    policy_index: usize,
    values: &[String],
) {
    for value in values {
        let normalized_value = value.trim();
        if normalized_value.is_empty() {
            continue;
        }

        if let Err(reason) = validate_policy_image_registry(normalized_value) {
            errors.push(ConfigValidationError::InvalidPolicyImageRegistry {
                policy_index,
                field: PolicyConfigField::AllowedImageRegistries,
                value: normalized_value.to_owned(),
                reason,
            });
        }
    }
}

/// Push a field-scoped error when a policy duration is invalid.
fn push_policy_duration_error(
    errors: &mut Vec<ConfigValidationError>,
    policy_index: usize,
    field: PolicyConfigField,
    value: &str,
) {
    if let Err(reason) = validate_policy_duration(value) {
        errors.push(ConfigValidationError::InvalidPolicyDuration {
            policy_index,
            field,
            reason,
        });
    }
}

/// Validate compact policy durations using the session TTL grammar.
fn validate_policy_duration(value: &str) -> Result<(), PolicyDurationError> {
    if value.is_empty() {
        return Err(PolicyDurationError::Empty);
    }

    if value.len() > POLICY_DURATION_MAX_LEN {
        return Err(PolicyDurationError::TooLong {
            max_len: POLICY_DURATION_MAX_LEN,
        });
    }

    let unit = value.chars().last().ok_or(PolicyDurationError::Empty)?;
    if !matches!(unit, 's' | 'm' | 'h' | 'd') {
        return Err(PolicyDurationError::InvalidUnit { unit });
    }

    let digits = &value[..value.len() - unit.len_utf8()];
    if digits.is_empty() || !digits.chars().all(|character| character.is_ascii_digit()) {
        return Err(PolicyDurationError::InvalidNumber);
    }

    if digits.trim_start_matches('0').is_empty() {
        return Err(PolicyDurationError::Zero);
    }

    Ok(())
}

/// Validate image registry host values with an optional numeric port.
fn validate_policy_image_registry(value: &str) -> Result<(), PolicyImageRegistryError> {
    if value.len() > POLICY_IMAGE_REGISTRY_MAX_LEN {
        return Err(PolicyImageRegistryError::TooLong {
            max_len: POLICY_IMAGE_REGISTRY_MAX_LEN,
        });
    }

    if let Some(character) = value
        .chars()
        .find(|character| !is_policy_image_registry_character(*character))
    {
        return Err(PolicyImageRegistryError::InvalidCharacter { character });
    }

    let (host, port) = match value.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (value, None),
    };

    if let Some(port) = port
        && (port.is_empty()
            || !port.chars().all(|character| character.is_ascii_digit())
            || port.trim_start_matches('0').is_empty())
    {
        return Err(PolicyImageRegistryError::InvalidPort);
    }

    if host.contains(':') {
        return Err(PolicyImageRegistryError::InvalidCharacter { character: ':' });
    }

    if host.is_empty() {
        return Err(PolicyImageRegistryError::InvalidBoundary);
    }

    let mut characters = host.chars();
    let first_character = characters
        .next()
        .ok_or(PolicyImageRegistryError::InvalidBoundary)?;
    let last_character = characters.next_back().unwrap_or(first_character);

    if !is_policy_image_registry_boundary(first_character)
        || !is_policy_image_registry_boundary(last_character)
    {
        return Err(PolicyImageRegistryError::InvalidBoundary);
    }

    for label in host.split('.') {
        if label.is_empty() {
            return Err(PolicyImageRegistryError::EmptyLabel);
        }

        if label.len() > POLICY_IMAGE_REGISTRY_LABEL_MAX_LEN {
            return Err(PolicyImageRegistryError::LabelTooLong {
                max_len: POLICY_IMAGE_REGISTRY_LABEL_MAX_LEN,
            });
        }

        let mut label_characters = label.chars();
        let label_first_character = label_characters
            .next()
            .ok_or(PolicyImageRegistryError::EmptyLabel)?;
        let label_last_character = label_characters
            .next_back()
            .unwrap_or(label_first_character);

        if !is_policy_image_registry_boundary(label_first_character)
            || !is_policy_image_registry_boundary(label_last_character)
        {
            return Err(PolicyImageRegistryError::InvalidBoundary);
        }
    }

    Ok(())
}

/// Return true for characters allowed in policy image registry values.
fn is_policy_image_registry_character(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || matches!(character, '-' | '.' | ':')
}

/// Return true for policy image registry label boundary characters.
fn is_policy_image_registry_boundary(character: char) -> bool {
    character.is_ascii_lowercase() || character.is_ascii_digit()
}

/// Load a Kply project configuration file from disk.
///
/// # Errors
///
/// Returns [`ConfigLoadError`] when the file cannot be read or parsed.
pub fn load_config_path(path: impl AsRef<Path>) -> Result<KplyConfig, ConfigLoadError> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path).map_err(|source| ConfigLoadError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_norway::from_str(&contents).map_err(|source| ConfigLoadError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

/// Error returned when loading a project configuration fails.
#[non_exhaustive]
#[derive(Debug)]
pub enum ConfigLoadError {
    /// Config file could not be read.
    Read {
        /// Config file path that failed to read.
        path: PathBuf,
        /// Underlying filesystem read error.
        source: io::Error,
    },
    /// Config file could not be parsed as YAML.
    Parse {
        /// Config file path that failed to parse.
        path: PathBuf,
        /// Underlying YAML parse error.
        source: serde_norway::Error,
    },
}

impl fmt::Display for ConfigLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read config file `{}`: {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                write!(
                    formatter,
                    "failed to parse config file `{}`: {source}",
                    path.display()
                )
            }
        }
    }
}

impl error::Error for ConfigLoadError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}

/// Discover the nearest Kply project configuration from the current directory.
pub fn discover_config_path() -> io::Result<Option<PathBuf>> {
    let current_dir = std::env::current_dir()?;
    Ok(discover_config_path_from(current_dir))
}

/// Discover the nearest Kply project configuration from `start` upward.
pub fn discover_config_path_from(start: impl AsRef<Path>) -> Option<PathBuf> {
    start
        .as_ref()
        .ancestors()
        .map(|directory| directory.join(CANONICAL_CONFIG_FILENAME))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::{
        AppConfig, AppConfigField, AppConfigs, CANONICAL_CONFIG_FILENAME, CheckConfig,
        CheckConfigs, ConfigLoadError, ConfigValidationError, ConfigValidationErrors,
        ConfigVersion, ConfigVersionError, EmptyConfigValidationErrors, KplyConfig,
        MutationModePolicy, PolicyConfig, PolicyConfigField, PolicyConfigs, PolicyDurationError,
        PolicyImageRegistryError, RouteStrategy, RoutingConfig, SecretHandlingPolicy,
        discover_config_path_from, load_config_path,
    };
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());
    const VALID_CONFIG_FIXTURES: &[&str] = &[
        "minimal-defaults",
        "complete-single-app",
        "multi-app-route-strategies",
        "policy-baseline",
    ];
    const INVALID_VALIDATION_CONFIG_FIXTURES: &[(&str, usize)] = &[
        ("invalid-empty-app-fields", 4),
        ("invalid-unsupported-version", 1),
    ];
    const INVALID_LOAD_CONFIG_FIXTURES: &[&str] = &[
        "invalid-unknown-top-level-field",
        "invalid-unknown-routing-field",
    ];

    #[test]
    fn uses_kply_yaml_as_canonical_config_filename() {
        assert_eq!(CANONICAL_CONFIG_FILENAME, "kply.yaml");
    }

    #[test]
    fn creates_top_level_config_with_explicit_sections() {
        let config = KplyConfig::new(
            ConfigVersion::new(7),
            AppConfigs::new(vec![app_config()]),
            RoutingConfig,
            CheckConfigs::new(vec![CheckConfig]),
            PolicyConfigs::new(vec![policy_config()]),
        );

        assert_eq!(config.version().get(), 7);
        assert_eq!(config.apps().entries(), &[app_config()]);
        assert_eq!(config.routing(), &RoutingConfig);
        assert_eq!(config.checks().entries(), &[CheckConfig]);
        assert_eq!(config.policies().entries(), &[policy_config()]);
    }

    #[test]
    fn exposes_current_supported_schema_version_range() {
        assert_eq!(ConfigVersion::MIN_SUPPORTED.get(), 1);
        assert_eq!(ConfigVersion::MAX_SUPPORTED, ConfigVersion::CURRENT);
        assert!(ConfigVersion::CURRENT.is_supported());
    }

    #[test]
    fn validates_supported_schema_versions() {
        assert_eq!(
            ConfigVersion::CURRENT.validate(),
            Ok(ConfigVersion::CURRENT)
        );
    }

    #[test]
    fn rejects_unsupported_schema_versions() {
        let version = ConfigVersion::new(ConfigVersion::MAX_SUPPORTED.get() + 1);

        assert!(!version.is_supported());
        assert_eq!(
            version.validate(),
            Err(ConfigVersionError::Unsupported {
                found: version,
                min_supported: ConfigVersion::MIN_SUPPORTED,
                max_supported: ConfigVersion::MAX_SUPPORTED,
            })
        );
        assert_eq!(
            version
                .validate()
                .expect_err("unsupported version")
                .to_string(),
            "unsupported config version 2; supported range is 1..=1"
        );
    }

    #[test]
    fn rejects_schema_versions_below_supported_range() {
        let version = ConfigVersion::new(0);

        assert!(!version.is_supported());
        assert_eq!(
            version.validate(),
            Err(ConfigVersionError::Unsupported {
                found: version,
                min_supported: ConfigVersion::MIN_SUPPORTED,
                max_supported: ConfigVersion::MAX_SUPPORTED,
            })
        );
    }

    #[test]
    fn validates_complete_config_model() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::new(vec![app_config()]),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::default(),
        );

        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn serializes_resolved_config_to_stable_json() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::new(vec![app_config()]),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::default(),
        );

        let value = serde_json::to_value(config).expect("resolved config JSON");

        assert_eq!(
            value,
            serde_json::json!({
                "version": 1,
                "apps": [
                    {
                        "name": "checkout",
                        "namespace": "shop",
                        "workload": "checkout-api",
                        "workload_kind": "Deployment",
                        "service": "checkout-http",
                        "default_image": "registry.example.com/shop/checkout:test",
                        "route_strategy": "header",
                    }
                ],
                "routing": {},
                "checks": [],
                "policies": [],
            })
        );
    }

    #[test]
    fn serializes_missing_default_image_as_null() {
        let config = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            None,
            RouteStrategy::Preview,
        );

        let value = serde_json::to_value(config).expect("app config JSON");

        assert_eq!(
            value,
            serde_json::json!({
                "name": "checkout",
                "namespace": "shop",
                "workload": "checkout-api",
                "workload_kind": "Deployment",
                "service": "checkout-http",
                "default_image": null,
                "route_strategy": "preview",
            })
        );
    }

    #[test]
    fn deserializes_valid_config_yaml() {
        let config: KplyConfig = serde_norway::from_str(
            r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    default_image: registry.example.com/shop/checkout:test
    route_strategy: header
checks: []
policies: []
"#,
        )
        .expect("valid config YAML");

        assert_eq!(config.version(), ConfigVersion::CURRENT);
        assert_eq!(config.apps().entries(), &[app_config()]);
        assert!(config.checks().is_empty());
        assert!(config.policies().is_empty());
    }

    #[test]
    fn deserializes_config_yaml_with_defaulted_sections() {
        let config: KplyConfig = serde_norway::from_str(
            r#"
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    route_strategy: preview
"#,
        )
        .expect("config YAML with defaults");

        assert_eq!(config.version(), ConfigVersion::CURRENT);
        assert_eq!(config.apps().entries()[0].default_image(), None);
        assert_eq!(
            config.apps().entries()[0].route_strategy(),
            RouteStrategy::Preview
        );
        assert!(config.checks().is_empty());
        assert!(config.policies().is_empty());
    }

    #[test]
    fn rejects_unknown_config_yaml_fields() {
        let error = serde_norway::from_str::<KplyConfig>(
            r#"
version: 1
unexpected: true
"#,
        )
        .expect_err("unknown field should be rejected");

        assert!(
            error.to_string().contains("unknown field"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_unknown_routing_config_yaml_fields() {
        let error = serde_norway::from_str::<KplyConfig>(
            r#"
version: 1
routing:
  mode: gateway
"#,
        )
        .expect_err("unknown routing field should be rejected");

        assert!(
            error.to_string().contains("unknown field"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn loads_config_from_file() {
        let workspace = TempDir::new().expect("temporary workspace");
        let config_path = write_config_contents(
            workspace.path(),
            r#"
version: 1
apps:
  - name: checkout
    namespace: shop
    workload: checkout-api
    service: checkout-http
    route_strategy: host
"#,
        );

        let config = load_config_path(config_path).expect("load config");

        assert_eq!(
            config.apps().entries()[0].route_strategy(),
            RouteStrategy::Host
        );
    }

    #[test]
    fn loads_valid_config_fixtures() {
        for fixture_name in VALID_CONFIG_FIXTURES {
            let config_path = kply_test::fixture_path(format!("config/{fixture_name}/kply.yaml"));

            let config = load_config_path(&config_path)
                .unwrap_or_else(|error| panic!("fixture {fixture_name} should load: {error}"));

            config
                .validate()
                .unwrap_or_else(|error| panic!("fixture {fixture_name} should validate: {error}"));
        }
    }

    #[test]
    fn validates_multi_app_route_strategy_fixture_shape() {
        let config = load_config_path(kply_test::fixture_path(
            "config/multi-app-route-strategies/kply.yaml",
        ))
        .expect("multi-app fixture should load");

        let route_strategies = config
            .apps()
            .entries()
            .iter()
            .map(AppConfig::route_strategy)
            .collect::<Vec<_>>();

        assert_eq!(
            route_strategies,
            vec![
                RouteStrategy::Header,
                RouteStrategy::Host,
                RouteStrategy::Preview
            ]
        );
    }

    #[test]
    fn validates_minimal_config_fixture_defaults() {
        let config = load_config_path(kply_test::fixture_path("config/minimal-defaults/kply.yaml"))
            .expect("minimal fixture should load");

        assert_eq!(config.version(), ConfigVersion::CURRENT);
        assert!(config.apps().is_empty());
        assert!(config.checks().is_empty());
        assert!(config.policies().is_empty());
        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn rejects_invalid_config_validation_fixtures() {
        for (fixture_name, expected_error_count) in INVALID_VALIDATION_CONFIG_FIXTURES {
            let config_path = kply_test::fixture_path(format!("config/{fixture_name}/kply.yaml"));
            let config = load_config_path(&config_path)
                .unwrap_or_else(|error| panic!("fixture {fixture_name} should load: {error}"));

            let errors = config
                .validate()
                .expect_err("fixture should fail validation");

            assert_eq!(
                errors.len(),
                *expected_error_count,
                "fixture {fixture_name} should report the expected validation error count"
            );
        }
    }

    #[test]
    fn rejects_invalid_config_load_fixtures() {
        for fixture_name in INVALID_LOAD_CONFIG_FIXTURES {
            let config_path = kply_test::fixture_path(format!("config/{fixture_name}/kply.yaml"));

            let error = load_config_path(&config_path).expect_err("fixture should fail to load");

            assert!(
                matches!(error, ConfigLoadError::Parse { .. }),
                "fixture {fixture_name} should fail during parsing"
            );
        }
    }

    #[test]
    fn snapshots_invalid_config_validation_messages() {
        let config = load_config_path(kply_test::fixture_path(
            "config/invalid-empty-app-fields/kply.yaml",
        ))
        .expect("invalid validation fixture should load");
        let errors = config
            .validate()
            .expect_err("fixture should fail validation");
        let messages = errors
            .errors()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");

        kply_test::insta::assert_snapshot!("invalid_config_validation_messages", messages);
    }

    #[test]
    fn snapshots_unsupported_config_version_message() {
        let config = load_config_path(kply_test::fixture_path(
            "config/invalid-unsupported-version/kply.yaml",
        ))
        .expect("unsupported version fixture should load");
        let errors = config
            .validate()
            .expect_err("fixture should fail validation");

        kply_test::insta::assert_snapshot!(
            "unsupported_config_version_message",
            errors.to_string()
        );
    }

    #[test]
    fn snapshots_config_load_parse_message() {
        let config_path =
            kply_test::fixture_path("config/invalid-unknown-top-level-field/kply.yaml");
        let config_path_text = config_path
            .to_str()
            .expect("fixture path should be valid UTF-8");
        let error = load_config_path(&config_path).expect_err("fixture should fail to load");
        let message = error.to_string().replace(config_path_text, "<config-path>");

        kply_test::insta::assert_snapshot!("config_load_parse_message", message);
    }

    #[test]
    fn reports_config_load_read_errors() {
        let workspace = TempDir::new().expect("temporary workspace");
        let error = load_config_path(workspace.path().join("missing.yaml"))
            .expect_err("missing config should fail");

        assert!(matches!(error, ConfigLoadError::Read { .. }));
    }

    #[test]
    fn reports_config_load_parse_errors() {
        let workspace = TempDir::new().expect("temporary workspace");
        let config_path = write_config_contents(workspace.path(), "version: [");

        let error = load_config_path(config_path).expect_err("malformed config should fail");

        assert!(matches!(error, ConfigLoadError::Parse { .. }));
    }

    #[test]
    fn validates_app_config_without_default_image() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::new(vec![AppConfig::new(
                "checkout",
                "shop",
                "checkout-api",
                "checkout-http",
                None,
                RouteStrategy::Header,
            )]),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::default(),
        );

        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn rejects_empty_config_validation_error_collection() {
        assert_eq!(
            ConfigValidationErrors::new(Vec::new()),
            Err(EmptyConfigValidationErrors)
        );
    }

    #[test]
    fn reports_unsupported_config_version_with_field_context() {
        let version = ConfigVersion::new(2);
        let config = KplyConfig::new(
            version,
            AppConfigs::default(),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::default(),
        );

        let errors = config.validate().expect_err("unsupported version");

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors.errors(),
            &[ConfigValidationError::UnsupportedVersion(
                ConfigVersionError::Unsupported {
                    found: version,
                    min_supported: ConfigVersion::MIN_SUPPORTED,
                    max_supported: ConfigVersion::MAX_SUPPORTED,
                }
            )]
        );
        assert_eq!(
            errors.to_string(),
            "version: unsupported config version 2; supported range is 1..=1"
        );
    }

    #[test]
    fn reports_empty_app_fields_with_paths() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::new(vec![
                AppConfig::new(
                    "",
                    " ",
                    "\t",
                    "",
                    Some(" ".to_string()),
                    RouteStrategy::Header,
                )
                .with_workload_kind(" "),
            ]),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::default(),
        );

        let errors = config.validate().expect_err("empty app fields");

        assert_eq!(
            errors.errors(),
            &[
                ConfigValidationError::EmptyAppField {
                    app_index: 0,
                    field: AppConfigField::Name,
                },
                ConfigValidationError::EmptyAppField {
                    app_index: 0,
                    field: AppConfigField::Namespace,
                },
                ConfigValidationError::EmptyAppField {
                    app_index: 0,
                    field: AppConfigField::Workload,
                },
                ConfigValidationError::EmptyAppField {
                    app_index: 0,
                    field: AppConfigField::WorkloadKind,
                },
                ConfigValidationError::EmptyAppField {
                    app_index: 0,
                    field: AppConfigField::Service,
                },
                ConfigValidationError::EmptyAppField {
                    app_index: 0,
                    field: AppConfigField::DefaultImage,
                },
            ]
        );
        assert_eq!(
            errors.to_string(),
            "6 config validation errors; first error: apps[0].name: field is required"
        );
    }

    #[test]
    fn reports_empty_app_fields_with_matching_app_indexes() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::new(vec![
                AppConfig::new(
                    "",
                    "shop",
                    "checkout-api",
                    "checkout-http",
                    None,
                    RouteStrategy::Header,
                ),
                AppConfig::new(
                    "catalog",
                    "",
                    "catalog-api",
                    "catalog-http",
                    None,
                    RouteStrategy::Host,
                ),
            ]),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::default(),
        );

        let errors = config.validate().expect_err("empty app fields");

        assert_eq!(
            errors.errors(),
            &[
                ConfigValidationError::EmptyAppField {
                    app_index: 0,
                    field: AppConfigField::Name,
                },
                ConfigValidationError::EmptyAppField {
                    app_index: 1,
                    field: AppConfigField::Namespace,
                },
            ]
        );
    }

    #[test]
    fn reports_app_field_names() {
        assert_eq!(AppConfigField::Name.as_str(), "name");
        assert_eq!(AppConfigField::Namespace.as_str(), "namespace");
        assert_eq!(AppConfigField::Workload.as_str(), "workload");
        assert_eq!(AppConfigField::WorkloadKind.as_str(), "workload_kind");
        assert_eq!(AppConfigField::Service.as_str(), "service");
        assert_eq!(AppConfigField::DefaultImage.as_str(), "default_image");
    }

    #[test]
    fn reports_policy_field_names() {
        assert_eq!(PolicyConfigField::Name.as_str(), "name");
        assert_eq!(PolicyConfigField::Description.as_str(), "description");
        assert_eq!(
            PolicyConfigField::AllowedNamespaces.as_str(),
            "allowed_namespaces"
        );
        assert_eq!(
            PolicyConfigField::AllowedWorkloadKinds.as_str(),
            "allowed_workload_kinds"
        );
        assert_eq!(
            PolicyConfigField::AllowedImageRegistries.as_str(),
            "allowed_image_registries"
        );
        assert_eq!(
            PolicyConfigField::AllowedRouteStrategies.as_str(),
            "allowed_route_strategies"
        );
        assert_eq!(PolicyConfigField::MaxSessionTtl.as_str(), "max_session_ttl");
        assert_eq!(PolicyConfigField::MutationMode.as_str(), "mutation_mode");
        assert_eq!(
            PolicyConfigField::SecretHandling.as_str(),
            "secret_handling"
        );
    }

    #[test]
    fn creates_policy_config_with_explicit_fields() {
        let config = policy_config();

        assert_eq!(config.name(), "sandbox-defaults");
        assert!(!config.enabled());
        assert_eq!(
            config.description(),
            Some("Default sandbox boundaries for local agent sessions")
        );
        assert_eq!(
            config.allowed_namespaces(),
            &["shop".to_string(), "kply-demo".to_string()]
        );
        assert_eq!(
            config.allowed_workload_kinds(),
            &["Deployment".to_string(), "StatefulSet".to_string()]
        );
        assert_eq!(
            config.allowed_image_registries(),
            &[
                "registry.example.com".to_string(),
                "localhost:5000".to_string()
            ]
        );
        assert_eq!(
            config.allowed_route_strategies(),
            &[RouteStrategy::Header, RouteStrategy::Preview]
        );
        assert_eq!(config.max_session_ttl(), Some("2h"));
        assert_eq!(
            config.mutation_mode(),
            Some(MutationModePolicy::SandboxOnly)
        );
        assert_eq!(
            config.secret_handling(),
            Some(SecretHandlingPolicy::MetadataOnly)
        );
    }

    #[test]
    fn defaults_policy_config_to_enabled_without_description() {
        let config = PolicyConfig::new("sandbox-defaults");

        assert_eq!(config.name(), "sandbox-defaults");
        assert!(config.enabled());
        assert_eq!(config.description(), None);
        assert!(config.allowed_namespaces().is_empty());
        assert!(config.allowed_workload_kinds().is_empty());
        assert!(config.allowed_image_registries().is_empty());
        assert!(config.allowed_route_strategies().is_empty());
        assert_eq!(config.max_session_ttl(), None);
        assert_eq!(config.mutation_mode(), None);
        assert_eq!(config.secret_handling(), None);
    }

    #[test]
    fn deserializes_policy_config_yaml_with_defaulted_fields() {
        let config: KplyConfig = serde_norway::from_str(
            r#"
version: 1
policies:
  - name: sandbox-defaults
    allowed_namespaces:
      - shop
    allowed_workload_kinds:
      - Deployment
    allowed_image_registries:
      - registry.example.com
      - localhost:5000
    allowed_route_strategies:
      - header
    max_session_ttl: 30m
    mutation_mode: sandbox-only
    secret_handling: metadata-only
"#,
        )
        .expect("valid policy config YAML");

        assert_eq!(
            config.policies().entries(),
            &[PolicyConfig::new("sandbox-defaults")
                .with_allowed_namespaces(["shop"])
                .with_allowed_workload_kinds(["Deployment"])
                .with_allowed_image_registries(["registry.example.com", "localhost:5000"])
                .with_allowed_route_strategies([RouteStrategy::Header])
                .with_max_session_ttl("30m")
                .with_mutation_mode(MutationModePolicy::SandboxOnly)
                .with_secret_handling(SecretHandlingPolicy::MetadataOnly)]
        );
        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn rejects_empty_policy_fields_with_paths() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::default(),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::new(vec![
                PolicyConfig::new(" ").with_description(""),
                PolicyConfig::new("sandbox-defaults").with_description("\t"),
            ]),
        );

        let errors = config.validate().expect_err("empty policy fields");

        assert_eq!(
            errors.errors(),
            &[
                ConfigValidationError::EmptyPolicyField {
                    policy_index: 0,
                    field: PolicyConfigField::Name,
                },
                ConfigValidationError::EmptyPolicyField {
                    policy_index: 0,
                    field: PolicyConfigField::Description,
                },
                ConfigValidationError::EmptyPolicyField {
                    policy_index: 1,
                    field: PolicyConfigField::Description,
                },
            ]
        );
        assert_eq!(
            errors.to_string(),
            "3 config validation errors; first error: policies[0].name: field is required"
        );
    }

    #[test]
    fn rejects_empty_and_duplicate_policy_allowed_namespaces() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::default(),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::new(vec![
                PolicyConfig::new("sandbox-defaults")
                    .with_allowed_namespaces(["shop", " ", "shop"]),
            ]),
        );

        let errors = config
            .validate()
            .expect_err("invalid allowed namespaces should fail");

        assert_eq!(
            errors.errors(),
            &[
                ConfigValidationError::EmptyPolicyListEntry {
                    policy_index: 0,
                    field: PolicyConfigField::AllowedNamespaces,
                    entry_index: 1,
                },
                ConfigValidationError::DuplicatePolicyListEntry {
                    policy_index: 0,
                    field: PolicyConfigField::AllowedNamespaces,
                    value: "shop".to_string(),
                },
            ]
        );
        assert_eq!(
            errors.to_string(),
            "2 config validation errors; first error: policies[0].allowed_namespaces[1]: field is required"
        );
    }

    #[test]
    fn rejects_empty_and_duplicate_policy_allowed_workload_kinds() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::default(),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::new(vec![
                PolicyConfig::new("sandbox-defaults").with_allowed_workload_kinds([
                    "Deployment",
                    "",
                    " Deployment ",
                ]),
            ]),
        );

        let errors = config
            .validate()
            .expect_err("invalid allowed workload kinds should fail");

        assert_eq!(
            errors.errors(),
            &[
                ConfigValidationError::EmptyPolicyListEntry {
                    policy_index: 0,
                    field: PolicyConfigField::AllowedWorkloadKinds,
                    entry_index: 1,
                },
                ConfigValidationError::DuplicatePolicyListEntry {
                    policy_index: 0,
                    field: PolicyConfigField::AllowedWorkloadKinds,
                    value: "Deployment".to_string(),
                },
            ]
        );
        assert_eq!(
            errors.to_string(),
            "2 config validation errors; first error: policies[0].allowed_workload_kinds[1]: field is required"
        );
    }

    #[test]
    fn rejects_invalid_policy_allowed_image_registries() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::default(),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::new(vec![
                PolicyConfig::new("sandbox-defaults").with_allowed_image_registries([
                    "registry.example.com",
                    " ",
                    " registry.example.com ",
                    "REGISTRY.example.com",
                    "localhost:",
                    "registry.example.com/team",
                ]),
            ]),
        );

        let errors = config
            .validate()
            .expect_err("invalid allowed image registries should fail");

        assert_eq!(
            errors.errors(),
            &[
                ConfigValidationError::EmptyPolicyListEntry {
                    policy_index: 0,
                    field: PolicyConfigField::AllowedImageRegistries,
                    entry_index: 1,
                },
                ConfigValidationError::DuplicatePolicyListEntry {
                    policy_index: 0,
                    field: PolicyConfigField::AllowedImageRegistries,
                    value: "registry.example.com".to_string(),
                },
                ConfigValidationError::InvalidPolicyImageRegistry {
                    policy_index: 0,
                    field: PolicyConfigField::AllowedImageRegistries,
                    value: "REGISTRY.example.com".to_string(),
                    reason: PolicyImageRegistryError::InvalidCharacter { character: 'R' },
                },
                ConfigValidationError::InvalidPolicyImageRegistry {
                    policy_index: 0,
                    field: PolicyConfigField::AllowedImageRegistries,
                    value: "localhost:".to_string(),
                    reason: PolicyImageRegistryError::InvalidPort,
                },
                ConfigValidationError::InvalidPolicyImageRegistry {
                    policy_index: 0,
                    field: PolicyConfigField::AllowedImageRegistries,
                    value: "registry.example.com/team".to_string(),
                    reason: PolicyImageRegistryError::InvalidCharacter { character: '/' },
                },
            ]
        );
        assert_eq!(
            errors.to_string(),
            "5 config validation errors; first error: policies[0].allowed_image_registries[1]: field is required"
        );
    }

    #[test]
    fn rejects_duplicate_policy_allowed_route_strategies() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::default(),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::new(vec![
                PolicyConfig::new("sandbox-defaults").with_allowed_route_strategies([
                    RouteStrategy::Header,
                    RouteStrategy::Preview,
                    RouteStrategy::Header,
                ]),
            ]),
        );

        let errors = config
            .validate()
            .expect_err("duplicate allowed route strategies should fail");

        assert_eq!(
            errors.errors(),
            &[ConfigValidationError::DuplicatePolicyListEntry {
                policy_index: 0,
                field: PolicyConfigField::AllowedRouteStrategies,
                value: "header".to_string(),
            }]
        );
        assert_eq!(
            errors.to_string(),
            "policies[0].allowed_route_strategies: duplicate value `header`"
        );
    }

    #[test]
    fn rejects_invalid_policy_max_session_ttl() {
        let config = KplyConfig::new(
            ConfigVersion::CURRENT,
            AppConfigs::default(),
            RoutingConfig,
            CheckConfigs::default(),
            PolicyConfigs::new(vec![
                PolicyConfig::new("sandbox-defaults").with_max_session_ttl("0m"),
                PolicyConfig::new("short-sandbox").with_max_session_ttl("forever"),
            ]),
        );

        let errors = config.validate().expect_err("invalid max TTL should fail");

        assert_eq!(
            errors.errors(),
            &[
                ConfigValidationError::InvalidPolicyDuration {
                    policy_index: 0,
                    field: PolicyConfigField::MaxSessionTtl,
                    reason: PolicyDurationError::Zero,
                },
                ConfigValidationError::InvalidPolicyDuration {
                    policy_index: 1,
                    field: PolicyConfigField::MaxSessionTtl,
                    reason: PolicyDurationError::InvalidUnit { unit: 'r' },
                },
            ]
        );
        assert_eq!(
            errors.to_string(),
            "2 config validation errors; first error: policies[0].max_session_ttl: invalid duration; value must be greater than zero"
        );
    }

    #[test]
    fn serializes_policy_config_to_stable_json() {
        let value = serde_json::to_value(policy_config()).expect("policy should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "name": "sandbox-defaults",
                "enabled": false,
                "description": "Default sandbox boundaries for local agent sessions",
                "allowed_namespaces": ["shop", "kply-demo"],
                "allowed_workload_kinds": ["Deployment", "StatefulSet"],
                "allowed_image_registries": ["registry.example.com", "localhost:5000"],
                "allowed_route_strategies": ["header", "preview"],
                "max_session_ttl": "2h",
                "mutation_mode": "sandbox-only",
                "secret_handling": "metadata-only",
            })
        );
    }

    #[test]
    fn creates_app_config_with_explicit_fields() {
        let config = app_config();

        assert_eq!(config.name(), "checkout");
        assert_eq!(config.namespace(), "shop");
        assert_eq!(config.workload(), "checkout-api");
        assert_eq!(config.workload_kind(), "Deployment");
        assert_eq!(config.service(), "checkout-http");
        assert_eq!(
            config.default_image(),
            Some("registry.example.com/shop/checkout:test")
        );
        assert_eq!(config.route_strategy(), RouteStrategy::Header);
    }

    #[test]
    fn creates_app_config_without_default_image() {
        let config = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            None,
            RouteStrategy::Host,
        );

        assert_eq!(config.default_image(), None);
        assert_eq!(config.workload_kind(), "Deployment");
        assert_eq!(config.route_strategy(), RouteStrategy::Host);
    }

    #[test]
    fn creates_app_config_with_explicit_workload_kind() {
        let config = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            None,
            RouteStrategy::Host,
        )
        .with_workload_kind("StatefulSet");

        assert_eq!(config.workload_kind(), "StatefulSet");
    }

    #[test]
    fn renders_route_strategy_names() {
        assert_eq!(RouteStrategy::Header.as_str(), "header");
        assert_eq!(RouteStrategy::Host.as_str(), "host");
        assert_eq!(RouteStrategy::Preview.as_str(), "preview");
    }

    #[test]
    fn renders_mutation_mode_policy_names() {
        assert_eq!(MutationModePolicy::ReadOnly.as_str(), "read-only");
        assert_eq!(MutationModePolicy::SandboxOnly.as_str(), "sandbox-only");
        assert_eq!(MutationModePolicy::RouteMutation.as_str(), "route-mutation");
    }

    #[test]
    fn renders_secret_handling_policy_names() {
        assert_eq!(SecretHandlingPolicy::MetadataOnly.as_str(), "metadata-only");
        assert_eq!(
            SecretHandlingPolicy::DenyReferences.as_str(),
            "deny-references"
        );
    }

    #[test]
    fn defaults_to_current_empty_top_level_config() {
        let config = KplyConfig::default();

        assert_eq!(config.version(), ConfigVersion::CURRENT);
        assert!(config.apps().is_empty());
        assert_eq!(config.routing(), &RoutingConfig);
        assert!(config.checks().is_empty());
        assert!(config.policies().is_empty());
    }

    #[test]
    fn discovers_config_from_current_directory() {
        let _guard = CURRENT_DIR_LOCK.lock().expect("current directory lock");
        let workspace = TempDir::new().expect("temporary workspace");
        let config_path = write_config(workspace.path());
        let original_dir = env::current_dir().expect("current directory");

        env::set_current_dir(workspace.path()).expect("set current directory");
        let discovered = super::discover_config_path().expect("discover config");
        env::set_current_dir(original_dir).expect("restore current directory");

        assert_eq!(
            discovered
                .as_deref()
                .map(fs::canonicalize)
                .transpose()
                .expect("canonical discovered path"),
            Some(fs::canonicalize(config_path).expect("canonical config path"))
        );
    }

    #[test]
    fn discovers_config_in_start_directory() {
        let workspace = TempDir::new().expect("temporary workspace");
        let config_path = write_config(workspace.path());

        assert_eq!(
            discover_config_path_from(workspace.path()),
            Some(config_path)
        );
    }

    #[test]
    fn discovers_nearest_config_from_nested_directory() {
        let workspace = TempDir::new().expect("temporary workspace");
        let parent_config = write_config(workspace.path());
        let nested = workspace.path().join("services/api");
        fs::create_dir_all(&nested).expect("nested directory");
        let nested_config = write_config(&nested);

        assert_eq!(discover_config_path_from(&nested), Some(nested_config));
        assert_ne!(discover_config_path_from(&nested), Some(parent_config));
    }

    #[test]
    fn discovers_parent_config_from_nested_directory() {
        let workspace = TempDir::new().expect("temporary workspace");
        let config_path = write_config(workspace.path());
        let nested = workspace.path().join("services/api");
        fs::create_dir_all(&nested).expect("nested directory");

        assert_eq!(discover_config_path_from(nested), Some(config_path));
    }

    #[test]
    fn returns_none_when_no_config_exists() {
        let workspace = TempDir::new().expect("temporary workspace");
        let nested = workspace.path().join("services/api");
        fs::create_dir_all(&nested).expect("nested directory");

        assert_eq!(discover_config_path_from(nested), None);
    }

    #[test]
    fn ignores_directories_named_like_config() {
        let workspace = TempDir::new().expect("temporary workspace");
        fs::create_dir(workspace.path().join(CANONICAL_CONFIG_FILENAME))
            .expect("config-named directory");

        assert_eq!(discover_config_path_from(workspace.path()), None);
    }

    fn write_config(directory: &Path) -> std::path::PathBuf {
        write_config_contents(directory, "version: 1\n")
    }

    fn write_config_contents(directory: &Path, contents: &str) -> std::path::PathBuf {
        let config_path = directory.join(CANONICAL_CONFIG_FILENAME);
        fs::write(&config_path, contents).expect("config file");
        config_path
    }

    fn app_config() -> AppConfig {
        AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("registry.example.com/shop/checkout:test".to_string()),
            RouteStrategy::Header,
        )
    }

    fn policy_config() -> PolicyConfig {
        PolicyConfig::new("sandbox-defaults")
            .with_enabled(false)
            .with_description("Default sandbox boundaries for local agent sessions")
            .with_allowed_namespaces(["shop", "kply-demo"])
            .with_allowed_workload_kinds(["Deployment", "StatefulSet"])
            .with_allowed_image_registries(["registry.example.com", "localhost:5000"])
            .with_allowed_route_strategies([RouteStrategy::Header, RouteStrategy::Preview])
            .with_max_session_ttl("2h")
            .with_mutation_mode(MutationModePolicy::SandboxOnly)
            .with_secret_handling(SecretHandlingPolicy::MetadataOnly)
    }
}
