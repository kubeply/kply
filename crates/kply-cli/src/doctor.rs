//! Top-level local readiness checks.

use anyhow::Result;
use kply_cli::cli::Cli;
use std::process::ExitCode;

use crate::local::find_command_in_path;
use crate::{EXIT_BLOCKING, exit_code, resolved_config};

/// Render top-level Kply readiness checks.
pub(crate) fn render_doctor(cli: &Cli) -> Result<ExitCode> {
    let checks = vec![
        config_check(cli),
        kubeconfig_check()?,
        command_check("kubectl", &["kubectl"]),
    ];

    let ready = checks.iter().all(DoctorCheck::is_ok);
    let status = if ready { "ready" } else { "blocked" };

    if cli.json {
        let value = serde_json::json!({
            "command": "doctor",
            "status": status,
            "checks": checks.iter().map(DoctorCheck::to_json).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if !cli.quiet {
        println!("kply doctor");
        println!("status: {status}");
        println!("checks: {}", checks.len());
        for check in &checks {
            println!("  {}: {} -> {}", check.status, check.name, check.message);
        }
    }

    Ok(if ready {
        ExitCode::SUCCESS
    } else {
        exit_code(EXIT_BLOCKING)
    })
}

#[derive(Debug)]
struct DoctorCheck {
    name: &'static str,
    status: &'static str,
    message: String,
}

impl DoctorCheck {
    fn ok(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            status: "ok",
            message: message.into(),
        }
    }

    fn invalid(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            status: "invalid",
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

fn config_check(cli: &Cli) -> DoctorCheck {
    let config = match resolved_config(cli) {
        Ok(config) => config,
        Err(error) => return DoctorCheck::invalid("config", error.to_string()),
    };

    if let Err(errors) = config.validate() {
        return DoctorCheck::invalid("config", errors.to_string());
    }

    if let Some(path) = &cli.config {
        DoctorCheck::ok("config", format!("{} is valid", path.display()))
    } else if cli.no_config {
        DoctorCheck::ok("config", "--no-config uses the default in-memory config")
    } else {
        DoctorCheck::ok("config", "default in-memory config is valid")
    }
}

fn kubeconfig_check() -> Result<DoctorCheck> {
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    match runtime.block_on(kply_k8s::load_kube_config()) {
        Ok(_) => Ok(DoctorCheck::ok(
            "kubeconfig",
            "kubeconfig resolved for the current context",
        )),
        Err(error) => {
            let error = kply_k8s::DiscoveryError::from_kubeconfig_error_redacted(&error);
            if error.code.as_str() == "missing_kubeconfig" {
                Ok(DoctorCheck::missing("kubeconfig", error.message))
            } else {
                Ok(DoctorCheck::invalid("kubeconfig", error.message))
            }
        }
    }
}

fn command_check(name: &'static str, candidates: &[&str]) -> DoctorCheck {
    if let Some((command, path)) = candidates
        .iter()
        .find_map(|command| find_command_in_path(command).map(|path| (*command, path)))
    {
        return DoctorCheck::ok(name, format!("{command} at {}", path.display()));
    }

    DoctorCheck::missing(name, format!("missing one of: {}", candidates.join(", ")))
}
