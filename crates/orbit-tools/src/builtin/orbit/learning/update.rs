use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitLearningUpdateTool;

impl Tool for OrbitLearningUpdateTool {
    fn schema(&self) -> ToolSchema {
        let parameters = vec![
            ToolParam {
                name: "id".to_string(),
                description: "Learning ID to update.".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            ToolParam {
                name: "summary".to_string(),
                description: "Replace `summary` (≤ 280 chars).".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "scope".to_string(),
                description:
                    "Replace `scope` entirely: `{ paths?: string[], tags?: string[] }`.".to_string(),
                param_type: "object".to_string(),
                required: false,
            },
            ToolParam {
                name: "body".to_string(),
                description: "Replace `body`.".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "evidence".to_string(),
                description: "Replace `evidence` array.".to_string(),
                param_type: "object_list".to_string(),
                required: false,
            },
            ToolParam {
                name: "priority".to_string(),
                description:
                    "Replace `priority`. Pass an integer to set, `null` to clear, or omit to leave unchanged."
                        .to_string(),
                param_type: "integer".to_string(),
                required: false,
            },
        ];
        ToolSchema {
            name: "orbit.learning.update".to_string(),
            description:
                "Partial update of a learning. Rejects updates on superseded records (use `orbit.learning.supersede` instead)."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::LearningUpdate)
    }
}
