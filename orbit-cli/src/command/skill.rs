use clap::{Args, Subcommand};
use orbit_core::command::skill::{SkillDoctorResult, SkillDoctorStatus};
use orbit_core::skill_catalog::LoadedSkill;
use orbit_core::{OrbitError, OrbitRuntime};
use serde_json::{Value, json};

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
    List(SkillListArgs),
    Show(SkillShowArgs),
    Doctor(SkillDoctorArgs),
}

impl Execute for SkillSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            SkillSubcommand::List(args) => args.execute(runtime),
            SkillSubcommand::Show(args) => args.execute(runtime),
            SkillSubcommand::Doctor(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct SkillListArgs {
    #[arg(long)]
    pub json: bool,
}

impl Execute for SkillListArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let skills = runtime.list_file_skills()?;
        if self.json {
            let values = skills.iter().map(skill_summary_json).collect::<Vec<_>>();
            crate::output::json::print_pretty(&Value::Array(values))
        } else {
            println!("{:<24} {:<10} {:<8} SUMMARY", "ID", "HASH", "TAGS");
            for skill in skills {
                let summary = skill
                    .meta
                    .as_ref()
                    .and_then(|meta| meta.summary.clone())
                    .unwrap_or_default();
                let tags = skill.meta.as_ref().map(|meta| meta.tags.len()).unwrap_or(0);
                println!(
                    "{:<24} {:<10} {:<8} {}",
                    skill.id,
                    &skill.content_hash[..10],
                    tags,
                    summary
                );
            }
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct SkillShowArgs {
    pub name: String,
    #[arg(long)]
    pub json: bool,
}

impl Execute for SkillShowArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let skill = runtime.show_file_skill(&self.name)?;
        if self.json {
            crate::output::json::print_pretty(&skill_to_json(&skill))
        } else {
            println!("Skill:         {}", skill.id);
            println!("Path:          {}", skill.path.display());
            println!("Content hash:  {}", skill.content_hash);
            println!("\nBehavioral Contract (SKILL.md):");
            println!("{}", skill.content);
            println!("\nStructured Metadata (meta.json):");
            match &skill.meta_raw {
                Some(value) => println!(
                    "{}",
                    serde_json::to_string_pretty(value)
                        .map_err(|e| OrbitError::Execution(e.to_string()))?
                ),
                None => println!("(none)"),
            }
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct SkillDoctorArgs {
    #[arg(long)]
    pub json: bool,
}

impl Execute for SkillDoctorArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let rows = runtime.doctor_file_skills()?;
        if self.json {
            let values = rows.iter().map(doctor_row_json).collect::<Vec<_>>();
            return crate::output::json::print_pretty(&Value::Array(values));
        }

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
}

fn skill_summary_json(skill: &LoadedSkill) -> Value {
    json!({
        "id": skill.id,
        "content_hash": skill.content_hash,
        "path": skill.path,
        "meta": skill.meta,
    })
}

fn skill_to_json(skill: &LoadedSkill) -> Value {
    json!({
        "id": skill.id,
        "path": skill.path,
        "content_hash": skill.content_hash,
        "content": skill.content,
        "sections": {
            "purpose": skill.sections.purpose,
            "behavioral_constraints": skill.sections.behavioral_constraints,
            "output_requirements": skill.sections.output_requirements,
            "evaluation_focus": skill.sections.evaluation_focus,
            "prohibitions": skill.sections.prohibitions,
            "examples": skill.sections.examples,
        },
        "meta": skill.meta,
        "meta_raw": skill.meta_raw,
        "output_schema": skill.output_schema,
    })
}

fn doctor_row_json(row: &SkillDoctorResult) -> Value {
    json!({
        "skill_id": row.skill_name,
        "status": match row.status {
            SkillDoctorStatus::Ok => "ok",
            SkillDoctorStatus::Warning => "warning",
            SkillDoctorStatus::Error => "error",
        },
        "message": row.message,
    })
}
