use std::fs;

use orbit_types::{OrbitError, ToolSchema};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct FsReadTool;

impl Tool for FsReadTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "fs.read".to_string(),
            description: "Read a UTF-8 text file from disk".to_string(),
        }
    }

    fn execute(&self, _ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| OrbitError::InvalidInput("missing `path`".to_string()))?;

        let content = fs::read_to_string(path).map_err(|e| OrbitError::Io(e.to_string()))?;

        Ok(json!({
            "path": path,
            "content": content,
        }))
    }
}
