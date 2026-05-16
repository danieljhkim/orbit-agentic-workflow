use orbit_common::groundhog::FailureReport;
use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{GroundhogBuiltinAction, Tool, ToolContext};

pub struct OrbitGroundhogCheckpointFailureTool;

impl Tool for OrbitGroundhogCheckpointFailureTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.groundhog.checkpoint_failure".to_string(),
            description:
                "Publish a Groundhog checkpoint failure report to the active runner context"
                    .to_string(),
            parameters: vec![
                ToolParam {
                    name: "what_tried".to_string(),
                    description: "What the agent attempted during the failed day".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "what_happened".to_string(),
                    description: "Observed failure mode for the attempted day".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "next_attempt_plan".to_string(),
                    description: "Distilled plan for the next retry attempt".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::require_groundhog_fields(
            &input,
            "checkpoint_failure",
            &["what_tried", "what_happened", "next_attempt_plan"],
        )?;
        let input: FailureReport = serde_json::from_value(input).map_err(|error| {
            OrbitError::InvalidInput(format!(
                "groundhog checkpoint_failure input validation failed: {error}"
            ))
        })?;
        super::super::execute_groundhog_action(
            ctx,
            GroundhogBuiltinAction::CheckpointFailure,
            "checkpoint_failure",
            &input,
        )
    }
}
