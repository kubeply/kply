//! CLI argument placeholders for the future Kply command surface.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

    /// Path to an explicit configuration file, stored as a [`PathBuf`].
    #[arg(
        long,
        value_name = "PATH",
        global = true,
        help = "Specify an explicit configuration file path"
    )]
    pub config: Option<PathBuf>,

    /// Disable configuration discovery and loading.
    #[arg(long, global = true, conflicts_with = "config")]
    pub no_config: bool,

    /// Optional top-level command.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Top-level placeholder CLI commands.
#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Print top-level help.
    Help,
    /// Manage future sandbox sessions.
    Session,
    /// Inspect configured application targets.
    App {
        /// Optional application command.
        #[command(subcommand)]
        command: Option<AppCommand>,
    },
    /// Manage Kply configuration.
    Config {
        /// Optional configuration command.
        #[command(subcommand)]
        command: Option<ConfigCommand>,
    },
    /// Inspect Kubernetes cluster facts.
    Cluster {
        /// Optional cluster command.
        #[command(subcommand)]
        command: Option<ClusterCommand>,
    },
    /// Generate future shell completion scripts.
    Completion,
    /// Read future session reports.
    Report,
}

impl Command {
    /// Placeholder command groups that intentionally have no behavior yet.
    pub const PLACEHOLDER_GROUPS: &'static [Self] = &[
        Self::Session,
        Self::App { command: None },
        Self::Config { command: None },
        Self::Cluster { command: None },
        Self::Completion,
        Self::Report,
    ];

    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Session => "session",
            Self::App { .. } => "app",
            Self::Config { .. } => "config",
            Self::Cluster { .. } => "cluster",
            Self::Completion => "completion",
            Self::Report => "report",
        }
    }
}

/// Application target commands.
#[derive(Clone, Debug, Subcommand)]
pub enum AppCommand {
    /// List configured application targets.
    List,
    /// Inspect one configured application target.
    Inspect {
        /// Configured app name to inspect.
        app: String,
    },
    /// Print one configured application graph as JSON.
    Graph {
        /// Configured app name to graph.
        app: String,
    },
}

impl AppCommand {
    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Inspect { .. } => "inspect",
            Self::Graph { .. } => "graph",
        }
    }
}

/// Kubernetes cluster commands.
#[derive(Clone, Copy, Debug, Subcommand)]
pub enum ClusterCommand {
    /// Show read-only cluster facts from kubeconfig.
    Info,
}

impl ClusterCommand {
    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Info => "info",
        }
    }
}

/// Configuration commands.
#[derive(Clone, Copy, Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show the resolved Kply configuration.
    Show,
    /// Validate the resolved Kply configuration.
    Validate,
}

impl ConfigCommand {
    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Validate => "validate",
        }
    }
}
