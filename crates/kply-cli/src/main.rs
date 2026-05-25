//! Command-line entrypoint for the Kply placeholder CLI.

use anyhow::Result;
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use kply_cli::cli::{Cli, Command, ConfigCommand};
use kply_config::{ConfigLoadError, KplyConfig, load_config_path};
use std::ffi::OsString;
use std::process::ExitCode;

const EXIT_USAGE: i32 = 2;
const EXIT_INTERNAL: i32 = 3;
const EXIT_BLOCKING: i32 = 1;

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

    match &cli.command {
        Some(Command::Help) => {
            Cli::command().print_help()?;
            println!();
            return Ok(ExitCode::SUCCESS);
        }
        Some(Command::Config {
            command: Some(ConfigCommand::Show),
        }) => return render_config_show(&cli),
        Some(Command::Config {
            command: Some(ConfigCommand::Validate),
        }) => return render_config_validate(&cli),
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

/// Render the currently resolved configuration.
fn render_config_show(cli: &Cli) -> Result<ExitCode> {
    let config = match resolved_config(cli) {
        Ok(config) => config,
        Err(error) => return render_config_load_error(&error, cli.json),
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else if !cli.quiet {
        println!("kply config show");
        println!("{}", serde_json::to_string_pretty(&config)?);
    }

    Ok(ExitCode::SUCCESS)
}

/// Validate the currently resolved configuration.
fn render_config_validate(cli: &Cli) -> Result<ExitCode> {
    let config = match resolved_config(cli) {
        Ok(config) => config,
        Err(error) => return render_config_load_error(&error, cli.json),
    };

    match config.validate() {
        Ok(()) => {
            if cli.json {
                let value = serde_json::json!({
                    "status": "valid",
                    "errors": []
                });
                println!("{}", serde_json::to_string_pretty(&value)?);
            } else if !cli.quiet {
                println!("kply config validate");
                println!("Config is valid.");
            }

            Ok(ExitCode::SUCCESS)
        }
        Err(errors) => {
            if cli.json {
                let value = serde_json::json!({
                    "status": "invalid",
                    "errors": errors.errors().iter().map(ToString::to_string).collect::<Vec<_>>()
                });
                eprintln!("{}", serde_json::to_string_pretty(&value)?);
            } else {
                eprintln!("kply error: config validation\n\n{errors}");
            }

            Ok(exit_code(EXIT_BLOCKING))
        }
    }
}

/// Render config file load errors as user-facing config errors.
fn render_config_load_error(error: &ConfigLoadError, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let message = error.to_string();
        let value = serde_json::json!({
            "error": {
                "code": "config",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: config\n\n{error}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Resolve the configuration used by config commands.
///
/// If `--config` is provided, load that explicit file with [`load_config_path`].
/// Otherwise, return the default in-memory config shape. Automatic config file
/// discovery is intentionally not wired into CLI behavior yet.
fn resolved_config(cli: &Cli) -> std::result::Result<KplyConfig, ConfigLoadError> {
    if let Some(path) = &cli.config {
        return load_config_path(path);
    }

    Ok(KplyConfig::default())
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

    let command = cli
        .command
        .as_ref()
        .map_or("<none>", |command| command.name());
    let config = cli
        .config
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<none>".to_owned());
    eprintln!(
        "debug: command={command} json={} quiet={} no_color={} config={} no_config={}",
        cli.json, cli.quiet, cli.no_color, config, cli.no_config
    );
}
