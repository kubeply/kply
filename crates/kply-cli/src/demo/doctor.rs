//! Local demo prerequisite checks.

use anyhow::Result;
use kply_cli::cli::Cli;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const EXIT_BLOCKING: u8 = 1;
const DEMO_CONFIG_PATH: &str = "fixtures/demo/ecommerce-basic/kply.yaml";
const DEMO_MANIFEST_PATHS: [&str; 6] = [
    "fixtures/demo/ecommerce-basic/manifests/namespace.yaml",
    "fixtures/demo/ecommerce-basic/manifests/frontend.yaml",
    "fixtures/demo/ecommerce-basic/manifests/backend.yaml",
    "fixtures/demo/ecommerce-basic/manifests/backend-broken.yaml",
    "fixtures/demo/ecommerce-basic/manifests/backend-fixed.yaml",
    "fixtures/demo/ecommerce-basic/manifests/catalog.yaml",
];
const CONTAINER_RUNTIME_COMMANDS: [&str; 3] = ["docker", "podman", "nerdctl"];

/// Render local demo prerequisite checks.
pub(crate) fn render_demo_doctor(cli: &Cli) -> Result<ExitCode> {
    let checks = demo_doctor_checks();
    let ready = checks.iter().all(DemoDoctorCheck::is_ok);
    let status = if ready { "ready" } else { "blocked" };

    if cli.json {
        let value = serde_json::json!({
            "command": "demo doctor",
            "status": status,
            "checks": checks.iter().map(DemoDoctorCheck::to_json).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if !cli.quiet {
        println!("kply demo doctor");
        println!("status: {status}");
        println!("checks: {}", checks.len());
        for check in &checks {
            println!("  {}: {} -> {}", check.status, check.name, check.message);
        }
    }

    Ok(if ready {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_BLOCKING)
    })
}

#[derive(Debug)]
struct DemoDoctorCheck {
    name: &'static str,
    status: &'static str,
    message: String,
}

impl DemoDoctorCheck {
    fn ok(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            status: "ok",
            message: message.into(),
        }
    }

    fn missing(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            status: "missing",
            message: message.into(),
        }
    }

    fn is_ok(&self) -> bool {
        self.status == "ok"
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "status": self.status,
            "message": self.message
        })
    }
}

fn demo_doctor_checks() -> Vec<DemoDoctorCheck> {
    let mut checks = Vec::new();
    checks.push(path_check("demo_config", DEMO_CONFIG_PATH));

    for manifest_path in DEMO_MANIFEST_PATHS {
        checks.push(path_check("demo_manifest", manifest_path));
    }

    checks.push(command_check("kind", &["kind"]));
    checks.push(command_check("kubectl", &["kubectl"]));
    checks.push(command_check(
        "container_runtime",
        &CONTAINER_RUNTIME_COMMANDS,
    ));
    checks
}

fn path_check(name: &'static str, relative_path: &str) -> DemoDoctorCheck {
    let path = repository_path(relative_path);
    if path.is_file() {
        DemoDoctorCheck::ok(name, relative_path)
    } else if path.exists() {
        DemoDoctorCheck::missing(
            name,
            format!("{relative_path} exists but is not a regular file"),
        )
    } else {
        DemoDoctorCheck::missing(name, format!("{relative_path} was not found"))
    }
}

fn command_check(name: &'static str, candidates: &[&str]) -> DemoDoctorCheck {
    if let Some((command, path)) = candidates
        .iter()
        .find_map(|command| find_command_in_path(command).map(|path| (*command, path)))
    {
        return DemoDoctorCheck::ok(name, format!("{command} at {}", path.display()));
    }

    DemoDoctorCheck::missing(name, format!("missing one of: {}", candidates.join(", ")))
}

fn repository_path(relative_path: &str) -> PathBuf {
    let current_dir_path = std::env::current_dir()
        .map(|current_dir| current_dir.join(relative_path))
        .unwrap_or_else(|_| PathBuf::from(relative_path));
    if current_dir_path.exists() {
        return current_dir_path;
    }

    workspace_root_from_manifest_dir()
        .map(|root| root.join(relative_path))
        .unwrap_or_else(|| PathBuf::from(relative_path))
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

fn find_command_in_path(command: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|directory| {
        command_path_candidates(&directory, command)
            .into_iter()
            .find(|candidate| is_executable_file(candidate))
    })
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
