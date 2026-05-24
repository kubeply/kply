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
    /// Manage future sandbox sessions.
    Session,
    /// Inspect future application targets.
    App,
    /// Manage future Kply configuration.
    Config,
    /// Inspect future cluster capabilities.
    Cluster,
    /// Read future session reports.
    Report,
}

impl Command {
    /// Return the stable command name used in CLI output.
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Session => "session",
            Self::App => "app",
            Self::Config => "config",
            Self::Cluster => "cluster",
            Self::Report => "report",
        }
    }
}
