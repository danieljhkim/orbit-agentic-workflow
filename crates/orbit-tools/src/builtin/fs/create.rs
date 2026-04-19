use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use orbit_types::{OrbitError, ToolParam, ToolSchema};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct FsCreateTool;

impl Tool for FsCreateTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "fs.create".to_string(),
            description: "Create a new UTF-8 text file; fails if the file already exists"
                .to_string(),
            parameters: vec![
                ToolParam {
                    name: "path".to_string(),
                    description: "Path to the file to create".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "content".to_string(),
                    description: "Optional UTF-8 text content to write".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let path_str = input
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| OrbitError::InvalidInput("missing `path`".to_string()))?;
        let content = input
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default();

        let canonical = super::check_workspace_boundary(ctx, Path::new(path_str))?;
        super::check_file_lock(ctx, &canonical)?;
        let policy = super::check_modify_policy(ctx, &canonical)?;

        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&canonical)
            .map_err(|e| OrbitError::Io(e.to_string()))?;
        file.write_all(content.as_bytes())
            .map_err(|e| OrbitError::Io(e.to_string()))?;
        super::emit_success(ctx, policy.as_ref())?;

        Ok(json!({
            "path": canonical.display().to_string(),
            "bytes_written": content.len(),
            "created": true,
        }))
    }
}
