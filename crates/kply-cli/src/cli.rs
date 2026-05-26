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
    Session {
        /// Optional session command.
        #[command(subcommand)]
        command: Option<SessionCommand>,
    },
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
    /// Manage the local Kply demo.
    Demo {
        /// Optional demo command.
        #[command(subcommand)]
        command: Option<DemoCommand>,
    },
    /// Generate future shell completion scripts.
    Completion,
    /// Read future session reports.
    Report,
}

impl Command {
    /// Placeholder command groups that intentionally have no behavior yet.
    pub const PLACEHOLDER_GROUPS: &'static [Self] = &[
        Self::Session { command: None },
        Self::App { command: None },
        Self::Config { command: None },
        Self::Cluster { command: None },
        Self::Demo { command: None },
        Self::Completion,
        Self::Report,
    ];

    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Session { .. } => "session",
            Self::App { .. } => "app",
            Self::Config { .. } => "config",
            Self::Cluster { .. } => "cluster",
            Self::Demo { .. } => "demo",
            Self::Completion => "completion",
            Self::Report => "report",
        }
    }
}

/// Sandbox session commands.
#[derive(Clone, Debug, Subcommand)]
pub enum SessionCommand {
    /// Plan a future sandbox session for one configured app.
    Plan {
        /// Configured app name to plan.
        app: String,
        /// Candidate image reference for the sandbox workload.
        #[arg(long, value_name = "IMAGE")]
        image: Option<String>,
        /// Namespace override for the planned sandbox resources.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
        /// Lifetime for the planned sandbox session.
        #[arg(long = "ttl", value_name = "DURATION")]
        time_to_live: Option<String>,
        /// Route strategy override for the planned sandbox session.
        #[arg(long, value_name = "STRATEGY")]
        route_strategy: Option<String>,
    },
}

impl SessionCommand {
    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Plan { .. } => "plan",
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
    /// Print one configured application graph.
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

/// Local demo commands.
#[derive(Clone, Copy, Debug, Subcommand)]
pub enum DemoCommand {
    /// Check local prerequisites for the demo.
    Doctor,
    /// Install the baseline local demo resources.
    Install,
    /// Reset the local demo resources to the baseline state.
    Reset,
    /// Tear down the local demo namespace.
    Teardown,
}

impl DemoCommand {
    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Doctor => "doctor",
            Self::Install => "install",
            Self::Reset => "reset",
            Self::Teardown => "teardown",
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
