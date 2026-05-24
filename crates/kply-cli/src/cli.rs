//! CLI argument placeholders for the future Kply command surface.

use clap::{Parser, Subcommand};

/// Top-level placeholder CLI options.
#[derive(Debug, Parser)]
#[command(
    author,
    about,
    disable_help_subcommand = true,
    disable_version_flag = true
)]
pub struct Cli {
    /// Print the kply version.
    #[arg(long)]
    pub version: bool,

    /// Print placeholder output as JSON.
    #[arg(long)]
    pub json: bool,

    /// Optional top-level command.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Top-level placeholder CLI commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print top-level help.
    Help,
}
