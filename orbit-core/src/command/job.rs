use orbit_store::JobCreateParams as StoreWorkCreateParams;
use orbit_types::{OrbitError, OrbitEvent, Job};
use serde_json::Value;

use crate::OrbitRuntime;

pub struct JobAddParams {
    pub id: String,
    pub spec_type: String,
    pub description: String,
    pub input_schema_json: Value,
    pub output_schema_json: Value,
    pub artifact_path_template: Option<String>,
    pub skill_refs: Vec<String>,
    pub identity_id: Option<String>,
    pub assigned_to: Option<String>,
    pub created_by: Option<String>,
}

impl OrbitRuntime {
    pub fn add_job(&self, params: JobAddParams) -> Result<Job, OrbitError> {
        validate_job_params(&params)?;
        let _ = self.resolve_job_skill_refs(&params.skill_refs)?;
        let identity_id = params.identity_id.clone();
        let mut assigned_to = params.assigned_to.clone();
        let mut created_by = params.created_by.clone();
        if let Some(id) = identity_id.as_ref() {
            let resolved = self.resolve_identity(id)?;
            if assigned_to.is_none() {
                assigned_to = Some(resolved.name.clone());
            }
            if created_by.is_none() {
                created_by = Some(resolved.name);
            }
        }

        let job = self.context.job_store.add_job(StoreWorkCreateParams {
            id: params.id,
            spec_type: params.spec_type,
            description: params.description,
            input_schema_json: params.input_schema_json,
            output_schema_json: params.output_schema_json,
            artifact_path_template: params.artifact_path_template,
            skill_refs: params.skill_refs,
            identity_id,
            assigned_to,
            created_by,
        })?;
        self.record_event(OrbitEvent::JobAdded {
            id: job.id.clone(),
        })?;
        Ok(job)
    }

    pub fn list_jobs(&self, include_inactive: bool) -> Result<Vec<Job>, OrbitError> {
        self.context.job_store.list_jobs(include_inactive)
    }

    pub fn show_job(&self, id: &str) -> Result<Job, OrbitError> {
        self.context
            .job_store
            .get_job(id)?
            .ok_or_else(|| OrbitError::JobNotFound(id.to_string()))
    }

    pub fn delete_job(&self, id: &str) -> Result<(), OrbitError> {
        let changed = self.context.job_store.disable_job(id)?;
        if !changed {
            return Err(OrbitError::JobNotFound(id.to_string()));
        }
        self.record_event(OrbitEvent::JobDisabled { id: id.to_string() })
    }
}

fn validate_job_params(params: &JobAddParams) -> Result<(), OrbitError> {
    if params.id.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "job id must not be empty".to_string(),
        ));
    }
    if params.spec_type.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "job type must not be empty".to_string(),
        ));
    }
    if params.description.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "job description must not be empty".to_string(),
        ));
    }
    if !params.input_schema_json.is_object() {
        return Err(OrbitError::InvalidInput(
            "input schema must be a JSON object".to_string(),
        ));
    }
    if !params.output_schema_json.is_object() {
        return Err(OrbitError::InvalidInput(
            "output schema must be a JSON object".to_string(),
        ));
    }
    if params.skill_refs.iter().any(|v| v.trim().is_empty()) {
        return Err(OrbitError::InvalidInput(
            "skill_refs must not contain empty values".to_string(),
        ));
    }

    Ok(())
}
