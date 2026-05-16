use orbit_common::groundhog::SideEffect;
use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{GroundhogBuiltinAction, Tool, ToolContext};

pub struct OrbitGroundhogCheckpointSuccessTool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CheckpointSuccessInput {
    summary: String,
    side_effects: Vec<SideEffect>,
}

impl Tool for OrbitGroundhogCheckpointSuccessTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.groundhog.checkpoint_success".to_string(),
            description:
                "Publish a Groundhog checkpoint success summary to the active runner context"
                    .to_string(),
            parameters: vec![
                ToolParam {
                    name: "summary".to_string(),
                    description: "Distilled checkpoint summary that survives into the chronicle"
                        .to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "side_effects".to_string(),
                    description:
                        "Persisted side effects as an array of Groundhog `SideEffect` objects"
                            .to_string(),
                    param_type: "array".to_string(),
                    required: true,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::require_groundhog_fields(
            &input,
            "checkpoint_success",
            &["summary", "side_effects"],
        )?;
        let input: CheckpointSuccessInput = serde_json::from_value(input).map_err(|error| {
            OrbitError::InvalidInput(format!(
                "groundhog checkpoint_success input validation failed: {error}"
            ))
        })?;
        super::super::execute_groundhog_action(
            ctx,
            GroundhogBuiltinAction::CheckpointSuccess,
            "checkpoint_success",
            &input,
        )
    }
}
