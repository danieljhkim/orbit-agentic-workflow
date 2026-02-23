use clap::{Args, Subcommand};
use orbit_core::command::workflow::WorkflowAddParams;
use orbit_core::{OrbitError, OrbitRuntime, Workflow};
use serde_json::{Value, json};

use crate::command::Execute;

#[derive(Args)]
pub struct WorkflowCommand {
    #[command(subcommand)]
    pub command: WorkflowSubcommand,
}

impl Execute for WorkflowCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum WorkflowSubcommand {
    Add(WorkflowAddArgs),
    List(WorkflowListArgs),
    Show(WorkflowShowArgs),
    Delete(WorkflowDeleteArgs),
}

impl Execute for WorkflowSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            WorkflowSubcommand::Add(args) => args.execute(runtime),
            WorkflowSubcommand::List(args) => args.execute(runtime),
            WorkflowSubcommand::Show(args) => args.execute(runtime),
            WorkflowSubcommand::Delete(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct WorkflowAddArgs {
    #[arg(long)]
    pub id: String,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub definition_json: String,
    #[arg(long)]
    pub json: bool,
}

impl Execute for WorkflowAddArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let definition_json = parse_json_object(&self.definition_json)?;
        let workflow = runtime.add_workflow(WorkflowAddParams {
            id: self.id,
            name: self.name,
            definition_json,
        })?;

        if self.json {
            crate::output::json::print_pretty(&workflow_to_json(&workflow))
        } else {
            println!("{}", workflow.id);
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct WorkflowListArgs {
    #[arg(long)]
    pub all: bool,
    #[arg(long)]
    pub json: bool,
}

impl Execute for WorkflowListArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let workflows = runtime.list_workflows(self.all)?;
        if self.json {
            let values = workflows.iter().map(workflow_to_json).collect::<Vec<_>>();
            crate::output::json::print_pretty(&Value::Array(values))
        } else {
            println!("{:<24} {:<8} NAME", "ID", "ACTIVE");
            for workflow in &workflows {
                println!(
                    "{:<24} {:<8} {}",
                    workflow.id, workflow.is_active, workflow.name
                );
            }
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct WorkflowShowArgs {
    pub id: String,
    #[arg(long)]
    pub json: bool,
}

impl Execute for WorkflowShowArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let workflow = runtime.show_workflow(&self.id)?;
        if self.json {
            crate::output::json::print_pretty(&workflow_to_json(&workflow))
        } else {
            println!("ID:        {}", workflow.id);
            println!("Name:      {}", workflow.name);
            println!("Active:    {}", workflow.is_active);
            println!("Created:   {}", workflow.created_at.to_rfc3339());
            println!("Updated:   {}", workflow.updated_at.to_rfc3339());
            println!(
                "Definition: {}",
                serde_json::to_string_pretty(&workflow.definition_json).unwrap_or_default()
            );
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct WorkflowDeleteArgs {
    pub id: String,
}

impl Execute for WorkflowDeleteArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        runtime.delete_workflow(&self.id)?;
        println!("Deleted workflow '{}'", self.id);
        Ok(())
    }
}

fn parse_json_object(raw: &str) -> Result<Value, OrbitError> {
    let value = serde_json::from_str::<Value>(raw).map_err(|e| {
        OrbitError::InvalidInput(format!("definition_json must be valid JSON: {e}"))
    })?;
    if !value.is_object() {
        return Err(OrbitError::InvalidInput(
            "definition_json must be a JSON object".to_string(),
        ));
    }
    Ok(value)
}

fn workflow_to_json(workflow: &Workflow) -> Value {
    json!({
        "id": workflow.id,
        "name": workflow.name,
        "definition_json": workflow.definition_json,
        "is_active": workflow.is_active,
        "created_at": workflow.created_at.to_rfc3339(),
        "updated_at": workflow.updated_at.to_rfc3339(),
    })
}
