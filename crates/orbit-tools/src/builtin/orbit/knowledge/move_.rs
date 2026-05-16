use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use orbit_knowledge::commands::write as knowledge_write;
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct OrbitKnowledgeMoveTool;

impl Tool for OrbitKnowledgeMoveTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.graph.move".to_string(),
            description: "Use when you need to move a symbol safely. Prefer over grep when copy-paste could desync source and graph state.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "selector".to_string(),
                    description: "Source symbol selector.".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "target_file".to_string(),
                    description: "Destination file path.".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "position".to_string(),
                    description: "Insert after this selector.".to_string(),
                    param_type: "string".to_string(),
                    required: false,
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
        let target_file = crate::require_str(&input, "target_file")?;
        let reason = input
            .get("reason")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned);
        let position = input
            .get("position")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned);

        let context = knowledge_write::MutationContext {
            graph: super::command_context(ctx, &input)?,
            workspace_root: super::resolve_workspace_root_with_override(ctx, &input)?,
        };

        Ok(knowledge_write::run(knowledge_write::MutationInput::Move {
            context,
            selector: selector_str,
            target_file,
            position,
            reason,
        })
        .map_err(super::knowledge_error_to_orbit)?
        .value)
    }
}
