//! Local demo prerequisite checks.

use anyhow::Result;
use kply_cli::cli::Cli;
use std::process::ExitCode;

use crate::demo::{
    CONTAINER_RUNTIME_COMMANDS, DEMO_CONFIG_PATH, DEMO_MANIFEST_PATHS, find_command_in_path,
    repository_path,
};

const EXIT_BLOCKING: u8 = 1;

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
