use orbit_store::WorkInsertParams;
use orbit_types::{OrbitError, OrbitEvent, Work};
use serde_json::Value;

use crate::OrbitRuntime;

pub struct WorkAddParams {
    pub id: String,
    pub spec_type: String,
    pub description: String,
    pub input_schema_json: Value,
    pub output_schema_json: Value,
    pub artifact_path_template: Option<String>,
    pub skill_refs: Vec<String>,
}

impl OrbitRuntime {
    pub fn add_work(&self, params: WorkAddParams) -> Result<Work, OrbitError> {
        validate_work_params(&params)?;

        self.with_mutation(|tx| {
            let spec = tx.insert_work(&WorkInsertParams {
                id: params.id.clone(),
                spec_type: params.spec_type.clone(),
                description: params.description.clone(),
                input_schema_json: params.input_schema_json.clone(),
                output_schema_json: params.output_schema_json.clone(),
                artifact_path_template: params.artifact_path_template.clone(),
                skill_refs: params.skill_refs.clone(),
            })?;
            Ok((spec.clone(), OrbitEvent::WorkAdded { id: spec.id }))
        })
    }

    pub fn list_works(&self, include_inactive: bool) -> Result<Vec<Work>, OrbitError> {
        self.context.store.list_works(include_inactive)
    }

    pub fn show_work(&self, id: &str) -> Result<Work, OrbitError> {
        self.context
            .store
            .get_work(id)?
            .ok_or_else(|| OrbitError::WorkNotFound(id.to_string()))
    }

    pub fn delete_work(&self, id: &str) -> Result<(), OrbitError> {
        self.with_mutation(|tx| {
            let changed = tx.disable_work(id)?;
            if !changed {
                return Err(OrbitError::WorkNotFound(id.to_string()));
            }
            Ok(((), OrbitEvent::WorkDisabled { id: id.to_string() }))
        })
    }
}

fn validate_work_params(params: &WorkAddParams) -> Result<(), OrbitError> {
    if params.id.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "work id must not be empty".to_string(),
        ));
    }
    if params.spec_type.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "work type must not be empty".to_string(),
        ));
    }
    if params.description.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "work description must not be empty".to_string(),
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
