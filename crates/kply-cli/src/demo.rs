//! Local demo command implementations.

use std::path::{Path, PathBuf};

pub(crate) mod doctor;
pub(crate) mod install;
pub(crate) mod reset;
pub(crate) mod teardown;

/// Namespace that contains every local demo Kubernetes resource.
pub(crate) const DEMO_NAMESPACE: &str = "kply-demo";
/// Repository-relative path to the local demo Kply configuration.
pub(crate) const DEMO_CONFIG_PATH: &str = "fixtures/demo/ecommerce-basic/kply.yaml";
/// Ordered manifest list applied by `kply demo install` for the baseline demo.
pub(crate) const DEMO_BASELINE_MANIFEST_PATHS: [&str; 4] = [
    "fixtures/demo/ecommerce-basic/manifests/namespace.yaml",
    "fixtures/demo/ecommerce-basic/manifests/catalog.yaml",
    "fixtures/demo/ecommerce-basic/manifests/frontend.yaml",
    "fixtures/demo/ecommerce-basic/manifests/backend.yaml",
];
/// Complete manifest list checked by `kply demo doctor`.
pub(crate) const DEMO_MANIFEST_PATHS: [&str; 6] = [
    "fixtures/demo/ecommerce-basic/manifests/namespace.yaml",
    "fixtures/demo/ecommerce-basic/manifests/frontend.yaml",
    "fixtures/demo/ecommerce-basic/manifests/backend.yaml",
    "fixtures/demo/ecommerce-basic/manifests/backend-broken.yaml",
    "fixtures/demo/ecommerce-basic/manifests/backend-fixed.yaml",
    "fixtures/demo/ecommerce-basic/manifests/catalog.yaml",
];
/// Deployments that `kply demo install` waits for after applying manifests.
pub(crate) const DEMO_ROLLOUT_DEPLOYMENTS: [&str; 3] =
    ["catalog-api", "storefront-web", "checkout-api"];
/// Container runtime commands accepted by `kply demo doctor`.
pub(crate) const CONTAINER_RUNTIME_COMMANDS: [&str; 3] = ["docker", "podman", "nerdctl"];

/// Resolve a repository-relative path from the current directory, workspace root, or raw fallback.
pub(crate) fn repository_path(relative_path: &str) -> PathBuf {
    if let Ok(current_dir) = std::env::current_dir() {
        let current_dir_path = current_dir.join(relative_path);
        if current_dir_path.exists() {
            return current_dir_path;
        }
    }

    workspace_root_from_manifest_dir()
        .map(|root| root.join(relative_path))
        .unwrap_or_else(|| PathBuf::from(relative_path))
}

/// Find the first executable command candidate available on `PATH`.
pub(crate) fn find_command_in_path(command: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|directory| {
        command_path_candidates(&directory, command)
            .into_iter()
            .find(|candidate| is_executable_file(candidate))
    })
}

fn workspace_root_from_manifest_dir() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|ancestor| {
            let manifest = ancestor.join("Cargo.toml");
            std::fs::read_to_string(manifest)
                .is_ok_and(|contents| contents.lines().any(|line| line.trim() == "[workspace]"))
        })
        .map(Path::to_path_buf)
}

fn is_executable_file(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::metadata(path)
            .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
    }

    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn command_path_candidates(directory: &Path, command: &str) -> Vec<PathBuf> {
    let mut candidates = vec![directory.join(command)];
    let executable_suffix = std::env::consts::EXE_SUFFIX;
    if !executable_suffix.is_empty() && !command.ends_with(executable_suffix) {
        candidates.push(directory.join(format!("{command}{executable_suffix}")));
    }
    candidates
}
