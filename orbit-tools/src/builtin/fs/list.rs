use std::fs;

use orbit_types::{OrbitError, ToolSchema};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct FsListTool;

impl Tool for FsListTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "fs.list".to_string(),
            description: "List directory entries".to_string(),
        }
    }

    fn execute(&self, _ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| OrbitError::InvalidInput("missing `path`".to_string()))?;

        let entries = fs::read_dir(path)
            .map_err(|e| OrbitError::Io(e.to_string()))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path().to_string_lossy().to_string())
            .collect::<Vec<_>>();

        Ok(json!({"path": path, "entries": entries}))
    }
}
