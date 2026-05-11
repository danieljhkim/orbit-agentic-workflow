use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitLearningAddTool;

impl Tool for OrbitLearningAddTool {
    fn schema(&self) -> ToolSchema {
        let mut parameters = vec![
            ToolParam {
                name: "summary".to_string(),
                description:
                    "One-line headline (≤ 280 chars). Required."
                        .to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            ToolParam {
                name: "scope".to_string(),
                description:
                    "Scope object: `{ paths?: string[], tags?: string[] }`. Matches are evaluated as `paths OR tags` per §3.3."
                        .to_string(),
                param_type: "object".to_string(),
                required: true,
            },
            ToolParam {
                name: "body".to_string(),
                description: "Long-form prose (markdown).".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "evidence".to_string(),
                description:
                    "Array of `{ kind: \"task\"|\"commit\"|\"external\", ref: string }` entries. Optional."
                        .to_string(),
                param_type: "object_list".to_string(),
                required: false,
            },
            ToolParam {
                name: "priority".to_string(),
                description:
                    "Optional priority (0–255). Used as the secondary ranking key in `search`; higher wins."
                        .to_string(),
                param_type: "integer".to_string(),
                required: false,
            },
        ];
        parameters.extend(super::super::model_identity_params());
        ToolSchema {
            name: "orbit.learning.add".to_string(),
            description:
                "Create an active project learning. Returns `{ id, created_at }` plus the full record."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::LearningAdd)
    }
}
