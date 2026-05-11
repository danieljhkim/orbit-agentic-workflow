use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitLearningSupersedeTool;

impl Tool for OrbitLearningSupersedeTool {
    fn schema(&self) -> ToolSchema {
        let parameters = vec![
            ToolParam {
                name: "id".to_string(),
                description: "ID of the learning being superseded.".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            ToolParam {
                name: "with".to_string(),
                description:
                    "ID of the replacement learning. Must differ from `id`. Both records must exist."
                        .to_string(),
                param_type: "string".to_string(),
                required: true,
            },
        ];
        ToolSchema {
            name: "orbit.learning.supersede".to_string(),
            description:
                "Mark `id` as superseded by `with`. Writes both pointers atomically and excludes the old record from default search."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::LearningSupersede)
    }
}
