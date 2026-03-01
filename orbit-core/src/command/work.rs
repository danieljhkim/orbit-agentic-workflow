use orbit_store::WorkInsertParams;
use orbit_types::{OrbitError, OrbitEvent, Work};
use serde_json::Value;

use crate::OrbitRuntime;
use crate::config::PersistenceType;
use crate::work_file_store::FileWorkInsert;

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
        let _ = self.resolve_work_skill_refs(&params.skill_refs)?;

        if self.context.work_persistence_type == PersistenceType::File {
            let work_id = params.id.clone();
            self.with_file_mutation(
                || {
                    self.context.work_file_store.insert_work(&FileWorkInsert {
                        id: params.id,
                        spec_type: params.spec_type,
                        description: params.description,
                        input_schema_json: params.input_schema_json,
                        output_schema_json: params.output_schema_json,
                        artifact_path_template: params.artifact_path_template,
                        skill_refs: params.skill_refs,
                    })
                },
                OrbitEvent::WorkAdded { id: work_id },
            )
        } else {
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
    }

    pub fn list_works(&self, include_inactive: bool) -> Result<Vec<Work>, OrbitError> {
        if self.context.work_persistence_type == PersistenceType::File {
            self.context.work_file_store.list_works(include_inactive)
        } else {
            self.context.store.list_works(include_inactive)
        }
    }

    pub fn show_work(&self, id: &str) -> Result<Work, OrbitError> {
        if self.context.work_persistence_type == PersistenceType::File {
            self.context
                .work_file_store
                .get_work(id)?
                .ok_or_else(|| OrbitError::WorkNotFound(id.to_string()))
        } else {
            self.context
                .store
                .get_work(id)?
                .ok_or_else(|| OrbitError::WorkNotFound(id.to_string()))
        }
    }

    pub fn delete_work(&self, id: &str) -> Result<(), OrbitError> {
        if self.context.work_persistence_type == PersistenceType::File {
            self.with_file_mutation(
                || {
                    let changed = self.context.work_file_store.disable_work(id)?;
                    if !changed {
                        return Err(OrbitError::WorkNotFound(id.to_string()));
                    }
                    Ok(())
                },
                OrbitEvent::WorkDisabled { id: id.to_string() },
            )
        } else {
            self.with_mutation(|tx| {
                let changed = tx.disable_work(id)?;
                if !changed {
                    return Err(OrbitError::WorkNotFound(id.to_string()));
                }
                Ok(((), OrbitEvent::WorkDisabled { id: id.to_string() }))
            })
        }
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
