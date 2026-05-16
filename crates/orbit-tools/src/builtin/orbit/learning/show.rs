use orbit_common::types::{OrbitError, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitLearningShowTool;

impl Tool for OrbitLearningShowTool {
    fn schema(&self) -> ToolSchema {
        let parameters = super::super::orbit_id_params("learning");
        ToolSchema {
            name: "orbit.learning.show".to_string(),
            description:
                "Fetch a single learning by ID as JSON. Returns the full record including body and evidence."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::LearningShow)
    }
}
