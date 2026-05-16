use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use orbit_knowledge::commands::write as knowledge_write;
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct OrbitKnowledgeDeleteTool;

impl Tool for OrbitKnowledgeDeleteTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.graph.delete".to_string(),
            description: "Use when you need to delete a symbol safely. Prefer over grep when text delete could hit the wrong node.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "selector".to_string(),
                    description: "Symbol selector.".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "reason".to_string(),
                    description: "Optional change note.".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "workspace_path".to_string(),
                    description: "Override workspace root.".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let selector_str = crate::require_str(&input, "selector")?;
        let reason = input
            .get("reason")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned);

        let context = knowledge_write::MutationContext {
            graph: super::command_context(ctx, &input)?,
            workspace_root: super::resolve_workspace_root_with_override(ctx, &input)?,
        };

        Ok(
            knowledge_write::run(knowledge_write::MutationInput::Delete {
                context,
                selector: selector_str,
                reason,
            })
            .map_err(super::knowledge_error_to_orbit)?
            .value,
        )
    }
}
