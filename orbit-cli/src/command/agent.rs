use clap::{Args, Subcommand};
use orbit_core::command::agent::AgentRunOptions;
use orbit_core::{AgentSessionStatus, OrbitError, OrbitRuntime};

use crate::command::Execute;

#[derive(Args)]
pub struct AgentCommand {
    #[command(subcommand)]
    pub command: AgentSubcommand,
}

impl Execute for AgentCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum AgentSubcommand {
    Run(AgentRunArgs),
}

impl Execute for AgentSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            AgentSubcommand::Run(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct AgentRunArgs {
    #[arg(long)]
    pub task: String,
    #[arg(long)]
    pub identity: Option<String>,
}

impl Execute for AgentRunArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let result = runtime.run_agent_task_with_options(
            &self.task,
            AgentRunOptions {
                identity_id: self.identity,
            },
        )?;
        let status = match result.status {
            AgentSessionStatus::Running => "running",
            AgentSessionStatus::Completed => "completed",
            AgentSessionStatus::Failed => "failed",
        };
        println!(
            "session_id={};task_id={};tool_calls_executed={};status={}",
            result.session_id, result.task_id, result.tool_calls_executed, status
        );
        Ok(())
    }
}
