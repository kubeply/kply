//! Kubernetes adapters for future safe session execution.

use std::path::Path;

use kube::{
    Config,
    config::{KubeConfigOptions, Kubeconfig, KubeconfigError},
};

/// Load Kubernetes client config using standard kubeconfig conventions.
///
/// This reads the kubeconfig selected by `KUBECONFIG`, or `~/.kube/config`
/// when `KUBECONFIG` is not set. It does not contact the cluster.
///
/// # Errors
///
/// Returns [`KubeconfigError`] when kube-rs cannot find, read, parse, or
/// resolve the selected kubeconfig.
pub async fn load_kube_config() -> Result<Config, KubeconfigError> {
    load_kube_config_with_options(&KubeConfigOptions::default()).await
}

/// Load Kubernetes client config using explicit kubeconfig selection options.
///
/// This keeps context, cluster, and user selection aligned with kube-rs and
/// Kubernetes client conventions. It does not contact the cluster.
///
/// # Errors
///
/// Returns [`KubeconfigError`] when kube-rs cannot find, read, parse, or
/// resolve the selected kubeconfig.
pub async fn load_kube_config_with_options(
    options: &KubeConfigOptions,
) -> Result<Config, KubeconfigError> {
    Config::from_kubeconfig(options).await
}

/// Load Kubernetes client config from an explicit kubeconfig path.
///
/// This helper is primarily useful for deterministic tests and future CLI
/// paths that need to resolve a known kubeconfig file. It does not contact the
/// cluster.
///
/// # Errors
///
/// Returns [`KubeconfigError`] when kube-rs cannot read, parse, or resolve the
/// kubeconfig at `path`.
pub async fn load_kube_config_path(path: impl AsRef<Path>) -> Result<Config, KubeconfigError> {
    let kubeconfig = Kubeconfig::read_from(path)?;
    Config::from_custom_kubeconfig(kubeconfig, &KubeConfigOptions::default()).await
}

#[cfg(test)]
mod tests {
    use super::{load_kube_config_path, load_kube_config_with_options};
    use kube::config::KubeConfigOptions;
    use std::env;
    use tokio::sync::Mutex;

    static KUBECONFIG_ENV_LOCK: Mutex<()> = Mutex::const_new(());

    #[tokio::test]
    async fn loads_kube_config_from_explicit_path() {
        let workspace = kply_test::temp_workspace();
        let kubeconfig_path = kply_test::write_fake_kubeconfig(&workspace);

        let config = load_kube_config_path(kubeconfig_path)
            .await
            .expect("fake kubeconfig should load");

        assert_eq!(config.cluster_url.to_string(), "https://127.0.0.1:6443/");
        assert_eq!(config.default_namespace, "default");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn loads_kube_config_with_explicit_context_option() {
        let workspace = kply_test::temp_workspace();
        let kubeconfig_path = kply_test::write_temp_file(
            &workspace,
            "kubeconfig.yaml",
            r#"
apiVersion: v1
kind: Config
clusters:
  - name: cluster-a
    cluster:
      server: https://127.0.0.1:6443
users:
  - name: user-a
    user:
      token: fake-token
contexts:
  - name: context-a
    context:
      cluster: cluster-a
      user: user-a
      namespace: qa
current-context: context-a
"#,
        );
        let options = KubeConfigOptions {
            context: Some("context-a".to_owned()),
            ..KubeConfigOptions::default()
        };
        let _env_lock = KUBECONFIG_ENV_LOCK.lock().await;
        let previous_kubeconfig = env::var_os("KUBECONFIG");

        // SAFETY: environment mutation is serialized by KUBECONFIG_ENV_LOCK and
        // restored before releasing the lock.
        unsafe {
            env::set_var("KUBECONFIG", &kubeconfig_path);
        }
        let result = load_kube_config_with_options(&options).await;
        // SAFETY: restore the process environment to the value captured before
        // this test changed KUBECONFIG while still holding KUBECONFIG_ENV_LOCK.
        unsafe {
            if let Some(previous_kubeconfig) = previous_kubeconfig {
                env::set_var("KUBECONFIG", previous_kubeconfig);
            } else {
                env::remove_var("KUBECONFIG");
            }
        }

        let config = result.expect("fake kubeconfig should resolve");

        assert_eq!(config.default_namespace, "qa");
    }

    #[tokio::test]
    async fn reports_missing_explicit_kube_config_path() {
        let workspace = kply_test::temp_workspace();
        let missing_path = workspace.path().join("missing").join("kubeconfig.yaml");

        let error = load_kube_config_path(missing_path)
            .await
            .expect_err("missing kubeconfig should fail");

        assert!(
            matches!(error, kube::config::KubeconfigError::ReadConfig(_, _)),
            "unexpected kubeconfig error: {error}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn loads_kube_config_from_kubeconfig_environment_variable() {
        let workspace = kply_test::temp_workspace();
        let kubeconfig_path = kply_test::write_fake_kubeconfig(&workspace);
        let _env_lock = KUBECONFIG_ENV_LOCK.lock().await;
        let previous_kubeconfig = env::var_os("KUBECONFIG");

        // SAFETY: environment mutation is serialized by KUBECONFIG_ENV_LOCK and
        // restored before releasing the lock.
        unsafe {
            env::set_var("KUBECONFIG", &kubeconfig_path);
        }
        let result = load_kube_config_with_options(&KubeConfigOptions::default()).await;
        // SAFETY: restore the process environment to the value captured before
        // this test changed KUBECONFIG while still holding KUBECONFIG_ENV_LOCK.
        unsafe {
            if let Some(previous_kubeconfig) = previous_kubeconfig {
                env::set_var("KUBECONFIG", previous_kubeconfig);
            } else {
                env::remove_var("KUBECONFIG");
            }
        }

        let config = result.expect("KUBECONFIG-selected fake kubeconfig should load");

        assert_eq!(config.cluster_url.to_string(), "https://127.0.0.1:6443/");
    }
}
