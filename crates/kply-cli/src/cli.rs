//! CLI argument placeholders for the future Kply command surface.

use clap::{Parser, Subcommand, ValueEnum};
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
    /// Run verification checks for sandbox sessions.
    Check {
        /// Optional check command.
        #[command(subcommand)]
        command: Option<CheckCommand>,
    },
    /// Plan and manage temporary route changes.
    Route {
        /// Optional route command.
        #[command(subcommand)]
        command: Option<RouteCommand>,
    },
    /// Manage the local Kply demo.
    Demo {
        /// Required demo command.
        #[command(subcommand)]
        command: DemoCommand,
    },
    /// Generate future shell completion scripts.
    Completion,
    /// Read future session reports.
    Report {
        /// Optional report command.
        #[command(subcommand)]
        command: Option<ReportCommand>,
    },
}

impl Command {
    /// Placeholder command groups that intentionally have no behavior yet.
    pub const PLACEHOLDER_GROUPS: &'static [Self] = &[
        Self::Session { command: None },
        Self::App { command: None },
        Self::Config { command: None },
        Self::Cluster { command: None },
        Self::Check { command: None },
        Self::Route { command: None },
        Self::Completion,
        Self::Report { command: None },
    ];

    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Session { .. } => "session",
            Self::App { .. } => "app",
            Self::Config { .. } => "config",
            Self::Cluster { .. } => "cluster",
            Self::Check { .. } => "check",
            Self::Route { .. } => "route",
            Self::Demo { .. } => "demo",
            Self::Completion => "completion",
            Self::Report { .. } => "report",
        }
    }
}

/// Session report commands.
#[derive(Clone, Debug, Subcommand)]
pub enum ReportCommand {
    /// Show the report for one sandbox session.
    Show {
        /// Session id to inspect.
        session: String,
        /// Namespace containing the Kply sandbox session.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
    },
    /// Export the report for one sandbox session.
    Export {
        /// Session id to export.
        session: String,
        /// Namespace containing the Kply sandbox session.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
        /// Report export format.
        #[arg(long, value_enum, value_name = "FORMAT")]
        format: ReportExportFormat,
    },
}

impl ReportCommand {
    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Show { .. } => "show",
            Self::Export { .. } => "export",
        }
    }
}

/// Supported session report export formats.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ReportExportFormat {
    /// Export the report as JSON.
    Json,
}

/// Temporary route commands.
#[derive(Clone, Debug, Subcommand)]
pub enum RouteCommand {
    /// Plan temporary routing for one sandbox session.
    Plan {
        /// Session id to route.
        session: String,
        /// Namespace containing the Kply sandbox session.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
    },
    /// Apply temporary routing for one sandbox session.
    Apply {
        /// Session id to route.
        session: String,
        /// Namespace containing the Kply sandbox session.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
        /// Confirm that temporary route mutation is intended.
        #[arg(long)]
        confirm_route_mutation: bool,
    },
    /// Plan cleanup of temporary routing for one sandbox session.
    Cleanup {
        /// Session id to clean up.
        session: String,
        /// Namespace containing the Kply sandbox session.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
    },
}

impl RouteCommand {
    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Plan { .. } => "plan",
            Self::Apply { .. } => "apply",
            Self::Cleanup { .. } => "cleanup",
        }
    }
}

/// Verification check commands.
#[derive(Clone, Debug, Subcommand)]
pub enum CheckCommand {
    /// Run verification checks for one sandbox session.
    Run {
        /// Session id to verify.
        session: String,
        /// Namespace containing the Kply sandbox session.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
    },
}

impl CheckCommand {
    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Run { .. } => "run",
        }
    }
}

/// Sandbox session commands.
#[derive(Clone, Debug, Subcommand)]
pub enum SessionCommand {
    /// Plan cleanup of one sandbox session.
    Cleanup {
        /// Session id to clean up.
        session: String,
        /// Delete matching sandbox resources from the cluster.
        #[arg(long)]
        apply: bool,
        /// List matching sandbox resources without deleting them.
        #[arg(long)]
        dry_run: bool,
        /// Namespace containing the Kply sandbox session.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
    },
    /// List sandbox sessions recorded in cluster metadata.
    List {
        /// Namespace to inspect for Kply sandbox sessions.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
    },
    /// Show one sandbox session recorded in cluster metadata.
    Status {
        /// Session id to inspect.
        session: String,
        /// Namespace to inspect for the Kply sandbox session.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
    },
    /// Plan creation of sandbox resources for one configured app.
    Create {
        /// Configured app name to create a session for.
        app: String,
        /// Apply sandbox resources to the cluster instead of rendering a dry-run plan.
        #[arg(long)]
        apply: bool,
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
    /// Generate sandbox Kubernetes manifests for one configured app.
    Manifests {
        /// Configured app name to generate manifests for.
        app: String,
        /// Print generated Kubernetes manifests as a YAML stream.
        #[arg(long, conflicts_with = "json")]
        yaml: bool,
        /// Candidate image reference for the sandbox workload.
        #[arg(long, value_name = "IMAGE")]
        image: Option<String>,
        /// Namespace override for the generated sandbox resources.
        #[arg(long, value_name = "NAMESPACE")]
        namespace: Option<String>,
        /// Lifetime for the generated sandbox session.
        #[arg(long = "ttl", value_name = "DURATION")]
        time_to_live: Option<String>,
        /// Route strategy override for the generated sandbox session.
        #[arg(long, value_name = "STRATEGY")]
        route_strategy: Option<String>,
    },
}

impl SessionCommand {
    /// Return the stable command name used in CLI output.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Cleanup { .. } => "cleanup",
            Self::List { .. } => "list",
            Self::Status { .. } => "status",
            Self::Create { .. } => "create",
            Self::Plan { .. } => "plan",
            Self::Manifests { .. } => "manifests",
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
