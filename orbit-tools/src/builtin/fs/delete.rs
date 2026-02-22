use std::fs;

use orbit_types::{OrbitError, ToolSchema};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct FsDeleteTool;

impl Tool for FsDeleteTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "fs.delete".to_string(),
            description: "Delete a file from disk".to_string(),
        }
    }

    fn execute(&self, _ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| OrbitError::InvalidInput("missing `path`".to_string()))?;

        fs::remove_file(path).map_err(|e| OrbitError::Io(e.to_string()))?;

        Ok(json!({"path": path, "deleted": true}))
    }
}
