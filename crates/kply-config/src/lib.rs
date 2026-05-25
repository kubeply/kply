//! Configuration primitives for future Kply project and cluster settings.

use std::io;
use std::path::{Path, PathBuf};
use std::{error, fmt};

/// Canonical Kply project configuration filename.
pub const CANONICAL_CONFIG_FILENAME: &str = "kply.yaml";

/// Top-level Kply project configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KplyConfig {
    version: ConfigVersion,
    apps: AppConfigs,
    routing: RoutingConfig,
    checks: CheckConfigs,
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

/// Top-level application config collection.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
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

/// Application target configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    name: String,
    namespace: String,
    workload: String,
    service: String,
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
}

/// Routing strategy requested for an application target.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

/// Top-level routing config section.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RoutingConfig;

/// Top-level check config collection.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
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

/// Placeholder for a future check config entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckConfig;

/// Top-level policy config collection.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
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

/// Placeholder for a future policy config entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyConfig;

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
        AppConfig, AppConfigs, CANONICAL_CONFIG_FILENAME, CheckConfig, CheckConfigs, ConfigVersion,
        ConfigVersionError, KplyConfig, PolicyConfig, PolicyConfigs, RouteStrategy, RoutingConfig,
        discover_config_path_from,
    };
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());

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
            PolicyConfigs::new(vec![PolicyConfig]),
        );

        assert_eq!(config.version().get(), 7);
        assert_eq!(config.apps().entries(), &[app_config()]);
        assert_eq!(config.routing(), &RoutingConfig);
        assert_eq!(config.checks().entries(), &[CheckConfig]);
        assert_eq!(config.policies().entries(), &[PolicyConfig]);
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
    fn creates_app_config_with_explicit_fields() {
        let config = app_config();

        assert_eq!(config.name(), "checkout");
        assert_eq!(config.namespace(), "shop");
        assert_eq!(config.workload(), "checkout-api");
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
        assert_eq!(config.route_strategy(), RouteStrategy::Host);
    }

    #[test]
    fn renders_route_strategy_names() {
        assert_eq!(RouteStrategy::Header.as_str(), "header");
        assert_eq!(RouteStrategy::Host.as_str(), "host");
        assert_eq!(RouteStrategy::Preview.as_str(), "preview");
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
        let config_path = directory.join(CANONICAL_CONFIG_FILENAME);
        fs::write(&config_path, "version: 1\n").expect("config file");
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
}
