mod cli;

use anyhow::{Result, bail};
use clap::Parser;
use cli::{Cli, Command, SessionSubcommand};
use kply_checks::default_session_checks;
use kply_core::{RouteHeader, SessionPlan, WorkloadRef, render_human_plan};

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.version {
        if cli.json {
            let value = serde_json::json!({
                "name": "kply",
                "version": env!("CARGO_PKG_VERSION"),
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        } else {
            println!("kply {}", env!("CARGO_PKG_VERSION"));
        }
        return Ok(());
    }

    match cli.command {
        Some(Command::Session(session)) => match session.command {
            SessionSubcommand::Create(args) => {
                let route_header = match (args.route_header, args.route_value) {
                    (Some(name), Some(value)) => Some(RouteHeader::new(name, value)),
                    (None, None) => None,
                    (Some(_), None) => bail!("--route-value is required with --route-header"),
                    (None, Some(_)) => bail!("--route-header is required with --route-value"),
                };

                let plan = SessionPlan::new(
                    WorkloadRef::new(args.namespace, args.workload),
                    args.image,
                    route_header,
                    default_session_checks(),
                    args.dry_run,
                );

                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&plan)?);
                } else {
                    print!("{}", render_human_plan(&plan));
                }
            }
        },
        None => {
            println!("kply {}", env!("CARGO_PKG_VERSION"));
            println!("Run `kply --help` for usage.");
        }
    }

    Ok(())
}
