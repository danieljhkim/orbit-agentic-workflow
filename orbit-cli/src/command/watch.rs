use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};

use crate::command::Execute;

#[derive(Args)]
pub struct WatchCommand {
    #[command(subcommand)]
    pub command: WatchSubcommand,
}

impl Execute for WatchCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum WatchSubcommand {
    Run(WatchRunArgs),
}

impl Execute for WatchSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            WatchSubcommand::Run(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct WatchRunArgs {
    #[arg(long, default_value = ".")]
    pub path: String,
}

impl Execute for WatchRunArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        runtime.execute_watch_run_command(&self.path)?;
        crate::output::table::print_line(format!("watch trigger recorded for {}", self.path));
        Ok(())
    }
}
