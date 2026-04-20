use orbit_common::types::{OrbitError, TaskPlanCheckpoint, ToolParam, ToolSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{GroundhogBuiltinAction, Tool, ToolContext};

pub struct OrbitGroundhogCheckpointDeviateTool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CheckpointDeviateInput {
    new_checkpoint_spec: TaskPlanCheckpoint,
    rationale: String,
}

impl Tool for OrbitGroundhogCheckpointDeviateTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.groundhog.checkpoint_deviate".to_string(),
            description:
                "Push a new Groundhog checkpoint onto the active deviation stack".to_string(),
            parameters: vec![
                ToolParam {
                    name: "new_checkpoint_spec".to_string(),
                    description:
                        "Replacement checkpoint object matching the shared task-plan checkpoint schema"
                            .to_string(),
                    param_type: "object".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "rationale".to_string(),
                    description: "Why the current checkpoint needs a deviation".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::require_groundhog_fields(
            &input,
            "checkpoint_deviate",
            &["new_checkpoint_spec", "rationale"],
        )?;
        let input: CheckpointDeviateInput = serde_json::from_value(input).map_err(|error| {
            OrbitError::InvalidInput(format!(
                "groundhog checkpoint_deviate input validation failed: {error}"
            ))
        })?;
        super::execute_groundhog_action(
            ctx,
            GroundhogBuiltinAction::CheckpointDeviate,
            "checkpoint_deviate",
            &input,
        )
    }
}
