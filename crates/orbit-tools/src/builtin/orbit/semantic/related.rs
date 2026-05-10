use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitSemanticRelatedTool;

impl Tool for OrbitSemanticRelatedTool {
    fn schema(&self) -> ToolSchema {
        let mut parameters = vec![
            ToolParam {
                name: "id".to_string(),
                description: "Task ID.".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            ToolParam {
                name: "limit".to_string(),
                description: "Maximum number of related tasks to return.".to_string(),
                param_type: "number".to_string(),
                required: false,
            },
            ToolParam {
                name: "embedding_model".to_string(),
                description: "Optional semantic embedding model alias, such as bge-small."
                    .to_string(),
                param_type: "string".to_string(),
                required: false,
            },
        ];
        parameters.extend(super::super::model_identity_params());
        ToolSchema {
            name: "orbit.semantic.related".to_string(),
            description: "Cosine-similarity neighbors for an indexed task. Read-only.".to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::SemanticRelated)
    }
}
