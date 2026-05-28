//! Top-level local readiness checks.

use anyhow::Result;
use kply_cli::cli::Cli;
use kply_config::{ConfigLoadError, KplyConfig};
use serde::Serialize;
use std::process::ExitCode;

use crate::demo::CONTAINER_RUNTIME_COMMANDS;
use crate::local::find_command_in_path;
use crate::{EXIT_BLOCKING, exit_code, resolved_config};

/// Render top-level Kply readiness checks.
pub(crate) fn render_doctor(cli: &Cli, capability_report: bool) -> Result<ExitCode> {
    if capability_report {
        return render_capability_report(cli);
    }

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
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
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

fn render_capability_report(cli: &Cli) -> Result<ExitCode> {
    let report = CapabilityReport::collect(cli)?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if !cli.quiet {
        print!("{}", render_capability_report_text(&report));
    }

    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, Serialize)]
struct CapabilityReport {
    schema_version: u16,
    command: &'static str,
    collection: &'static str,
    anonymized: bool,
    omitted: Vec<&'static str>,
    kply_version: &'static str,
    os: &'static str,
    arch: &'static str,
    config: CapabilityConfigReport,
    kubeconfig: CapabilityStatusReport,
    local_tools: Vec<CapabilityToolReport>,
}

impl CapabilityReport {
    fn collect(cli: &Cli) -> Result<Self> {
        Ok(Self {
            schema_version: 1,
            command: "doctor --capability-report",
            collection: "opt_in",
            anonymized: true,
            omitted: vec![
                "paths",
                "cluster_urls",
                "resource_names",
                "namespaces",
                "hostnames",
                "secret_values",
            ],
            kply_version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            config: CapabilityConfigReport::collect(cli),
            kubeconfig: capability_kubeconfig_report()?,
            local_tools: capability_tool_reports(),
        })
    }
}

#[derive(Debug, Serialize)]
struct CapabilityConfigReport {
    source: &'static str,
    status: &'static str,
    reason: Option<&'static str>,
    app_count: Option<usize>,
    policy_count: Option<usize>,
    enabled_policy_count: Option<usize>,
}

impl CapabilityConfigReport {
    fn collect(cli: &Cli) -> Self {
        let source = if cli.config.is_some() {
            "explicit"
        } else if cli.no_config {
            "disabled"
        } else {
            "default"
        };

        match resolved_config(cli) {
            Ok(config) => Self::from_validated_config(source, &config),
            Err(error) => Self {
                source,
                status: "unreadable",
                reason: Some(config_load_error_reason(&error)),
                app_count: None,
                policy_count: None,
                enabled_policy_count: None,
            },
        }
    }

    fn from_validated_config(source: &'static str, config: &KplyConfig) -> Self {
        if config.validate().is_err() {
            return Self {
                source,
                status: "invalid",
                reason: Some("validation_error"),
                app_count: None,
                policy_count: None,
                enabled_policy_count: None,
            };
        }

        let policies = config.policies().entries();
        Self {
            source,
            status: "valid",
            reason: None,
            app_count: Some(config.apps().entries().len()),
            policy_count: Some(policies.len()),
            enabled_policy_count: Some(policies.iter().filter(|policy| policy.enabled()).count()),
        }
    }
}

#[derive(Debug, Serialize)]
struct CapabilityStatusReport {
    status: &'static str,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct CapabilityToolReport {
    name: &'static str,
    present: bool,
}

fn config_load_error_reason(error: &ConfigLoadError) -> &'static str {
    match error {
        ConfigLoadError::Read { .. } => "read_error",
        ConfigLoadError::Parse { .. } => "parse_error",
        _ => "load_error",
    }
}

fn capability_kubeconfig_report() -> Result<CapabilityStatusReport> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
    match runtime.block_on(kply_k8s::load_kube_config()) {
        Ok(_) => Ok(CapabilityStatusReport {
            status: "resolved",
            reason: None,
        }),
        Err(error) => {
            let error = kply_k8s::DiscoveryError::from_kubeconfig_error_redacted(&error);
            Ok(CapabilityStatusReport {
                status: "unavailable",
                reason: Some(error.code.as_str().to_owned()),
            })
        }
    }
}

fn capability_tool_reports() -> Vec<CapabilityToolReport> {
    let mut reports = vec![CapabilityToolReport {
        name: "kubectl",
        present: find_command_in_path("kubectl").is_some(),
    }];
    reports.extend(
        CONTAINER_RUNTIME_COMMANDS
            .iter()
            .map(|command| CapabilityToolReport {
                name: command,
                present: find_command_in_path(command).is_some(),
            }),
    );
    reports
}

fn render_capability_report_text(report: &CapabilityReport) -> String {
    let config_reason = report.config.reason.unwrap_or("none");
    let kubeconfig_reason = report.kubeconfig.reason.as_deref().unwrap_or("none");
    let mut output = format!(
        "kply doctor --capability-report\nschema_version: {}\ncollection: {}\nanonymized: {}\nomitted: {}\nkply_version: {}\nos: {}\narch: {}\nconfig_source: {}\nconfig_status: {}\nconfig_reason: {}\napp_count: {}\npolicy_count: {}\nenabled_policy_count: {}\nkubeconfig_status: {}\nkubeconfig_reason: {}\nlocal_tools: {}\n",
        report.schema_version,
        report.collection,
        report.anonymized,
        report.omitted.join(","),
        report.kply_version,
        report.os,
        report.arch,
        report.config.source,
        report.config.status,
        config_reason,
        optional_count(report.config.app_count),
        optional_count(report.config.policy_count),
        optional_count(report.config.enabled_policy_count),
        report.kubeconfig.status,
        kubeconfig_reason,
        report.local_tools.len()
    );

    for tool in &report.local_tools {
        output.push_str(&format!("  {}: present={}\n", tool.name, tool.present));
    }

    output
}

fn optional_count(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}
