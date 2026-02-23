use orbit_store::ExecutionSpecInsertParams;
use orbit_types::{ExecutionSpec, OrbitError, OrbitEvent};
use serde_json::Value;

use crate::OrbitRuntime;

pub struct ExecutionSpecAddParams {
    pub id: String,
    pub spec_type: String,
    pub description: String,
    pub input_schema_json: Value,
    pub output_schema_json: Value,
    pub artifact_path_template: Option<String>,
    pub skill_refs: Vec<String>,
}

impl OrbitRuntime {
    pub fn add_execution_spec(
        &self,
        params: ExecutionSpecAddParams,
    ) -> Result<ExecutionSpec, OrbitError> {
        validate_execution_spec_params(&params)?;

        self.with_mutation(|tx| {
            let spec = tx.insert_execution_spec(&ExecutionSpecInsertParams {
                id: params.id.clone(),
                spec_type: params.spec_type.clone(),
                description: params.description.clone(),
                input_schema_json: params.input_schema_json.clone(),
                output_schema_json: params.output_schema_json.clone(),
                artifact_path_template: params.artifact_path_template.clone(),
                skill_refs: params.skill_refs.clone(),
            })?;
            Ok((spec.clone(), OrbitEvent::ExecutionSpecAdded { id: spec.id }))
        })
    }

    pub fn list_execution_specs(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<ExecutionSpec>, OrbitError> {
        self.context.store.list_execution_specs(include_inactive)
    }

    pub fn show_execution_spec(&self, id: &str) -> Result<ExecutionSpec, OrbitError> {
        self.context
            .store
            .get_execution_spec(id)?
            .ok_or_else(|| OrbitError::ExecutionSpecNotFound(id.to_string()))
    }

    pub fn delete_execution_spec(&self, id: &str) -> Result<(), OrbitError> {
        self.with_mutation(|tx| {
            let changed = tx.disable_execution_spec(id)?;
            if !changed {
                return Err(OrbitError::ExecutionSpecNotFound(id.to_string()));
            }
            Ok(((), OrbitEvent::ExecutionSpecDisabled { id: id.to_string() }))
        })
    }
}

fn validate_execution_spec_params(params: &ExecutionSpecAddParams) -> Result<(), OrbitError> {
    if params.id.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "execution spec id must not be empty".to_string(),
        ));
    }
    if params.spec_type.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "execution spec type must not be empty".to_string(),
        ));
    }
    if params.description.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "execution spec description must not be empty".to_string(),
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
