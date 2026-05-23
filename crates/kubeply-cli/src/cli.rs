use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, about, disable_version_flag = true)]
pub struct Cli {
    /// Print version information.
    #[arg(long, global = true)]
    pub version: bool,

    /// Print command output as JSON where supported.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Work with safe Kubernetes sessions for agents.
    Session(SessionCommand),
}

#[derive(Debug, Clone, Args)]
pub struct SessionCommand {
    #[command(subcommand)]
    pub command: SessionSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SessionSubcommand {
    /// Create a planned or active sandbox session.
    Create(SessionCreateArgs),
}

#[derive(Debug, Clone, Args)]
pub struct SessionCreateArgs {
    /// Kubernetes workload name.
    pub workload: String,

    /// Kubernetes namespace for the workload.
    #[arg(long, default_value = "default")]
    pub namespace: String,

    /// Proposed sandbox image.
    #[arg(long)]
    pub image: String,

    /// Header name used to route agent/test traffic.
    #[arg(long)]
    pub route_header: Option<String>,

    /// Header value used to route agent/test traffic.
    #[arg(long)]
    pub route_value: Option<String>,

    /// Plan the session without creating Kubernetes resources.
    #[arg(long)]
    pub dry_run: bool,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command, SessionSubcommand};

    #[test]
    fn parses_session_create() {
        let cli = Cli::try_parse_from([
            "kubeply",
            "session",
            "create",
            "backend-api",
            "--namespace",
            "shop",
            "--image",
            "ghcr.io/acme/backend:fix",
            "--route-header",
            "x-kubeply-session",
            "--route-value",
            "fix-123",
            "--dry-run",
        ])
        .expect("session create should parse");

        let Some(Command::Session(session)) = cli.command else {
            panic!("expected session command");
        };
        let SessionSubcommand::Create(args) = session.command;
        assert_eq!(args.workload, "backend-api");
        assert_eq!(args.namespace, "shop");
        assert!(args.dry_run);
    }
}
