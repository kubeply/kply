//! Local demo baseline installer.

use anyhow::Result;
use kply_cli::cli::Cli;
use serde::Serialize;
use std::process::{Command, ExitCode, Output};

use crate::demo::{
    DEMO_BASELINE_MANIFEST_PATHS, DEMO_NAMESPACE, DEMO_ROLLOUT_DEPLOYMENTS, find_command_in_path,
    repository_path,
};

const EXIT_BLOCKING: u8 = 1;
const KUBECTL_COMMAND: &str = "kubectl";
const KUBECTL_ROLLOUT_TIMEOUT: &str = "5m";

/// Install the local demo baseline resources into the current Kubernetes context.
pub(crate) fn render_demo_install(cli: &Cli) -> Result<ExitCode> {
    render_demo_baseline(cli, DemoBaselineCommand::Install)
}

/// Apply the baseline demo resources for install-like demo commands.
pub(crate) fn render_demo_baseline(cli: &Cli, command: DemoBaselineCommand) -> Result<ExitCode> {
    let result = apply_demo_baseline(command);

    match result {
        Ok(report) => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if !cli.quiet {
                println!("kply {}", report.command);
                println!("status: {}", report.status);
                println!("namespace: {}", report.namespace);
                println!("applied_manifests: {}", report.applied_manifests.len());
                for manifest in &report.applied_manifests {
                    println!("  applied: {manifest}");
                }
                println!("rollouts: {}", report.ready_deployments.len());
                for deployment in &report.ready_deployments {
                    println!("  ready: deployment/{deployment}");
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            render_baseline_error(cli, command, &error)?;
            Ok(ExitCode::from(EXIT_BLOCKING))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DemoBaselineCommand {
    Install,
    Reset,
}

impl DemoBaselineCommand {
    fn command_name(self) -> &'static str {
        match self {
            Self::Install => "demo install",
            Self::Reset => "demo reset",
        }
    }

    fn status(self) -> &'static str {
        match self {
            Self::Install => "installed",
            Self::Reset => "reset",
        }
    }
}

#[derive(Debug, Serialize)]
struct DemoInstallReport {
    command: &'static str,
    status: &'static str,
    namespace: &'static str,
    applied_manifests: Vec<&'static str>,
    ready_deployments: Vec<&'static str>,
}

#[derive(Debug)]
struct DemoInstallError {
    code: &'static str,
    message: String,
    command: Vec<String>,
    stderr: String,
}

fn apply_demo_baseline(
    command: DemoBaselineCommand,
) -> std::result::Result<DemoInstallReport, DemoInstallError> {
    if find_command_in_path(KUBECTL_COMMAND).is_none() {
        return Err(DemoInstallError {
            code: "missing_kubectl",
            message: "kubectl was not found on PATH".to_owned(),
            command: vec![KUBECTL_COMMAND.to_owned()],
            stderr: String::new(),
        });
    }

    for manifest in DEMO_BASELINE_MANIFEST_PATHS {
        let path = repository_path(manifest);
        let path = path.to_string_lossy();
        run_kubectl(&["apply", "-f", path.as_ref()])?;
    }

    for deployment in DEMO_ROLLOUT_DEPLOYMENTS {
        run_kubectl(&[
            "-n",
            DEMO_NAMESPACE,
            "rollout",
            "status",
            "--timeout",
            KUBECTL_ROLLOUT_TIMEOUT,
            &format!("deployment/{deployment}"),
        ])?;
    }

    Ok(DemoInstallReport {
        command: command.command_name(),
        status: command.status(),
        namespace: DEMO_NAMESPACE,
        applied_manifests: DEMO_BASELINE_MANIFEST_PATHS.to_vec(),
        ready_deployments: DEMO_ROLLOUT_DEPLOYMENTS.to_vec(),
    })
}

fn run_kubectl(args: &[&str]) -> std::result::Result<Output, DemoInstallError> {
    let output = Command::new(KUBECTL_COMMAND)
        .args(args)
        .output()
        .map_err(|error| DemoInstallError {
            code: "kubectl_exec",
            message: format!("failed to run kubectl: {error}"),
            command: command_preview(args),
            stderr: String::new(),
        })?;

    if output.status.success() {
        return Ok(output);
    }

    Err(DemoInstallError {
        code: "kubectl_failed",
        message: format!(
            "kubectl exited with status {}",
            output
                .status
                .code()
                .map_or_else(|| "unknown".to_owned(), |code| code.to_string())
        ),
        command: command_preview(args),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

fn command_preview(args: &[&str]) -> Vec<String> {
    std::iter::once(KUBECTL_COMMAND.to_owned())
        .chain(args.iter().map(|arg| (*arg).to_owned()))
        .collect()
}

fn render_baseline_error(
    cli: &Cli,
    command: DemoBaselineCommand,
    error: &DemoInstallError,
) -> Result<()> {
    if cli.json {
        let value = serde_json::json!({
            "error": {
                "code": error.code,
                "exit_code": EXIT_BLOCKING,
                "message": error.message,
                "command": error.command,
                "stderr": error.stderr
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!(
            "kply error: {}\n\n{}",
            command.command_name(),
            error.message
        );
        if !error.command.is_empty() {
            eprintln!("command: {}", error.command.join(" "));
        }
        if !error.stderr.is_empty() {
            eprintln!("stderr: {}", error.stderr);
        }
    }

    Ok(())
}
