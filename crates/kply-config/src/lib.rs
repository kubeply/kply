//! Configuration primitives for future Kply project and cluster settings.

use std::io;
use std::path::{Path, PathBuf};

/// Canonical Kply project configuration filename.
pub const CANONICAL_CONFIG_FILENAME: &str = "kply.yaml";

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
    use super::{CANONICAL_CONFIG_FILENAME, discover_config_path_from};
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
