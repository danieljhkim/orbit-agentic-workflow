use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitLearningPruneTool;

impl Tool for OrbitLearningPruneTool {
    fn schema(&self) -> ToolSchema {
        let parameters = vec![
            ToolParam {
                name: "stale_only".to_string(),
                description:
                    "When true, only report stale learnings without modifying state. Defaults to true; combine with `delete: true` to archive."
                        .to_string(),
                param_type: "boolean".to_string(),
                required: false,
            },
            ToolParam {
                name: "delete".to_string(),
                description:
                    "When true, archive every stale learning by flipping its status to `superseded` with `superseded_by: null`."
                        .to_string(),
                param_type: "boolean".to_string(),
                required: false,
            },
        ];
        ToolSchema {
            name: "orbit.learning.prune".to_string(),
            description:
                "Report or archive stale learnings per the §7.3 staleness rules. Returns `{ stale: string[], deleted: string[] }`."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::LearningPrune)
    }
}
