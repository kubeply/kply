//! Command-line entrypoint for the Kply placeholder CLI.

use anyhow::Result;
use clap::{CommandFactory, Parser};
use kply_cli::cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();

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
