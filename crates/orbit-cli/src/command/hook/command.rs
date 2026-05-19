use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};

use crate::command::Execute;

use super::pretooluse::PretooluseArgs;

#[derive(Args)]
#[command(about = "Run Orbit-owned editor hooks")]
pub struct HookCommand {
    #[command(subcommand)]
    pub command: HookSubcommand,
}

impl Execute for HookCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum HookSubcommand {
    /// Inject project-learning reminders for Claude Code PreToolUse hooks
    #[command(name = "pretooluse")]
    Pretooluse(PretooluseArgs),
}

impl Execute for HookSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            HookSubcommand::Pretooluse(args) => args.execute(runtime),
        }
    }
}
