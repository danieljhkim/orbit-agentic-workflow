use std::fs;
use std::path::Path;

use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct FsMkdirTool;

impl Tool for FsMkdirTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "fs.mkdir".to_string(),
            description: "Create a directory and any missing parent directories".to_string(),
            parameters: vec![ToolParam {
                name: "path".to_string(),
                description: "Path to the directory to create".to_string(),
                param_type: "string".to_string(),
                required: true,
            }],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| OrbitError::InvalidInput("missing `path`".to_string()))?;

        let canonical = super::check_workspace_boundary(ctx, Path::new(path))?;
        let policy = super::check_modify_policy(ctx, &canonical)?;
        fs::create_dir_all(&canonical).map_err(|e| OrbitError::Io(e.to_string()))?;
        super::emit_success(ctx, policy.as_ref())?;

        Ok(json!({
            "path": canonical.display().to_string(),
            "created": true,
        }))
    }
}
