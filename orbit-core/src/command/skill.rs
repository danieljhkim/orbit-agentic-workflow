use std::collections::HashSet;
use std::path::Path;

use chrono::Utc;
use orbit_types::{OrbitError, OrbitEvent, Role, Skill};

use crate::OrbitRuntime;

pub struct SkillAddParams {
    pub name: String,
    pub description: Option<String>,
    pub instructions: String,
    pub context_files: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub role: Role,
}

#[derive(Default)]
pub struct SkillUpdateParams {
    pub description: Option<Option<String>>,
    pub instructions: Option<String>,
    pub context_files: Option<Vec<String>>,
    pub allowed_tools: Option<Vec<String>>,
    pub role: Option<Role>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillDoctorStatus {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct SkillDoctorResult {
    pub skill_name: String,
    pub status: SkillDoctorStatus,
    pub message: String,
}

impl OrbitRuntime {
    pub fn add_skill(&self, params: SkillAddParams) -> Result<Skill, OrbitError> {
        self.validate_skill_fields(
            &params.instructions,
            &params.context_files,
            &params.allowed_tools,
        )?;

        let now = Utc::now();
        let skill = Skill {
            schema_version: 1,
            name: params.name,
            description: params.description,
            instructions: params.instructions,
            context_files: dedup_keep_first(params.context_files),
            allowed_tools: dedup_keep_first(params.allowed_tools),
            role: params.role,
            created_at: now,
            updated_at: now,
        };

        self.with_mutation(|tx| {
            tx.insert_skill(&skill)?;
            Ok((
                skill.clone(),
                OrbitEvent::SkillAdded {
                    name: skill.name.clone(),
                },
            ))
        })
    }

    pub fn list_skills(&self) -> Result<Vec<Skill>, OrbitError> {
        self.context.store.list_skills()
    }

    pub fn show_skill(&self, name: &str) -> Result<Skill, OrbitError> {
        self.context
            .store
            .get_skill(name)?
            .ok_or_else(|| OrbitError::SkillNotFound(name.to_string()))
    }

    pub fn update_skill(&self, name: &str, params: SkillUpdateParams) -> Result<Skill, OrbitError> {
        let existing = self.show_skill(name)?;
        let updated = Skill {
            schema_version: existing.schema_version,
            name: existing.name.clone(),
            description: params.description.unwrap_or(existing.description),
            instructions: params.instructions.unwrap_or(existing.instructions),
            context_files: dedup_keep_first(params.context_files.unwrap_or(existing.context_files)),
            allowed_tools: dedup_keep_first(params.allowed_tools.unwrap_or(existing.allowed_tools)),
            role: params.role.unwrap_or(existing.role),
            created_at: existing.created_at,
            updated_at: Utc::now(),
        };

        self.validate_skill_fields(
            &updated.instructions,
            &updated.context_files,
            &updated.allowed_tools,
        )?;

        self.with_mutation(|tx| {
            let changed = tx.update_skill(&updated)?;
            if !changed {
                return Err(OrbitError::SkillNotFound(name.to_string()));
            }
            Ok((
                updated.clone(),
                OrbitEvent::SkillUpdated {
                    name: updated.name.clone(),
                },
            ))
        })
    }

    pub fn delete_skill(&self, name: &str) -> Result<(), OrbitError> {
        self.with_mutation(|tx| {
            let changed = tx.delete_skill(name)?;
            if !changed {
                return Err(OrbitError::SkillNotFound(name.to_string()));
            }
            Ok((
                (),
                OrbitEvent::SkillDeleted {
                    name: name.to_string(),
                },
            ))
        })
    }

    pub fn attach_skill_to_task(&self, task_id: &str, skill_name: &str) -> Result<(), OrbitError> {
        let _ = self.get_task(task_id)?;
        let _ = self.show_skill(skill_name)?;

        self.with_mutation(|tx| {
            let changed = tx.attach_skill_to_task(task_id, skill_name)?;
            if !changed {
                return Err(OrbitError::InvalidInput(format!(
                    "skill '{skill_name}' already attached to task '{task_id}'"
                )));
            }
            Ok((
                (),
                OrbitEvent::SkillAttached {
                    task_id: task_id.to_string(),
                    skill_name: skill_name.to_string(),
                },
            ))
        })
    }

    pub fn detach_skill_from_task(
        &self,
        task_id: &str,
        skill_name: &str,
    ) -> Result<(), OrbitError> {
        self.with_mutation(|tx| {
            let changed = tx.detach_skill_from_task(task_id, skill_name)?;
            if !changed {
                return Err(OrbitError::InvalidInput(format!(
                    "skill '{skill_name}' is not attached to task '{task_id}'"
                )));
            }
            Ok((
                (),
                OrbitEvent::SkillDetached {
                    task_id: task_id.to_string(),
                    skill_name: skill_name.to_string(),
                },
            ))
        })
    }

    pub fn list_task_skills(&self, task_id: &str) -> Result<Vec<Skill>, OrbitError> {
        let _ = self.get_task(task_id)?;
        self.context.store.list_task_skills(task_id)
    }

    pub fn doctor_skills(&self) -> Result<Vec<SkillDoctorResult>, OrbitError> {
        let skills = self.list_skills()?;
        let mut results = Vec::new();

        for skill in &skills {
            if skill.instructions.trim().is_empty() {
                results.push(SkillDoctorResult {
                    skill_name: skill.name.clone(),
                    status: SkillDoctorStatus::Error,
                    message: "instructions must not be empty".to_string(),
                });
                continue;
            }

            if let Some(missing) = skill
                .context_files
                .iter()
                .find(|path| !Path::new(path.as_str()).exists())
            {
                results.push(SkillDoctorResult {
                    skill_name: skill.name.clone(),
                    status: SkillDoctorStatus::Warning,
                    message: format!("missing context file: {missing}"),
                });
                continue;
            }

            if let Some(missing_tool) = skill
                .allowed_tools
                .iter()
                .find(|name| !self.context.registry.has(name))
            {
                results.push(SkillDoctorResult {
                    skill_name: skill.name.clone(),
                    status: SkillDoctorStatus::Error,
                    message: format!("unknown tool: {missing_tool}"),
                });
                continue;
            }

            results.push(SkillDoctorResult {
                skill_name: skill.name.clone(),
                status: SkillDoctorStatus::Ok,
                message: String::new(),
            });
        }

        Ok(results)
    }

    fn validate_skill_fields(
        &self,
        instructions: &str,
        context_files: &[String],
        allowed_tools: &[String],
    ) -> Result<(), OrbitError> {
        if instructions.trim().is_empty() {
            return Err(OrbitError::SkillValidation(
                "instructions must not be empty".to_string(),
            ));
        }

        if context_files.iter().any(|p| p.trim().is_empty()) {
            return Err(OrbitError::SkillValidation(
                "context_files must not contain empty entries".to_string(),
            ));
        }

        for tool in allowed_tools {
            if !self.context.registry.has(tool) {
                return Err(OrbitError::SkillValidation(format!(
                    "allowed tool '{tool}' is not registered"
                )));
            }
        }

        Ok(())
    }
}

fn dedup_keep_first(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}
