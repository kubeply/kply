//! Command-line entrypoint for the Kply placeholder CLI.

use anyhow::Result;
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use kply_cli::cli::{Cli, Command};
use std::ffi::OsString;
use std::process::ExitCode;

const EXIT_USAGE: i32 = 2;
const EXIT_INTERNAL: i32 = 3;

fn main() -> ExitCode {
    match run() {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("kply error: internal\n\n{error:#}");
            exit_code(EXIT_INTERNAL)
        }
    }
}

fn run() -> Result<ExitCode> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let wants_json = args_have_flag(&args, "--json");
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(error) => return render_parse_error(error, wants_json),
    };

    print_verbose_trace(&cli);

    match cli.command {
        Some(Command::Help) => {
            Cli::command().print_help()?;
            println!();
            return Ok(ExitCode::SUCCESS);
        }
        Some(command) => {
            if cli.json {
                let value = serde_json::json!({
                    "command": command.name(),
                    "status": "placeholder"
                });
                println!("{}", serde_json::to_string_pretty(&value)?);
            } else {
                if !cli.quiet {
                    println!("kply {}", command.name());
                    println!("Command group is defined but behavior is intentionally pending.");
                }
            }
            return Ok(ExitCode::SUCCESS);
        }
        None => {}
    }

    if cli.version {
        if cli.json {
            let value = serde_json::json!({
                "name": "kply",
                "version": env!("CARGO_PKG_VERSION")
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
            return Ok(ExitCode::SUCCESS);
        }

        println!("kply {}", env!("CARGO_PKG_VERSION"));
        return Ok(ExitCode::SUCCESS);
    }

    if cli.json {
        let value = serde_json::json!({
            "name": "kply",
            "version": env!("CARGO_PKG_VERSION"),
            "status": "placeholder"
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if !cli.quiet {
        println!("kply {}", env!("CARGO_PKG_VERSION"));
        println!("Placeholder CLI. Roadmap and commands are intentionally pending.");
    }

    Ok(ExitCode::SUCCESS)
}

/// Render Clap parse results through Kply's stable exit-code contract.
fn render_parse_error(error: clap::Error, wants_json: bool) -> Result<ExitCode> {
    match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            print!("{error}");
            Ok(ExitCode::SUCCESS)
        }
        _ => {
            if wants_json {
                render_json_usage_error(&error)?;
            } else {
                eprintln!("kply error: usage\n\n{error}");
            }
            Ok(exit_code(EXIT_USAGE))
        }
    }
}

/// Return true when a raw argument list includes a boolean flag.
fn args_have_flag(args: &[OsString], flag: &str) -> bool {
    args.iter().skip(1).any(|arg| arg == flag)
}

/// Render a usage error as structured JSON for agents.
fn render_json_usage_error(error: &clap::Error) -> Result<()> {
    let details = error.to_string();
    let message = details.lines().next().unwrap_or("usage error");
    let value = serde_json::json!({
        "error": {
            "code": "usage",
            "exit_code": EXIT_USAGE,
            "message": message,
            "details": details
        }
    });

    eprintln!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

/// Convert documented small integer exit codes into process exit codes.
fn exit_code(code: i32) -> ExitCode {
    let Ok(code) = u8::try_from(code) else {
        return ExitCode::from(EXIT_INTERNAL as u8);
    };
    ExitCode::from(code)
}

/// Print deterministic debug context when verbose mode is enabled.
fn print_verbose_trace(cli: &Cli) {
    if !cli.verbose {
        return;
    }

    let command = cli.command.map_or("<none>", |command| command.name());
    eprintln!(
        "debug: command={command} json={} quiet={} no_color={}",
        cli.json, cli.quiet, cli.no_color
    );
}
