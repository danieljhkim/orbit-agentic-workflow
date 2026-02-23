use clap::{Args, Subcommand};
use orbit_core::command::execution_spec::ExecutionSpecAddParams;
use orbit_core::{ExecutionSpec, OrbitError, OrbitRuntime};
use serde_json::{Value, json};

use crate::command::Execute;

#[derive(Args)]
pub struct ExecutionSpecCommand {
    #[command(subcommand)]
    pub command: ExecutionSpecSubcommand,
}

impl Execute for ExecutionSpecCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum ExecutionSpecSubcommand {
    Add(ExecutionSpecAddArgs),
    List(ExecutionSpecListArgs),
    Show(ExecutionSpecShowArgs),
    Delete(ExecutionSpecDeleteArgs),
}

impl Execute for ExecutionSpecSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            ExecutionSpecSubcommand::Add(args) => args.execute(runtime),
            ExecutionSpecSubcommand::List(args) => args.execute(runtime),
            ExecutionSpecSubcommand::Show(args) => args.execute(runtime),
            ExecutionSpecSubcommand::Delete(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct ExecutionSpecAddArgs {
    #[arg(long)]
    pub id: String,
    #[arg(long = "type")]
    pub spec_type: String,
    #[arg(long)]
    pub description: String,
    #[arg(long)]
    pub input_schema: String,
    #[arg(long)]
    pub output_schema: String,
    #[arg(long)]
    pub artifact_path_template: Option<String>,
    #[arg(long, default_value = "")]
    pub skill_refs: String,
    #[arg(long)]
    pub json: bool,
}

impl Execute for ExecutionSpecAddArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let input_schema_json = parse_json_object(&self.input_schema, "input_schema")?;
        let output_schema_json = parse_json_object(&self.output_schema, "output_schema")?;
        let skill_refs = parse_csv(&self.skill_refs);

        let spec = runtime.add_execution_spec(ExecutionSpecAddParams {
            id: self.id,
            spec_type: self.spec_type,
            description: self.description,
            input_schema_json,
            output_schema_json,
            artifact_path_template: self.artifact_path_template,
            skill_refs,
        })?;

        if self.json {
            crate::output::json::print_pretty(&execution_spec_to_json(&spec))
        } else {
            println!("{}", spec.id);
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct ExecutionSpecListArgs {
    #[arg(long)]
    pub all: bool,
    #[arg(long)]
    pub json: bool,
}

impl Execute for ExecutionSpecListArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let specs = runtime.list_execution_specs(self.all)?;
        if self.json {
            let values = specs.iter().map(execution_spec_to_json).collect::<Vec<_>>();
            crate::output::json::print_pretty(&Value::Array(values))
        } else {
            println!("{:<24} {:<14} {:<8} DESCRIPTION", "ID", "TYPE", "ACTIVE");
            for spec in &specs {
                println!(
                    "{:<24} {:<14} {:<8} {}",
                    spec.id, spec.spec_type, spec.is_active, spec.description
                );
            }
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct ExecutionSpecShowArgs {
    pub id: String,
    #[arg(long)]
    pub json: bool,
}

impl Execute for ExecutionSpecShowArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let spec = runtime.show_execution_spec(&self.id)?;
        if self.json {
            crate::output::json::print_pretty(&execution_spec_to_json(&spec))
        } else {
            println!("ID:                  {}", spec.id);
            println!("Type:                {}", spec.spec_type);
            println!("Description:         {}", spec.description);
            println!(
                "Artifact Template:   {}",
                spec.artifact_path_template.unwrap_or_default()
            );
            println!("Skill Refs:          {}", spec.skill_refs.join(","));
            println!("Active:              {}", spec.is_active);
            println!("Created:             {}", spec.created_at.to_rfc3339());
            println!("Updated:             {}", spec.updated_at.to_rfc3339());
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct ExecutionSpecDeleteArgs {
    pub id: String,
}

impl Execute for ExecutionSpecDeleteArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        runtime.delete_execution_spec(&self.id)?;
        println!("Deleted work '{}'", self.id);
        Ok(())
    }
}

fn parse_json_object(raw: &str, field: &str) -> Result<Value, OrbitError> {
    let value = serde_json::from_str::<Value>(raw)
        .map_err(|e| OrbitError::InvalidInput(format!("{field} must be valid JSON: {e}")))?;
    if !value.is_object() {
        return Err(OrbitError::InvalidInput(format!(
            "{field} must be a JSON object"
        )));
    }
    Ok(value)
}

fn parse_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn execution_spec_to_json(spec: &ExecutionSpec) -> Value {
    json!({
        "id": spec.id,
        "type": spec.spec_type,
        "description": spec.description,
        "input_schema_json": spec.input_schema_json,
        "output_schema_json": spec.output_schema_json,
        "artifact_path_template": spec.artifact_path_template,
        "skill_refs": spec.skill_refs,
        "is_active": spec.is_active,
        "created_at": spec.created_at.to_rfc3339(),
        "updated_at": spec.updated_at.to_rfc3339(),
    })
}
