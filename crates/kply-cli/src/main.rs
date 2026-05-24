//! Command-line entrypoint for the Kply placeholder CLI.

use anyhow::Result;
use clap::{CommandFactory, Parser};
use kply_cli::cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    print_verbose_trace(&cli);

    match cli.command {
        Some(Command::Help) => {
            Cli::command().print_help()?;
            println!();
            return Ok(());
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
            return Ok(());
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
            return Ok(());
        }

        println!("kply {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
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

    Ok(())
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
