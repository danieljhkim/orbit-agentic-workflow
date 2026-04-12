use orbit_knowledge::{Selector, load_task_working_graph, save_task_working_graph};
use orbit_types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{Tool, ToolContext};

use super::knowledge_write::{
    initialize_working_graph, resolve_knowledge_dir, resolve_workspace_root_with_override,
};

pub struct OrbitKnowledgeAddTool;

impl Tool for OrbitKnowledgeAddTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.graph.add".to_string(),
            description: "Add a new symbol to a source file, writing to disk and updating the working graph. Rejects if the symbol already exists.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "selector".to_string(),
                    description: "Symbol selector like `symbol:path#name:kind` for the new symbol".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "source".to_string(),
                    description: "Full source code for the new symbol".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "position".to_string(),
                    description: "Optional anchor selector like `after:symbol:path#name:kind`. Inserts after the anchor. Omit to append before `#[cfg(test)]` or at end of file.".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "reason".to_string(),
                    description: "Optional reason, stored in version chain".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "workspace_path".to_string(),
                    description: "Optional workspace root override for branch/worktree targeting".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let selector_str = crate::require_str(&input, "selector")?;
        let source = crate::require_str(&input, "source")?;
        let reason = input
            .get("reason")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string());
        let position_str = input
            .get("position")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string());

        let selector: Selector = selector_str
            .parse()
            .map_err(|e| OrbitError::InvalidInput(format!("{e}")))?;

        if !matches!(selector, Selector::Symbol { .. }) {
            return Err(OrbitError::InvalidInput(
                "graph.add requires a symbol selector (symbol:path#name:kind)".to_string(),
            ));
        }

        let workspace_root_buf = resolve_workspace_root_with_override(ctx, &input)?;
        let workspace_root = workspace_root_buf.as_path();

        let knowledge_dir = resolve_knowledge_dir(ctx, &input)?;
        let mut working_graph =
            match load_task_working_graph(ctx.orbit_root.as_deref(), ctx.task_id.as_deref())? {
                Some(graph) => graph,
                None => initialize_working_graph(&knowledge_dir, &selector, workspace_root)?,
            };

        // Reject if the symbol already exists
        if working_graph.has_leaf(&selector) {
            let err = orbit_knowledge::WriteError::leaf_already_exists(&selector_str);
            return Err(OrbitError::Execution(
                serde_json::to_value(&err)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| format!("{err:?}")),
            ));
        }

        // Acquire lock
        let lock_owner = ctx
            .agent_name
            .as_deref()
            .or(ctx.task_id.as_deref())
            .unwrap_or("unknown");
        let lock_path = orbit_knowledge::lock::lock_store_path(&knowledge_dir);
        orbit_knowledge::lock::with_lock_store(&lock_path, |store| {
            store.lock(
                &selector_str,
                lock_owner,
                ctx.task_id.as_deref(),
                reason.as_deref().unwrap_or("adding"),
            )
        })
        .map_err(|e| OrbitError::Execution(format!("lock failed: {e}")))?;

        // Parse optional position
        let position_selector = parse_position(position_str.as_deref())?;

        let result = working_graph
            .insert_leaf(
                &selector,
                &source,
                position_selector.as_ref(),
                reason.as_deref(),
                workspace_root,
            )
            .map_err(|e| {
                serde_json::to_value(&e)
                    .map(|v| OrbitError::Execution(v.to_string()))
                    .unwrap_or_else(|_| OrbitError::Execution(format!("{e:?}")))
            })?;

        save_task_working_graph(
            ctx.orbit_root.as_deref(),
            ctx.task_id.as_deref(),
            &working_graph,
        )?;

        serde_json::to_value(result)
            .map_err(|e| OrbitError::Execution(format!("serialize result: {e}")))
    }
}

fn parse_position(position: Option<&str>) -> Result<Option<Selector>, OrbitError> {
    let Some(pos) = position else {
        return Ok(None);
    };
    let selector_str = pos.strip_prefix("after:").unwrap_or(pos);
    let selector: Selector = selector_str
        .parse()
        .map_err(|e| OrbitError::InvalidInput(format!("invalid position: {e}")))?;
    Ok(Some(selector))
}
