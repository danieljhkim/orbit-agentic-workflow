use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};

use crate::command::Execute;

#[derive(Args)]
pub struct TaskCommand {
    #[command(subcommand)]
    pub command: TaskSubcommand,
}

impl Execute for TaskCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum TaskSubcommand {
    Add(TaskAddArgs),
    List,
}

impl Execute for TaskSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            TaskSubcommand::Add(args) => args.execute(runtime),
            TaskSubcommand::List => {
                for task in runtime.list_tasks()? {
                    crate::output::table::print_line(task.to_string());
                }
                Ok(())
            }
        }
    }
}

#[derive(Args)]
pub struct TaskAddArgs {
    pub title: String,
}

impl Execute for TaskAddArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let task = runtime.add_task(&self.title)?;
        crate::output::table::print_line(task.to_string());
        Ok(())
    }
}
