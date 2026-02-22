use std::fs;

use clap::{Args, Subcommand};
use orbit_core::command::skill::{SkillAddParams, SkillDoctorStatus, SkillUpdateParams};
use orbit_core::{OrbitError, OrbitRuntime, Role};

use crate::command::Execute;

#[derive(Args)]
pub struct SkillCommand {
    #[command(subcommand)]
    pub command: SkillSubcommand,
}

impl Execute for SkillCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum SkillSubcommand {
    Add(SkillAddArgs),
    List,
    Show(SkillShowArgs),
    Update(SkillUpdateArgs),
    Delete(SkillDeleteArgs),
    Attach(SkillAttachArgs),
    Detach(SkillDetachArgs),
    Doctor,
}

impl Execute for SkillSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            SkillSubcommand::Add(args) => args.execute(runtime),
            SkillSubcommand::List => execute_list(runtime),
            SkillSubcommand::Show(args) => args.execute(runtime),
            SkillSubcommand::Update(args) => args.execute(runtime),
            SkillSubcommand::Delete(args) => args.execute(runtime),
            SkillSubcommand::Attach(args) => args.execute(runtime),
            SkillSubcommand::Detach(args) => args.execute(runtime),
            SkillSubcommand::Doctor => execute_doctor(runtime),
        }
    }
}

#[derive(Args)]
pub struct SkillAddArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub description: Option<String>,
    /// Path to instructions file
    #[arg(long)]
    pub instructions: Option<String>,
    /// Comma-separated context file paths
    #[arg(long, default_value = "")]
    pub context: String,
    /// Comma-separated tool names
    #[arg(long = "allowed-tools", default_value = "")]
    pub allowed_tools: String,
    #[arg(long, value_enum, default_value_t = Role::Agent)]
    pub role: Role,
}

impl Execute for SkillAddArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let instructions = read_text_file_opt(self.instructions)?;
        let created = runtime.add_skill(SkillAddParams {
            name: self.name,
            description: self.description,
            instructions,
            context_files: parse_csv(self.context),
            allowed_tools: parse_csv(self.allowed_tools),
            role: self.role,
        })?;
        println!("Added skill '{}'", created.name);
        Ok(())
    }
}

#[derive(Args)]
pub struct SkillShowArgs {
    pub name: String,
}

impl Execute for SkillShowArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let skill = runtime.show_skill(&self.name)?;
        println!("Name:          {}", skill.name);
        println!("Schema:        {}", skill.schema_version);
        println!("Role:          {}", skill.role);
        println!(
            "Description:   {}",
            skill.description.clone().unwrap_or_default()
        );
        println!("Instructions:  {}", skill.instructions);
        println!("Context files: {}", skill.context_files.join(", "));
        println!("Allowed tools: {}", skill.allowed_tools.join(", "));
        Ok(())
    }
}

#[derive(Args)]
pub struct SkillUpdateArgs {
    pub name: String,
    #[arg(long)]
    pub description: Option<String>,
    /// Path to instructions file
    #[arg(long)]
    pub instructions: Option<String>,
    /// Comma-separated context file paths
    #[arg(long)]
    pub context: Option<String>,
    /// Comma-separated tool names
    #[arg(long = "allowed-tools")]
    pub allowed_tools: Option<String>,
    #[arg(long, value_enum)]
    pub role: Option<Role>,
}

impl Execute for SkillUpdateArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let instructions = match self.instructions {
            Some(path) => Some(read_text_file(path)?),
            None => None,
        };

        let updated = runtime.update_skill(
            &self.name,
            SkillUpdateParams {
                description: self.description.map(Some),
                instructions,
                context_files: self.context.map(parse_csv),
                allowed_tools: self.allowed_tools.map(parse_csv),
                role: self.role,
            },
        )?;
        println!("Updated skill '{}'", updated.name);
        Ok(())
    }
}

#[derive(Args)]
pub struct SkillDeleteArgs {
    pub name: String,
}

impl Execute for SkillDeleteArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        runtime.delete_skill(&self.name)?;
        println!("Deleted skill '{}'", self.name);
        Ok(())
    }
}

#[derive(Args)]
pub struct SkillAttachArgs {
    pub task_id: String,
    pub skill_name: String,
}

impl Execute for SkillAttachArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        runtime.attach_skill_to_task(&self.task_id, &self.skill_name)?;
        println!("Attached skill '{}' to '{}'", self.skill_name, self.task_id);
        Ok(())
    }
}

#[derive(Args)]
pub struct SkillDetachArgs {
    pub task_id: String,
    pub skill_name: String,
}

impl Execute for SkillDetachArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        runtime.detach_skill_from_task(&self.task_id, &self.skill_name)?;
        println!(
            "Detached skill '{}' from '{}'",
            self.skill_name, self.task_id
        );
        Ok(())
    }
}

fn execute_list(runtime: &OrbitRuntime) -> Result<(), OrbitError> {
    let skills = runtime.list_skills()?;
    println!("{:<24} {:<8} {:<8} DESCRIPTION", "NAME", "ROLE", "TOOLS");
    for skill in &skills {
        println!(
            "{:<24} {:<8} {:<8} {}",
            skill.name,
            skill.role.to_string(),
            skill.allowed_tools.len(),
            skill.description.clone().unwrap_or_default()
        );
    }
    Ok(())
}

fn execute_doctor(runtime: &OrbitRuntime) -> Result<(), OrbitError> {
    let rows = runtime.doctor_skills()?;
    let mut issues = 0usize;
    println!("{:<24} {:<10} DETAILS", "SKILL", "STATUS");
    for row in &rows {
        let status = match row.status {
            SkillDoctorStatus::Ok => "ok",
            SkillDoctorStatus::Warning => "warning",
            SkillDoctorStatus::Error => "ERROR",
        };
        if row.status != SkillDoctorStatus::Ok {
            issues += 1;
        }
        println!("{:<24} {:<10} {}", row.skill_name, status, row.message);
    }

    if issues == 0 {
        println!("\nAll skills healthy.");
    } else {
        println!("\n{} issue(s) found.", issues);
    }
    Ok(())
}

fn parse_csv(raw: String) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn read_text_file(path: String) -> Result<String, OrbitError> {
    fs::read_to_string(&path)
        .map_err(|e| OrbitError::InvalidInput(format!("failed to read `{path}`: {e}")))
}

fn read_text_file_opt(path: Option<String>) -> Result<String, OrbitError> {
    match path {
        Some(path) => read_text_file(path),
        None => Ok(String::new()),
    }
}
