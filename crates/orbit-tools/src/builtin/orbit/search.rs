use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitSearchTool;

impl Tool for OrbitSearchTool {
    fn schema(&self) -> ToolSchema {
        let mut parameters = vec![
            ToolParam {
                name: "query".to_string(),
                description:
                    "Free-text query. Defaults to lexical matching unless semantic is true."
                        .to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "semantic".to_string(),
                description:
                    "Opt into hybrid BM25 + cosine ranking for task vectors; other kinds remain lexical."
                        .to_string(),
                param_type: "boolean".to_string(),
                required: false,
            },
            ToolParam {
                name: "related".to_string(),
                description:
                    "Task ID for cosine-neighbor lookup. Mutually exclusive with query and implies semantic mode."
                        .to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "kind".to_string(),
                description: "Corpus kind: task, doc, learning, adr, or all. Default: all."
                    .to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "limit".to_string(),
                description: "Maximum number of results. Default: 10.".to_string(),
                param_type: "integer".to_string(),
                required: false,
            },
            ToolParam {
                name: "field".to_string(),
                description:
                    "Optional indexed task field filter for semantic task search.".to_string(),
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
        parameters.extend(super::model_identity_params());
        ToolSchema {
            name: "orbit.search".to_string(),
            description:
                "Search tasks, docs, learnings, and ADRs. Semantic vector search currently applies to tasks only."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::execute_host_action(ctx, input, OrbitBuiltinAction::Search)
    }
}
