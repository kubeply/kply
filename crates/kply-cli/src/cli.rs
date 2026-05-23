//! CLI argument placeholders for the future Kply command surface.

use clap::Parser;

/// Top-level placeholder CLI options.
#[derive(Debug, Parser)]
#[command(author, about, disable_version_flag = true)]
pub struct Cli {
    /// Print placeholder output as JSON.
    #[arg(long)]
    pub json: bool,
}
