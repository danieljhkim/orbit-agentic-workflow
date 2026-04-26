use clap::Args;
use orbit_core::{OrbitError, OrbitRuntime};

use crate::command::Execute;
use crate::command::run;

#[derive(Args)]
#[command(about = "View execution logs for a job run")]
pub struct LogsCommand {
    /// Run ID to inspect
    pub run_id: String,

    /// Show only a specific step by target_id
    #[arg(long)]
    pub step: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for LogsCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        eprintln!(
            "[deprecated] use \"orbit run show {}\" for the step table or \"orbit run logs {}\" for raw stdout/stderr",
            self.run_id, self.run_id
        );
        run::print_legacy_logs_summary(runtime, &self.run_id, self.step.as_deref(), self.json)
    }
}
