use std::path::PathBuf;

use clap::{Args, Subcommand};
use orbit_core::command::design::{check_fails, check_workspace_path, format_check_report};
use orbit_core::{OrbitError, OrbitRuntime};

use crate::command::Execute;

#[derive(Args)]
#[command(about = "Design doc operations")]
pub struct DesignCommand {
    #[command(subcommand)]
    pub command: DesignSubcommand,
}

#[derive(Subcommand)]
pub enum DesignSubcommand {
    /// Check design docs for decay against referenced code
    Check(DesignCheckArgs),
}

#[derive(Args)]
pub struct DesignCheckArgs {
    /// Exit 0 even if stale docs are found
    #[arg(long)]
    pub warn_only: bool,
    /// Also fail when docs reference files that no longer exist
    #[arg(long)]
    pub include_missing: bool,
    /// Override the workspace root containing docs/design/
    #[arg(long)]
    pub workspace: Option<PathBuf>,
}

impl Execute for DesignCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self.command {
            DesignSubcommand::Check(args) => check(runtime, args),
        }
    }
}

fn check(_runtime: &OrbitRuntime, args: DesignCheckArgs) -> Result<(), OrbitError> {
    let workspace = match args.workspace {
        Some(workspace) => workspace,
        None => std::env::current_dir().map_err(|error| OrbitError::Io(error.to_string()))?,
    };
    let report = check_workspace_path(&workspace, args.include_missing)?;
    let output = format_check_report(&report);
    if !output.is_empty() {
        print!("{output}");
    }
    if check_fails(&report, args.warn_only, args.include_missing) {
        return Err(OrbitError::Execution(
            "design docs are stale; update Last updated or pass --warn-only".to_string(),
        ));
    }
    Ok(())
}
