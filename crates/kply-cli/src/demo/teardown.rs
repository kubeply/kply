//! Local demo labeled-resource teardown.

use anyhow::Result;
use kply_cli::cli::Cli;
use serde::Serialize;
use std::process::{Command, ExitCode, Output};

use crate::demo::{DEMO_NAMESPACE, find_command_in_path};

const EXIT_BLOCKING: u8 = 1;
const KUBECTL_COMMAND: &str = "kubectl";
const KUBECTL_DELETE_TIMEOUT: &str = "5m";
const DEMO_RESOURCE_SELECTOR: &str = "app.kubernetes.io/part-of=kply-demo";
const DEMO_RESOURCE_TYPES: &str = "deployment,service";

/// Tear down labeled local demo resources in the current Kubernetes context.
pub(crate) fn render_demo_teardown(cli: &Cli) -> Result<ExitCode> {
    let result = teardown_demo();

    match result {
        Ok(report) => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if !cli.quiet {
                println!("kply demo teardown");
                println!("status: {}", report.status);
                println!("namespace: {}", report.namespace);
                println!("deleted_resources: {}", report.deleted_resources.len());
                for resource in &report.deleted_resources {
                    println!("  deleted: {resource}");
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            render_teardown_error(cli, &error)?;
            Ok(ExitCode::from(EXIT_BLOCKING))
        }
    }
}

#[derive(Debug, Serialize)]
struct DemoTeardownReport {
    command: &'static str,
    status: &'static str,
    namespace: &'static str,
    deleted_resources: Vec<String>,
}

#[derive(Debug)]
struct DemoTeardownError {
    code: &'static str,
    message: String,
    command: Vec<String>,
    stderr: String,
}

fn teardown_demo() -> std::result::Result<DemoTeardownReport, DemoTeardownError> {
    if find_command_in_path(KUBECTL_COMMAND).is_none() {
        return Err(DemoTeardownError {
            code: "missing_kubectl",
            message: "kubectl was not found on PATH".to_owned(),
            command: vec![KUBECTL_COMMAND.to_owned()],
            stderr: String::new(),
        });
    }

    run_kubectl(&[
        "-n",
        DEMO_NAMESPACE,
        "delete",
        DEMO_RESOURCE_TYPES,
        "--selector",
        DEMO_RESOURCE_SELECTOR,
        "--ignore-not-found",
        "--wait=true",
        "--timeout",
        KUBECTL_DELETE_TIMEOUT,
    ])?;

    Ok(DemoTeardownReport {
        command: "demo teardown",
        status: "torn_down",
        namespace: DEMO_NAMESPACE,
        deleted_resources: vec![format!(
            "{DEMO_RESOURCE_TYPES} --selector {DEMO_RESOURCE_SELECTOR}"
        )],
    })
}

fn run_kubectl(args: &[&str]) -> std::result::Result<Output, DemoTeardownError> {
    let output = Command::new(KUBECTL_COMMAND)
        .args(args)
        .output()
        .map_err(|error| DemoTeardownError {
            code: "kubectl_exec",
            message: format!("failed to run kubectl: {error}"),
            command: command_preview(args),
            stderr: String::new(),
        })?;

    if output.status.success() {
        return Ok(output);
    }

    Err(DemoTeardownError {
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

fn render_teardown_error(cli: &Cli, error: &DemoTeardownError) -> Result<()> {
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
        eprintln!("kply error: demo teardown\n\n{}", error.message);
        if !error.command.is_empty() {
            eprintln!("command: {}", error.command.join(" "));
        }
        if !error.stderr.is_empty() {
            eprintln!("stderr: {}", error.stderr);
        }
    }

    Ok(())
}
