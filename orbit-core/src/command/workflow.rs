use orbit_store::WorkflowInsertParams;
use orbit_types::{OrbitError, OrbitEvent, Workflow};
use serde_json::Value;

use crate::OrbitRuntime;

pub struct WorkflowAddParams {
    pub id: String,
    pub name: String,
    pub definition_json: Value,
}

impl OrbitRuntime {
    pub fn add_workflow(&self, params: WorkflowAddParams) -> Result<Workflow, OrbitError> {
        validate_workflow_params(&params)?;

        self.with_mutation(|tx| {
            let workflow = tx.insert_workflow(&WorkflowInsertParams {
                id: params.id.clone(),
                name: params.name.clone(),
                definition_json: params.definition_json.clone(),
            })?;
            Ok((
                workflow.clone(),
                OrbitEvent::WorkflowAdded { id: workflow.id },
            ))
        })
    }

    pub fn list_workflows(&self, include_inactive: bool) -> Result<Vec<Workflow>, OrbitError> {
        self.context.store.list_workflows(include_inactive)
    }

    pub fn show_workflow(&self, id: &str) -> Result<Workflow, OrbitError> {
        self.context
            .store
            .get_workflow(id)?
            .ok_or_else(|| OrbitError::WorkflowNotFound(id.to_string()))
    }

    pub fn delete_workflow(&self, id: &str) -> Result<(), OrbitError> {
        self.with_mutation(|tx| {
            let changed = tx.disable_workflow(id)?;
            if !changed {
                return Err(OrbitError::WorkflowNotFound(id.to_string()));
            }
            Ok(((), OrbitEvent::WorkflowDisabled { id: id.to_string() }))
        })
    }
}

fn validate_workflow_params(params: &WorkflowAddParams) -> Result<(), OrbitError> {
    if params.id.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "workflow id must not be empty".to_string(),
        ));
    }
    if params.name.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "workflow name must not be empty".to_string(),
        ));
    }
    if !params.definition_json.is_object() {
        return Err(OrbitError::InvalidInput(
            "workflow definition must be a JSON object".to_string(),
        ));
    }
    Ok(())
}
