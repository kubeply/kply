//! Configuration primitives for future Kply project and cluster settings.

use std::io;
use std::path::{Path, PathBuf};

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

    /// Create a config schema version.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Return the numeric config schema version.
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl Default for ConfigVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

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

/// Placeholder for a future app config entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig;

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
        KplyConfig, PolicyConfig, PolicyConfigs, RoutingConfig, discover_config_path_from,
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
            AppConfigs::new(vec![AppConfig]),
            RoutingConfig,
            CheckConfigs::new(vec![CheckConfig]),
            PolicyConfigs::new(vec![PolicyConfig]),
        );

        assert_eq!(config.version().get(), 7);
        assert_eq!(config.apps().entries(), &[AppConfig]);
        assert_eq!(config.routing(), &RoutingConfig);
        assert_eq!(config.checks().entries(), &[CheckConfig]);
        assert_eq!(config.policies().entries(), &[PolicyConfig]);
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
}
