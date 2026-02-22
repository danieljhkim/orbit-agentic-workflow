use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};
use serde_json::{Map, Value};

use crate::command::Execute;

#[derive(Args)]
pub struct ToolCommand {
    #[command(subcommand)]
    pub command: ToolSubcommand,
}

impl Execute for ToolCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum ToolSubcommand {
    Run(ToolRunArgs),
}

impl Execute for ToolSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            ToolSubcommand::Run(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct ToolRunArgs {
    pub name: String,
    #[arg(long)]
    pub path: Option<String>,
    #[arg(long)]
    pub content: Option<String>,
    #[arg(long)]
    pub program: Option<String>,
    #[arg(long = "arg")]
    pub args: Vec<String>,
    #[arg(long)]
    pub timeout_ms: Option<u64>,
}

impl Execute for ToolRunArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let mut input = Map::new();
        if let Some(path) = self.path {
            input.insert("path".to_string(), Value::String(path));
        }
        if let Some(content) = self.content {
            input.insert("content".to_string(), Value::String(content));
        }
        if let Some(program) = self.program {
            input.insert("program".to_string(), Value::String(program));
        }
        if !self.args.is_empty() {
            input.insert(
                "args".to_string(),
                Value::Array(self.args.into_iter().map(Value::String).collect()),
            );
        }
        if let Some(timeout_ms) = self.timeout_ms {
            input.insert("timeout_ms".to_string(), Value::Number(timeout_ms.into()));
        }

        let output = runtime.execute_tool_command(&self.name, Value::Object(input))?;
        crate::output::json::print_pretty(&output)
    }
}
