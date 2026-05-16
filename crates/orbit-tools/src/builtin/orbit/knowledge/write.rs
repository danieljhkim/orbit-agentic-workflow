use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use orbit_knowledge::commands::write as knowledge_write;
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct OrbitKnowledgeWriteTool;

impl Tool for OrbitKnowledgeWriteTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.graph.write".to_string(),
            description: "Use when you need a graph-aware edit. Prefer over grep when text search cannot safely target the node to change.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "selector".to_string(),
                    description: "File or symbol selector.".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "new_source".to_string(),
                    description: "Replacement source.".to_string(),
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
                    name: "start_line".to_string(),
                    description: "File-write start line.".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "end_line".to_string(),
                    description: "File-write end line.".to_string(),
                    param_type: "number".to_string(),
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
                ToolParam {
                    name: "knowledge_dir".to_string(),
                    description: "Override knowledge dir.".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let selector_str = crate::require_str(&input, "selector")?;
        let new_source = require_new_source(&input)?;
        let reason = optional_str(&input, "reason");
        let position_str = optional_str(&input, "position");
        let context = knowledge_write::MutationContext {
            graph: super::command_context(ctx, &input)?,
            workspace_root: super::resolve_workspace_root_with_override(ctx, &input)?,
        };

        Ok(knowledge_write::run(knowledge_write::MutationInput::Write {
            context,
            selector: selector_str,
            new_source,
            position: position_str,
            start_line: input.get("start_line").and_then(Value::as_u64),
            end_line: input.get("end_line").and_then(Value::as_u64),
            reason,
        })
        .map_err(super::knowledge_error_to_orbit)?
        .value)
    }
}

fn require_new_source(input: &Value) -> Result<String, OrbitError> {
    let value = input
        .get("new_source")
        .ok_or_else(|| OrbitError::InvalidInput("missing `new_source`".to_string()))?;
    let raw = value
        .as_str()
        .ok_or_else(|| OrbitError::InvalidInput("`new_source` must be a string".to_string()))?;
    if raw.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "`new_source` must not be empty".to_string(),
        ));
    }
    Ok(raw.to_string())
}

fn optional_str(input: &Value, key: &str) -> Option<String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}
