use std::fs;

use orbit_types::{OrbitError, ToolSchema};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct FsWriteTool;

impl Tool for FsWriteTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "fs.write".to_string(),
            description: "Write UTF-8 text content to disk".to_string(),
        }
    }

    fn execute(&self, _ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| OrbitError::InvalidInput("missing `path`".to_string()))?;
        let content = input
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| OrbitError::InvalidInput("missing `content`".to_string()))?;

        fs::write(path, content).map_err(|e| OrbitError::Io(e.to_string()))?;

        Ok(json!({
            "path": path,
            "bytes_written": content.len(),
        }))
    }
}
