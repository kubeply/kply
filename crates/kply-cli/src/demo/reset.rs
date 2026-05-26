//! Local demo baseline resetter.

use anyhow::Result;
use kply_cli::cli::Cli;
use std::process::ExitCode;

use crate::demo::install::{DemoBaselineCommand, render_demo_baseline};

/// Reset the local demo resources to the baseline state.
pub(crate) fn render_demo_reset(cli: &Cli) -> Result<ExitCode> {
    render_demo_baseline(cli, DemoBaselineCommand::Reset)
}
