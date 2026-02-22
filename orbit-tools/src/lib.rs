use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use orbit_exec::{ExecRequest, NoSandbox, run_process};
use orbit_types::{OrbitError, ToolSchema};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Default)]
pub struct ToolContext {
    pub cwd: Option<String>,
}

pub trait Tool: Send + Sync {
    fn schema(&self) -> ToolSchema;
    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError>;
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let schema = tool.schema();
        self.tools.insert(schema.name, Arc::new(tool));
    }

    pub fn register_builtins(&mut self) {
        self.register(FsReadTool);
        self.register(FsWriteTool);
        self.register(ProcSpawnTool);
        self.register(TimeNowTool);
    }

    pub fn execute(
        &self,
        name: &str,
        ctx: &ToolContext,
        input: Value,
    ) -> Result<Value, OrbitError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| OrbitError::ToolNotFound(name.to_string()))?;
        tool.execute(ctx, input)
    }

    pub fn schemas(&self) -> Vec<ToolSchema> {
        self.tools.values().map(|t| t.schema()).collect()
    }
}

struct FsReadTool;

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

struct FsWriteTool;

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

struct ProcSpawnTool;

impl Tool for ProcSpawnTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "proc.spawn".to_string(),
            description: "Spawn a process with timeout and capture output".to_string(),
        }
    }

    fn execute(&self, _ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let program = input
            .get("program")
            .and_then(Value::as_str)
            .ok_or_else(|| OrbitError::InvalidInput("missing `program`".to_string()))?
            .to_string();

        let args = input
            .get("args")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let timeout_ms = input.get("timeout_ms").and_then(Value::as_u64);

        let exec_result = run_process(
            &ExecRequest {
                program,
                args,
                timeout_ms,
            },
            &NoSandbox,
        )?;

        serde_json::to_value(exec_result)
            .map_err(|e| OrbitError::Execution(format!("serialize exec result: {e}")))
    }
}

struct TimeNowTool;

impl Tool for TimeNowTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "time.now".to_string(),
            description: "Return current UTC timestamp".to_string(),
        }
    }

    fn execute(&self, _ctx: &ToolContext, _input: Value) -> Result<Value, OrbitError> {
        Ok(json!({
            "now": chrono::Utc::now().to_rfc3339(),
        }))
    }
}

pub fn map_input_from_pairs(pairs: impl IntoIterator<Item = (String, String)>) -> Value {
    let mut map = Map::new();
    for (key, value) in pairs {
        map.insert(key, Value::String(value));
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn registry_contains_expected_builtins() {
        let mut registry = ToolRegistry::new();
        registry.register_builtins();
        let schemas = registry.schemas();

        let names = schemas.into_iter().map(|s| s.name).collect::<Vec<_>>();
        assert!(names.contains(&"fs.read".to_string()));
        assert!(names.contains(&"fs.write".to_string()));
        assert!(names.contains(&"proc.spawn".to_string()));
        assert!(names.contains(&"time.now".to_string()));
    }

    #[test]
    fn fs_read_returns_file_contents() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("note.txt");
        fs::write(&path, "hello").expect("write file");

        let mut registry = ToolRegistry::new();
        registry.register_builtins();

        let output = registry
            .execute(
                "fs.read",
                &ToolContext::default(),
                json!({"path": path.to_string_lossy()}),
            )
            .expect("tool executes");

        assert_eq!(output["content"], "hello");
    }
}
