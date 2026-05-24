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
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress nonessential human-readable output.
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Print local debugging details to stderr.
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Disable ANSI color output.
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Optional top-level command.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Top-level placeholder CLI commands.
#[derive(Clone, Copy, Debug, Subcommand)]
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
    /// Generate future shell completion scripts.
    Completion,
    /// Read future session reports.
    Report,
}

impl Command {
    /// Placeholder command groups that intentionally have no behavior yet.
    pub const PLACEHOLDER_GROUPS: &'static [Self] = &[
        Self::Session,
        Self::App,
        Self::Config,
        Self::Cluster,
        Self::Completion,
        Self::Report,
    ];

    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Session => "session",
            Self::App => "app",
            Self::Config => "config",
            Self::Cluster => "cluster",
            Self::Completion => "completion",
            Self::Report => "report",
        }
    }
}
