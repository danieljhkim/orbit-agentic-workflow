use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};

use crate::command::Execute;

#[derive(Args)]
pub struct JobCommand {
    #[command(subcommand)]
    pub command: JobSubcommand,
}

impl Execute for JobCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum JobSubcommand {
    Run,
}

impl Execute for JobSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            JobSubcommand::Run => {
                let count = runtime.execute_job_run_command()?;
                crate::output::table::print_line(format!("ran_jobs={count}"));
                Ok(())
            }
        }
    }
}
