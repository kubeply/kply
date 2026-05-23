use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub apps: Vec<AppConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub namespace: String,
    pub workload: String,
    pub default_route_header: Option<String>,
}

pub fn load_config(path: &Path) -> Result<ProjectConfig> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    serde_yaml::from_str(&source).context("failed to parse kply config")
}

#[cfg(test)]
mod tests {
    use super::ProjectConfig;

    #[test]
    fn parses_app_config() {
        let source = r#"
apps:
  - name: backend-api
    namespace: shop
    workload: deployment/backend-api
    default_route_header: x-kply-session
"#;

        let config: ProjectConfig = serde_yaml::from_str(source).expect("config should parse");

        assert_eq!(config.apps[0].name, "backend-api");
    }
}
