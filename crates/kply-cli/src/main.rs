//! Command-line entrypoint for the Kply placeholder CLI.

use anyhow::Result;
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use kply_cli::cli::{Cli, Command};
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
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => return Ok(render_parse_error(error)),
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
fn render_parse_error(error: clap::Error) -> ExitCode {
    match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            print!("{error}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("kply error: usage\n\n{error}");
            exit_code(EXIT_USAGE)
        }
    }
}

/// Convert documented small integer exit codes into process exit codes.
fn exit_code(code: i32) -> ExitCode {
    let code = u8::try_from(code).expect("documented exit code should fit in u8");
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
