use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};
use serde_json::json;

use crate::command::Execute;

#[derive(Args)]
#[command(about = "Run one-shot Orbit data migrations")]
pub struct MigrateCommand {
    #[command(subcommand)]
    pub command: MigrateSubcommand,
}

impl Execute for MigrateCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum MigrateSubcommand {
    /// Convert legacy friction tasks into .orbit/frictions records
    Frictions(MigrateFrictionsArgs),
}

impl Execute for MigrateSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            MigrateSubcommand::Frictions(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct MigrateFrictionsArgs {
    /// Output JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for MigrateFrictionsArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let summary = runtime.migrate_legacy_frictions()?;
        if self.json {
            return crate::output::json::print_pretty(&json!({
                "created": summary.created,
                "skipped": summary.skipped,
            }));
        }
        println!(
            "migrated legacy frictions: {} created, {} skipped",
            summary.created, summary.skipped
        );
        Ok(())
    }
}
