use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitSemanticSearchTool;

impl Tool for OrbitSemanticSearchTool {
    fn schema(&self) -> ToolSchema {
        let mut parameters = vec![
            ToolParam {
                name: "query".to_string(),
                description: "Semantic query text.".to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            ToolParam {
                name: "limit".to_string(),
                description: "Maximum number of results to return.".to_string(),
                param_type: "number".to_string(),
                required: false,
            },
            ToolParam {
                name: "field".to_string(),
                description:
                    "Optional indexed task field filter, such as title, description, plan, acceptance, or execution_summary."
                        .to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "kind".to_string(),
                description: "Optional source kind filter. Phase 1 supports task.".to_string(),
                param_type: "string".to_string(),
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
            name: "orbit.semantic.search".to_string(),
            description:
                "Hybrid BM25 and cosine semantic search over indexed task fields. Read-only."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::SemanticSearch)
    }
}
