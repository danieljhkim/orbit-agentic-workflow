use std::path::PathBuf;

use orbit_knowledge::service::deps::crate_dependencies;
use orbit_types::{OrbitError, ToolParam, ToolSchema};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct OrbitKnowledgeDepsTool;

impl Tool for OrbitKnowledgeDepsTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.graph.deps".to_string(),
            description: "List direct `orbit-*` dependencies per workspace crate, derived from Cargo.toml declarations.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "crate".to_string(),
                    description: "Optional workspace crate name to filter to (e.g. `orbit-engine`). Omit for all crates.".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "workspace_path".to_string(),
                    description: "Optional workspace root override; defaults to the active workspace.".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let crate_filter = super::optional_string(&input, "crate")?;
        let workspace_root = resolve_workspace_root(ctx, &input)?;

        let deps = crate_dependencies(&workspace_root, crate_filter.as_deref())
            .map_err(|e| OrbitError::Execution(format!("crate_dependencies: {e}")))?;

        Ok(json!({
            "workspace": workspace_root.display().to_string(),
            "crates": deps,
        }))
    }
}

fn resolve_workspace_root(ctx: &ToolContext, input: &Value) -> Result<PathBuf, OrbitError> {
    if let Some(ws) = input.get("workspace_path").and_then(Value::as_str)
        && !ws.trim().is_empty()
    {
        return Ok(PathBuf::from(ws));
    }
    ctx.workspace_root
        .clone()
        .ok_or_else(|| OrbitError::InvalidInput("workspace_root is required".to_string()))
}
